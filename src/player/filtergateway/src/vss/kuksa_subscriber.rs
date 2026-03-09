/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Kuksa.val Databroker gRPC subscription implementation
//!
//! This module provides the VssSubscriber client for subscribing to VSS signals
//! from Kuksa.val Databroker via gRPC.
//!
//! # Reference
//! Implementation pattern follows fms-forwarder/src/vehicle_abstraction.rs

use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

use kuksa_rust_sdk::kuksa::common::ClientTraitV2;
use kuksa_rust_sdk::kuksa::val::v2::KuksaClientV2;

use common::logd;

use super::types::{VssError, VssTrigger, VssValue};

/// Kuksa.val Databroker VSS subscriber
///
/// Manages connections and subscriptions to Kuksa.val Databroker for receiving
/// vehicle signal updates.
pub struct VssSubscriber {
    client: KuksaClientV2,
    #[allow(dead_code)]
    databroker_uri: String,
    subscribed_paths: Arc<RwLock<Vec<String>>>,
}

impl VssSubscriber {
    /// Create a new VssSubscriber
    ///
    /// # Arguments
    ///
    /// * `databroker_uri` - Kuksa.val Databroker URI (e.g., "http://databroker:55556")
    ///
    /// # Returns
    ///
    /// `Result<Self, VssError>` - New subscriber instance or error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let subscriber = VssSubscriber::new("http://databroker:55556").await?;
    /// ```
    ///
    /// # Note
    ///
    /// Integration tests for this method require a running Kuksa.val Databroker instance.
    /// Unit tests verify the core type conversion logic via `extract_value` tests.
    pub async fn new(databroker_uri: &str) -> Result<Self, VssError> {
        logd!(1,
            "Creating VssSubscriber for Databroker at {}",
            databroker_uri
        );

        let uri = http::Uri::try_from(databroker_uri)
            .map_err(|e| VssError::InvalidUri(format!("{}", e)))?;

        let client = KuksaClientV2::new(uri);

        logd!(1, "VssSubscriber created successfully");

        Ok(Self {
            client,
            databroker_uri: databroker_uri.to_string(),
            subscribed_paths: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Subscribe to VSS signal paths and send triggers on changes
    ///
    /// # Arguments
    ///
    /// * `vss_paths` - List of VSS paths to subscribe to
    /// * `trigger_sender` - Channel to send triggers when signals change
    ///
    /// # Returns
    ///
    /// `Result<(), VssError>` - Success or error
    ///
    /// # Note
    ///
    /// This method spawns a background task for receiving subscription updates.
    /// Integration tests require a running Kuksa.val Databroker instance.
    #[allow(dead_code)]
    pub async fn subscribe(
        &mut self,
        vss_paths: Vec<String>,
        trigger_sender: Sender<VssTrigger>,
    ) -> Result<(), VssError> {
        if vss_paths.is_empty() {
            logd!(1, "No VSS paths to subscribe");
            return Ok(());
        }

        logd!(1,
            "Subscribing to {} VSS paths: {:?}",
            vss_paths.len(),
            vss_paths
        );

        // Store subscribed paths
        {
            let mut paths = self.subscribed_paths.write().await;
            paths.extend(vss_paths.clone());
        }

        // Subscribe pattern following fms-forwarder/vehicle_abstraction.rs
        match self.client.subscribe(vss_paths.clone(), None, None).await {
            Ok(mut response) => {
                let paths_for_log = vss_paths.clone();

                // Spawn background task to receive messages
                tokio::task::spawn(async move {
                    logd!(2,
                        "VSS subscription stream started for paths: {:?}",
                        paths_for_log
                    );

                    loop {
                        match response.message().await {
                            Ok(Some(resp)) => {
                                for (path, datapoint) in resp.entries {
                                    if let Some(value) = Self::extract_value(&datapoint) {
                                        let trigger = VssTrigger {
                                            path: path.clone(),
                                            value,
                                            timestamp: SystemTime::now(),
                                        };

                                        logd!(2,
                                            "VSS trigger: {} = {:?}",
                                            path,
                                            trigger.value
                                        );

                                        if let Err(e) = trigger_sender.send(trigger).await {
                                            logd!(1, "Failed to send VSS trigger: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                logd!(1, "VSS subscription stream ended");
                                break;
                            }
                            Err(e) => {
                                logd!(1, "VSS subscription error: {}", e);
                                break;
                            }
                        }
                    }
                });

                Ok(())
            }
            Err(e) => {
                logd!(1, "Failed to subscribe to VSS paths: {}", e);
                Err(VssError::Subscribe(format!("{}", e)))
            }
        }
    }

    /// Get a single VSS value
    ///
    /// # Arguments
    ///
    /// * `path` - VSS signal path to query
    ///
    /// # Returns
    ///
    /// `Option<VssValue>` - The current value or None if not available
    ///
    /// # Note
    ///
    /// Integration tests require a running Kuksa.val Databroker instance.
    #[allow(dead_code)]
    pub async fn get_value(&mut self, path: &str) -> Option<VssValue> {
        logd!(2, "Getting VSS value for: {}", path);

        match self.client.get_values(vec![path.to_string()]).await {
            Ok(response) => response.first().and_then(Self::extract_value),
            Err(e) => {
                logd!(1, "Failed to get VSS value for {}: {}", path, e);
                None
            }
        }
    }

    /// Extract VssValue from Kuksa Datapoint
    ///
    /// # Arguments
    ///
    /// * `datapoint` - Kuksa datapoint from response
    ///
    /// # Returns
    ///
    /// `Option<VssValue>` - Extracted value or None
    fn extract_value(datapoint: &kuksa_rust_sdk::v2_proto::Datapoint) -> Option<VssValue> {
        datapoint.value.as_ref().and_then(|v| {
            v.typed_value.as_ref().map(|tv| {
                use kuksa_rust_sdk::v2_proto::value::TypedValue;
                match tv {
                    TypedValue::Bool(b) => VssValue::Bool(*b),
                    TypedValue::Int32(i) => VssValue::Int32(*i),
                    TypedValue::Int64(i) => VssValue::Int64(*i),
                    TypedValue::Float(f) => VssValue::Float(*f),
                    TypedValue::Double(d) => VssValue::Double(*d),
                    TypedValue::String(s) => VssValue::String(s.clone()),
                    _ => VssValue::Unknown,
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_value_bool() {
        use kuksa_rust_sdk::v2_proto::value::TypedValue;
        use kuksa_rust_sdk::v2_proto::{Datapoint, Value};

        let datapoint = Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::Bool(true)),
            }),
        };

        let result = VssSubscriber::extract_value(&datapoint);
        assert_eq!(result, Some(VssValue::Bool(true)));
    }

    #[test]
    fn test_extract_value_int32() {
        use kuksa_rust_sdk::v2_proto::value::TypedValue;
        use kuksa_rust_sdk::v2_proto::{Datapoint, Value};

        let datapoint = Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::Int32(42)),
            }),
        };

        let result = VssSubscriber::extract_value(&datapoint);
        assert_eq!(result, Some(VssValue::Int32(42)));
    }

    #[test]
    fn test_extract_value_string() {
        use kuksa_rust_sdk::v2_proto::value::TypedValue;
        use kuksa_rust_sdk::v2_proto::{Datapoint, Value};

        let datapoint = Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::String("test".to_string())),
            }),
        };

        let result = VssSubscriber::extract_value(&datapoint);
        assert_eq!(result, Some(VssValue::String("test".to_string())));
    }

    #[test]
    fn test_extract_value_none() {
        use kuksa_rust_sdk::v2_proto::Datapoint;

        let datapoint = Datapoint {
            timestamp: None,
            value: None,
        };

        let result = VssSubscriber::extract_value(&datapoint);
        assert_eq!(result, None);
    }
}
