// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::fmt;

mod coils;
mod data;
pub(crate) mod rtu;
pub(crate) mod tcp;

pub use self::{coils::*, data::*};
use byteorder::{BigEndian, ByteOrder};

/// The location of all bytes that belong to the frame.
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLocation {
    /// The index where the frame starts
    pub start: usize,
    /// Number of bytes that belong to the frame
    pub size: usize,
}

impl FrameLocation {
    /// One past the last byte of the frame.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.start + self.size
    }
}

/// A Modbus function code.
///
/// It is represented by an unsigned 8 bit integer.
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionCode {
    /// Modbus Function Code: `01` (`0x01`).
    ReadCoils,

    /// Modbus Function Code: `02` (`0x02`).
    ReadDiscreteInputs,

    /// Modbus Function Code: `05` (`0x05`).
    WriteSingleCoil,

    /// Modbus Function Code: `06` (`0x06`).
    WriteSingleRegister,

    /// Modbus Function Code: `03` (`0x03`).
    ReadHoldingRegisters,

    /// Modbus Function Code: `04` (`0x04`).
    ReadInputRegisters,

    /// Modbus Function Code: `15` (`0x0F`).
    WriteMultipleCoils,

    /// Modbus Function Code: `16` (`0x10`).
    WriteMultipleRegisters,

    /// Modbus Function Code: `22` (`0x16`).
    MaskWriteRegister,

    /// Modbus Function Code: `23` (`0x17`).
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

    // TODO:
    // - ReadFileRecord
    // - WriteFileRecord
    // TODO:
    // - Read FifoQueue
    // - EncapsulatedInterfaceTransport
    // - CanOpenGeneralReferenceRequestAndResponsePdu
    // - ReadDeviceIdentification
    /// Custom Modbus Function Code.
    Custom(u8),
}

impl FunctionCode {
    /// Create a new [`FunctionCode`] with `value`.
    #[must_use]
    pub const fn new(value: u8) -> Self {
        match value {
            0x01 => Self::ReadCoils,
            0x02 => Self::ReadDiscreteInputs,
            0x05 => Self::WriteSingleCoil,
            0x06 => Self::WriteSingleRegister,
            0x03 => Self::ReadHoldingRegisters,
            0x04 => Self::ReadInputRegisters,
            0x0F => Self::WriteMultipleCoils,
            0x10 => Self::WriteMultipleRegisters,
            0x16 => Self::MaskWriteRegister,
            0x17 => Self::ReadWriteMultipleRegisters,
            #[cfg(feature = "rtu")]
            0x07 => Self::ReadExceptionStatus,
            #[cfg(feature = "rtu")]
            0x08 => Self::Diagnostics,
            #[cfg(feature = "rtu")]
            0x0B => Self::GetCommEventCounter,
            #[cfg(feature = "rtu")]
            0x0C => Self::GetCommEventLog,
            #[cfg(feature = "rtu")]
            0x11 => Self::ReportServerId,
            code => FunctionCode::Custom(code),
        }
    }

    /// Get the [`u8`] value of the current [`FunctionCode`].
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::ReadCoils => 0x01,
            Self::ReadDiscreteInputs => 0x02,
            Self::WriteSingleCoil => 0x05,
            Self::WriteSingleRegister => 0x06,
            Self::ReadHoldingRegisters => 0x03,
            Self::ReadInputRegisters => 0x04,
            Self::WriteMultipleCoils => 0x0F,
            Self::WriteMultipleRegisters => 0x10,
            Self::MaskWriteRegister => 0x16,
            Self::ReadWriteMultipleRegisters => 0x17,
            #[cfg(feature = "rtu")]
            Self::ReadExceptionStatus => 0x07,
            #[cfg(feature = "rtu")]
            Self::Diagnostics => 0x08,
            #[cfg(feature = "rtu")]
            Self::GetCommEventCounter => 0x0B,
            #[cfg(feature = "rtu")]
            Self::GetCommEventLog => 0x0C,
            #[cfg(feature = "rtu")]
            Self::ReportServerId => 0x11,
            Self::Custom(code) => code,
        }
    }
}

impl fmt::Display for FunctionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value().fmt(f)
    }
}

/// A Modbus sub-function code is represented by an unsigned 16 bit integer.
#[cfg(feature = "rtu")]
pub(crate) type SubFunctionCode = u16;

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

/// Raw PDU data
type RawData<'r> = &'r [u8];

/// A request represents a message from the client (master) to the server (slave).
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request<'r> {
    ReadCoils(Address, Quantity),
    ReadDiscreteInputs(Address, Quantity),
    WriteSingleCoil(Address, Coil),
    WriteMultipleCoils(Address, Coils<'r>),
    ReadInputRegisters(Address, Quantity),
    ReadHoldingRegisters(Address, Quantity),
    WriteSingleRegister(Address, Word),
    WriteMultipleRegisters(Address, Data<'r>),
    ReadWriteMultipleRegisters(Address, Quantity, Address, Data<'r>),
    #[cfg(feature = "rtu")]
    ReadExceptionStatus,
    #[cfg(feature = "rtu")]
    Diagnostics(SubFunctionCode, Data<'r>),
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
    Custom(FunctionCode, &'r [u8]),
}

/// A server (slave) exception response.
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionResponse {
    pub function: FunctionCode,
    pub exception: Exception,
}

/// Represents a message from the client (slave) to the server (master).
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestPdu<'r>(pub Request<'r>);

/// Represents a message from the server (slave) to the client (master).
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponsePdu<'r>(pub Result<Response<'r>, ExceptionResponse>);

#[cfg(feature = "rtu")]
type Status = u16;
#[cfg(feature = "rtu")]
type EventCount = u16;
#[cfg(feature = "rtu")]
type MessageCount = u16;

/// The response data of a successful request.
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response<'r> {
    ReadCoils(Coils<'r>),
    ReadDiscreteInputs(Coils<'r>),
    WriteSingleCoil(Address, Coil),
    WriteMultipleCoils(Address, Quantity),
    ReadInputRegisters(Data<'r>),
    ReadHoldingRegisters(Data<'r>),
    WriteSingleRegister(Address, Word),
    WriteMultipleRegisters(Address, Quantity),
    ReadWriteMultipleRegisters(Data<'r>),
    #[cfg(feature = "rtu")]
    ReadExceptionStatus(u8),
    #[cfg(feature = "rtu")]
    Diagnostics(Data<'r>),
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
    Custom(FunctionCode, &'r [u8]),
}

impl<'r> From<Request<'r>> for FunctionCode {
    fn from(r: Request<'r>) -> Self {
        use Request as R;

        match r {
            R::ReadCoils(_, _) => Self::ReadCoils,
            R::ReadDiscreteInputs(_, _) => Self::ReadDiscreteInputs,
            R::WriteSingleCoil(_, _) => Self::WriteSingleCoil,
            R::WriteMultipleCoils(_, _) => Self::WriteMultipleCoils,
            R::ReadInputRegisters(_, _) => Self::ReadInputRegisters,
            R::ReadHoldingRegisters(_, _) => Self::ReadHoldingRegisters,
            R::WriteSingleRegister(_, _) => Self::WriteSingleRegister,
            R::WriteMultipleRegisters(_, _) => Self::WriteMultipleRegisters,
            R::ReadWriteMultipleRegisters(_, _, _, _) => Self::ReadWriteMultipleRegisters,
            #[cfg(feature = "rtu")]
            R::ReadExceptionStatus => Self::ReadExceptionStatus,
            #[cfg(feature = "rtu")]
            R::Diagnostics(_, _) => Self::Diagnostics,
            #[cfg(feature = "rtu")]
            R::GetCommEventCounter => Self::GetCommEventCounter,
            #[cfg(feature = "rtu")]
            R::GetCommEventLog => Self::GetCommEventLog,
            #[cfg(feature = "rtu")]
            R::ReportServerId => Self::ReportServerId,
            R::Custom(code, _) => code,
        }
    }
}

impl<'r> From<Response<'r>> for FunctionCode {
    fn from(r: Response<'r>) -> Self {
        use Response as R;

        match r {
            R::ReadCoils(_) => Self::ReadCoils,
            R::ReadDiscreteInputs(_) => Self::ReadDiscreteInputs,
            R::WriteSingleCoil(_, _) => Self::WriteSingleCoil,
            R::WriteMultipleCoils(_, _) => Self::WriteMultipleCoils,
            R::ReadInputRegisters(_) => Self::ReadInputRegisters,
            R::ReadHoldingRegisters(_) => Self::ReadHoldingRegisters,
            R::WriteSingleRegister(_, _) => Self::WriteSingleRegister,
            R::WriteMultipleRegisters(_, _) => Self::WriteMultipleRegisters,
            R::ReadWriteMultipleRegisters(_) => Self::ReadWriteMultipleRegisters,
            #[cfg(feature = "rtu")]
            R::ReadExceptionStatus(_) => Self::ReadExceptionStatus,
            #[cfg(feature = "rtu")]
            R::Diagnostics(_) => Self::Diagnostics,
            #[cfg(feature = "rtu")]
            R::GetCommEventCounter(_, _) => Self::GetCommEventCounter,
            #[cfg(feature = "rtu")]
            R::GetCommEventLog(_, _, _, _) => Self::GetCommEventLog,
            #[cfg(feature = "rtu")]
            R::ReportServerId(_, _) => Self::ReportServerId,
            R::Custom(code, _) => code,
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

impl Exception {
    const fn get_name(self) -> &'static str {
        match self {
            Self::IllegalFunction => "Illegal function",
            Self::IllegalDataAddress => "Illegal data address",
            Self::IllegalDataValue => "Illegal data value",
            Self::ServerDeviceFailure => "Server device failure",
            Self::Acknowledge => "Acknowledge",
            Self::ServerDeviceBusy => "Server device busy",
            Self::MemoryParityError => "Memory parity error",
            Self::GatewayPathUnavailable => "Gateway path unavailable",
            Self::GatewayTargetDevice => "Gateway target device failed to respond",
        }
    }
}

impl fmt::Display for Exception {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

#[cfg(all(feature = "defmt", target_os = "none"))]
impl defmt::Format for Exception {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", self.get_name())
    }
}

impl Request<'_> {
    /// Number of bytes required for a serialized PDU frame.
    #[must_use]
    pub const fn pdu_len(&self) -> usize {
        match *self {
            Self::ReadCoils(_, _)
            | Self::ReadDiscreteInputs(_, _)
            | Self::ReadInputRegisters(_, _)
            | Self::ReadHoldingRegisters(_, _)
            | Self::WriteSingleRegister(_, _)
            | Self::WriteSingleCoil(_, _) => 5,
            Self::WriteMultipleCoils(_, coils) => 6 + coils.packed_len(),
            Self::WriteMultipleRegisters(_, words) => 6 + words.data.len(),
            Self::ReadWriteMultipleRegisters(_, _, _, words) => 10 + words.data.len(),
            Self::Custom(_, data) => 1 + data.len(),
            #[cfg(feature = "rtu")]
            _ => todo!(), // TODO
        }
    }
}

impl Response<'_> {
    /// Number of bytes required for a serialized PDU frame.
    #[must_use]
    pub const fn pdu_len(&self) -> usize {
        match *self {
            Self::ReadCoils(coils) | Self::ReadDiscreteInputs(coils) => 2 + coils.packed_len(),
            Self::WriteSingleCoil(_, _) => 5,
            Self::WriteMultipleCoils(_, _)
            | Self::WriteMultipleRegisters(_, _)
            | Self::WriteSingleRegister(_, _) => 5,
            Self::ReadInputRegisters(words)
            | Self::ReadHoldingRegisters(words)
            | Self::ReadWriteMultipleRegisters(words) => 2 + words.len() * 2,
            Self::Custom(_, data) => 1 + data.len(),
            #[cfg(feature = "rtu")]
            Self::ReadExceptionStatus(_) => 2,
            #[cfg(feature = "rtu")]
            _ => unimplemented!(), // TODO
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn function_code_into_u8() {
        let x: u8 = FunctionCode::WriteMultipleCoils.value();
        assert_eq!(x, 15);
        let x: u8 = FunctionCode::Custom(0xBB).value();
        assert_eq!(x, 0xBB);
    }

    #[test]
    fn function_code_from_u8() {
        assert_eq!(FunctionCode::new(15), FunctionCode::WriteMultipleCoils);
        assert_eq!(FunctionCode::new(0xBB), FunctionCode::Custom(0xBB));
    }

    #[test]
    fn function_code_from_request() {
        use Request::*;
        let requests = &[
            (ReadCoils(0, 0), 1),
            (ReadDiscreteInputs(0, 0), 2),
            (WriteSingleCoil(0, true), 5),
            (
                WriteMultipleCoils(
                    0,
                    Coils {
                        quantity: 0,
                        data: &[],
                    },
                ),
                0x0F,
            ),
            (ReadInputRegisters(0, 0), 0x04),
            (ReadHoldingRegisters(0, 0), 0x03),
            (WriteSingleRegister(0, 0), 0x06),
            (
                WriteMultipleRegisters(
                    0,
                    Data {
                        quantity: 0,
                        data: &[],
                    },
                ),
                0x10,
            ),
            (
                ReadWriteMultipleRegisters(
                    0,
                    0,
                    0,
                    Data {
                        quantity: 0,
                        data: &[],
                    },
                ),
                0x17,
            ),
            (Custom(FunctionCode::Custom(88), &[]), 88),
        ];
        for (req, expected) in requests {
            let code: u8 = FunctionCode::from(*req).value();
            assert_eq!(*expected, code);
        }
    }

    #[test]
    fn function_code_from_response() {
        use Response::*;
        let responses = &[
            (
                ReadCoils(Coils {
                    quantity: 0,
                    data: &[],
                }),
                1,
            ),
            (
                ReadDiscreteInputs(Coils {
                    quantity: 0,
                    data: &[],
                }),
                2,
            ),
            (WriteSingleCoil(0x0, false), 5),
            (WriteMultipleCoils(0x0, 0x0), 0x0F),
            (
                ReadInputRegisters(Data {
                    quantity: 0,
                    data: &[],
                }),
                0x04,
            ),
            (
                ReadHoldingRegisters(Data {
                    quantity: 0,
                    data: &[],
                }),
                0x03,
            ),
            (WriteSingleRegister(0, 0), 0x06),
            (WriteMultipleRegisters(0, 0), 0x10),
            (
                ReadWriteMultipleRegisters(Data {
                    quantity: 0,
                    data: &[],
                }),
                0x17,
            ),
            (Custom(FunctionCode::Custom(99), &[]), 99),
        ];
        for (req, expected) in responses {
            let code: u8 = FunctionCode::from(*req).value();
            assert_eq!(*expected, code);
        }
    }

    #[test]
    fn test_request_pdu_len() {
        assert_eq!(Request::ReadCoils(0x12, 5).pdu_len(), 5);
        assert_eq!(Request::WriteSingleRegister(0x12, 0x33).pdu_len(), 5);
        let buf = &mut [0, 0];
        assert_eq!(
            Request::WriteMultipleCoils(0, Coils::from_bools(&[true, false], buf).unwrap())
                .pdu_len(),
            7
        );
        // TODO: extend test
    }

    #[test]
    fn test_response_pdu_len() {
        let buf = &mut [0, 0];
        assert_eq!(
            Response::ReadCoils(Coils::from_bools(&[true], buf).unwrap()).pdu_len(),
            3
        );
        // TODO: extend test
    }

    #[test]
    fn frame_location_end() {
        assert_eq!(FrameLocation { start: 0, size: 3 }.end(), 3);
        assert_eq!(FrameLocation { start: 2, size: 3 }.end(), 5);
        assert_eq!(FrameLocation { start: 2, size: 0 }.end(), 2);
    }
}
