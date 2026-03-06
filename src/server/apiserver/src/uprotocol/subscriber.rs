// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! uProtocol Subscriber 구현
//!
//! Zenoh transport를 통해 Cloud로부터 시나리오를 구독

use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use up_rust::{UTransport, UUri};
use up_transport_zenoh::{zenoh_config, UPTransportZenoh};

use super::config::UProtocolConfig;
use super::listener::{ScenarioDownload, ScenarioListener};

use common::logd;

/// uProtocol Subscriber 시작
///
/// # Arguments
/// * `config` - uProtocol 설정
/// * `scenario_sender` - 수신된 시나리오를 전달할 채널
///
/// # Returns
/// * `Ok(transport)` - 성공 시 transport 인스턴스 반환 (리스너 유지를 위해)
/// * `Err` - 초기화 실패
pub async fn start_subscriber(
    config: &UProtocolConfig,
    scenario_sender: Sender<ScenarioDownload>,
) -> Result<Arc<UPTransportZenoh>, Box<dyn std::error::Error + Send + Sync>> {
    logd!(
        2,
        "Starting uProtocol subscriber for: {}",
        config.topic_filter
    );

    // Zenoh 설정 로드
    let zenoh_cfg = zenoh_config::Config::from_file(&config.zenoh_config_path)
        .map_err(|e| format!("Failed to load Zenoh config: {}", e))?;

    // URI에서 authority 추출
    let parts: Vec<&str> = config
        .local_uri
        .trim_start_matches("up://")
        .split('/')
        .collect();
    let authority = parts.first().unwrap_or(&"pullpiri-api").to_string();

    // Transport 생성
    let transport = UPTransportZenoh::builder(authority)?
        .with_config(zenoh_cfg)
        .build()
        .await?;

    let transport = Arc::new(transport);

    // Topic filter URI 파싱
    let filter = UUri::from_str(&config.topic_filter)?;

    // Listener 등록
    let listener = Arc::new(ScenarioListener::new(scenario_sender));
    transport.register_listener(&filter, None, listener).await?;

    logd!(2, "uProtocol subscriber started successfully");

    Ok(transport)
}
