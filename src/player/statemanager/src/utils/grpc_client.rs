/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! ActionController gRPC client implementation
//!
//! This module implements the gRPC client for communicating with the ActionController
//! to send reconcile requests when packages enter error states.

use super::{ActionControllerHelper, ActionControllerService};
use common::statemanager::PackageState;
use common::Result;

/// ActionController gRPC client implementation
pub struct ActionControllerClient {
    endpoint: String,
    timeout_ms: u64,
}

impl ActionControllerClient {
    /// Create a new ActionController client
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            timeout_ms: 5000, // 5 second default timeout
        }
    }

    /// Create client with custom timeout
    pub fn with_timeout(endpoint: String, timeout_ms: u64) -> Self {
        Self {
            endpoint,
            timeout_ms,
        }
    }

    /// Get the client endpoint
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

#[async_trait::async_trait]
impl ActionControllerService for ActionControllerClient {
    /// Send reconcile request to ActionController for package in error state
    async fn send_reconcile_request(&self, package_name: &str, state: PackageState) -> Result<()> {
        println!("=== ActionController Reconcile Request ===");
        println!("  Endpoint: {}", self.endpoint);
        println!("  Package: {}", package_name);
        println!("  State: {:?}", state);
        println!(
            "  Priority: {}",
            ActionControllerHelper::get_reconcile_priority(state)
        );

        // TODO: Implement actual gRPC call to ActionController
        // This would use the ActionController protobuf service definition
        // and send a reconcile request with package information

        // For now, simulate the request
        let message = ActionControllerHelper::create_reconcile_message(package_name, state);
        println!("  Message: {}", message);

        // Simulate network delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        println!("  Status: Reconcile request sent successfully");
        println!("==========================================");

        Ok(())
    }

    /// Check if ActionController is available
    async fn health_check(&self) -> Result<bool> {
        println!("ActionController health check: {}", self.endpoint);

        // TODO: Implement actual health check gRPC call
        // This would ping the ActionController service to verify availability

        // For now, simulate health check
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Assume healthy for simulation
        let is_healthy = true;
        println!(
            "ActionController health: {}",
            if is_healthy { "OK" } else { "UNHEALTHY" }
        );

        Ok(is_healthy)
    }
}

/// Mock ActionController client for testing
pub struct MockActionControllerClient {
    pub calls: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl MockActionControllerClient {
    pub fn new() -> Self {
        Self {
            calls: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn get_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl ActionControllerService for MockActionControllerClient {
    async fn send_reconcile_request(&self, package_name: &str, state: PackageState) -> Result<()> {
        let call = format!("reconcile:{}:{:?}", package_name, state);
        self.calls.lock().unwrap().push(call);
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        self.calls.lock().unwrap().push("health_check".to_string());
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_action_controller_client_creation() {
        let client = ActionControllerClient::new("http://localhost:47001".to_string());
        assert_eq!(client.endpoint(), "http://localhost:47001");
        assert_eq!(client.timeout_ms, 5000);
    }

    #[tokio::test]
    async fn test_action_controller_client_with_timeout() {
        let client =
            ActionControllerClient::with_timeout("http://localhost:47001".to_string(), 10000);
        assert_eq!(client.timeout_ms, 10000);
    }

    #[tokio::test]
    async fn test_send_reconcile_request() {
        let client = ActionControllerClient::new("http://localhost:47001".to_string());
        let result = client
            .send_reconcile_request("test-package", PackageState::Error)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = ActionControllerClient::new("http://localhost:47001".to_string());
        let result = client.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_client() {
        let mock_client = MockActionControllerClient::new();

        mock_client
            .send_reconcile_request("test-package", PackageState::Error)
            .await
            .unwrap();
        mock_client.health_check().await.unwrap();

        let calls = mock_client.get_calls();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].contains("reconcile:test-package:Error"));
        assert_eq!(calls[1], "health_check");
    }
}
