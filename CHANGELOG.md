<!-- SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Changelog

## v0.2.0 (2025-09-30)

- Added TCP client implementation
- Added `defmt` feature
- Added payload slice accessor to `Data`
- Made `MAX_FRAME_LEN` public
- Fixed compilation if `rtu` feature is disabled
- Switched to MSRV 1.85.0 and Edition 2024

## v0.1.1 (2025-02-23)

- Added Modbus RTU client implementation
- Made `Encode` trait public
- Check for empty frames
- Added `ReadExceptionStatus` at PDU Response

## v0.1.0 (2024-03-26)

- Initial release
