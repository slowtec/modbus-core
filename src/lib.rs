#![no_std]

use core::fmt;

mod error;
pub mod rtu;
pub mod util;

pub use error::*;

/// A Modbus function code.
///
/// It is represented by an unsigned 8 bit integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnCode {
    ReadCoils,
    ReadDiscreteInputs,
    WriteSingleCoil,
    WriteMultipleCoils,
    ReadInputRegisters,
    ReadHoldingRegisters,
    WriteSingleRegister,
    WriteMultipleRegisters,
    ReadWriteMultipleRegisters,
    #[cfg(feature = "rtu")]
    ReadExceptionStatus,
    #[cfg(feature = "rtu")]
    Diagnostics,
    #[cfg(feature = "rtu")]
    GetCommEventCounter,
    #[cfg(feature = "rtu")]
    GetCommEventLog,
    #[cfg(feature = "rtu")]
    ReportServerId,
    //TODO:
    //- ReadFileRecord
    //- WriteFileRecord
    //- MaskWriteRegiger
    //TODO:
    //- Read FifoQueue
    //- EncapsulatedInterfaceTransport
    //- CanOpenGeneralReferenceRequestAndResponsePdu
    //- ReadDeviceIdentification
    Custom(u8),
}

impl From<u8> for FnCode {
    fn from(c: u8) -> Self {
        use FnCode::*;

        match c {
            0x01 => ReadCoils,
            0x02 => ReadDiscreteInputs,
            0x05 => WriteSingleCoil,
            0x0F => WriteMultipleCoils,
            0x04 => ReadInputRegisters,
            0x03 => ReadHoldingRegisters,
            0x06 => WriteSingleRegister,
            0x10 => WriteMultipleRegisters,
            0x17 => ReadWriteMultipleRegisters,
            #[cfg(feature = "rtu")]
            0x07 => ReadExceptionStatus,
            #[cfg(feature = "rtu")]
            0x08 => Diagnostics,
            #[cfg(feature = "rtu")]
            0x0B => GetCommEventCounter,
            #[cfg(feature = "rtu")]
            0x0C => GetCommEventLog,
            #[cfg(feature = "rtu")]
            0x11 => ReportServerId,
            _ => Custom(c),
        }
    }
}

impl Into<u8> for FnCode {
    fn into(self: Self) -> u8 {
        use FnCode::*;

        match self {
            ReadCoils => 0x01,
            ReadDiscreteInputs => 0x02,
            WriteSingleCoil => 0x05,
            WriteMultipleCoils => 0x0F,
            ReadInputRegisters => 0x04,
            ReadHoldingRegisters => 0x03,
            WriteSingleRegister => 0x06,
            WriteMultipleRegisters => 0x10,
            ReadWriteMultipleRegisters => 0x17,
            #[cfg(feature = "rtu")]
            ReadExceptionStatus => 0x07,
            #[cfg(feature = "rtu")]
            Diagnostics => 0x08,
            #[cfg(feature = "rtu")]
            GetCommEventCounter => 0x0B,
            #[cfg(feature = "rtu")]
            GetCommEventLog => 0x0C,
            #[cfg(feature = "rtu")]
            ReportServerId => 0x11,
            Custom(c) => c,
        }
    }
}

/// A Modbus sub-function code is represented by an unsigned 16 bit integer.
pub(crate) type SubFnCode = u16;

/// A Modbus address is represented by 16 bit (from `0` to `65535`).
pub(crate) type Address = u16;

/// A Coil represents a single bit.
///
/// - `true` is equivalent to `ON`, `1` and `0xFF00`.
/// - `false` is equivalent to `OFF`, `0` and `0x0000`.
pub(crate) type Coil = bool;

/// Modbus uses 16 bit for its data items (big-endian representation).
pub(crate) type Word = u16;

/// Number of items to process (`0` - `65535`).
pub(crate) type Quantity = u16;

/// A request represents a message from the client (master) to the server (slave).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request<'r> {
    ReadCoils(Address, Quantity),
    ReadDiscreteInputs(Address, Quantity),
    WriteSingleCoil(Address, Coil),
    WriteMultipleCoils(Address, &'r [Coil]),
    ReadInputRegisters(Address, Quantity),
    ReadHoldingRegisters(Address, Quantity),
    WriteSingleRegister(Address, Word),
    WriteMultipleRegisters(Address, &'r [Word]),
    ReadWriteMultipleRegisters(Address, Quantity, Address, &'r [Word]),
    #[cfg(feature = "rtu")]
    ReadExceptionStatus,
    #[cfg(feature = "rtu")]
    Diagnostics(SubFnCode, &'r [Word]),
    #[cfg(feature = "rtu")]
    GetCommEventCounter,
    #[cfg(feature = "rtu")]
    GetCommEventLog,
    #[cfg(feature = "rtu")]
    ReportServerId,
    //TODO:
    //- ReadFileRecord
    //- WriteFileRecord
    //- MaskWriteRegiger
    //TODO:
    //- Read FifoQueue
    //- EncapsulatedInterfaceTransport
    //- CanOpenGeneralReferenceRequestAndResponsePdu
    //- ReadDeviceIdentification
    Custom(FnCode, &'r [u8]),
}

#[cfg(feature = "rtu")]
type Status = u16;
#[cfg(feature = "rtu")]
type EventCount = u16;
#[cfg(feature = "rtu")]
type MessageCount = u16;

/// The response data of a successfull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response<'r> {
    ReadCoils(&'r [Coil]),
    ReadDiscreteInputs(&'r [Coil]),
    WriteSingleCoil(Address),
    WriteMultipleCoils(Address, Quantity),
    ReadInputRegisters(&'r [Word]),
    ReadHoldingRegisters(&'r [Word]),
    WriteSingleRegister(Address, Word),
    WriteMultipleRegisters(Address, Quantity),
    ReadWriteMultipleRegisters(&'r [Word]),
    #[cfg(feature = "rtu")]
    ReadExceptionStatus(u8),
    #[cfg(feature = "rtu")]
    Diagnostics(&'r [Word]),
    #[cfg(feature = "rtu")]
    GetCommEventCounter(Status, EventCount),
    #[cfg(feature = "rtu")]
    GetCommEventLog(Status, EventCount, MessageCount, &'r [u8]),
    #[cfg(feature = "rtu")]
    ReportServerId(&'r [u8], bool),
    //TODO:
    //- ReadFileRecord
    //- WriteFileRecord
    //- MaskWriteRegiger
    //TODO:
    //- Read FifoQueue
    //- EncapsulatedInterfaceTransport
    //- CanOpenGeneralReferenceRequestAndResponsePdu
    //- ReadDeviceIdentification
    Custom(FnCode, &'r [u8]),
}

impl<'r> From<Request<'r>> for FnCode {
    fn from(r: Request<'r>) -> Self {
        use FnCode as c;
        use Request::*;

        match r {
            ReadCoils(_, _) => c::ReadCoils,
            ReadDiscreteInputs(_, _) => c::ReadDiscreteInputs,
            WriteSingleCoil(_, _) => c::WriteSingleCoil,
            WriteMultipleCoils(_, _) => c::WriteMultipleCoils,
            ReadInputRegisters(_, _) => c::ReadInputRegisters,
            ReadHoldingRegisters(_, _) => c::ReadHoldingRegisters,
            WriteSingleRegister(_, _) => c::WriteSingleRegister,
            WriteMultipleRegisters(_, _) => c::WriteMultipleRegisters,
            ReadWriteMultipleRegisters(_, _, _, _) => c::ReadWriteMultipleRegisters,
            #[cfg(feature = "rtu")]
            ReadExceptionStatus => c::ReadExceptionStatus,
            #[cfg(feature = "rtu")]
            Diagnostics(_, _) => c::Diagnostics,
            #[cfg(feature = "rtu")]
            GetCommEventCounter => c::GetCommEventCounter,
            #[cfg(feature = "rtu")]
            GetCommEventLog => c::GetCommEventLog,
            #[cfg(feature = "rtu")]
            ReportServerId => c::ReportServerId,
            Custom(code, _) => code,
        }
    }
}

impl<'r> From<Response<'r>> for FnCode {
    fn from(r: Response<'r>) -> Self {
        use FnCode as c;
        use Response::*;

        match r {
            ReadCoils(_) => c::ReadCoils,
            ReadDiscreteInputs(_) => c::ReadDiscreteInputs,
            WriteSingleCoil(_) => c::WriteSingleCoil,
            WriteMultipleCoils(_, _) => c::WriteMultipleCoils,
            ReadInputRegisters(_) => c::ReadInputRegisters,
            ReadHoldingRegisters(_) => c::ReadHoldingRegisters,
            WriteSingleRegister(_, _) => c::WriteSingleRegister,
            WriteMultipleRegisters(_, _) => c::WriteMultipleRegisters,
            ReadWriteMultipleRegisters(_) => c::ReadWriteMultipleRegisters,
            #[cfg(feature = "rtu")]
            ReadExceptionStatus(_) => c::ReadExceptionStatus,
            #[cfg(feature = "rtu")]
            Diagnostics(_) => c::Diagnostics,
            #[cfg(feature = "rtu")]
            GetCommEventCounter(_, _) => c::GetCommEventCounter,
            #[cfg(feature = "rtu")]
            GetCommEventLog(_, _, _, _) => c::GetCommEventLog,
            #[cfg(feature = "rtu")]
            ReportServerId(_, _) => c::ReportServerId,
            Custom(code, _) => code,
        }
    }
}

/// A server (slave) exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    ServerDeviceFailure = 0x04,
    Acknowledge = 0x05,
    ServerDeviceBusy = 0x06,
    MemoryParityError = 0x08,
    GatewayPathUnavailable = 0x0A,
    GatewayTargetDevice = 0x0B,
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Exception::*;

        let desc = match *self {
            IllegalFunction => "Illegal function",
            IllegalDataAddress => "Illegal data address",
            IllegalDataValue => "Illegal data value",
            ServerDeviceFailure => "Server device failure",
            Acknowledge => "Acknowledge",
            ServerDeviceBusy => "Server device busy",
            MemoryParityError => "Memory parity error",
            GatewayPathUnavailable => "Gateway path unavailable",
            GatewayTargetDevice => "Gateway target device failed to respond",
        };
        write!(f, "{}", desc)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn function_code_into_u8() {
        let x: u8 = FnCode::WriteMultipleCoils.into();
        assert_eq!(x, 15);
        let x: u8 = FnCode::Custom(0xBB).into();
        assert_eq!(x, 0xBB);
    }

    #[test]
    fn function_code_from_u8() {
        assert_eq!(FnCode::from(15), FnCode::WriteMultipleCoils);
        assert_eq!(FnCode::from(0xBB), FnCode::Custom(0xBB));
    }

    #[test]
    fn function_code_from_request() {
        use Request::*;
        let requests = &[
            (ReadCoils(0, 0), 1),
            (ReadDiscreteInputs(0, 0), 2),
            (WriteSingleCoil(0, true), 5),
            (WriteMultipleCoils(0, &[]), 0x0F),
            (ReadInputRegisters(0, 0), 0x04),
            (ReadHoldingRegisters(0, 0), 0x03),
            (WriteSingleRegister(0, 0), 0x06),
            (WriteMultipleRegisters(0, &[]), 0x10),
            (ReadWriteMultipleRegisters(0, 0, 0, &[]), 0x17),
            (Custom(FnCode::Custom(88), &[]), 88),
        ];
        for (req, expected) in requests {
            let code: u8 = FnCode::from(*req).into();
            assert_eq!(*expected, code);
        }
    }

    #[test]
    fn function_code_from_response() {
        use Response::*;
        let responses = &[
            (ReadCoils(&[]), 1),
            (ReadDiscreteInputs(&[]), 2),
            (WriteSingleCoil(0x0), 5),
            (WriteMultipleCoils(0x0, 0x0), 0x0F),
            (ReadInputRegisters(&[]), 0x04),
            (ReadHoldingRegisters(&[]), 0x03),
            (WriteSingleRegister(0, 0), 0x06),
            (WriteMultipleRegisters(0, 0), 0x10),
            (ReadWriteMultipleRegisters(&[]), 0x17),
            (Custom(FnCode::Custom(99), &[]), 99),
        ];
        for (req, expected) in responses {
            let code: u8 = FnCode::from(*req).into();
            assert_eq!(*expected, code);
        }
    }

}
