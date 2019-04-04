/// modbus-core Error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Invalid coil value
    CoilValue(u16),
    /// Invalid buffer size
    BufferSize,
    /// Invalid function code
    FnCode(u8),
}
