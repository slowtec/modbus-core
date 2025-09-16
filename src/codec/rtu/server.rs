// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus RTU server (slave) specific functions.
use super::*;

/// Decode an RTU request.
pub fn decode_request(buf: &[u8]) -> Result<Option<RequestAdu<'_>>> {
    if buf.is_empty() {
        return Ok(None);
    }
    decode(DecoderType::Request, buf)
        .and_then(|frame| {
            let Some((DecodedFrame { slave, pdu }, _frame_pos)) = frame else {
                return Ok(None);
            };
            let hdr = Header { slave };
            // Decoding of the PDU should are unlikely to fail due
            // to transmission errors, because the frame's bytes
            // have already been verified with the CRC.
            Request::try_from(pdu)
                .map(RequestPdu)
                .map(|pdu| Some(RequestAdu { hdr, pdu }))
                .inspect_err(|&err| {
                    // Unrecoverable error
                    log::error!("Failed to decode request PDU: {err}");
                })
        })
        .map_err(|_| {
            // Decoding the transport frame is non-destructive and must
            // never fail!
            unreachable!();
        })
}

/// Encode an RTU response.
pub fn encode_response(adu: ResponseAdu<'_>, buf: &mut [u8]) -> Result<usize> {
    let ResponseAdu { hdr, pdu } = adu;
    if buf.len() < 2 {
        return Err(Error::BufferSize);
    }
    let len = pdu.encode(&mut buf[1..])?;
    if buf.len() < len + 3 {
        return Err(Error::BufferSize);
    }
    buf[0] = hdr.slave;
    let crc = crc16(&buf[0..=len]);
    BigEndian::write_u16(&mut buf[len + 1..], crc);
    Ok(len + 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_empty_request() {
        let req = decode_request(&[]).unwrap();
        assert!(req.is_none());
    }

    #[test]
    fn decode_partly_received_request() {
        let buf = &[
            0x12, // slave address
            0x16, // function code
        ];
        let req = decode_request(buf).unwrap();
        assert!(req.is_none());
    }

    #[test]
    fn decode_write_single_register_request() {
        let buf = &[
            0x12, // slave address
            0x06, // function code
            0x22, // addr
            0x22, // addr
            0xAB, // value
            0xCD, // value
            0x9F, // crc
            0xBE, // crc
        ];
        let adu = decode_request(buf).unwrap().unwrap();
        let RequestAdu { hdr, pdu } = adu;
        let RequestPdu(pdu) = pdu;
        assert_eq!(hdr.slave, 0x12);
        assert_eq!(FunctionCode::from(pdu), FunctionCode::WriteSingleRegister);
    }

    #[test]
    fn encode_write_single_register_response() {
        let adu = ResponseAdu {
            hdr: Header { slave: 0x12 },
            pdu: ResponsePdu(Ok(Response::WriteSingleRegister(0x2222, 0xABCD))),
        };
        let buf = &mut [0; 100];
        let len = encode_response(adu, buf).unwrap();
        assert_eq!(len, 8);
        assert_eq!(buf[0], 0x12);
        assert_eq!(buf[1], 0x06);
        assert_eq!(buf[2], 0x22);
        assert_eq!(buf[3], 0x22);
        assert_eq!(buf[4], 0xAB);
        assert_eq!(buf[5], 0xCD);
        assert_eq!(buf[6], 0x9F);
        assert_eq!(buf[7], 0xBE);
    }
}
