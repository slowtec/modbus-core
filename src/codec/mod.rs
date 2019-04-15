use crate::{error::*, frame::*};
use byteorder::{BigEndian, ByteOrder};
use core::convert::TryFrom;

pub mod rtu;

/// The type of decoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderType {
    Request,
    Response,
}

type Result<T> = core::result::Result<T, Error>;

impl TryFrom<u8> for Exception {
    type Error = Error;

    fn try_from(code: u8) -> Result<Self> {
        use crate::frame::Exception::*;
        let ex = match code {
            0x01 => IllegalFunction,
            0x02 => IllegalDataAddress,
            0x03 => IllegalDataValue,
            0x04 => ServerDeviceFailure,
            0x05 => Acknowledge,
            0x06 => ServerDeviceBusy,
            0x08 => MemoryParityError,
            0x0A => GatewayPathUnavailable,
            0x0B => GatewayTargetDevice,
            _ => {
                return Err(Error::ExceptionCode(code));
            }
        };
        Ok(ex)
    }
}

impl From<ExceptionResponse> for [u8; 2] {
    fn from(ex: ExceptionResponse) -> [u8; 2] {
        let data = &mut [0; 2];
        let fn_code: u8 = ex.function.into();
        debug_assert!(fn_code < 0x80);
        data[0] = fn_code + 0x80;
        data[1] = ex.exception as u8;
        *data
    }
}

impl TryFrom<&[u8]> for ExceptionResponse {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let fn_err_code = bytes[0];
        if fn_err_code < 0x80 {
            return Err(Error::ExceptionFnCode(fn_err_code));
        }
        let function = (fn_err_code - 0x80).into();
        let exception = Exception::try_from(bytes[1])?;
        Ok(ExceptionResponse {
            function,
            exception,
        })
    }
}

impl<'r> TryFrom<&'r [u8]> for Request<'r> {
    type Error = Error;

    fn try_from(bytes: &'r [u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(Error::BufferSize);
        }

        let fn_code = bytes[0];

        if bytes.len() < min_request_pdu_len(fn_code.into()) {
            return Err(Error::BufferSize);
        }

        use crate::frame::Request::*;
        use FnCode as f;

        let req = match FnCode::from(fn_code) {
            f::ReadCoils
            | f::ReadDiscreteInputs
            | f::ReadInputRegisters
            | f::ReadHoldingRegisters
            | f::WriteSingleRegister => {
                let addr = BigEndian::read_u16(&bytes[1..3]);
                let quantity = BigEndian::read_u16(&bytes[3..5]);

                match FnCode::from(fn_code) {
                    f::ReadCoils => ReadCoils(addr, quantity),
                    f::ReadDiscreteInputs => ReadDiscreteInputs(addr, quantity),
                    f::ReadInputRegisters => ReadInputRegisters(addr, quantity),
                    f::ReadHoldingRegisters => ReadHoldingRegisters(addr, quantity),
                    f::WriteSingleRegister => WriteSingleRegister(addr, quantity),
                    _ => unreachable!(),
                }
            }
            f::WriteSingleCoil => WriteSingleCoil(
                BigEndian::read_u16(&bytes[1..3]),
                u16_coil_to_bool(BigEndian::read_u16(&bytes[3..5]))?,
            ),
            f::WriteMultipleCoils => {
                let address = BigEndian::read_u16(&bytes[1..3]);
                let quantity = BigEndian::read_u16(&bytes[3..5]) as usize;
                let byte_count = bytes[5];
                if bytes.len() < (6 + byte_count as usize) {
                    return Err(Error::ByteCount(byte_count));
                }
                let data = &bytes[6..];
                let coils = Coils { quantity, data };
                WriteMultipleCoils(address, coils)
            }
            f::WriteMultipleRegisters => {
                let address = BigEndian::read_u16(&bytes[1..3]);
                let quantity = BigEndian::read_u16(&bytes[3..5]) as usize;
                let byte_count = bytes[5];
                if bytes.len() < (6 + byte_count as usize) {
                    return Err(Error::ByteCount(byte_count));
                }
                let data = Data {
                    quantity,
                    data: &bytes[6..6 + byte_count as usize],
                };
                WriteMultipleRegisters(address, data)
            }
            f::ReadWriteMultipleRegisters => {
                let read_address = BigEndian::read_u16(&bytes[1..3]);
                let read_quantity = BigEndian::read_u16(&bytes[3..5]);
                let write_address = BigEndian::read_u16(&bytes[5..7]);
                let write_quantity = BigEndian::read_u16(&bytes[7..9]) as usize;
                let write_count = bytes[9];
                if bytes.len() < (10 + write_count as usize) {
                    return Err(Error::ByteCount(write_count));
                }
                let data = Data {
                    quantity: write_quantity as usize,
                    data: &bytes[10..10 + write_count as usize],
                };
                ReadWriteMultipleRegisters(read_address, read_quantity, write_address, data)
            }
            _ => match fn_code {
                fn_code if fn_code < 0x80 => Custom(FnCode::Custom(fn_code), &bytes[1..]),
                _ => return Err(Error::FnCode(fn_code)),
            },
        };
        Ok(req)
    }
}

impl<'r> TryFrom<&'r [u8]> for Response<'r> {
    type Error = Error;

    fn try_from(bytes: &'r [u8]) -> Result<Self> {
        use crate::frame::Response::*;
        let fn_code = bytes[0];
        if bytes.len() < min_response_pdu_len(fn_code.into()) {
            return Err(Error::BufferSize);
        }
        use FnCode as f;
        let rsp = match FnCode::from(fn_code) {
            f::ReadCoils | FnCode::ReadDiscreteInputs => {
                let byte_count = bytes[1] as usize;
                if byte_count + 2 > bytes.len() {
                    return Err(Error::BufferSize);
                }
                let data = &bytes[2..byte_count + 2];
                // Here we have not information about the exact requested quantity
                // therefore we just assume that the whole byte is meant.
                let quantity = byte_count * 8;

                match FnCode::from(fn_code) {
                    FnCode::ReadCoils => ReadCoils(Coils { quantity, data }),
                    FnCode::ReadDiscreteInputs => ReadDiscreteInputs(Coils { quantity, data }),
                    _ => unreachable!(),
                }
            }
            f::WriteSingleCoil => WriteSingleCoil(BigEndian::read_u16(&bytes[1..])),

            f::WriteMultipleCoils | f::WriteSingleRegister | f::WriteMultipleRegisters => {
                let addr = BigEndian::read_u16(&bytes[1..]);
                let payload = BigEndian::read_u16(&bytes[3..]);
                match FnCode::from(fn_code) {
                    f::WriteMultipleCoils => WriteMultipleCoils(addr, payload),
                    f::WriteSingleRegister => WriteSingleRegister(addr, payload),
                    f::WriteMultipleRegisters => WriteMultipleRegisters(addr, payload),
                    _ => unreachable!(),
                }
            }
            f::ReadInputRegisters | f::ReadHoldingRegisters | f::ReadWriteMultipleRegisters => {
                let byte_count = bytes[1] as usize;
                let quantity = byte_count / 2;
                if byte_count + 2 > bytes.len() {
                    return Err(Error::BufferSize);
                }
                let data = &bytes[2..2 + byte_count];
                let data = Data { quantity, data };

                match FnCode::from(fn_code) {
                    f::ReadInputRegisters => ReadInputRegisters(data),
                    f::ReadHoldingRegisters => ReadHoldingRegisters(data),
                    f::ReadWriteMultipleRegisters => ReadWriteMultipleRegisters(data),
                    _ => unreachable!(),
                }
            }
            _ => Custom(FnCode::from(fn_code), &bytes[1..]),
        };
        Ok(rsp)
    }
}

/// Encode a struct into a buffer.
pub trait Encode {
    fn encode(&self, buf: &mut [u8]) -> Result<usize>;
}

impl<'r> Encode for Request<'r> {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < self.pdu_len() {
            return Err(Error::BufferSize);
        }
        use crate::frame::Request::*;
        buf[0] = FnCode::from(*self).into();
        match self {
            ReadCoils(address, payload)
            | ReadDiscreteInputs(address, payload)
            | ReadInputRegisters(address, payload)
            | ReadHoldingRegisters(address, payload)
            | WriteSingleRegister(address, payload) => {
                BigEndian::write_u16(&mut buf[1..], *address);
                BigEndian::write_u16(&mut buf[3..], *payload);
            }
            WriteSingleCoil(address, state) => {
                BigEndian::write_u16(&mut buf[1..], *address);
                BigEndian::write_u16(&mut buf[3..], bool_to_u16_coil(*state));
            }
            WriteMultipleCoils(address, coils) => {
                BigEndian::write_u16(&mut buf[1..], *address);
                let len = coils.len();
                BigEndian::write_u16(&mut buf[3..], len as u16);
                buf[5] = coils.packed_len() as u8;
                coils.copy_to(&mut buf[6..]);
            }
            WriteMultipleRegisters(address, words) => {
                BigEndian::write_u16(&mut buf[1..], *address);
                let len = words.len();
                BigEndian::write_u16(&mut buf[3..], len as u16);
                buf[5] = len as u8 * 2;
                for (idx, byte) in words.data.iter().enumerate() {
                    buf[idx + 6] = *byte;
                }
            }
            ReadWriteMultipleRegisters(read_address, quantity, write_address, words) => {
                BigEndian::write_u16(&mut buf[1..], *read_address);
                BigEndian::write_u16(&mut buf[3..], *quantity);
                BigEndian::write_u16(&mut buf[5..], *write_address);
                let n = words.len();
                BigEndian::write_u16(&mut buf[7..], n as u16);
                buf[9] = n as u8 * 2;
                for (idx, byte) in words.data.iter().enumerate() {
                    buf[idx + 10] = *byte;
                }
            }
            Custom(_, custom_data) => {
                custom_data.iter().enumerate().for_each(|(idx, d)| {
                    buf[idx + 1] = *d;
                });
            }
            _ => panic!(),
        }
        Ok(self.pdu_len())
    }
}

impl<'r> Encode for Response<'r> {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() < self.pdu_len() {
            return Err(Error::BufferSize);
        }
        use crate::frame::Response::*;
        buf[0] = FnCode::from(*self).into();
        match self {
            ReadCoils(coils) | ReadDiscreteInputs(coils) => {
                buf[1] = coils.packed_len() as u8;
                coils.copy_to(&mut buf[2..]);
            }
            ReadInputRegisters(registers)
            | ReadHoldingRegisters(registers)
            | ReadWriteMultipleRegisters(registers) => {
                buf[1] = (registers.len() * 2) as u8;
                registers.copy_to(&mut buf[2..]);
            }
            WriteSingleCoil(address) => {
                BigEndian::write_u16(&mut buf[1..], *address);
            }
            WriteMultipleCoils(address, payload)
            | WriteMultipleRegisters(address, payload)
            | WriteSingleRegister(address, payload) => {
                BigEndian::write_u16(&mut buf[1..], *address);
                BigEndian::write_u16(&mut buf[3..], *payload);
            }
            Custom(_, custom_data) => {
                for (idx, d) in custom_data.iter().enumerate() {
                    buf[idx + 1] = *d;
                }
            }
            _ => {
                // TODO:
                unimplemented!()
            }
        }
        Ok(self.pdu_len())
    }
}

impl<'r> Encode for ResponsePdu<'r> {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        match self.0 {
            Ok(res) => res.encode(buf),
            Err(e) => e.encode(buf),
        }
    }
}

impl Encode for ExceptionResponse {
    fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        let [code, ex]: [u8; 2] = (*self).into();
        buf[0] = code;
        buf[1] = ex;
        Ok(2)
    }
}

fn min_request_pdu_len(fn_code: FnCode) -> usize {
    use FnCode::*;
    match fn_code {
        ReadCoils | ReadDiscreteInputs | ReadInputRegisters | WriteSingleCoil
        | ReadHoldingRegisters | WriteSingleRegister => 5,
        WriteMultipleCoils => 6,
        WriteMultipleRegisters => 6,
        ReadWriteMultipleRegisters => 10,
        _ => 1,
    }
}

fn min_response_pdu_len(fn_code: FnCode) -> usize {
    use FnCode::*;
    match fn_code {
        ReadCoils
        | ReadDiscreteInputs
        | ReadInputRegisters
        | ReadHoldingRegisters
        | ReadWriteMultipleRegisters => 2,
        WriteSingleCoil => 3,
        WriteMultipleCoils | WriteSingleRegister | WriteMultipleRegisters => 5,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exception_response_into_bytes() {
        let bytes: [u8; 2] = ExceptionResponse {
            function: 0x03.into(),
            exception: Exception::IllegalDataAddress,
        }
        .into();
        assert_eq!(bytes[0], 0x83);
        assert_eq!(bytes[1], 0x02);
    }

    #[test]
    fn exception_response_from_bytes() {
        let data: &[u8] = &[0x79, 0x02];
        assert!(ExceptionResponse::try_from(data).is_err());

        let bytes: &[u8] = &[0x83, 0x02];
        let rsp = ExceptionResponse::try_from(bytes).unwrap();
        assert_eq!(
            rsp,
            ExceptionResponse {
                function: 0x03.into(),
                exception: Exception::IllegalDataAddress,
            }
        );
    }

    #[test]
    fn test_min_request_pdu_len() {
        use FnCode::*;

        assert_eq!(min_request_pdu_len(ReadCoils), 5);
        assert_eq!(min_request_pdu_len(ReadDiscreteInputs), 5);
        assert_eq!(min_request_pdu_len(ReadInputRegisters), 5);
        assert_eq!(min_request_pdu_len(WriteSingleCoil), 5);
        assert_eq!(min_request_pdu_len(ReadHoldingRegisters), 5);
        assert_eq!(min_request_pdu_len(WriteSingleRegister), 5);
        assert_eq!(min_request_pdu_len(WriteMultipleCoils), 6);
        assert_eq!(min_request_pdu_len(WriteMultipleRegisters), 6);
        assert_eq!(min_request_pdu_len(ReadWriteMultipleRegisters), 10);
    }

    #[test]
    fn test_min_response_pdu_len() {
        use FnCode::*;

        assert_eq!(min_response_pdu_len(ReadCoils), 2);
        assert_eq!(min_response_pdu_len(ReadDiscreteInputs), 2);
        assert_eq!(min_response_pdu_len(ReadInputRegisters), 2);
        assert_eq!(min_response_pdu_len(WriteSingleCoil), 3);
        assert_eq!(min_response_pdu_len(ReadHoldingRegisters), 2);
        assert_eq!(min_response_pdu_len(WriteSingleRegister), 5);
        assert_eq!(min_response_pdu_len(WriteMultipleCoils), 5);
        assert_eq!(min_response_pdu_len(WriteMultipleRegisters), 5);
        assert_eq!(min_response_pdu_len(ReadWriteMultipleRegisters), 2);
    }

    mod serialize_requests {
        use super::*;

        #[test]
        fn read_coils() {
            let bytes = &mut [0; 4];
            assert!(Request::ReadCoils(0x12, 4).encode(bytes).is_err());
            let bytes = &mut [0; 5];
            Request::ReadCoils(0x12, 4).encode(bytes).unwrap();
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x04);
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes = &mut [0; 5];
            Request::ReadDiscreteInputs(0x03, 19).encode(bytes).unwrap();
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x03);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 19);
        }

        #[test]
        fn write_single_coil() {
            let bytes = &mut [0; 5];
            Request::WriteSingleCoil(0x1234, true)
                .encode(bytes)
                .unwrap();
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x12);
            assert_eq!(bytes[2], 0x34);
            assert_eq!(bytes[3], 0xFF);
            assert_eq!(bytes[4], 0x00);
        }

        #[test]
        fn write_multiple_coils() {
            let states = &[true, false, true, true];
            let buf = &mut [0];
            let bytes = &mut [0; 7];
            Request::WriteMultipleCoils(0x3311, Coils::from_bools(states, buf).unwrap())
                .encode(bytes)
                .unwrap();
            assert_eq!(bytes[0], 0x0F);
            assert_eq!(bytes[1], 0x33);
            assert_eq!(bytes[2], 0x11);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x04);
            assert_eq!(bytes[5], 0x01);
            assert_eq!(bytes[6], 0b_0000_1101);
        }

        #[test]
        fn read_input_registers() {
            let bytes = &mut [0; 5];
            Request::ReadInputRegisters(0x09, 77).encode(bytes).unwrap();
            assert_eq!(bytes[0], 4);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn read_holding_registers() {
            let bytes = &mut [0; 5];
            Request::ReadHoldingRegisters(0x09, 77)
                .encode(bytes)
                .unwrap();
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn write_single_register() {
            let bytes = &mut [0; 5];
            Request::WriteSingleRegister(0x07, 0xABCD)
                .encode(bytes)
                .unwrap();
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let buf = &mut [0; 4];
            let bytes = &mut [0; 10];

            Request::WriteMultipleRegisters(
                0x06,
                Data::from_words(&[0xABCD, 0xEF12], buf).unwrap(),
            )
            .encode(bytes)
            .unwrap();

            // function code
            assert_eq!(bytes[0], 0x10);

            // write starting address
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);

            // quantity to write
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x02);

            // write byte count
            assert_eq!(bytes[5], 0x04);

            // values
            assert_eq!(bytes[6], 0xAB);
            assert_eq!(bytes[7], 0xCD);
            assert_eq!(bytes[8], 0xEF);
            assert_eq!(bytes[9], 0x12);
        }

        #[test]
        fn read_write_multiple_registers() {
            let buf = &mut [0; 4];
            let bytes = &mut [0; 14];
            let data = Data::from_words(&[0xABCD, 0xEF12], buf).unwrap();
            Request::ReadWriteMultipleRegisters(0x05, 51, 0x03, data)
                .encode(bytes)
                .unwrap();

            // function code
            assert_eq!(bytes[0], 0x17);

            // read starting address
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x05);

            // quantity to read
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x33);

            // write starting address
            assert_eq!(bytes[5], 0x00);
            assert_eq!(bytes[6], 0x03);

            // quantity to write
            assert_eq!(bytes[7], 0x00);
            assert_eq!(bytes[8], 0x02);

            // write byte count
            assert_eq!(bytes[9], 0x04);

            // values
            assert_eq!(bytes[10], 0xAB);
            assert_eq!(bytes[11], 0xCD);
            assert_eq!(bytes[12], 0xEF);
            assert_eq!(bytes[13], 0x12);
        }

        #[test]
        fn custom() {
            let bytes = &mut [0; 5];
            Request::Custom(FnCode::Custom(0x55), &[0xCC, 0x88, 0xAA, 0xFF])
                .encode(bytes)
                .unwrap();
            assert_eq!(bytes[0], 0x55);
            assert_eq!(bytes[1], 0xCC);
            assert_eq!(bytes[2], 0x88);
            assert_eq!(bytes[3], 0xAA);
            assert_eq!(bytes[4], 0xFF);
        }
    }

    mod deserialize_requests {
        use super::*;

        #[test]
        fn empty_request() {
            let data: &[u8] = &[];
            assert!(Request::try_from(data).is_err());
        }

        #[test]
        fn read_coils() {
            let data: &[u8] = &[0x01];
            assert!(Request::try_from(data).is_err());
            let data: &[u8] = &[0x01, 0x0, 0x0, 0x22];
            assert!(Request::try_from(data).is_err());

            let data: &[u8] = &[0x01, 0x00, 0x12, 0x0, 0x4];
            let req = Request::try_from(data).unwrap();
            assert_eq!(req, Request::ReadCoils(0x12, 4));
        }

        #[test]
        fn read_discrete_inputs() {
            let data: &[u8] = &[2, 0x00, 0x03, 0x00, 19];
            let req = Request::try_from(data).unwrap();
            assert_eq!(req, Request::ReadDiscreteInputs(0x03, 19));
        }

        #[test]
        fn write_single_coil() {
            let bytes: &[u8] = &[5, 0x12, 0x34, 0xFF, 0x00];
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::WriteSingleCoil(0x1234, true));
        }

        #[test]
        fn write_multiple_coils() {
            let data: &[u8] = &[0x0F, 0x33, 0x11, 0x00, 0x04, 0x02, 0b_0000_1101];
            assert!(Request::try_from(data).is_err());

            let data: &[u8] = &[
                0x0F, 0x33, 0x11, 0x00, 0x04, 0x00, // byte count == 0
            ];
            assert!(Request::try_from(data).is_ok());

            let bytes: &[u8] = &[0x0F, 0x33, 0x11, 0x00, 0x04, 0x01, 0b_0000_1101];
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(
                req,
                Request::WriteMultipleCoils(
                    0x3311,
                    Coils {
                        quantity: 4,
                        data: &[0b1101]
                    }
                )
            );
        }

        #[test]
        fn read_input_registers() {
            let bytes: &[u8] = &[4, 0x00, 0x09, 0x00, 0x4D];
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReadInputRegisters(0x09, 77));
        }

        #[test]
        fn read_holding_registers() {
            let bytes: &[u8] = &[3, 0x00, 0x09, 0x00, 0x4D];
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::ReadHoldingRegisters(0x09, 77));
        }

        #[test]
        fn write_single_register() {
            let bytes: &[u8] = &[6, 0x00, 0x07, 0xAB, 0xCD];
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(req, Request::WriteSingleRegister(0x07, 0xABCD));
        }

        #[test]
        fn write_multiple_registers() {
            let data: &[u8] = &[0x10, 0x00, 0x06, 0x00, 0x02, 0x05, 0xAB, 0xCD, 0xEF, 0x12];
            assert!(Request::try_from(data).is_err());

            let bytes: &[u8] = &[0x10, 0x00, 0x06, 0x00, 0x02, 0x04, 0xAB, 0xCD, 0xEF, 0x12];
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(
                req,
                Request::WriteMultipleRegisters(
                    0x06,
                    Data {
                        quantity: 2,
                        data: &[0xAB, 0xCD, 0xEF, 0x12]
                    }
                )
            );
            if let Request::WriteMultipleRegisters(_, data) = req {
                assert_eq!(data.get(0), Some(0xABCD));
                assert_eq!(data.get(1), Some(0xEF12));
            } else {
                unreachable!()
            };
        }

        #[test]
        fn read_write_multiple_registers() {
            let data: &[u8] = &[
                0x17, 0x00, 0x05, 0x00, 0x33, 0x00, 0x03, 0x00, 0x02, 0x05, 0xAB, 0xCD, 0xEF, 0x12,
            ];
            assert!(Request::try_from(data).is_err());
            let bytes: &[u8] = &[
                0x17, 0x00, 0x05, 0x00, 0x33, 0x00, 0x03, 0x00, 0x02, 0x04, 0xAB, 0xCD, 0xEF, 0x12,
            ];
            let req = Request::try_from(bytes).unwrap();
            let data = Data {
                quantity: 2,
                data: &[0xAB, 0xCD, 0xEF, 0x12],
            };
            assert_eq!(
                req,
                Request::ReadWriteMultipleRegisters(0x05, 51, 0x03, data)
            );
            if let Request::ReadWriteMultipleRegisters(_, _, _, data) = req {
                assert_eq!(data.get(0), Some(0xABCD));
                assert_eq!(data.get(1), Some(0xEF12));
            } else {
                unreachable!()
            };
        }

        #[test]
        fn custom() {
            let bytes: &[u8] = &[0x55, 0xCC, 0x88, 0xAA, 0xFF];
            let req = Request::try_from(bytes).unwrap();
            assert_eq!(
                req,
                Request::Custom(FnCode::Custom(0x55), &[0xCC, 0x88, 0xAA, 0xFF])
            );
        }
    }

    mod serialize_responses {
        use super::*;

        #[test]
        fn read_coils() {
            let buff: &mut [u8] = &mut [0];
            let res = Response::ReadCoils(
                Coils::from_bools(&[true, false, false, true, false], buff).unwrap(),
            );
            let bytes = &mut [0, 0];
            assert!(res.encode(bytes).is_err());
            let bytes = &mut [0, 0, 0];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1001);
        }

        #[test]
        fn read_discrete_inputs() {
            let buff: &mut [u8] = &mut [0];
            let res = Response::ReadDiscreteInputs(
                Coils::from_bools(&[true, false, true, true], buff).unwrap(),
            );
            let bytes = &mut [0, 0, 0];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1101);
        }

        #[test]
        fn write_single_coil() {
            let res = Response::WriteSingleCoil(0x33);
            let bytes = &mut [0, 0, 0];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x33);
        }

        #[test]
        fn write_multiple_coils() {
            let res = Response::WriteMultipleCoils(0x3311, 5);
            let bytes = &mut [0; 5];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 0x0F);
            assert_eq!(bytes[1], 0x33);
            assert_eq!(bytes[2], 0x11);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x05);
        }

        #[test]
        fn read_input_registers() {
            let buf: &mut [u8] = &mut [0; 6];
            let res = Response::ReadInputRegisters(
                Data::from_words(&[0xAA00, 0xCCBB, 0xEEDD], buf).unwrap(),
            );
            let bytes = &mut [0; 8];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 4);
            assert_eq!(bytes[1], 0x06);
            assert_eq!(bytes[2], 0xAA);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0xCC);
            assert_eq!(bytes[5], 0xBB);
            assert_eq!(bytes[6], 0xEE);
            assert_eq!(bytes[7], 0xDD);
        }

        #[test]
        fn read_holding_registers() {
            let buf: &mut [u8] = &mut [0; 4];
            let res =
                Response::ReadHoldingRegisters(Data::from_words(&[0xAA00, 0x1111], buf).unwrap());
            let bytes = &mut [0; 6];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x04);
            assert_eq!(bytes[2], 0xAA);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x11);
            assert_eq!(bytes[5], 0x11);
        }

        #[test]
        fn write_single_register() {
            let res = Response::WriteSingleRegister(0x07, 0xABCD);
            let bytes = &mut [0; 5];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let res = Response::WriteMultipleRegisters(0x06, 2);
            let bytes = &mut [0; 5];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 0x10);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x02);
        }

        #[test]
        fn read_write_multiple_registers() {
            let buf: &mut [u8] = &mut [0; 2];
            let res =
                Response::ReadWriteMultipleRegisters(Data::from_words(&[0x1234], buf).unwrap());
            let bytes = &mut [0; 4];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 0x17);
            assert_eq!(bytes[1], 0x02);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x34);
        }

        #[test]
        fn custom() {
            let res = Response::Custom(FnCode::Custom(0x55), &[0xCC, 0x88, 0xAA, 0xFF]);
            let bytes = &mut [0; 5];
            res.encode(bytes).unwrap();
            assert_eq!(bytes[0], 0x55);
            assert_eq!(bytes[1], 0xCC);
            assert_eq!(bytes[2], 0x88);
            assert_eq!(bytes[3], 0xAA);
            assert_eq!(bytes[4], 0xFF);
        }
    }

    mod deserialize_responses {
        use super::*;

        #[test]
        fn read_coils() {
            let bytes: &[u8] = &[1, 1, 0b_0000_1001];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadCoils(Coils {
                    quantity: 8,
                    data: &[0b_0000_1001]
                })
            );
        }

        #[test]
        fn read_no_coils() {
            let bytes: &[u8] = &[1, 0];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadCoils(Coils {
                    quantity: 0,
                    data: &[]
                })
            );
        }

        #[test]
        fn read_coils_with_invalid_byte_count() {
            let bytes: &[u8] = &[1, 2, 0x6];
            assert!(Response::try_from(bytes).is_err());
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes: &[u8] = &[2, 1, 0b_0000_1001];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadDiscreteInputs(Coils {
                    quantity: 8,
                    data: &[0b_0000_1001]
                })
            );
        }

        #[test]
        fn write_single_coil() {
            let bytes: &[u8] = &[5, 0x00, 0x33];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteSingleCoil(0x33));

            let broken_bytes: &[u8] = &[5, 0x00];
            assert!(Response::try_from(broken_bytes).is_err());
        }

        #[test]
        fn write_multiple_coils() {
            let bytes: &[u8] = &[0x0F, 0x33, 0x11, 0x00, 0x05];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteMultipleCoils(0x3311, 5));
            let broken_bytes: &[u8] = &[0x0F, 0x33, 0x11, 0x00];
            assert!(Response::try_from(broken_bytes).is_err());
        }

        #[test]
        fn read_input_registers() {
            let bytes: &[u8] = &[4, 0x06, 0xAA, 0x00, 0xCC, 0xBB, 0xEE, 0xDD];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadInputRegisters(Data {
                    quantity: 3,
                    data: &[0xAA, 0x00, 0xCC, 0xBB, 0xEE, 0xDD]
                })
            );
        }

        #[test]
        fn read_holding_registers() {
            let bytes: &[u8] = &[3, 0x04, 0xAA, 0x00, 0x11, 0x11];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadHoldingRegisters(Data {
                    quantity: 2,
                    data: &[0xAA, 0x00, 0x11, 0x11]
                })
            );
        }

        #[test]
        fn write_single_register() {
            let bytes: &[u8] = &[6, 0x00, 0x07, 0xAB, 0xCD];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteSingleRegister(0x07, 0xABCD));
            let broken_bytes: &[u8] = &[6, 0x00, 0x07, 0xAB];
            assert!(Response::try_from(broken_bytes).is_err());
        }

        #[test]
        fn write_multiple_registers() {
            let bytes: &[u8] = &[0x10, 0x00, 0x06, 0x00, 0x02];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::WriteMultipleRegisters(0x06, 2));
            let broken_bytes: &[u8] = &[0x10, 0x00, 0x06, 0x00];
            assert!(Response::try_from(broken_bytes).is_err());
        }

        #[test]
        fn read_write_multiple_registers() {
            let bytes: &[u8] = &[0x17, 0x02, 0x12, 0x34];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::ReadWriteMultipleRegisters(Data {
                    quantity: 1,
                    data: &[0x12, 0x34]
                })
            );
            let broken_bytes: &[u8] = &[0x17, 0x02, 0x12];
            assert!(Response::try_from(broken_bytes).is_err());
        }

        #[test]
        fn custom() {
            let bytes: &[u8] = &[0x55, 0xCC, 0x88, 0xAA, 0xFF];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(
                rsp,
                Response::Custom(FnCode::Custom(0x55), &[0xCC, 0x88, 0xAA, 0xFF])
            );
            let bytes: &[u8] = &[0x66];
            let rsp = Response::try_from(bytes).unwrap();
            assert_eq!(rsp, Response::Custom(FnCode::Custom(0x66), &[]));
        }
    }
}
