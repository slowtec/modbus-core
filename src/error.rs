/// modbus-core Error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Invalid coil value
    CoilValue,
    /// Invalid buffer size
    BufferSize,
}
