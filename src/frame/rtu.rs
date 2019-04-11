use super::*;

/// Slave ID
pub type SlaveId = u8;

/// RTU header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub slave: SlaveId,
}

/// RTU Request ADU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestAdu<'r> {
    pub hdr: Header,
    pub pdu: RequestPdu<'r>,
}

/// RTU Response ADU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseAdu<'r> {
    pub hdr: Header,
    pub pdu: ResponsePdu<'r>,
}
