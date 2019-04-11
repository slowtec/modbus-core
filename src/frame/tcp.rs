use super::*;

pub type TransactionId = u16;
pub type UnitId = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub transaction_id: TransactionId,
    pub unit_id: UnitId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestAdu<'r> {
    pub hdr: Header,
    pub pdu: RequestPdu<'r>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseAdu<'r> {
    pub hdr: Header,
    pub pdu: ResponsePdu<'r>,
}
