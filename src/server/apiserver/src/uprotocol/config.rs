// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! uProtocol 설정 관리
//!
//! 환경변수를 통한 uProtocol subscriber 설정

use std::env;

/// uProtocol Subscriber 설정
#[derive(Debug, Clone)]
pub struct UProtocolConfig {
    /// Zenoh 설정 파일 경로
    pub zenoh_config_path: String,
    /// 구독할 토픽 필터
    pub topic_filter: String,
    /// 로컬 서비스 URI
    pub local_uri: String,
}

impl UProtocolConfig {
    /// 환경변수에서 설정 로드
    ///
    /// # Returns
    /// - `Some(config)` - ZENOH_CONFIG가 설정된 경우
    /// - `None` - ZENOH_CONFIG가 없으면 uProtocol 비활성화
    pub fn from_env() -> Option<Self> {
        // ZENOH_CONFIG가 없으면 uProtocol subscriber 비활성화
        let zenoh_config_path = env::var("ZENOH_CONFIG").ok()?;

        let topic_filter = env::var("UPROTOCOL_TOPIC_FILTER")
            .unwrap_or_else(|_| "up://*/pullpiri/scenario/#".to_string());

        let local_uri = env::var("UPROTOCOL_LOCAL_URI")
            .unwrap_or_else(|_| "up://pullpiri-api/D301/1/0".to_string());

        Some(Self {
            zenoh_config_path,
            topic_filter,
            local_uri,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// Test from_env returns None when ZENOH_CONFIG is not set
    #[test]
    fn test_from_env_no_zenoh_config() {
        // Ensure ZENOH_CONFIG is not set
        env::remove_var("ZENOH_CONFIG");

        let result = UProtocolConfig::from_env();
        assert!(result.is_none());
    }

    /// Test from_env returns Some when ZENOH_CONFIG is set
    #[test]
    fn test_from_env_with_zenoh_config() {
        // Set required env var
        env::set_var("ZENOH_CONFIG", "/tmp/test-zenoh.json");

        let result = UProtocolConfig::from_env();
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.zenoh_config_path, "/tmp/test-zenoh.json");
        // Default values should be used
        assert!(config.topic_filter.contains("pullpiri"));

        // Cleanup
        env::remove_var("ZENOH_CONFIG");
    }

    /// Test from_env uses custom values when all env vars are set
    #[test]
    fn test_from_env_with_all_custom_values() {
        env::set_var("ZENOH_CONFIG", "/custom/zenoh.json");
        env::set_var("UPROTOCOL_TOPIC_FILTER", "up://custom/topic");
        env::set_var("UPROTOCOL_LOCAL_URI", "up://custom/uri");

        let result = UProtocolConfig::from_env();
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.zenoh_config_path, "/custom/zenoh.json");
        assert_eq!(config.topic_filter, "up://custom/topic");
        assert_eq!(config.local_uri, "up://custom/uri");

        // Cleanup
        env::remove_var("ZENOH_CONFIG");
        env::remove_var("UPROTOCOL_TOPIC_FILTER");
        env::remove_var("UPROTOCOL_LOCAL_URI");
    }
}
