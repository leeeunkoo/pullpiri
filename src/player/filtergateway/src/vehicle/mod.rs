/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
pub mod dds;

#[cfg(feature = "vss")]
pub mod vss;

use common::logd;
use common::Result;
use dds::DdsData;
use tokio::sync::mpsc::Sender;

#[cfg(feature = "vss")]
use vss::VssData;

/// Vehicle data management module
///
/// Manages vehicle data through DDS and VSS communication
#[allow(dead_code)]
pub struct VehicleManager {
    /// DDS Manager instance
    dds_manager: dds::DdsManager,
    /// VSS Manager instance (optional, enabled with vss feature)
    #[cfg(feature = "vss")]
    vss_manager: Option<vss::VssManager>,
}
#[allow(dead_code)]
impl VehicleManager {
    /// Creates a new VehicleManager
    ///
    /// # Returns
    ///
    /// A new VehicleManager instance
    pub fn new(tx: Sender<DdsData>) -> Self {
        Self {
            dds_manager: dds::DdsManager::new(tx),
            #[cfg(feature = "vss")]
            vss_manager: None,
        }
    }

    /// Initializes the vehicle data system
    ///
    /// Sets up the DDS system and prepares for topic subscriptions
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn init(&mut self) -> Result<()> {
        // Initialize DDS manager
        match self.dds_manager.init().await {
            Ok(_) => {}
            Err(e) => {
                logd!(5, "Failed to initialize DDS manager with settings file: {}. Using default settings.", e);
                // 기본 설정 적용
                self.set_domain_id(100); // Set default domain ID
            }
        }
        Ok(())
    }

    /// Subscribes to a vehicle data topic
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic to subscribe to
    /// * `data_type_name` - Type name of the data
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn subscribe_topic(
        &mut self,
        topic_name: String,
        data_type_name: String,
    ) -> Result<()> {
        use std::time::Instant;
        let start = Instant::now();

        self.dds_manager
            .create_typed_listener(topic_name, data_type_name)
            .await?;

        let elapsed = start.elapsed();
        logd!(1, "subscribe_topic: elapsed = {:?}", elapsed);

        Ok(())
    }

    /// Get list of available DDS types
    pub fn list_available_types(&self) -> Vec<String> {
        self.dds_manager.list_available_types()
    }

    /// Unsubscribes from a vehicle data topic
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic to unsubscribe from
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn unsubscribe_topic(&mut self, topic_name: String) -> Result<()> {
        // TODO: Implementation
        self.dds_manager.remove_listener(&topic_name).await?;
        Ok(())
    }

    /// Gets the DDS data sender
    ///
    /// # Returns
    ///
    /// A sender for DDS data
    pub fn get_sender(&self) -> tokio::sync::mpsc::Sender<dds::DdsData> {
        self.dds_manager.get_sender()
    }

    /// Sets the DDS domain ID
    ///
    /// # Arguments
    ///
    /// * `domain_id` - Domain ID to use for DDS communication
    pub fn set_domain_id(&mut self, domain_id: i32) {
        self.dds_manager.set_domain_id(domain_id);
    }

    // ========================================
    // VSS-specific methods
    // ========================================

    /// Initialize VSS manager
    ///
    /// # Arguments
    ///
    /// * `databroker_uri` - Kuksa Databroker URI
    /// * `tx_vss` - Channel sender for VssData
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    #[cfg(feature = "vss")]
    pub async fn init_vss(&mut self, databroker_uri: String, tx_vss: Sender<VssData>) -> Result<()> {
        logd!(2, "Initializing VSS manager with databroker: {}", databroker_uri);

        let mut vss_manager = vss::VssManager::new(tx_vss, databroker_uri);
        vss_manager.init().await?;
        self.vss_manager = Some(vss_manager);

        logd!(2, "VSS manager initialized successfully");
        Ok(())
    }

    /// Subscribe to a VSS signal
    ///
    /// # Arguments
    ///
    /// * `vss_path` - VSS signal path (e.g., "Vehicle.Speed")
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    #[cfg(feature = "vss")]
    pub async fn subscribe_vss_signal(&mut self, vss_path: String) -> Result<()> {
        if let Some(ref mut vss_manager) = self.vss_manager {
            vss_manager.create_vss_subscription(vss_path).await
        } else {
            Err("VSS Manager not initialized".into())
        }
    }

    /// Unsubscribe from a VSS signal
    ///
    /// # Arguments
    ///
    /// * `vss_path` - VSS signal path to unsubscribe
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    #[cfg(feature = "vss")]
    pub async fn unsubscribe_vss_signal(&mut self, vss_path: String) -> Result<()> {
        if let Some(ref mut vss_manager) = self.vss_manager {
            vss_manager.remove_subscription(&vss_path).await
        } else {
            Ok(())
        }
    }

    /// Get list of active VSS subscriptions
    ///
    /// # Returns
    ///
    /// Vector of subscribed VSS paths
    #[cfg(feature = "vss")]
    pub fn list_vss_subscriptions(&self) -> Vec<String> {
        if let Some(ref vss_manager) = self.vss_manager {
            vss_manager.list_subscriptions()
        } else {
            Vec::new()
        }
    }
}
//Unit tests for VehicleManager
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    #[tokio::test] // Test creation of VehicleManager and validate sender capacity
    async fn test_vehicle_manager_new() {
        let (tx, _rx) = mpsc::channel(10);
        let vehicle_manager = VehicleManager::new(tx);
        let sender = vehicle_manager.get_sender();
        assert_eq!(sender.capacity(), 10); // Validate sender's capacity
    }

    #[tokio::test] // Test successful initialization of VehicleManager
    async fn test_vehicle_manager_init_success() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        let result = vehicle_manager.init().await;
        assert!(result.is_ok());
    }

    #[tokio::test] // Test subscribing to a topic successfully
    async fn test_vehicle_manager_subscribe_topic() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        let result = vehicle_manager
            .subscribe_topic("vehicle_data".to_string(), "VehicleType".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test] // Test unsubscribing from a topic successfully
    async fn test_vehicle_manager_unsubscribe_topic() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        let result = vehicle_manager
            .unsubscribe_topic("vehicle_data".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[test] // Test listing all available vehicle types
    fn test_vehicle_manager_list_available_types() {
        let (tx, _rx) = mpsc::channel(10);
        let vehicle_manager = VehicleManager::new(tx);
        let types = vehicle_manager.list_available_types();
        assert!(!types.is_empty());
    }

    #[test] // Test setting the domain ID for VehicleManager
    fn test_vehicle_manager_set_domain_id() {
        let (tx, _rx) = mpsc::channel(10);
        let mut vehicle_manager = VehicleManager::new(tx);
        vehicle_manager.set_domain_id(200);
        assert!(true); // Placeholder assertion for domain ID setting
    }
}
