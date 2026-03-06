// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! uProtocol Publisher 구현
//! 
//! 참조: fms-forwarder/src/main.rs

use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, error};

use up_rust::{
    communication::{CallOptions, Publisher, SimplePublisher, UPayload},
    StaticUriProvider, UTransport, UUri,
};
use up_transport_zenoh::UPTransportZenoh;

use super::config::UProtocolConfig;

/// PULLPIRI 상태 메시지 (간단한 구조)
#[derive(Clone, serde::Serialize, prost::Message)]
pub struct PullpiriStatus {
    #[prost(string, tag = "1")]
    pub vehicle_id: String,
    #[prost(int64, tag = "2")]
    pub timestamp: i64,
    // 추가 필드는 기존 settingsservice의 데이터 구조에 맞게 확장
}

/// uProtocol Status Publisher
pub struct StatusPublisher {
    publisher: Arc<SimplePublisher<Arc<dyn UTransport>>>,
    resource_id: u16,
    topic: String,
    _transport: Arc<dyn UTransport>,
}

impl StatusPublisher {
    pub async fn new(config: &UProtocolConfig) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Creating uProtocol StatusPublisher for topic: {}", config.topic);
        
        let uri = UUri::from_str(&config.topic)?;
        let uri_provider = Arc::new(StaticUriProvider::try_from(&uri)?);
        
        let zenoh_config = zenoh::Config::from_file(&config.zenoh_config_path)?;
        
        let transport: Arc<dyn UTransport> = Arc::new(
            UPTransportZenoh::new(zenoh_config, uri_provider.get_source_uri()).await?
        );

        let resource_id = u16::try_from(uri.resource_id)?;
        let publisher = Arc::new(SimplePublisher::new(transport.clone(), uri_provider));

        info!("uProtocol StatusPublisher created successfully");
        
        Ok(Self { 
            publisher, 
            resource_id,
            topic: config.topic.clone(),
            _transport: transport,
        })
    }

    pub async fn publish(&self, status: &PullpiriStatus) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Publishing status for vehicle: {}", status.vehicle_id);
        
        let payload = UPayload::try_from_protobuf(status.clone())?;
        
        self.publisher
            .publish(
                self.resource_id,
                CallOptions::for_publish(None, None, None),
                Some(payload),
            )
            .await?;
            
        Ok(())
    }
}
