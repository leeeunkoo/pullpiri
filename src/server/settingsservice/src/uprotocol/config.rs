// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

use std::env;

/// uProtocol 설정
#[derive(Debug, Clone)]
pub struct UProtocolConfig {
    pub zenoh_config_path: String,
    pub topic: String,
    pub interval_secs: u64,
}

impl UProtocolConfig {
    /// 환경변수에서 설정 로드
    pub fn from_env() -> Option<Self> {
        let zenoh_config_path = env::var("ZENOH_CONFIG").ok()?;

        let topic = env::var("UPROTOCOL_TOPIC")
            .unwrap_or_else(|_| "up://pullpiri-settings/D200/1/D200".to_string());

        let interval_secs = env::var("STATUS_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Some(Self {
            zenoh_config_path,
            topic,
            interval_secs,
        })
    }
}
