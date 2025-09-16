// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Modbus TCP client (master) specific functions.
use super::*;

/// Encode an TCP request.
pub fn encode_request(adu: RequestAdu, buf: &mut [u8]) -> Result<usize> {
    let RequestAdu { hdr, pdu } = adu;
    if buf.len() < 2 {
        return Err(Error::BufferSize);
    }
    let len = pdu.encode(&mut buf[7..])?;
    if buf.len() < len + 7 {
        return Err(Error::BufferSize);
    }
    buf[..2].copy_from_slice(&hdr.transaction_id.to_be_bytes());
    buf[2..4].fill(0);
    buf[4..6].copy_from_slice(&(1 + len as u16).to_be_bytes());
    buf[6] = hdr.unit_id;
    Ok(len + 7)
}

/// Decode an TCP response.
pub fn decode_response(buf: &[u8]) -> Result<Option<ResponseAdu<'_>>> {
    if buf.is_empty() {
        return Ok(None);
    }
    decode(DecoderType::Response, buf)
        .and_then(|frame| {
            let Some((
                DecodedFrame {
                    transaction_id,
                    unit_id,
                    pdu,
                },
                _frame_pos,
            )) = frame
            else {
                return Ok(None);
            };
            let hdr = Header {
                transaction_id,
                unit_id,
            };
            // Decoding of the PDU should are unlikely to fail due
            // to transmission errors, because the frame's bytes
            // have already been verified with the CRC.

            ExceptionResponse::try_from(pdu)
                .map(|er| ResponsePdu(Err(er)))
                .or_else(|_| Response::try_from(pdu).map(|r| ResponsePdu(Ok(r))))
                .map(|pdu| Some(ResponseAdu { hdr, pdu }))
                .inspect_err(|&err| {
                    // Unrecoverable error
                    log::error!("Failed to decode Response PDU: {err}");
                })
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
                hdr: Header {
                    transaction_id: 0x1234,
                    unit_id: 0x12,
                },
                pdu: RequestPdu(Request::WriteSingleRegister(0x2222, 0xABCD)),
            },
            &mut buf,
        )
        .expect("Error encoding request");

        let req = &buf[..sz];
        assert_eq!(
            req,
            &[
                0x12, // transaction id
                0x34, // transaction id
                0x00, // protocol id
                0x00, // protocol id
                0x00, // length high byte
                0x06, // length low byte
                0x12, // slave address
                0x06, // function code
                0x22, // addr
                0x22, // addr
                0xAB, // value
                0xCD, // value
            ]
        );
    }

    #[test]
    fn decode_write_single_register_response() {
        use crate::frame::Response;
        let rsp = &[
            0x12, 0x34, 0x00, 0x00, 0x00, 0x06, 0x12, 0x06, 0x22, 0x22, 0xAB, 0xCD,
        ];

        assert!(matches!(
            decode_response(rsp),
            Ok(Some(ResponseAdu {
                hdr: Header {
                    transaction_id: 0x1234,
                    unit_id: 0x12
                },
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
