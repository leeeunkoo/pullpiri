// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! uProtocol 연동 모듈
//!
//! # Feature Flag
//! 이 모듈은 `uprotocol` feature가 활성화되어야 사용 가능합니다.

#[cfg(feature = "uprotocol")]
mod config;
#[cfg(feature = "uprotocol")]
mod publisher;

#[cfg(feature = "uprotocol")]
pub use config::UProtocolConfig;
#[cfg(feature = "uprotocol")]
pub use publisher::StatusPublisher;
