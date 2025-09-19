/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Storage abstraction layer for StateManager
//!
//! This module provides an abstraction layer for persisting state information
//! to ETCD according to PICCOLO specifications. It implements proper key formatting
//! and error handling for model and package state storage.

use common::statemanager::{ModelState, PackageState};
use common::Result;
use std::collections::HashMap;

pub mod etcd_storage;

pub use etcd_storage::EtcdStateStorage;

/// Trait for state storage operations
#[async_trait::async_trait]
pub trait StateStorage: Send + Sync {
    /// Store model state to persistent storage
    async fn put_model_state(&self, model_name: &str, state: ModelState) -> Result<()>;

    /// Retrieve model state from persistent storage
    async fn get_model_state(&self, model_name: &str) -> Result<Option<ModelState>>;

    /// Store package state to persistent storage
    async fn put_package_state(&self, package_name: &str, state: PackageState) -> Result<()>;

    /// Retrieve package state from persistent storage
    async fn get_package_state(&self, package_name: &str) -> Result<Option<PackageState>>;

    /// Get all model states with prefix
    async fn get_all_model_states(&self) -> Result<HashMap<String, ModelState>>;

    /// Get all package states with prefix
    async fn get_all_package_states(&self) -> Result<HashMap<String, PackageState>>;

    /// Get models associated with a package
    async fn get_package_models(&self, package_name: &str) -> Result<Vec<String>>;

    /// Store package-model relationship
    async fn put_package_models(&self, package_name: &str, model_names: &[String]) -> Result<()>;
}

/// Helper functions for key generation according to LLD specifications
pub struct KeyFormatter;

impl KeyFormatter {
    /// Generate ETCD key for model state storage
    /// Format: /model/{model_name}/state
    pub fn model_state_key(model_name: &str) -> String {
        format!("/model/{}/state", model_name)
    }

    /// Generate ETCD key for package state storage
    /// Format: /package/{package_name}/state
    pub fn package_state_key(package_name: &str) -> String {
        format!("/package/{}/state", package_name)
    }

    /// Generate ETCD key for package-model relationship
    /// Format: /package/{package_name}/models
    pub fn package_models_key(package_name: &str) -> String {
        format!("/package/{}/models", package_name)
    }

    /// Get prefix for all model states
    pub fn model_prefix() -> &'static str {
        "/model/"
    }

    /// Get prefix for all package states
    pub fn package_prefix() -> &'static str {
        "/package/"
    }
}

/// State conversion utilities
pub struct StateConverter;

impl StateConverter {
    /// Convert ModelState enum to string representation
    pub fn model_state_to_string(state: ModelState) -> &'static str {
        match state {
            ModelState::Unspecified => "Unspecified",
            ModelState::Pending => "Pending",
            ModelState::Running => "Running",
            ModelState::Succeeded => "Succeeded",
            ModelState::Failed => "Failed",
            ModelState::Unknown => "Unknown",
            ModelState::ContainerCreating => "ContainerCreating",
            ModelState::CrashLoopBackOff => "CrashLoopBackOff",
        }
    }

    /// Parse string to ModelState enum
    pub fn string_to_model_state(state_str: &str) -> Result<ModelState> {
        match state_str {
            "Unspecified" => Ok(ModelState::Unspecified),
            "Pending" => Ok(ModelState::Pending),
            "Running" => Ok(ModelState::Running),
            "Succeeded" => Ok(ModelState::Succeeded),
            "Failed" => Ok(ModelState::Failed),
            "Unknown" => Ok(ModelState::Unknown),
            "ContainerCreating" => Ok(ModelState::ContainerCreating),
            "CrashLoopBackOff" => Ok(ModelState::CrashLoopBackOff),
            _ => Err(format!("Invalid model state: {}", state_str).into()),
        }
    }

    /// Convert PackageState enum to string representation
    pub fn package_state_to_string(state: PackageState) -> &'static str {
        match state {
            PackageState::Unspecified => "idle",
            PackageState::Initializing => "Initializing",
            PackageState::Running => "running",
            PackageState::Degraded => "degraded",
            PackageState::Error => "error",
            PackageState::Paused => "paused",
            PackageState::Updating => "Updating",
        }
    }

    /// Parse string to PackageState enum
    pub fn string_to_package_state(state_str: &str) -> Result<PackageState> {
        match state_str {
            "idle" => Ok(PackageState::Unspecified),
            "Initializing" => Ok(PackageState::Initializing),
            "running" => Ok(PackageState::Running),
            "degraded" => Ok(PackageState::Degraded),
            "error" => Ok(PackageState::Error),
            "paused" => Ok(PackageState::Paused),
            "Updating" => Ok(PackageState::Updating),
            _ => Err(format!("Invalid package state: {}", state_str).into()),
        }
    }
}
