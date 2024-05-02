//! Modbus TCP server (slave) specific functions.
use super::*;

/// Decode an TCP request.
pub fn decode_request(buf: &[u8]) -> Result<Option<RequestAdu>> {
    if buf.is_empty() {
        return Ok(None);
    }
    let frame = decode(DecoderType::Request, buf)?;
    let Some((decoded_frame, _frame_pos)) = frame else {
        return Ok(None);
    };
    let DecodedFrame {
        transaction_id,
        unit_id,
        pdu,
    } = decoded_frame;
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
            log::error!("Failed to decode request PDU: {err}");
            err
        })
}

// Decode a TCP response
pub fn decode_response(buf: &[u8]) -> Result<Option<ResponseAdu>> {
    if buf.is_empty() {
        return Err(Error::BufferSize);
    }
    decode(DecoderType::Response, buf)
        .and_then(|frame| {
            let Some((decoded_frame, _frame_pos)) = frame else {
                return Ok(None);
            };
            let DecodedFrame {
                transaction_id,
                unit_id,
                pdu,
            } = decoded_frame;
            let hdr = Header {
                transaction_id,
                unit_id,
            };
            // Decoding of the PDU should are unlikely to fail due
            // to transmission errors, because the frame's bytes
            // have already been verified at the TCP level.

            Response::try_from(pdu)
                .map(Ok)
                .or_else(|_| ExceptionResponse::try_from(pdu).map(Err))
                .map(ResponsePdu)
                .map(|pdu| Some(ResponseAdu { hdr, pdu }))
                .map_err(|err| {
                    // Unrecoverable error
                    log::error!("Failed to decode response PDU: {err}");
                    err
                })
        })
        .map_err(|_| {
            // Decoding the transport frame is non-destructive and must
            // never fail!
            unreachable!();
        })
}

/// Encode an TCP response.
pub fn encode_response(adu: ResponseAdu, buf: &mut [u8]) -> Result<usize> {
    let ResponseAdu { hdr, pdu } = adu;
    if buf.len() < 7 {
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

pub fn encode_request(adu: RequestAdu, buf: &mut [u8]) -> Result<usize> {
    let RequestAdu { hdr, pdu } = adu;
    if buf.len() < 7 {
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
        assert_eq!(FunctionCode::from(pdu), FunctionCode::WriteSingleRegister);
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

    #[test]
    fn response_buffer_too_small() {
        let adu = ResponseAdu {
            hdr: Header {
                transaction_id: 42,
                unit_id: 0x12,
            },
            pdu: ResponsePdu(Ok(Response::WriteSingleRegister(0x2222, 0xABCD))),
        };
        let buf = &mut [0; 11];
        let res = encode_response(adu, buf).err().unwrap();
        assert_eq!(res, Error::BufferSize);
    }

    #[test]
    fn request_buffer_too_small() {
        let adu = RequestAdu {
            hdr: Header {
                transaction_id: 42,
                unit_id: 0x12,
            },
            pdu: RequestPdu(Request::WriteSingleRegister(0x2222, 0xABCD)),
        };
        let buf = &mut [0; 11];
        let res = encode_request(adu, buf).err().unwrap();
        assert_eq!(res, Error::BufferSize);
    }
}
