/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! VSS (Vehicle Signal Specification) data management module
//!
//! This module provides VSS data subscription and management functionality
//! following the same pattern as the DDS module.

use anyhow::anyhow;
use common::logd;
use common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc::{self, Sender};

// Re-export VSS types from vss module
use crate::vss::{VssSubscriber, VssTrigger, VssValue};

/// VSS data structure (mirrors DdsData structure for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VssData {
    /// VSS signal path (e.g., "Vehicle.Speed")
    pub name: String,
    /// String representation of the value
    pub value: String,
    /// Fields map for filter compatibility
    pub fields: HashMap<String, String>,
}

impl From<VssTrigger> for VssData {
    fn from(trigger: VssTrigger) -> Self {
        let value_string = match trigger.value {
            VssValue::String(s) => s,
            VssValue::Bool(b) => b.to_string(),
            VssValue::Int32(i) => i.to_string(),
            VssValue::Int64(i) => i.to_string(),
            VssValue::Float(f) => f.to_string(),
            VssValue::Double(d) => d.to_string(),
            VssValue::Unknown => "unknown".to_string(),
        };

        let mut fields = HashMap::new();
        fields.insert("value".to_string(), value_string.clone());

        VssData {
            name: trigger.path,
            value: value_string,
            fields,
        }
    }
}

/// VSS Manager - Manages VSS signal subscriptions
///
/// Follows the same pattern as DdsManager for consistency
pub struct VssManager {
    /// VSS subscriber instance
    subscriber: Option<VssSubscriber>,
    /// Active subscriptions (VSS path → trigger sender)
    subscriptions: HashMap<String, Sender<VssTrigger>>,
    /// Channel for sending VSS data to FilterGatewayManager
    tx: Sender<VssData>,
    /// Databroker URI
    databroker_uri: String,
}

impl VssManager {
    /// Create new VSS manager
    ///
    /// # Arguments
    ///
    /// * `tx` - Channel sender for VssData
    /// * `databroker_uri` - Kuksa Databroker URI (e.g., "http://localhost:55555")
    pub fn new(tx: Sender<VssData>, databroker_uri: String) -> Self {
        logd!(3, "Creating VssManager for Databroker at {}", databroker_uri);

        Self {
            subscriber: None,
            subscriptions: HashMap::new(),
            tx,
            databroker_uri,
        }
    }

    /// Initialize VSS manager and create subscriber
    ///
    /// # Returns
    ///
    /// `Result<()>` - Success or error
    pub async fn init(&mut self) -> Result<()> {
        logd!(2, "Initializing VssManager");

        // Create VssSubscriber
        let subscriber = VssSubscriber::new(&self.databroker_uri)
            .await
            .map_err(|e| anyhow!("Failed to create VssSubscriber: {}", e))?;

        self.subscriber = Some(subscriber);
        logd!(2, "VssManager initialized successfully");

        Ok(())
    }

    /// Create a subscription for a VSS signal path
    ///
    /// # Arguments
    ///
    /// * `vss_path` - VSS signal path (e.g., "Vehicle.Speed")
    ///
    /// # Returns
    ///
    /// `Result<()>` - Success or error
    pub async fn create_vss_subscription(&mut self, vss_path: String) -> Result<()> {
        // Check if already subscribed
        if self.subscriptions.contains_key(&vss_path) {
            logd!(4, "Already subscribed to VSS path: {}", vss_path);
            return Ok(());
        }

        logd!(
            2,
            "VssManager - Creating subscription for VSS path: {}",
            vss_path
        );

        // Create channel for VssTrigger
        let (trigger_tx, mut trigger_rx) = mpsc::channel::<VssTrigger>(100);
        let data_tx = self.tx.clone();
        let path_clone = vss_path.clone();

        // Spawn background task to convert VssTrigger to VssData
        tokio::spawn(async move {
            logd!(3, "VSS data converter task started for: {}", path_clone);

            while let Some(trigger) = trigger_rx.recv().await {
                let vss_data = VssData::from(trigger);

                logd!(
                    3,
                    "Converting VSS trigger to VssData: {} = {}",
                    vss_data.name,
                    vss_data.value
                );

                if let Err(e) = data_tx.send(vss_data).await {
                    logd!(
                        5,
                        "Failed to send VssData for {}: {}",
                        path_clone,
                        e
                    );
                    break;
                }
            }

            logd!(3, "VSS data converter task stopped for: {}", path_clone);
        });

        // Subscribe using VssSubscriber
        if let Some(ref mut subscriber) = self.subscriber {
            subscriber
                .subscribe(vec![vss_path.clone()], trigger_tx.clone())
                .await
                .map_err(|e| anyhow!("Failed to subscribe to {}: {}", vss_path, e))?;

            logd!(2, "Successfully subscribed to VSS path: {}", vss_path);
        } else {
            return Err(anyhow!("VssSubscriber not initialized").into());
        }

        self.subscriptions.insert(vss_path.clone(), trigger_tx);
        Ok(())
    }

    /// Remove subscription for a VSS signal path
    ///
    /// # Arguments
    ///
    /// * `vss_path` - VSS signal path to unsubscribe
    ///
    /// # Returns
    ///
    /// `Result<()>` - Success or error
    pub async fn remove_subscription(&mut self, vss_path: &str) -> Result<()> {
        if self.subscriptions.remove(vss_path).is_some() {
            logd!(2, "Removed VSS subscription for: {}", vss_path);
        } else {
            logd!(4, "No active subscription found for: {}", vss_path);
        }

        Ok(())
    }

    /// Get the VssData sender
    ///
    /// # Returns
    ///
    /// Sender for VssData
    pub fn get_sender(&self) -> Sender<VssData> {
        self.tx.clone()
    }

    /// List currently subscribed VSS paths
    ///
    /// # Returns
    ///
    /// Vector of subscribed VSS paths
    pub fn list_subscriptions(&self) -> Vec<String> {
        self.subscriptions.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_vss_manager_new() {
        let (tx, _rx) = mpsc::channel(10);
        let vss_manager = VssManager::new(tx, "http://localhost:55555".to_string());

        assert_eq!(vss_manager.databroker_uri, "http://localhost:55555");
        assert!(vss_manager.subscriber.is_none());
        assert!(vss_manager.subscriptions.is_empty());
    }

    #[test]
    fn test_vss_data_from_trigger_string() {
        let trigger = VssTrigger {
            path: "Vehicle.Speed".to_string(),
            value: VssValue::Double(65.5),
            timestamp: std::time::SystemTime::now(),
        };

        let vss_data = VssData::from(trigger);

        assert_eq!(vss_data.name, "Vehicle.Speed");
        assert_eq!(vss_data.value, "65.5");
        assert_eq!(vss_data.fields.get("value"), Some(&"65.5".to_string()));
    }

    #[test]
    fn test_vss_data_from_trigger_bool() {
        let trigger = VssTrigger {
            path: "Vehicle.IsMoving".to_string(),
            value: VssValue::Bool(true),
            timestamp: std::time::SystemTime::now(),
        };

        let vss_data = VssData::from(trigger);

        assert_eq!(vss_data.name, "Vehicle.IsMoving");
        assert_eq!(vss_data.value, "true");
    }

    #[tokio::test]
    async fn test_vss_manager_list_subscriptions() {
        let (tx, _rx) = mpsc::channel(10);
        let vss_manager = VssManager::new(tx, "http://localhost:55555".to_string());

        let subscriptions = vss_manager.list_subscriptions();
        assert!(subscriptions.is_empty());
    }
}
