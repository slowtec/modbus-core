// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus RTU

use super::*;
use crate::SlaveId;
use byteorder::{BigEndian, ByteOrder};

pub mod client;
pub mod server;
pub use crate::frame::rtu::*;

// [MODBUS over Serial Line Specification and Implementation Guide V1.02](http://modbus.org/docs/Modbus_over_serial_line_V1_02.pdf), page 13
// "The maximum size of a MODBUS RTU frame is 256 bytes."
pub const MAX_FRAME_LEN: usize = 256;

/// An extracted RTU PDU frame.
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedFrame<'a> {
    pub slave: SlaveId,
    pub pdu: &'a [u8],
}

/// The location of all bytes that belong to the frame.
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLocation {
    /// The index where the frame starts
    pub start: usize,
    /// Number of bytes that belong to the frame
    pub size: usize,
}

/// Decode RTU PDU frames from a buffer.
pub fn decode(
    decoder_type: DecoderType,
    buf: &[u8],
) -> Result<Option<(DecodedFrame<'_>, FrameLocation)>> {
    use DecoderType::{Request, Response};
    let mut drop_cnt = 0;

    if buf.is_empty() {
        return Err(Error::BufferSize);
    }

    loop {
        let mut retry = false;
        if drop_cnt + 1 >= buf.len() {
            return Ok(None);
        }
        let raw_frame = &buf[drop_cnt..];
        let res = match decoder_type {
            Request => request_pdu_len(raw_frame),
            Response => response_pdu_len(raw_frame),
        }
        .and_then(|pdu_len| {
            retry = false;
            let Some(pdu_len) = pdu_len else {
                // Incomplete frame
                return Ok(None);
            };
            extract_frame(raw_frame, pdu_len).map(|x| {
                x.map(|res| {
                    let frame_location = FrameLocation {
                        start: drop_cnt,
                        size: pdu_len + 3, // TODO: use 'const FOO:usize = 3;'
                    };
                    (res, frame_location)
                })
            })
        })
        .or_else(|err| {
            if drop_cnt + 1 >= MAX_FRAME_LEN {
                #[cfg(feature = "log")]
                log::error!(
                    "Giving up to decode frame after dropping {drop_cnt} byte(s): {:X?}",
                    &buf[0..drop_cnt]
                );
                return Err(err);
            }
            #[cfg(feature = "log")]
            log::warn!(
                "Failed to decode {} frame: {err}",
                match decoder_type {
                    Request => "request",
                    Response => "response",
                }
            );
            drop_cnt += 1;
            retry = true;
            Ok(None)
        });

        if !retry {
            return res;
        }
    }
}

/// Extract a PDU frame out of a buffer.
#[allow(clippy::similar_names)]
pub fn extract_frame(buf: &[u8], pdu_len: usize) -> Result<Option<DecodedFrame<'_>>> {
    if buf.is_empty() {
        return Err(Error::BufferSize);
    }

    let adu_len = 1 + pdu_len;
    if buf.len() >= adu_len + 2 {
        let (adu_buf, buf) = buf.split_at(adu_len);
        let (crc_buf, _) = buf.split_at(2);
        // Read trailing CRC and verify ADU
        let expected_crc = BigEndian::read_u16(crc_buf);
        let actual_crc = crc16(adu_buf);
        if expected_crc != actual_crc {
            return Err(Error::Crc(expected_crc, actual_crc));
        }
        let (slave_id, pdu_data) = adu_buf.split_at(1);
        let slave_id = slave_id[0];
        return Ok(Some(DecodedFrame {
            slave: slave_id,
            pdu: pdu_data,
        }));
    }
    // Incomplete frame
    Ok(None)
}

/// Calculate the CRC (Cyclic Redundancy Check) sum.
#[must_use]
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for x in data {
        crc ^= u16::from(*x);
        for _ in 0..8 {
            // if we followed clippy's suggestion to move out the crc >>= 1, the condition may not be met any more
            // the recommended action therefore makes no sense and it is better to allow this lint
            #[allow(clippy::branches_sharing_code)]
            if (crc & 0x0001) != 0 {
                crc >>= 1;
                crc ^= 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc.rotate_right(8)
}

/// Extract the PDU length out of the ADU request buffer.
pub const fn request_pdu_len(adu_buf: &[u8]) -> Result<Option<usize>> {
    if adu_buf.len() < 2 {
        return Ok(None);
    }
    let fn_code = adu_buf[1];
    let len = match fn_code {
        0x01..=0x06 => Some(5),
        0x07 | 0x0B | 0x0C | 0x11 => Some(1),
        0x0F | 0x10 => {
            if adu_buf.len() > 4 {
                Some(6 + adu_buf[4] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x16 => Some(7),
        0x18 => Some(3),
        0x17 => {
            if adu_buf.len() > 10 {
                Some(10 + adu_buf[10] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        _ => {
            return Err(Error::FnCode(fn_code));
        }
    };
    Ok(len)
}

/// Extract the PDU length out of the ADU response buffer.
pub fn response_pdu_len(adu_buf: &[u8]) -> Result<Option<usize>> {
    if adu_buf.len() < 2 {
        return Ok(None);
    }
    let fn_code = adu_buf[1];
    let len = match fn_code {
        0x01..=0x04 | 0x0C | 0x17 => {
            if adu_buf.len() > 2 {
                Some(2 + adu_buf[2] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x05 | 0x06 | 0x0B | 0x0F | 0x10 => Some(5),
        0x07 | 0x81..=0xAB => Some(2),
        0x16 => Some(7),
        0x18 => {
            if adu_buf.len() > 3 {
                Some(3 + BigEndian::read_u16(&adu_buf[2..=3]) as usize)
            } else {
                // incomplete frame
                None
            }
        }
        _ => return Err(Error::FnCode(fn_code)),
    };
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_crc16() {
        let msg = &[0x01, 0x03, 0x08, 0x2B, 0x00, 0x02];
        assert_eq!(crc16(msg), 0xB663);

        let msg = &[0x01, 0x03, 0x04, 0x00, 0x20, 0x00, 0x00];
        assert_eq!(crc16(msg), 0xFBF9);
    }

    #[test]
    fn test_request_pdu_len() {
        let buf = &mut [0x66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(request_pdu_len(buf).is_err());

        buf[1] = 0x01;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x02;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x03;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x04;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x05;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x06;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x07;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        // TODO: 0x08

        buf[1] = 0x0B;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        buf[1] = 0x0C;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        buf[1] = 0x0F;
        buf[4] = 99;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(105));

        buf[1] = 0x10;
        buf[4] = 99;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(105));

        buf[1] = 0x11;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        // TODO: 0x14

        // TODO: 0x15

        buf[1] = 0x16;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(7));

        buf[1] = 0x17;
        buf[10] = 99; // write byte count
        assert_eq!(request_pdu_len(buf).unwrap(), Some(109));

        buf[1] = 0x18;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(3));

        // TODO: 0x2B
    }

    #[test]
    fn test_get_response_pdu_len() {
        let buf = &mut [0x66, 0x01, 99];
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        let buf = &mut [0x66, 0x00, 99, 0x00];
        assert_eq!(response_pdu_len(buf).err().unwrap(), Error::FnCode(0));

        let buf = &mut [0x66, 0xee, 99, 0x00];
        assert_eq!(response_pdu_len(buf).err().unwrap(), Error::FnCode(0xee));

        buf[1] = 0x01;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[1] = 0x02;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[1] = 0x03;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[1] = 0x04;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[1] = 0x05;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x06;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x07;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(2));

        // TODO: 0x08

        buf[1] = 0x0B;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x0C;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[1] = 0x0F;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[1] = 0x10;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        // TODO: 0x11

        // TODO: 0x14

        // TODO: 0x15

        buf[1] = 0x16;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(7));

        buf[1] = 0x17;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[1] = 0x18;
        buf[2] = 0x01; // byte count Hi
        buf[3] = 0x00; // byte count Lo
        assert_eq!(response_pdu_len(buf).unwrap(), Some(259));

        // TODO: 0x2B

        for i in 0x81..0xAB {
            buf[1] = i;
            assert_eq!(response_pdu_len(buf).unwrap(), Some(2));
        }
    }

    mod frame_decoder {

        use super::*;

        #[test]
        fn extract_partly_received_rtu_frame() {
            let buf = &[
                0x12, // slave address
                0x02, // function code
                0x03, // byte count
                0x00, // data
                0x00, // data
                0x00, // data
                0x00, // CRC first byte
                      // missing crc second byte
            ];
            let pdu_len = request_pdu_len(buf).unwrap().unwrap();
            let res = extract_frame(buf, pdu_len).unwrap();
            assert!(res.is_none());
        }

        #[test]
        fn extract_usual_rtu_response_frame() {
            let buf = &[
                0x01, // slave address
                0x03, // function code
                0x04, // byte count
                0x89, //
                0x02, //
                0x42, //
                0xC7, //
                0x00, // crc
                0x9D, // crc
                0x03, // -- start of next frame
            ];
            let pdu_len = response_pdu_len(buf).unwrap().unwrap();
            let DecodedFrame { slave, pdu } = extract_frame(buf, pdu_len).unwrap().unwrap();
            assert_eq!(slave, 0x01);
            assert_eq!(pdu.len(), 6);
        }

        #[test]
        fn decode_rtu_response_drop_invalid_bytes() {
            let buf = &[
                0x42, // dropped byte
                0x43, // dropped byte
                0x01, // slave address
                0x03, // function code
                0x04, // byte count
                0x89, //
                0x02, //
                0x42, //
                0xC7, //
                0x00, // crc
                0x9D, // crc
                0x00,
            ];
            let (frame, location) = decode(DecoderType::Response, buf).unwrap().unwrap();
            assert_eq!(frame.slave, 0x01);
            assert_eq!(frame.pdu.len(), 6);
            assert_eq!(location.start, 2);
            assert_eq!(location.size, 9);
        }

        #[test]
        fn decode_rtu_response_with_max_drops() {
            let buf = &[0x42; 10];
            assert!(decode(DecoderType::Response, buf).unwrap().is_none());

            let buf = &mut [0x42; MAX_FRAME_LEN * 2];
            buf[256] = 0x01; // slave address
            buf[257] = 0x03; // function code
            buf[258] = 0x04; // byte count
            buf[259] = 0x89; //
            buf[260] = 0x02; //
            buf[261] = 0x42; //
            buf[262] = 0xC7; //
            buf[263] = 0x00; // crc
            buf[264] = 0x9D; // crc
            assert!(decode(DecoderType::Response, buf).is_err());
        }
    }
}
