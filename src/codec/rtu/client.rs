// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus RTU client (master) specific functions.
use super::*;

/// Encode and RTU request.
pub fn encode_request(adu: RequestAdu, buf: &mut [u8]) -> Result<usize> {
    let RequestAdu { hdr, pdu } = adu;
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

/// Decode an RTU response.
pub fn decode_response(buf: &[u8]) -> Result<Option<ResponseAdu<'_>>> {
    if buf.is_empty() {
        return Ok(None);
    }
    decode(DecoderType::Response, buf)
        .and_then(|frame| {
            let Some((DecodedFrame { slave, pdu }, _frame_pos)) = frame else {
                return Ok(None);
            };
            let hdr = Header { slave };
            // Decoding of the PDU should are unlikely to fail due
            // to transmission errors, because the frame's bytes
            // have already been verified with the CRC.

            let response = ExceptionResponse::try_from(pdu)
                .map(|er| ResponsePdu(Err(er)))
                .or_else(|_| Response::try_from(pdu).map(|r| ResponsePdu(Ok(r))))
                .map(|pdu| Some(ResponseAdu { hdr, pdu }));
            #[cfg(feature = "log")]
            response.inspect_err(|&err| {
                // Unrecoverable error
                log::error!("Failed to decode Response PDU: {err}");
            });
            response
        })
        .map_err(|_| {
            // Decoding the transport frame is non-destructive and must
            // never fail!
            unreachable!();
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_empty_response() {
        let req = decode_response(&[]).unwrap();
        assert!(req.is_none());
    }

    #[test]
    fn decode_partly_received_response() {
        let buf = &[
            0x12, // slave address
            0x16, // function code
        ];
        let req = decode_response(buf).unwrap();
        assert!(req.is_none());
    }

    #[test]
    fn encode_write_single_register_request() {
        let mut buf = [0u8; 255];
        let sz = encode_request(
            RequestAdu {
                hdr: Header { slave: 0x12 },
                pdu: RequestPdu(Request::WriteSingleRegister(0x2222, 0xABCD)),
            },
            &mut buf,
        )
        .expect("Error encoding request");

        let req = &buf[..sz];
        assert_eq!(
            req,
            &[
                0x12, // slave address
                0x06, // function code
                0x22, // addr
                0x22, // addr
                0xAB, // value
                0xCD, // value
                0x9F, // crc
                0xBE, // crc
            ]
        );
    }

    #[test]
    fn decode_write_single_register_response() {
        use crate::frame::Response;
        let rsp = &[0x12, 0x06, 0x22, 0x22, 0xAB, 0xCD, 0x9F, 0xBE];

        assert!(matches!(
            decode_response(rsp),
            Ok(Some(ResponseAdu {
                hdr: Header { slave: 0x12 },
                pdu: ResponsePdu(Ok(Response::WriteSingleRegister(0x2222, 0xABCD)))
            }))
        ));
    }

    #[test]
    fn decode_malformed_write_single_register_response() {
        let rsp = &[0x12, 0x06, 0x22, 0x22, 0xAB, 0x65, 0x9E];

        assert!(matches!(decode_response(rsp), Ok(None)));
    }

    #[test]
    fn decode_bad_crc_write_single_register_response() {
        let rsp = &[0x12, 0x06, 0x22, 0x22, 0xAB, 0xCD, 0x5F, 0xBE];

        assert!(matches!(decode_response(rsp), Ok(None)));
    }
}
