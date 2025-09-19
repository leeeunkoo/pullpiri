/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Package state evaluator implementation
//!
//! This module provides detailed state evaluation logic for packages based on
//! model states according to the PICCOLO specifications.

use common::statemanager::{ModelState, PackageState};
use std::collections::HashMap;

/// Evaluates package states based on model conditions
pub struct PackageStateEvaluator;

impl PackageStateEvaluator {
    /// Evaluate package state with detailed logic
    pub fn evaluate_state(model_states: &[ModelState]) -> PackageState {
        if model_states.is_empty() {
            return PackageState::Unspecified; // idle state
        }

        let state_counts = Self::count_model_states(model_states);
        let total = model_states.len();

        // Apply the state transition rules from LLD_SM_package.md
        Self::apply_state_transition_rules(&state_counts, total)
    }

    /// Count models by their state
    fn count_model_states(model_states: &[ModelState]) -> HashMap<ModelState, usize> {
        let mut counts = HashMap::new();

        for state in model_states {
            *counts.entry(*state).or_insert(0) += 1;
        }

        counts
    }

    /// Apply state transition rules according to LLD specifications
    fn apply_state_transition_rules(
        state_counts: &HashMap<ModelState, usize>,
        total: usize,
    ) -> PackageState {
        let dead_count = state_counts.get(&ModelState::Failed).unwrap_or(&0);
        let exited_count = state_counts.get(&ModelState::Succeeded).unwrap_or(&0);
        let paused_count = state_counts.get(&ModelState::Unknown).unwrap_or(&0);

        // Rule: 모든 model이 dead 상태일 때
        if *dead_count == total {
            return PackageState::Error;
        }

        // Rule: 일부 model이 dead 상태일 때 (일부(1개 이상) model이 dead 상태, 단 모든 model이 dead가 아닐 때)
        if *dead_count > 0 {
            return PackageState::Degraded;
        }

        // Rule: 모든 model이 exited 상태일 때
        if *exited_count == total {
            return PackageState::Unspecified; // Using Unspecified to represent exited
        }

        // Rule: 모든 model이 paused 상태일 때
        if *paused_count == total {
            return PackageState::Paused;
        }

        // Rule: 위 조건을 모두 만족하지 않을 때(기본 상태)
        PackageState::Running
    }

    /// Get detailed state information for debugging
    pub fn get_state_summary(model_states: &[ModelState]) -> String {
        if model_states.is_empty() {
            return "No models".to_string();
        }

        let state_counts = Self::count_model_states(model_states);
        let total = model_states.len();

        let mut summary = format!("Total models: {}, States: ", total);
        for (state, count) in &state_counts {
            summary.push_str(&format!("{:?}:{}, ", state, count));
        }

        summary.trim_end_matches(", ").to_string()
    }

    /// Check if package state should trigger ActionController notification
    pub fn should_notify_action_controller(
        old_state: PackageState,
        new_state: PackageState,
    ) -> bool {
        // Notify when transitioning to error state
        new_state == PackageState::Error && old_state != PackageState::Error
    }

    /// Get package models that are in problematic states
    pub fn get_problematic_models(
        model_names: &[String],
        model_states: &HashMap<String, ModelState>,
    ) -> Vec<(String, ModelState)> {
        let mut problematic = Vec::new();

        for model_name in model_names {
            if let Some(state) = model_states.get(model_name) {
                if matches!(state, ModelState::Failed | ModelState::Unknown) {
                    problematic.push((model_name.clone(), *state));
                }
            } else {
                // Missing model is also problematic
                problematic.push((model_name.clone(), ModelState::Failed));
            }
        }

        problematic
    }

    /// Calculate package health score (0.0 to 1.0)
    pub fn calculate_health_score(model_states: &[ModelState]) -> f32 {
        if model_states.is_empty() {
            return 1.0;
        }

        let healthy_count = model_states
            .iter()
            .filter(|state| matches!(state, ModelState::Running | ModelState::Succeeded))
            .count();

        healthy_count as f32 / model_states.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_model_states() {
        let model_states = vec![ModelState::Running, ModelState::Running, ModelState::Failed];

        let counts = PackageStateEvaluator::count_model_states(&model_states);
        assert_eq!(counts.get(&ModelState::Running), Some(&2));
        assert_eq!(counts.get(&ModelState::Failed), Some(&1));
    }

    #[test]
    fn test_apply_state_transition_rules_all_dead() {
        let mut state_counts = HashMap::new();
        state_counts.insert(ModelState::Failed, 2);

        let result = PackageStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, PackageState::Error);
    }

    #[test]
    fn test_apply_state_transition_rules_some_dead() {
        let mut state_counts = HashMap::new();
        state_counts.insert(ModelState::Running, 1);
        state_counts.insert(ModelState::Failed, 1);

        let result = PackageStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, PackageState::Degraded);
    }

    #[test]
    fn test_apply_state_transition_rules_all_exited() {
        let mut state_counts = HashMap::new();
        state_counts.insert(ModelState::Succeeded, 2);

        let result = PackageStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, PackageState::Unspecified);
    }

    #[test]
    fn test_apply_state_transition_rules_all_paused() {
        let mut state_counts = HashMap::new();
        state_counts.insert(ModelState::Unknown, 2);

        let result = PackageStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, PackageState::Paused);
    }

    #[test]
    fn test_apply_state_transition_rules_default_running() {
        let mut state_counts = HashMap::new();
        state_counts.insert(ModelState::Running, 1);
        state_counts.insert(ModelState::Pending, 1);

        let result = PackageStateEvaluator::apply_state_transition_rules(&state_counts, 2);
        assert_eq!(result, PackageState::Running);
    }

    #[test]
    fn test_get_state_summary() {
        let model_states = vec![ModelState::Running, ModelState::Failed];
        let summary = PackageStateEvaluator::get_state_summary(&model_states);

        assert!(summary.contains("Total models: 2"));
        assert!(summary.contains("Running:1"));
        assert!(summary.contains("Failed:1"));
    }

    #[test]
    fn test_should_notify_action_controller() {
        assert!(PackageStateEvaluator::should_notify_action_controller(
            PackageState::Running,
            PackageState::Error
        ));

        assert!(!PackageStateEvaluator::should_notify_action_controller(
            PackageState::Error,
            PackageState::Error
        ));

        assert!(!PackageStateEvaluator::should_notify_action_controller(
            PackageState::Running,
            PackageState::Degraded
        ));
    }

    #[test]
    fn test_get_problematic_models() {
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Running);
        model_states.insert("model2".to_string(), ModelState::Failed);
        model_states.insert("model3".to_string(), ModelState::Unknown);

        let model_names = vec![
            "model1".to_string(),
            "model2".to_string(),
            "model3".to_string(),
            "model4".to_string(), // missing
        ];

        let problematic =
            PackageStateEvaluator::get_problematic_models(&model_names, &model_states);
        assert_eq!(problematic.len(), 3);

        assert!(problematic.contains(&("model2".to_string(), ModelState::Failed)));
        assert!(problematic.contains(&("model3".to_string(), ModelState::Unknown)));
        assert!(problematic.contains(&("model4".to_string(), ModelState::Failed)));
    }

    #[test]
    fn test_calculate_health_score() {
        let model_states = vec![
            ModelState::Running,   // healthy
            ModelState::Succeeded, // healthy
            ModelState::Failed,    // unhealthy
            ModelState::Unknown,   // unhealthy
        ];

        let score = PackageStateEvaluator::calculate_health_score(&model_states);
        assert_eq!(score, 0.5); // 2 healthy out of 4 total

        let all_healthy = vec![ModelState::Running, ModelState::Succeeded];
        let score = PackageStateEvaluator::calculate_health_score(&all_healthy);
        assert_eq!(score, 1.0);

        let empty = vec![];
        let score = PackageStateEvaluator::calculate_health_score(&empty);
        assert_eq!(score, 1.0);
    }
}
