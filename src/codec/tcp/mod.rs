//! Modbus RTU

use super::*;
use byteorder::{BigEndian, ByteOrder};

pub mod server;
pub use crate::frame::tcp::*;

// [MODBUS MESSAGING ON TCP/IP IMPLEMENTATION GUIDE V1.0b](http://modbus.org/docs/Modbus_Messaging_Implementation_Guide_V1_0b.pdf), page 18
// "a MODBUS request needs a maximum of 256 bytes + the MBAP header size"
const MAX_FRAME_LEN: usize = 256;

/// An extracted TCP PDU frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedFrame<'a> {
    pub transaction_id: TransactionId,
    pub unit_id: UnitId,
    pub pdu: &'a [u8],
}

/// The location of all bytes that belong to the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLocation {
    /// The index where the frame starts
    pub start: usize,
    /// Number of bytes that belong to the frame
    pub size: usize,
}

/// Decode TCP PDU frames from a buffer.
pub fn decode(
    decoder_type: DecoderType,
    buf: &[u8],
) -> Result<Option<(DecodedFrame, FrameLocation)>> {
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
            if let Some(pdu_len) = pdu_len {
                extract_frame(raw_frame, pdu_len).map(|x| {
                    x.map(|res| {
                        (
                            res,
                            FrameLocation {
                                start: drop_cnt,
                                size: pdu_len + 7,
                            },
                        )
                    })
                })
            } else {
                // Incomplete frame
                Ok(None)
            }
        })
        .or_else(|err| {
            let pdu_type = match decoder_type {
                Request => "request",
                Response => "response",
            };
            if drop_cnt + 1 >= MAX_FRAME_LEN {
                log::error!(
                    "Giving up to decode frame after dropping {drop_cnt} byte(s): {:X?}",
                    &buf[0..drop_cnt]
                );
                return Err(err);
            }
            log::warn!("Failed to decode {pdu_type} frame: {err}");
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
pub fn extract_frame(buf: &[u8], pdu_len: usize) -> Result<Option<DecodedFrame>> {
    if buf.is_empty() {
        return Err(Error::BufferSize);
    }
    let adu_len = 7 + pdu_len;
    if buf.len() >= adu_len {
        let (adu_buf, _next_frame) = buf.split_at(adu_len);
        let (adu_buf, pdu_data) = adu_buf.split_at(7);
        let (transaction_buf, adu_buf) = adu_buf.split_at(2);
        let (protocol_buf, adu_buf) = adu_buf.split_at(2);
        let (length_buf, adu_buf) = adu_buf.split_at(2);
        let protocol_id = BigEndian::read_u16(protocol_buf);
        if protocol_id != 0 {
            return Err(Error::ProtocolNotModbus(protocol_id));
        }
        let transaction = BigEndian::read_u16(transaction_buf);
        let m_length = BigEndian::read_u16(length_buf) as usize;
        let unit = adu_buf[0];
        if m_length != pdu_len + 1 {
            return Err(Error::LengthMismatch(m_length, pdu_len + 1));
        }
        return Ok(Some(DecodedFrame {
            transaction_id: transaction,
            unit_id: unit,
            pdu: pdu_data,
        }));
    }
    // Incomplete frame
    Ok(None)
}

/// Extract the PDU length out of the ADU request buffer.
pub const fn request_pdu_len(adu_buf: &[u8]) -> Result<Option<usize>> {
    if adu_buf.len() < 8 {
        return Ok(None);
    }
    let fn_code = adu_buf[7];
    let len = match fn_code {
        0x01..=0x06 => Some(5),
        0x07 | 0x0B | 0x0C | 0x11 => Some(1),
        0x0F | 0x10 => {
            if adu_buf.len() > 10 {
                Some(6 + adu_buf[12] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x16 => Some(7),
        0x18 => Some(3),
        0x17 => {
            if adu_buf.len() > 16 {
                Some(10 + adu_buf[16] as usize)
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
    if adu_buf.len() < 8 {
        return Ok(None);
    }
    let fn_code = adu_buf[7];
    let len = match fn_code {
        0x01..=0x04 | 0x0C | 0x17 => {
            if adu_buf.len() > 8 {
                Some(2 + adu_buf[8] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x05 | 0x06 | 0x0B | 0x0F | 0x10 => Some(5),
        0x07 | 0x81..=0xAB => Some(2),
        0x16 => Some(7),
        0x18 => {
            if adu_buf.len() > 9 {
                Some(3 + BigEndian::read_u16(&adu_buf[8..=9]) as usize)
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
    fn test_request_pdu_len() {
        let buf = &mut [0x66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(request_pdu_len(buf).is_err());

        buf[7] = 0x01;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x02;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x03;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x04;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x05;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x06;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x07;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        // TODO: 0x08

        buf[7] = 0x0B;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        buf[7] = 0x0C;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        buf[7] = 0x0F;
        buf[12] = 99;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(105));

        buf[7] = 0x10;
        buf[12] = 99;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(105));

        buf[7] = 0x11;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(1));

        // TODO: 0x14

        // TODO: 0x15

        buf[7] = 0x16;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(7));

        buf[7] = 0x17;
        buf[16] = 99; // write byte count
        assert_eq!(request_pdu_len(buf).unwrap(), Some(109));

        buf[7] = 0x18;
        assert_eq!(request_pdu_len(buf).unwrap(), Some(3));

        // TODO: 0x2B
    }

    #[test]
    fn test_get_response_pdu_len() {
        let buf = &mut [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x01, 99];
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        let buf = &mut [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x00, 99, 0x00];
        assert_eq!(response_pdu_len(buf).err().unwrap(), Error::FnCode(0));

        let buf = &mut [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0xee, 99, 0x00];
        assert_eq!(response_pdu_len(buf).err().unwrap(), Error::FnCode(0xee));

        buf[7] = 0x01;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[7] = 0x02;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[7] = 0x03;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[7] = 0x04;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[7] = 0x05;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x06;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x07;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(2));

        // TODO: 0x08

        buf[7] = 0x0B;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x0C;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[7] = 0x0F;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        buf[7] = 0x10;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(5));

        // TODO: 0x11

        // TODO: 0x14

        // TODO: 0x15

        buf[7] = 0x16;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(7));

        buf[7] = 0x17;
        assert_eq!(response_pdu_len(buf).unwrap(), Some(101));

        buf[7] = 0x18;
        buf[8] = 0x01; // byte count Hi
        buf[9] = 0x00; // byte count Lo
        assert_eq!(response_pdu_len(buf).unwrap(), Some(259));

        // TODO: 0x2B

        for i in 0x81..0xAB {
            buf[7] = i;
            assert_eq!(response_pdu_len(buf).unwrap(), Some(2));
        }
    }

    mod frame_decoder {

        use super::*;

        #[test]
        fn extract_partly_received_tcp_frame() {
            let buf = &[
                0x01, // transaction id
                0x02, // transaction id
                0x00, // protocol id
                0x00, // protocol id
                0x00, // length
                0x06, // length
                0x01, // unit id
                0x02, // function code
                0x03, // byte count
                0x00, // data
                0x00, // data
                      // missing final data byte
            ];
            let pdu_len = request_pdu_len(buf).unwrap().unwrap();
            let res = extract_frame(buf, pdu_len).unwrap();
            assert!(res.is_none());
        }

        #[test]
        fn extract_usual_tcp_response_frame() {
            let buf = &[
                0x01, // transaction id
                0x02, // transaction id
                0x00, // protocol id
                0x00, // protocol id
                0x00, // length
                0x07, // length
                0x01, // unit id
                0x03, // function code
                0x04, // byte count
                0x89, //
                0x02, //
                0x42, //
                0xC7, //
                0x03, // -- start of next frame
            ];
            let pdu_len = response_pdu_len(buf).unwrap().unwrap();
            let DecodedFrame {
                transaction_id,
                unit_id,
                pdu,
            } = extract_frame(buf, pdu_len).unwrap().unwrap();
            assert_eq!(transaction_id, 258);
            assert_eq!(unit_id, 0x01);
            assert_eq!(pdu.len(), 6);
        }

        #[test]
        fn decode_tcp_response_drop_invalid_bytes() {
            let buf = &[
                0x42, // dropped byte
                0x43, // dropped byte
                0x01, // transaction id
                0x02, // transaction id
                0x00, // protocol id
                0x00, // protocol id
                0x00, // length
                0x07, // length
                0x01, // unit id
                0x03, // function code
                0x04, // byte count
                0x89, //
                0x02, //
                0x42, //
                0xC7, //
                0x00, //next frame
            ];
            let (frame, location) = decode(DecoderType::Response, buf).unwrap().unwrap();
            assert_eq!(frame.transaction_id, 258);
            assert_eq!(frame.unit_id, 0x01);
            assert_eq!(frame.pdu.len(), 6);
            assert_eq!(location.start, 2);
            assert_eq!(location.size, 13);
        }

        #[test]
        fn decode_tcp_response_with_max_drops() {
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
            assert!(decode(DecoderType::Response, buf).is_err());
        }
    }
}
