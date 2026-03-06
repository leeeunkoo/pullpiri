// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! uProtocol 수신 모듈
//!
//! Cloud에서 Vehicle로 시나리오를 전달하는 uProtocol Subscriber 구현
//!
//! # Feature Flag
//! 이 모듈은 `uprotocol` feature가 활성화되어야 사용 가능합니다.

#[cfg(feature = "uprotocol")]
mod config;
#[cfg(feature = "uprotocol")]
mod listener;
#[cfg(feature = "uprotocol")]
mod subscriber;

#[cfg(feature = "uprotocol")]
pub use config::UProtocolConfig;
#[cfg(feature = "uprotocol")]
pub use listener::ScenarioDownload;
#[cfg(feature = "uprotocol")]
pub use subscriber::start_subscriber;
