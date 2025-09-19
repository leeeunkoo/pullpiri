/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Model state management module
//!
//! This module implements the model state transition logic according to LLD_SM_model.md.
//! It evaluates container states and determines the appropriate model state based on
//! the state transition rules defined in the PICCOLO specifications.

use common::monitoringserver::{ContainerInfo, ContainerList};
use common::statemanager::ModelState;
use std::collections::HashMap;

pub mod state_evaluator;

pub use state_evaluator::ModelStateEvaluator;

/// Model state transition logic implementation
///
/// According to LLD_SM_model.md section 3.2:
/// - Created: model의 최초 상태 (생성 시 기본 상태)
/// - Paused: 모든 container가 paused 상태일 때
/// - Exited: 모든 container가 exited 상태일 때  
/// - Dead: 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패
/// - Running: 위 조건을 모두 만족하지 않을 때(기본 상태)
pub struct ModelStateManager;

impl ModelStateManager {
    /// Evaluate model state based on container states
    pub fn evaluate_model_state(containers: &[ContainerInfo]) -> ModelState {
        if containers.is_empty() {
            return ModelState::Unknown;
        }

        let mut paused_count = 0;
        let mut exited_count = 0;
        let mut dead_count = 0;
        let mut _running_count = 0;

        for container in containers {
            // Extract state from the state HashMap - typically has a "Status" key
            let state_str = Self::extract_container_state(container);
            match state_str.as_str() {
                "paused" => paused_count += 1,
                "exited" => exited_count += 1,
                "dead" => dead_count += 1,
                "running" => _running_count += 1,
                _ => {} // Other states are treated as neutral
            }
        }

        let total_containers = containers.len();

        // Apply state transition rules from LLD
        if dead_count > 0 {
            // 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패
            ModelState::Failed // Using Failed to represent Dead state
        } else if exited_count == total_containers {
            // 모든 container가 exited 상태일 때
            ModelState::Succeeded // Using Succeeded to represent Exited state
        } else if paused_count == total_containers {
            // 모든 container가 paused 상태일 때
            ModelState::Unknown // Using Unknown to represent Paused state (no direct mapping in proto)
        } else {
            // 위 조건을 모두 만족하지 않을 때(기본 상태)
            ModelState::Running
        }
    }

    /// Extract container state from the state HashMap
    fn extract_container_state(container: &ContainerInfo) -> String {
        // Try common keys for container state
        for key in &["Status", "status", "State", "state"] {
            if let Some(state) = container.state.get(*key) {
                return state.clone();
            }
        }

        // If no standard state key found, return "unknown"
        "unknown".to_string()
    }

    /// Extract model states from container list
    pub fn extract_model_states_from_containers(
        container_list: &ContainerList,
    ) -> HashMap<String, ModelState> {
        let mut model_states = HashMap::new();

        // Group containers by model name
        let mut model_containers: HashMap<String, Vec<&ContainerInfo>> = HashMap::new();

        for container in &container_list.containers {
            if let Some(model_name) = Self::extract_model_name_from_container(container) {
                model_containers
                    .entry(model_name)
                    .or_insert_with(Vec::new)
                    .push(container);
            }
        }

        // Evaluate state for each model
        for (model_name, containers) in model_containers {
            let state =
                Self::evaluate_model_state(&containers.into_iter().cloned().collect::<Vec<_>>());
            model_states.insert(model_name, state);
        }

        model_states
    }

    /// Extract model name from container metadata
    /// This is a simplified implementation - in real scenarios, this would
    /// extract the model name from container labels or annotations
    fn extract_model_name_from_container(container: &ContainerInfo) -> Option<String> {
        // Try to get model name from container names (first name if multiple)
        if let Some(container_name) = container.names.first() {
            // For now, assume model name is part of container name
            // In real implementation, this would parse labels/annotations
            if let Some(slash_pos) = container_name.find('/') {
                Some(container_name[..slash_pos].to_string())
            } else {
                Some(container_name.clone())
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_model_state_all_running() {
        let containers = vec![
            create_test_container("test-container-1", "running"),
            create_test_container("test-container-2", "running"),
        ];

        let state = ModelStateManager::evaluate_model_state(&containers);
        assert_eq!(state, ModelState::Running);
    }

    #[test]
    fn test_evaluate_model_state_all_exited() {
        let containers = vec![
            create_test_container("test-container-1", "exited"),
            create_test_container("test-container-2", "exited"),
        ];

        let state = ModelStateManager::evaluate_model_state(&containers);
        assert_eq!(state, ModelState::Succeeded);
    }

    #[test]
    fn test_evaluate_model_state_some_dead() {
        let containers = vec![
            create_test_container("test-container-1", "running"),
            create_test_container("test-container-2", "dead"),
        ];

        let state = ModelStateManager::evaluate_model_state(&containers);
        assert_eq!(state, ModelState::Failed);
    }

    #[test]
    fn test_evaluate_model_state_all_paused() {
        let containers = vec![
            create_test_container("test-container-1", "paused"),
            create_test_container("test-container-2", "paused"),
        ];

        let state = ModelStateManager::evaluate_model_state(&containers);
        assert_eq!(state, ModelState::Unknown);
    }

    #[test]
    fn test_evaluate_model_state_empty_containers() {
        let containers = vec![];
        let state = ModelStateManager::evaluate_model_state(&containers);
        assert_eq!(state, ModelState::Unknown);
    }

    #[test]
    fn test_extract_model_name_from_container() {
        let container = create_test_container_with_name("model1/container1");

        let model_name = ModelStateManager::extract_model_name_from_container(&container);
        assert_eq!(model_name, Some("model1".to_string()));
    }

    // Helper function to create test containers
    fn create_test_container(name: &str, state: &str) -> ContainerInfo {
        let mut state_map = HashMap::new();
        state_map.insert("Status".to_string(), state.to_string());

        ContainerInfo {
            id: format!("{}_id", name),
            names: vec![name.to_string()],
            image: "test-image".to_string(),
            state: state_map,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        }
    }

    fn create_test_container_with_name(name: &str) -> ContainerInfo {
        create_test_container(name, "running")
    }
}
