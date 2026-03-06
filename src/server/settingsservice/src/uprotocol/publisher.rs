// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! uProtocol Publisher 구현
//!
//! Zenoh transport를 통한 uProtocol 메시지 발행

use std::sync::Arc;
use tracing::{debug, info};

use up_rust::{LocalUriProvider, StaticUriProvider, UMessageBuilder, UPayloadFormat, UTransport};
use up_transport_zenoh::{zenoh_config, UPTransportZenoh};

use super::config::UProtocolConfig;

/// uProtocol Status Publisher
pub struct StatusPublisher {
    transport: Arc<UPTransportZenoh>,
    uri_provider: Arc<StaticUriProvider>,
    resource_id: u16,
    vehicle_id: String,
}

impl StatusPublisher {
    pub async fn new(config: &UProtocolConfig) -> Result<Self, Box<dyn std::error::Error>> {
        info!(
            "Creating uProtocol StatusPublisher for topic: {}",
            config.topic
        );

        // Parse the topic URI to extract resource_id
        // Expected format: up://pullpiri-settings/D200/1/D200 or simplified version
        let parts: Vec<&str> = config
            .topic
            .trim_start_matches("up://")
            .split('/')
            .collect();

        // Extract resource_id from the last segment, or use default
        let resource_id_str = if parts.is_empty() {
            "D200"
        } else {
            parts.last().unwrap()
        };

        // Parse resource_id (may be hex like "D200" which is 53760 in decimal, or "0x8001")
        let resource_id: u16 = if resource_id_str.starts_with("0x") {
            u16::from_str_radix(&resource_id_str[2..], 16)?
        } else if resource_id_str.chars().all(|c| c.is_ascii_hexdigit())
            && resource_id_str.len() == 4
        {
            // Assume 4-character hex without 0x prefix
            u16::from_str_radix(resource_id_str, 16)?
        } else {
            // Try decimal parse, fallback to default with warning
            match resource_id_str.parse() {
                Ok(id) => id,
                Err(_) => {
                    info!(
                        "Failed to parse resource_id '{}', using default 0x8001",
                        resource_id_str
                    );
                    0x8001
                }
            }
        };

        // Create URI provider for settingsservice
        let uri_provider = Arc::new(StaticUriProvider::new("pullpiri-settings", 0xD200, 1));

        // Get authority from config or use default
        let authority = parts.first().unwrap_or(&"pullpiri-settings").to_string();

        // Load Zenoh configuration from file
        let zenoh_cfg = zenoh_config::Config::from_file(&config.zenoh_config_path)
            .map_err(|e| format!("Failed to load Zenoh config: {}", e))?;

        // Build the transport using the builder pattern
        let transport = UPTransportZenoh::builder(authority)?
            .with_config(zenoh_cfg)
            .build()
            .await?;

        info!(
            "uProtocol StatusPublisher created successfully (resource_id: 0x{:04X})",
            resource_id
        );

        // Note: VEHICLE_ID must be set before service startup
        // It is read once during publisher initialization and does not change at runtime
        let vehicle_id = std::env::var("VEHICLE_ID").unwrap_or_else(|_| "vehicle-001".to_string());

        Ok(Self {
            transport: Arc::new(transport),
            uri_provider,
            resource_id,
            vehicle_id,
        })
    }

    pub async fn publish_status(&self) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let data = format!(
            r#"{{"vehicle_id":"{}","timestamp":{}}}"#,
            self.vehicle_id, timestamp
        );

        debug!("Publishing status for vehicle: {}", self.vehicle_id);

        // Get the topic URI for this resource
        let topic_uri = self.uri_provider.get_resource_uri(self.resource_id);

        // Build and send the message
        let umessage = UMessageBuilder::publish(topic_uri)
            .build_with_payload(data, UPayloadFormat::UPAYLOAD_FORMAT_JSON)?;

        self.transport.send(umessage).await?;

        Ok(())
    }
}
