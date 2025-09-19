/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Model state evaluator implementation
//!
//! This module provides detailed state evaluation logic for models based on
//! container states according to the PICCOLO specifications.

use common::monitoringserver::ContainerInfo;
use common::statemanager::ModelState;
use std::collections::HashMap;

/// Evaluates model states based on container conditions
pub struct ModelStateEvaluator;

impl ModelStateEvaluator {
    /// Evaluate model state with detailed logic
    pub fn evaluate_state(containers: &[ContainerInfo]) -> ModelState {
        if containers.is_empty() {
            return ModelState::Unknown;
        }

        let state_counts = Self::count_container_states(containers);
        let total = containers.len();

        // Apply the state transition rules from LLD_SM_model.md
        Self::apply_state_transition_rules(&state_counts, total)
    }

    /// Count containers by their state
    fn count_container_states(containers: &[ContainerInfo]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();

        for container in containers {
            let state = Self::extract_container_state(container);
            *counts.entry(state).or_insert(0) += 1;
        }

        counts
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

    /// Apply state transition rules according to LLD specifications
    fn apply_state_transition_rules(
        state_counts: &HashMap<String, usize>,
        total: usize,
    ) -> ModelState {
        let dead_count = state_counts.get("dead").unwrap_or(&0);
        let exited_count = state_counts.get("exited").unwrap_or(&0);
        let paused_count = state_counts.get("paused").unwrap_or(&0);
        let _running_count = state_counts.get("running").unwrap_or(&0);

        // Rule: 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패
        if *dead_count > 0 {
            return ModelState::Failed;
        }

        // Rule: 모든 container가 exited 상태일 때
        if *exited_count == total {
            return ModelState::Succeeded;
        }

        // Rule: 모든 container가 paused 상태일 때
        if *paused_count == total {
            return ModelState::Unknown; // Using Unknown to represent paused
        }

        // Rule: 위 조건을 모두 만족하지 않을 때(기본 상태)
        ModelState::Running
    }

    /// Get detailed state information for debugging
    pub fn get_state_summary(containers: &[ContainerInfo]) -> String {
        if containers.is_empty() {
            return "No containers".to_string();
        }

        let state_counts = Self::count_container_states(containers);
        let total = containers.len();

        let mut summary = format!("Total containers: {}, States: ", total);
        for (state, count) in &state_counts {
            summary.push_str(&format!("{}:{}, ", state, count));
        }

        summary.trim_end_matches(", ").to_string()
    }

    /// Check if model state should trigger package state update
    pub fn should_trigger_package_update(old_state: ModelState, new_state: ModelState) -> bool {
        // Trigger update if state actually changed
        old_state != new_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_container(name: &str, state: &str) -> ContainerInfo {
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

    #[test]
    fn test_count_container_states() {
        let containers = vec![
            create_container("c1", "running"),
            create_container("c2", "running"),
            create_container("c3", "dead"),
        ];

        let counts = ModelStateEvaluator::count_container_states(&containers);
        assert_eq!(counts.get("running"), Some(&2));
        assert_eq!(counts.get("dead"), Some(&1));
    }

    #[test]
    fn test_apply_state_transition_rules_dead() {
        let mut state_counts = HashMap::new();
        state_counts.insert("running".to_string(), 1);
        state_counts.insert("dead".to_string(), 1);

        let result = ModelStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, ModelState::Failed);
    }

    #[test]
    fn test_apply_state_transition_rules_all_exited() {
        let mut state_counts = HashMap::new();
        state_counts.insert("exited".to_string(), 2);

        let result = ModelStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, ModelState::Succeeded);
    }

    #[test]
    fn test_apply_state_transition_rules_all_paused() {
        let mut state_counts = HashMap::new();
        state_counts.insert("paused".to_string(), 2);

        let result = ModelStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, ModelState::Unknown);
    }

    #[test]
    fn test_apply_state_transition_rules_default_running() {
        let mut state_counts = HashMap::new();
        state_counts.insert("running".to_string(), 1);
        state_counts.insert("creating".to_string(), 1);

        let result = ModelStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, ModelState::Running);
    }

    #[test]
    fn test_get_state_summary() {
        let containers = vec![
            create_container("c1", "running"),
            create_container("c2", "dead"),
        ];

        let summary = ModelStateEvaluator::get_state_summary(&containers);
        assert!(summary.contains("Total containers: 2"));
        assert!(summary.contains("running:1"));
        assert!(summary.contains("dead:1"));
    }

    #[test]
    fn test_should_trigger_package_update() {
        assert!(ModelStateEvaluator::should_trigger_package_update(
            ModelState::Running,
            ModelState::Failed
        ));

        assert!(!ModelStateEvaluator::should_trigger_package_update(
            ModelState::Running,
            ModelState::Running
        ));
    }
}
