//! Modbus RTU

use super::*;
use byteorder::{BigEndian, ByteOrder};

type Result<T> = core::result::Result<T, Error>;

/// Slave ID
pub type SlaveId = u8;

/// Extract a PDU frame out of a buffer.
pub fn extract_frame(buf: &[u8], pdu_len: usize) -> Result<Option<(SlaveId, &[u8])>> {
    let adu_len = 1 + pdu_len;
    if buf.len() >= adu_len + 2 {
        let (adu_buf, buf) = buf.split_at(adu_len);
        let (crc_buf, _) = buf.split_at(2);
        // Read trailing CRC and verify ADU
        let expected_crc = BigEndian::read_u16(&crc_buf);
        let actual_crc = crc16(adu_buf);
        if expected_crc != actual_crc {
            return Err(Error::Crc(expected_crc, actual_crc));
        }
        let (slave_id, pdu_data) = adu_buf.split_at(1);
        let slave_id = slave_id[0];
        return Ok(Some((slave_id, pdu_data)));
    }
    // Incomplete frame
    Ok(None)
}

/// Calculate the CRC (Cyclic Redundancy Check) sum.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for x in data {
        crc ^= u16::from(*x);
        for _ in 0..8 {
            if (crc & 0x0001) != 0 {
                crc >>= 1;
                crc ^= 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    (crc << 8 | crc >> 8)
}

/// Extract the PDU length out of the ADU request buffer.
pub fn request_pdu_len(adu_buf: &[u8]) -> Result<Option<usize>> {
    if adu_buf.len() < 2 {
        return Ok(None);
    }
    let fn_code = adu_buf[1];
    let len = match fn_code {
        0x01...0x06 => Some(5),
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
        0x01...0x04 | 0x0C | 0x17 => {
            if adu_buf.len() > 2 {
                Some(2 + adu_buf[2] as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x05 | 0x06 | 0x0B | 0x0F | 0x10 => Some(5),
        0x07 => Some(2),
        0x16 => Some(7),
        0x18 => {
            if adu_buf.len() > 3 {
                Some(3 + BigEndian::read_u16(&adu_buf[2..=3]) as usize)
            } else {
                // incomplete frame
                None
            }
        }
        0x81...0xAB => Some(2),
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
            let (id, res) = extract_frame(buf, pdu_len).unwrap().unwrap();
            assert_eq!(id, 0x01);
            assert_eq!(res.len(), 6);
        }
    }
}
