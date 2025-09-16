// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;

/// Slave ID
pub type SlaveId = u8;

/// RTU header
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub slave: SlaveId,
}

/// RTU Request ADU
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestAdu<'r> {
    pub hdr: Header,
    pub pdu: RequestPdu<'r>,
}

/// RTU Response ADU
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseAdu<'r> {
    pub hdr: Header,
    pub pdu: ResponsePdu<'r>,
}
