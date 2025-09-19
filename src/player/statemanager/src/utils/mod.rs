/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! ActionController gRPC client for StateManager
//!
//! This module provides functionality to send reconcile requests to the ActionController
//! when packages enter error states, as required by LLD_SM_package.md.

use common::statemanager::PackageState;
use common::Result;

pub mod grpc_client;

pub use grpc_client::ActionControllerClient;

/// ActionController communication interface
#[async_trait::async_trait]
pub trait ActionControllerService: Send + Sync {
    /// Send reconcile request to ActionController for package in error state
    async fn send_reconcile_request(&self, package_name: &str, state: PackageState) -> Result<()>;

    /// Check if ActionController is available
    async fn health_check(&self) -> Result<bool>;
}

/// Helper functions for ActionController integration
pub struct ActionControllerHelper;

impl ActionControllerHelper {
    /// Check if package state requires ActionController notification
    pub fn requires_notification(state: PackageState) -> bool {
        matches!(state, PackageState::Error)
    }

    /// Generate reconcile request message
    pub fn create_reconcile_message(package_name: &str, state: PackageState) -> String {
        format!(
            "Package '{}' requires reconciliation - current state: {:?}",
            package_name, state
        )
    }

    /// Determine reconcile priority based on package state
    pub fn get_reconcile_priority(state: PackageState) -> u32 {
        match state {
            PackageState::Error => 1,    // High priority
            PackageState::Degraded => 2, // Medium priority
            _ => 3,                      // Low priority
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_notification() {
        assert!(ActionControllerHelper::requires_notification(
            PackageState::Error
        ));
        assert!(!ActionControllerHelper::requires_notification(
            PackageState::Running
        ));
        assert!(!ActionControllerHelper::requires_notification(
            PackageState::Degraded
        ));
    }

    #[test]
    fn test_create_reconcile_message() {
        let message =
            ActionControllerHelper::create_reconcile_message("test-package", PackageState::Error);
        assert!(message.contains("test-package"));
        assert!(message.contains("reconciliation"));
        assert!(message.contains("Error"));
    }

    #[test]
    fn test_get_reconcile_priority() {
        assert_eq!(
            ActionControllerHelper::get_reconcile_priority(PackageState::Error),
            1
        );
        assert_eq!(
            ActionControllerHelper::get_reconcile_priority(PackageState::Degraded),
            2
        );
        assert_eq!(
            ActionControllerHelper::get_reconcile_priority(PackageState::Running),
            3
        );
    }
}
