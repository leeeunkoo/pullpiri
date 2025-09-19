/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! ETCD implementation of StateStorage trait
//!
//! This module provides the ETCD-specific implementation for state persistence
//! according to the PICCOLO specifications and LLD requirements.

use super::{KeyFormatter, StateConverter, StateStorage};
use common::statemanager::{ModelState, PackageState};
use common::Result;
use std::collections::HashMap;

/// ETCD implementation of StateStorage
pub struct EtcdStateStorage;

impl EtcdStateStorage {
    /// Create a new ETCD state storage instance
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl StateStorage for EtcdStateStorage {
    /// Store model state to ETCD with specified key format
    async fn put_model_state(&self, model_name: &str, state: ModelState) -> Result<()> {
        let key = KeyFormatter::model_state_key(model_name);
        let value = StateConverter::model_state_to_string(state);

        if let Err(e) = common::etcd::put(&key, value).await {
            eprintln!("Failed to save model state for {}: {:?}", model_name, e);
            return Err(format!("Failed to save model state: {:?}", e).into());
        }

        println!("Saved model state: {} = {}", key, value);
        Ok(())
    }

    /// Retrieve model state from ETCD
    async fn get_model_state(&self, model_name: &str) -> Result<Option<ModelState>> {
        let key = KeyFormatter::model_state_key(model_name);

        match common::etcd::get(&key).await {
            Ok(value) => match StateConverter::string_to_model_state(&value) {
                Ok(state) => Ok(Some(state)),
                Err(e) => {
                    eprintln!(
                        "Failed to parse model state '{}' for {}: {:?}",
                        value, model_name, e
                    );
                    Ok(None)
                }
            },
            Err(e) => {
                eprintln!("Failed to get model state for {}: {:?}", model_name, e);
                Ok(None)
            }
        }
    }

    /// Store package state to ETCD with specified key format
    async fn put_package_state(&self, package_name: &str, state: PackageState) -> Result<()> {
        let key = KeyFormatter::package_state_key(package_name);
        let value = StateConverter::package_state_to_string(state);

        if let Err(e) = common::etcd::put(&key, value).await {
            eprintln!("Failed to save package state for {}: {:?}", package_name, e);
            return Err(format!("Failed to save package state: {:?}", e).into());
        }

        println!("Saved package state: {} = {}", key, value);
        Ok(())
    }

    /// Retrieve package state from ETCD
    async fn get_package_state(&self, package_name: &str) -> Result<Option<PackageState>> {
        let key = KeyFormatter::package_state_key(package_name);

        match common::etcd::get(&key).await {
            Ok(value) => match StateConverter::string_to_package_state(&value) {
                Ok(state) => Ok(Some(state)),
                Err(e) => {
                    eprintln!(
                        "Failed to parse package state '{}' for {}: {:?}",
                        value, package_name, e
                    );
                    Ok(None)
                }
            },
            Err(e) => {
                eprintln!("Failed to get package state for {}: {:?}", package_name, e);
                Ok(None)
            }
        }
    }

    /// Get all model states from ETCD using prefix query
    async fn get_all_model_states(&self) -> Result<HashMap<String, ModelState>> {
        let prefix = KeyFormatter::model_prefix();
        let mut states = HashMap::new();

        match common::etcd::get_all_with_prefix(prefix).await {
            Ok(kvs) => {
                for kv in kvs {
                    if let Some(model_name) = Self::extract_model_name_from_key(&kv.key) {
                        match StateConverter::string_to_model_state(&kv.value) {
                            Ok(state) => {
                                states.insert(model_name, state);
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to parse model state '{}' from key {}: {:?}",
                                    kv.value, kv.key, e
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get all model states: {:?}", e);
                return Err(format!("Failed to get all model states: {:?}", e).into());
            }
        }

        Ok(states)
    }

    /// Get all package states from ETCD using prefix query
    async fn get_all_package_states(&self) -> Result<HashMap<String, PackageState>> {
        let prefix = KeyFormatter::package_prefix();
        let mut states = HashMap::new();

        match common::etcd::get_all_with_prefix(prefix).await {
            Ok(kvs) => {
                for kv in kvs {
                    if let Some(package_name) = Self::extract_package_name_from_key(&kv.key) {
                        match StateConverter::string_to_package_state(&kv.value) {
                            Ok(state) => {
                                states.insert(package_name, state);
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to parse package state '{}' from key {}: {:?}",
                                    kv.value, kv.key, e
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get all package states: {:?}", e);
                return Err(format!("Failed to get all package states: {:?}", e).into());
            }
        }

        Ok(states)
    }

    /// Get models associated with a package
    async fn get_package_models(&self, package_name: &str) -> Result<Vec<String>> {
        let key = KeyFormatter::package_models_key(package_name);

        match common::etcd::get(&key).await {
            Ok(value) => {
                // Parse comma-separated model names
                let models: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                Ok(models)
            }
            Err(e) => {
                eprintln!("Failed to get package models for {}: {:?}", package_name, e);
                Ok(Vec::new()) // Return empty list if not found
            }
        }
    }

    /// Store package-model relationship
    async fn put_package_models(&self, package_name: &str, model_names: &[String]) -> Result<()> {
        let key = KeyFormatter::package_models_key(package_name);
        let value = model_names.join(",");

        if let Err(e) = common::etcd::put(&key, &value).await {
            eprintln!(
                "Failed to save package models for {}: {:?}",
                package_name, e
            );
            return Err(format!("Failed to save package models: {:?}", e).into());
        }

        println!("Saved package models: {} = {}", key, value);
        Ok(())
    }
}

impl EtcdStateStorage {
    /// Extract model name from ETCD key
    /// Key format: /model/{model_name}/state
    fn extract_model_name_from_key(key: &str) -> Option<String> {
        if !key.starts_with("/model/") || !key.ends_with("/state") {
            return None;
        }

        let model_part = &key[7..]; // Remove "/model/"
        let end_pos = model_part.len() - 6; // Remove "/state"
        if end_pos > 0 {
            Some(model_part[..end_pos].to_string())
        } else {
            None
        }
    }

    /// Extract package name from ETCD key
    /// Key format: /package/{package_name}/state
    fn extract_package_name_from_key(key: &str) -> Option<String> {
        if !key.starts_with("/package/") || !key.ends_with("/state") {
            return None;
        }

        let package_part = &key[9..]; // Remove "/package/"
        let end_pos = package_part.len() - 6; // Remove "/state"
        if end_pos > 0 {
            Some(package_part[..end_pos].to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_model_name_from_key() {
        assert_eq!(
            EtcdStateStorage::extract_model_name_from_key("/model/test_model/state"),
            Some("test_model".to_string())
        );
        assert_eq!(
            EtcdStateStorage::extract_model_name_from_key("/model/my-model/state"),
            Some("my-model".to_string())
        );
        assert_eq!(
            EtcdStateStorage::extract_model_name_from_key("/model/test_model/invalid"),
            None
        );
        assert_eq!(
            EtcdStateStorage::extract_model_name_from_key("/invalid/test_model/state"),
            None
        );
    }

    #[test]
    fn test_extract_package_name_from_key() {
        assert_eq!(
            EtcdStateStorage::extract_package_name_from_key("/package/test_package/state"),
            Some("test_package".to_string())
        );
        assert_eq!(
            EtcdStateStorage::extract_package_name_from_key("/package/my-package/state"),
            Some("my-package".to_string())
        );
        assert_eq!(
            EtcdStateStorage::extract_package_name_from_key("/package/test_package/invalid"),
            None
        );
        assert_eq!(
            EtcdStateStorage::extract_package_name_from_key("/invalid/test_package/state"),
            None
        );
    }
}
