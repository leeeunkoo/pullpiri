/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Package state management module
//!
//! This module implements the package state transition logic according to LLD_SM_package.md.
//! It evaluates model states and determines the appropriate package state based on
//! the cascading state transition rules defined in the PICCOLO specifications.

use common::statemanager::{ModelState, PackageState};
use std::collections::HashMap;

pub mod state_evaluator;

pub use state_evaluator::PackageStateEvaluator;

/// Package state transition logic implementation
///
/// According to LLD_SM_package.md section 3.1:
/// - idle: 맨 처음 package의 상태 (생성 시 기본 상태)
/// - paused: 모든 model이 paused 상태일 때
/// - exited: 모든 model이 exited 상태일 때
/// - degraded: 일부 model이 dead 상태일 때 (일부(1개 이상) model이 dead 상태, 단 모든 model이 dead가 아닐 때)
/// - error: 모든 model이 dead 상태일 때
/// - running: 위 조건을 모두 만족하지 않을 때(기본 상태)
pub struct PackageStateManager;

impl PackageStateManager {
    /// Evaluate package state based on model states
    pub fn evaluate_package_state(model_states: &[ModelState]) -> PackageState {
        if model_states.is_empty() {
            return PackageState::Unspecified; // idle state
        }

        let mut paused_count = 0; // Unknown represents paused in our mapping
        let mut exited_count = 0; // Succeeded represents exited
        let mut dead_count = 0; // Failed represents dead
        let mut running_count = 0; // Counts normal running models

        for state in model_states {
            match state {
                ModelState::Unknown => paused_count += 1,   // paused
                ModelState::Succeeded => exited_count += 1, // exited
                ModelState::Failed => dead_count += 1,      // dead
                ModelState::Running => running_count += 1,
                _ => {} // Other states are treated as neutral
            }
        }

        let total_models = model_states.len();

        // Apply state transition rules from LLD
        if dead_count == total_models {
            // 모든 model이 dead 상태일 때
            PackageState::Error
        } else if dead_count > 0 {
            // 일부 model이 dead 상태일 때 (일부(1개 이상) model이 dead 상태, 단 모든 model이 dead가 아닐 때)
            PackageState::Degraded
        } else if exited_count == total_models {
            // 모든 model이 exited 상태일 때
            PackageState::Unspecified // Using Unspecified to represent exited (no direct mapping)
        } else if paused_count == total_models {
            // 모든 model이 paused 상태일 때
            PackageState::Paused
        } else {
            // 위 조건을 모두 만족하지 않을 때(기본 상태)
            PackageState::Running
        }
    }

    /// Evaluate package state from model state map
    pub fn evaluate_package_state_from_map(
        _package_name: &str,
        model_names: &[String],
        model_states: &HashMap<String, ModelState>,
    ) -> PackageState {
        let mut package_model_states = Vec::new();

        for model_name in model_names {
            if let Some(state) = model_states.get(model_name) {
                package_model_states.push(*state);
            } else {
                // Model state not found - treat as unknown/dead
                package_model_states.push(ModelState::Failed);
            }
        }

        Self::evaluate_package_state(&package_model_states)
    }

    /// Check if package state change requires ActionController notification
    pub fn requires_action_controller_notification(state: PackageState) -> bool {
        matches!(state, PackageState::Error)
    }

    /// Get human-readable description of package state
    pub fn get_state_description(state: PackageState) -> &'static str {
        match state {
            PackageState::Unspecified => "idle",
            PackageState::Initializing => "initializing",
            PackageState::Running => "running",
            PackageState::Degraded => "degraded",
            PackageState::Error => "error",
            PackageState::Paused => "paused",
            PackageState::Updating => "updating",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_package_state_all_running() {
        let model_states = vec![ModelState::Running, ModelState::Running];
        let state = PackageStateManager::evaluate_package_state(&model_states);
        assert_eq!(state, PackageState::Running);
    }

    #[test]
    fn test_evaluate_package_state_all_dead() {
        let model_states = vec![ModelState::Failed, ModelState::Failed];
        let state = PackageStateManager::evaluate_package_state(&model_states);
        assert_eq!(state, PackageState::Error);
    }

    #[test]
    fn test_evaluate_package_state_some_dead() {
        let model_states = vec![ModelState::Running, ModelState::Failed];
        let state = PackageStateManager::evaluate_package_state(&model_states);
        assert_eq!(state, PackageState::Degraded);
    }

    #[test]
    fn test_evaluate_package_state_all_exited() {
        let model_states = vec![ModelState::Succeeded, ModelState::Succeeded];
        let state = PackageStateManager::evaluate_package_state(&model_states);
        assert_eq!(state, PackageState::Unspecified); // exited
    }

    #[test]
    fn test_evaluate_package_state_all_paused() {
        let model_states = vec![ModelState::Unknown, ModelState::Unknown];
        let state = PackageStateManager::evaluate_package_state(&model_states);
        assert_eq!(state, PackageState::Paused);
    }

    #[test]
    fn test_evaluate_package_state_empty() {
        let model_states = vec![];
        let state = PackageStateManager::evaluate_package_state(&model_states);
        assert_eq!(state, PackageState::Unspecified); // idle
    }

    #[test]
    fn test_requires_action_controller_notification() {
        assert!(PackageStateManager::requires_action_controller_notification(PackageState::Error));
        assert!(
            !PackageStateManager::requires_action_controller_notification(PackageState::Running)
        );
        assert!(
            !PackageStateManager::requires_action_controller_notification(PackageState::Degraded)
        );
    }

    #[test]
    fn test_get_state_description() {
        assert_eq!(
            PackageStateManager::get_state_description(PackageState::Running),
            "running"
        );
        assert_eq!(
            PackageStateManager::get_state_description(PackageState::Error),
            "error"
        );
        assert_eq!(
            PackageStateManager::get_state_description(PackageState::Degraded),
            "degraded"
        );
    }

    #[test]
    fn test_evaluate_package_state_from_map() {
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Running);
        model_states.insert("model2".to_string(), ModelState::Failed);

        let model_names = vec!["model1".to_string(), "model2".to_string()];
        let state = PackageStateManager::evaluate_package_state_from_map(
            "test_package",
            &model_names,
            &model_states,
        );

        assert_eq!(state, PackageState::Degraded);
    }

    #[test]
    fn test_evaluate_package_state_from_map_missing_model() {
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Running);
        // model2 is missing

        let model_names = vec!["model1".to_string(), "model2".to_string()];
        let state = PackageStateManager::evaluate_package_state_from_map(
            "test_package",
            &model_names,
            &model_states,
        );

        // Should be degraded because missing model is treated as dead
        assert_eq!(state, PackageState::Degraded);
    }
}
