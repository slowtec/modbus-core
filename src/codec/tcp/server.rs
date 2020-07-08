//! Modbus TCP server (slave) specific functions.
use super::*;

/// Decode an TCP request.
pub fn decode_request(buf: &[u8]) -> Result<Option<RequestAdu>> {
    decode(DecoderType::Request, buf)
        .and_then(|frame| {
            if let Some((
                DecodedFrame {
                    transaction_id,
                    unit_id,
                    pdu,
                },
                _frame_pos,
            )) = frame
            {
                let hdr = Header {
                    transaction_id,
                    unit_id,
                };
                // Decoding of the PDU should are unlikely to fail due
                // to transmission errors, because the frame's bytes
                // have already been verified at the TCP level.
                Request::try_from(pdu)
                    .map(RequestPdu)
                    .map(|pdu| Some(RequestAdu { hdr, pdu }))
                    .map_err(|err| {
                        // Unrecoverable error
                        error!("Failed to decode request PDU: {}", err);
                        err
                    })
            } else {
                Ok(None)
            }
        })
        .or_else(|error| Err(error))
}

/// Encode an TCP response.
pub fn encode_response(adu: ResponseAdu, buf: &mut [u8]) -> Result<usize> {
    let ResponseAdu { hdr, pdu } = adu;
    if buf.len() < 2 {
        return Err(Error::BufferSize);
    }
    BigEndian::write_u16(&mut buf[0..2], hdr.transaction_id);
    BigEndian::write_u16(&mut buf[2..4], 0); //MODBUS Protocol
    buf[6] = hdr.unit_id;
    let len = pdu.encode(&mut buf[7..])?;
    if buf.len() < len + 7 {
        return Err(Error::BufferSize);
    }
    BigEndian::write_u16(&mut buf[4..6], (len + 1) as u16);

    Ok(len + 7)
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
            0x00, // garbage
            0x01, //
        ];
        let req = decode_request(buf).unwrap();
        assert!(req.is_none());
    }

    #[test]
    fn decode_write_single_register_request() {
        let buf = &[
            0x00, // Transaction id
            0x2a, // Transaction id
            0x00, // Protocol id
            0x00, // Protocol id
            0x00, // length
            0x06, // length
            0x12, // unit id
            0x06, // function code
            0x22, // addr
            0x22, // addr
            0xAB, // value
            0xCD, // value
        ];
        let adu = decode_request(buf).unwrap().unwrap();
        let RequestAdu { hdr, pdu } = adu;
        let RequestPdu(pdu) = pdu;
        assert_eq!(hdr.transaction_id, 42);
        assert_eq!(hdr.unit_id, 0x12);
        assert_eq!(FnCode::from(pdu), FnCode::WriteSingleRegister);
    }

    #[test]
    fn decode_wrong_protocol() {
        let buf = &[
            0x00, // Transaction id
            0x2a, // Transaction id
            0x00, // Protocol id
            0x01, // Protocol id
            0x00, // length
            0x06, // length
            0x12, // unit id
            0x06, // function code
            0x22, // addr
            0x22, // addr
            0xAB, // value
            0xCD, // value
        ];
        assert!(decode_request(buf).unwrap().is_none());
    }

    #[test]
    fn encode_write_single_register_response() {
        let adu = ResponseAdu {
            hdr: Header {
                transaction_id: 42,
                unit_id: 0x12,
            },
            pdu: ResponsePdu(Ok(Response::WriteSingleRegister(0x2222, 0xABCD))),
        };
        let buf = &mut [0; 100];
        let len = encode_response(adu, buf).unwrap();
        assert_eq!(len, 12);
        assert_eq!(buf[0], 0x00);
        assert_eq!(buf[1], 0x2a);
        assert_eq!(buf[2], 0x00);
        assert_eq!(buf[3], 0x00);
        assert_eq!(buf[4], 0x00);
        assert_eq!(buf[5], 0x06);
        assert_eq!(buf[6], 0x12);
        assert_eq!(buf[7], 0x06);
        assert_eq!(buf[8], 0x22);
        assert_eq!(buf[9], 0x22);
        assert_eq!(buf[10], 0xAB);
        assert_eq!(buf[11], 0xCD);
    }
}
