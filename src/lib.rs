// SPDX-FileCopyrightText: Copyright (c) 2018-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]
#![no_std]

mod codec;
mod error;
mod frame;

pub use codec::rtu;
pub use codec::tcp;
pub use error::*;
pub use frame::*;
