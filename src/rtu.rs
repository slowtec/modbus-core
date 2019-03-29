//! Modbus RTU

use super::*;
use byteorder::{BigEndian, ByteOrder};

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
pub fn request_pdu_len(adu_buf: &[u8]) -> Result<Option<usize>, Error> {
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
            return Err(Error::FnCode);
        }
    };
    Ok(len)
}

/// Extract the PDU length out of the ADU response buffer.
pub fn response_pdu_len(adu_buf: &[u8]) -> Result<Option<usize>, Error> {
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
        _ => return Err(Error::FnCode),
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
        //let mut buf = BytesMut::new();

        //buf.extend_from_slice(&[0x66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
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
        assert_eq!(response_pdu_len(buf).err().unwrap(), Error::FnCode);

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
}
