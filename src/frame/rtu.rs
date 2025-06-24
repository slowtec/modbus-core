use super::*;

/// Slave ID
pub type SlaveId = u8;

/// RTU header
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub slave: SlaveId,
}

/// RTU Request ADU
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestAdu<'r> {
    pub hdr: Header,
    pub pdu: RequestPdu<'r>,
}

/// RTU Response ADU
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseAdu<'r> {
    pub hdr: Header,
    pub pdu: ResponsePdu<'r>,
}
