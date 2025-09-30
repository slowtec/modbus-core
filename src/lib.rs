// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]
#![no_std]
// Default lints
#![warn(unreachable_pub)]
#![warn(unsafe_code)]
#![warn(unused)]
// Clippy lints
#![warn(clippy::missing_const_for_fn)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)] // FIXME
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)] // TODO
#![allow(clippy::wildcard_imports)]

#[cfg(feature = "std")]
extern crate std;

mod codec;
mod error;
mod frame;
mod slave;

pub use codec::rtu;
pub use codec::tcp;
pub use codec::*;
pub use error::*;
pub use frame::*;
pub use slave::*;
