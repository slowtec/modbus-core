// SPDX-FileCopyrightText: Copyright (c) 2018-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc = include_str!("../README.md")]
#![no_std]
// Default lints
#![warn(unreachable_pub)]
#![warn(unsafe_code)]
#![warn(unused)]
// Clippy lints
#![warn(clippy::pedantic)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::wildcard_imports)]

mod codec;
mod error;
mod frame;

pub use codec::rtu;
pub use codec::tcp;
pub use error::*;
pub use frame::*;
