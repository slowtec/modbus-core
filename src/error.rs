use core::fmt;

/// modbus-core Error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Invalid coil value
    CoilValue(u16),
    /// Invalid buffer size
    BufferSize,
    /// Invalid function code
    FnCode(u8),
    /// Invalid exception code
    ExceptionCode(u8),
    /// Invalid exception function code
    ExceptionFnCode(u8),
    /// Invalid CRC
    Crc(u16, u16),
    /// Invalid byte count
    ByteCount(u8),
    /// Length Mismatch
    LengthMismatch(usize, usize),
    /// Protocol not Modbus
    ProtocolNotModbus(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CoilValue(v) => write!(f, "Invalid coil value: {v}"),
            Self::BufferSize => write!(f, "Invalid buffer size"),
            Self::FnCode(fn_code) => write!(f, "Invalid function code: 0x{fn_code:0>2X}"),
            Self::ExceptionCode(code) => write!(f, "Invalid exception code:0x {code:0>2X}"),
            Self::ExceptionFnCode(code) => {
                write!(f, "Invalid exception function code:0x {code:0>2X}")
            }
            Self::Crc(expected, actual) => write!(
                f,
                "Invalid CRC: expected = 0x{expected:0>4X}, actual = 0x{actual:0>4X}"
            ),
            Self::ByteCount(cnt) => write!(f, "Invalid byte count: {cnt}"),
            Self::LengthMismatch(length_field, pdu_len) => write!(
                f,
                "Length Mismatch: Length Field: {length_field}, PDU Len + 1: {pdu_len}"
            ),
            Self::ProtocolNotModbus(protocol_id) => {
                write!(f, "Protocol not Modbus(0), recieved {protocol_id} instead")
            }
        }
    }
}

#[cfg(all(feature = "defmt", target_os = "none"))]
impl defmt::Format for Error {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Self::CoilValue(v) => defmt::write!(f, "Invalid coil value: {}", v),
            Self::BufferSize => defmt::write!(f, "Invalid buffer size"),
            Self::FnCode(fn_code) => defmt::write!(f, "Invalid function code: {=u8:#04x}", fn_code),
            Self::ExceptionCode(code) => defmt::write!(f, "Invalid exception code: {=u8:#04x}", code),
            Self::ExceptionFnCode(code) => {
                defmt::write!(f, "Invalid exception function code: {=u8:#04x}", code)
            }
            Self::Crc(expected, actual) => defmt::write!(f,
                "Invalid CRC: expected = {=u16:#06x}, actual = {=u16:#06x}",
                expected, actual
            ),
            Self::ByteCount(cnt) => defmt::write!(f, "Invalid byte count: {}", cnt),
            Self::LengthMismatch(length_field, pdu_len) => defmt::write!(f,
                "Length Mismatch: Length Field: {}, PDU Len + 1: {}",
                length_field, pdu_len
            ),
            Self::ProtocolNotModbus(protocol_id) => {
                defmt::write!(f, "Protocol not Modbus(0), recieved {} instead", protocol_id)
            }
        }
    }
}