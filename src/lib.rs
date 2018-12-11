#![no_std]

/// A Modbus function code is represented by an unsigned 8 bit integer.
pub(crate) type FunctionCode = u8;

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
    Custom(FunctionCode, &'r [u8]),
}

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
    Custom(FunctionCode, &'r [u8]),
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
