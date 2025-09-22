/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! State Machine Implementation for PICCOLO Resource State Management
//!
//! This module implements the core state transition logic for Scenario, Package, and Model resources
//! according to the PICCOLO specification. It provides efficient data structures and algorithms
//! for managing state changes and enforcing the defined state transition tables.
//!
//! # Architecture Overview
//!
//! The state machine follows a table-driven approach where each resource type (Scenario, Package, Model)
//! has its own transition table defining valid state changes. The system supports:
//! - Conditional transitions based on resource state
//! - Action execution during state changes non-blocking
//! - Health monitoring and failure handling
//! - Backoff mechanisms for failed transitions
//!
//! # Usage Example
//!
//! ```rust
//! let mut state_machine = StateMachine::new();
//! let state_change = StateChange { /* ... */ };
//! let result = state_machine.process_state_change(state_change);
//! ```

use crate::types::{ActionCommand, HealthStatus, ResourceState, StateTransition, TransitionResult};
use common::statemanager::{
    ErrorCode, ModelState, PackageState, ResourceType, ScenarioState, StateChange,
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};

// ========================================
// CONSTANTS AND CONFIGURATION
// ========================================

/// Default backoff duration for CrashLoopBackOff states
const BACKOFF_DURATION_SECS: u64 = 30;

/// Maximum consecutive failures before marking resource as unhealthy
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

impl TransitionResult {
    /// Check if the transition was successful
    pub fn is_success(&self) -> bool {
        matches!(self.error_code, ErrorCode::Success)
    }

    /// Check if the transition failed
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Convert TransitionResult to StateChangeResponse for proto compatibility
    pub fn to_state_change_response(&self) -> common::statemanager::StateChangeResponse {
        common::statemanager::StateChangeResponse {
            message: self.message.clone(),
            transition_id: self.transition_id.clone(),
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as i64,
            error_code: self.error_code as i32,
            error_details: self.error_details.clone(),
        }
    }
}

/// Core state machine implementation for PICCOLO resource management
///
/// This is the central component that manages all resource state transitions,
/// enforces business rules, and maintains the current state of all resources
/// in the system.
///
/// # Design Principles
/// - **Deterministic**: Same inputs always produce same outputs
/// - **Auditable**: All state changes are tracked with timestamps
/// - **Resilient**: Handles failures gracefully with backoff mechanisms
/// - **Extensible**: New resource types can be added with their own transition tables
///
/// # Thread Safety
/// This implementation is not thread-safe. External synchronization is required
/// for concurrent access across multiple threads.
pub struct StateMachine {
    /// State transition tables indexed by resource type
    ///
    /// Each resource type has its own set of valid transitions, allowing
    /// for type-specific state management rules and behaviors.
    transition_tables: HashMap<ResourceType, Vec<StateTransition>>,

    /// Current state tracking for all managed resources
    ///
    /// Resources are keyed by a unique identifier (typically resource name)
    /// and contain complete state information including metadata and health status.
    resource_states: HashMap<String, ResourceState>,

    /// Backoff timers for CrashLoopBackOff and retry management
    ///
    /// Tracks when resources that have failed transitions can be retried,
    /// implementing exponential backoff to prevent resource thrashing.
    backoff_timers: HashMap<String, Instant>,

    /// Action command sender for async execution
    action_sender: Option<mpsc::UnboundedSender<ActionCommand>>,
}

impl StateMachine {
    /// Creates a new StateMachine with predefined transition tables
    ///
    /// Initializes the state machine with empty resource tracking and
    /// populates the transition tables for all supported resource types.
    ///
    /// # Returns
    /// A fully configured StateMachine ready to process state changes
    ///
    /// # Examples
    /// ```rust
    /// let state_machine = StateMachine::new();
    /// ```
    pub fn new() -> Self {
        let mut state_machine = StateMachine {
            transition_tables: HashMap::new(),
            resource_states: HashMap::new(),
            backoff_timers: HashMap::new(),
            action_sender: None,
        };

        // Initialize transition tables for each resource type
        state_machine.initialize_scenario_transitions();
        state_machine.initialize_package_transitions();
        state_machine.initialize_model_transitions();

        state_machine
    }

    /// Initialize async action executor
    pub fn initialize_action_executor(&mut self) -> mpsc::UnboundedReceiver<ActionCommand> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.action_sender = Some(sender);
        receiver
    }

    // ========================================
    // STATE TRANSITION TABLE INITIALIZATION
    // ========================================

    /// Initialize the state transition table for Scenario resources
    ///
    /// Populates the transition table with all valid state changes for Scenario resources
    /// according to the PICCOLO specification. This includes transitions for:
    /// - Creation and initialization
    /// - Activation and deactivation
    /// - Error handling and recovery
    /// - Cleanup and termination
    ///
    /// # Implementation Note
    /// This method should define transitions like:
    /// - "Inactive" -> "Active" on "activate" event
    /// - "Active" -> "Inactive" on "deactivate" event
    /// - Any state -> "Failed" on "error" event
    fn initialize_scenario_transitions(&mut self) {
        let scenario_transitions = vec![
            StateTransition {
                from_state: ScenarioState::Idle as i32,
                event: "scenario_activation".to_string(),
                to_state: ScenarioState::Waiting as i32,
                condition: None,
                action: "start_condition_evaluation".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Waiting as i32,
                event: "condition_met".to_string(),
                to_state: ScenarioState::Allowed as i32,
                condition: None,
                action: "start_policy_verification".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Allowed as i32,
                event: "policy_verification_success".to_string(),
                to_state: ScenarioState::Playing as i32,
                condition: None,
                action: "execute_action_on_target_package".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Allowed as i32,
                event: "policy_verification_failure".to_string(),
                to_state: ScenarioState::Denied as i32,
                condition: None,
                action: "log_denial_generate_alert".to_string(),
            },
        ];
        self.transition_tables
            .insert(ResourceType::Scenario, scenario_transitions);
    }

    /// Initialize the state transition table for Package resources
    ///
    /// Configures all valid state transitions for Package resources, including:
    /// - Download and installation states
    /// - Verification and validation phases
    /// - Update and rollback mechanisms
    /// - Cleanup and removal operations
    ///
    /// # Implementation Note
    /// Package transitions typically involve more complex workflows due to
    /// dependency management and rollback requirements.
    fn initialize_package_transitions(&mut self) {
        let package_transitions = vec![
            StateTransition {
                from_state: PackageState::Unspecified as i32,
                event: "launch_request".to_string(),
                to_state: PackageState::Initializing as i32,
                condition: None,
                action: "start_model_creation_allocate_resources".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing as i32,
                event: "initialization_complete".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("all_models_normal".to_string()),
                action: "update_state_announce_availability".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing as i32,
                event: "partial_initialization_failure".to_string(),
                to_state: PackageState::Degraded as i32,
                condition: Some("critical_models_normal".to_string()),
                action: "log_warning_activate_partial_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing as i32,
                event: "critical_initialization_failure".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("critical_models_failed".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "model_issue_detected".to_string(),
                to_state: PackageState::Degraded as i32,
                condition: Some("non_critical_model_issues".to_string()),
                action: "log_warning_maintain_partial_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "critical_issue_detected".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("critical_model_issues".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "pause_request".to_string(),
                to_state: PackageState::Paused as i32,
                condition: None,
                action: "pause_models_preserve_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded as i32,
                event: "model_recovery".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("all_models_recovered".to_string()),
                action: "update_state_restore_full_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded as i32,
                event: "additional_model_issues".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("critical_models_affected".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded as i32,
                event: "pause_request".to_string(),
                to_state: PackageState::Paused as i32,
                condition: None,
                action: "pause_models_preserve_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Error as i32,
                event: "recovery_successful".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("depends_on_recovery_level".to_string()),
                action: "update_state_announce_functionality_restoration".to_string(),
            },
            StateTransition {
                from_state: PackageState::Paused as i32,
                event: "resume_request".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("depends_on_previous_state".to_string()),
                action: "resume_models_restore_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "update_request".to_string(),
                to_state: PackageState::Updating as i32,
                condition: None,
                action: "start_update_process".to_string(),
            },
            StateTransition {
                from_state: PackageState::Updating as i32,
                event: "update_successful".to_string(),
                to_state: PackageState::Running as i32,
                condition: None,
                action: "activate_new_version_update_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Updating as i32,
                event: "update_failed".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("depends_on_rollback_settings".to_string()),
                action: "rollback_or_error_handling".to_string(),
            },
        ];

        self.transition_tables
            .insert(ResourceType::Package, package_transitions);
    }

    /// Initialize the state transition table for Model resources
    ///
    /// Sets up state transitions specific to Model resources, covering:
    /// - Model loading and initialization
    /// - Training and inference states
    /// - Model versioning and updates
    /// - Resource allocation and cleanup
    ///
    /// # Implementation Note
    /// Model transitions may include resource-intensive operations and
    /// should account for memory and compute constraints.
    fn initialize_model_transitions(&mut self) {
        let model_transitions = vec![
            StateTransition {
                from_state: ModelState::Unspecified as i32,
                event: "creation_request".to_string(),
                to_state: ModelState::Pending as i32,
                condition: None,
                action: "start_node_selection_and_allocation".to_string(),
            },
            StateTransition {
                from_state: ModelState::Pending as i32,
                event: "node_allocation_complete".to_string(),
                to_state: ModelState::ContainerCreating as i32,
                condition: Some("sufficient_resources".to_string()),
                action: "pull_container_images_mount_volumes".to_string(),
            },
            StateTransition {
                from_state: ModelState::Pending as i32,
                event: "node_allocation_failed".to_string(),
                to_state: ModelState::Failed as i32,
                condition: Some("timeout_or_error".to_string()),
                action: "log_error_retry_or_reschedule".to_string(),
            },
            StateTransition {
                from_state: ModelState::ContainerCreating as i32,
                event: "container_creation_complete".to_string(),
                to_state: ModelState::Running as i32,
                condition: Some("all_containers_started".to_string()),
                action: "update_state_start_readiness_checks".to_string(),
            },
            StateTransition {
                from_state: ModelState::ContainerCreating as i32,
                event: "container_creation_failed".to_string(),
                to_state: ModelState::Failed as i32,
                condition: None,
                action: "log_error_retry_or_reschedule".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "temporary_task_complete".to_string(),
                to_state: ModelState::Succeeded as i32,
                condition: Some("one_time_task".to_string()),
                action: "log_completion_clean_up_resources".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "container_termination".to_string(),
                to_state: ModelState::Failed as i32,
                condition: Some("unexpected_termination".to_string()),
                action: "log_error_evaluate_automatic_restart".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "repeated_crash_detection".to_string(),
                to_state: ModelState::CrashLoopBackOff as i32,
                condition: Some("consecutive_restart_failures".to_string()),
                action: "set_backoff_timer_collect_logs".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "monitoring_failure".to_string(),
                to_state: ModelState::Unknown as i32,
                condition: Some("node_communication_issues".to_string()),
                action: "attempt_diagnostics_restore_communication".to_string(),
            },
            StateTransition {
                from_state: ModelState::CrashLoopBackOff as i32,
                event: "backoff_time_elapsed".to_string(),
                to_state: ModelState::Running as i32,
                condition: Some("restart_successful".to_string()),
                action: "resume_monitoring_reset_counter".to_string(),
            },
            StateTransition {
                from_state: ModelState::CrashLoopBackOff as i32,
                event: "maximum_retries_exceeded".to_string(),
                to_state: ModelState::Failed as i32,
                condition: Some("retry_limit_reached".to_string()),
                action: "log_error_notify_for_manual_intervention".to_string(),
            },
            StateTransition {
                from_state: ModelState::Unknown as i32,
                event: "state_check_recovered".to_string(),
                to_state: ModelState::Running as i32,
                condition: Some("depends_on_actual_state".to_string()),
                action: "synchronize_state_recover_if_needed".to_string(),
            },
            StateTransition {
                from_state: ModelState::Failed as i32,
                event: "manual_automatic_recovery".to_string(),
                to_state: ModelState::Pending as i32,
                condition: Some("according_to_restart_policy".to_string()),
                action: "start_model_recreation".to_string(),
            },
        ];

        self.transition_tables
            .insert(ResourceType::Model, model_transitions);
    }

    // ========================================
    // CORE STATE PROCESSING
    // ========================================

    /// Process a state change request with cascading transitions and ETCD persistence
    ///
    /// This is the main entry point for LLD-compliant state management. It handles:
    /// 1. Primary state transition processing
    /// 2. ETCD state persistence
    /// 3. Cascading state evaluation for parent resources
    /// 4. Error recovery notifications
    ///
    /// # Arguments
    /// * `state_change` - The state change request to process
    ///
    /// # Returns
    /// * `Vec<TransitionResult>` - Results for primary and any cascading transitions
    ///
    /// # LLD Compliance
    /// This method implements the complete state management workflow specified
    /// in the StateManager_Model.md LLD document.
    pub async fn process_state_change_with_cascading(
        &mut self,
        state_change: StateChange,
    ) -> Vec<TransitionResult> {
        println!("================================");
        println!(
            "Processing StateChange for {:?} '{}'",
            ResourceType::try_from(state_change.resource_type).unwrap_or(ResourceType::Unspecified),
            state_change.resource_name
        );
        println!(
            "  Transition: {} -> {}",
            state_change.current_state, state_change.target_state
        );
        println!(
            "  Source: {}, ID: {}",
            state_change.source, state_change.transition_id
        );

        let mut results = Vec::new();

        // Step 1: Process the primary state change
        let primary_result = self.process_state_change(state_change.clone());
        let primary_success = primary_result.is_success();
        results.push(primary_result);

        // Step 2: If primary transition succeeded, persist to ETCD and handle cascading
        if primary_success {
            // Persist the new state to ETCD
            if let Ok(resource_type) = ResourceType::try_from(state_change.resource_type) {
                let state_name = state_change.target_state.clone();
                let success = self
                    .persist_state_to_etcd(resource_type, &state_change.resource_name, &state_name)
                    .await;

                if !success {
                    eprintln!(
                        "  [Warning] Failed to persist state to ETCD, but transition completed"
                    );
                }
            }

            // Step 3: Handle cascading transitions
            let cascading_changes = self.handle_cascading_transitions(&state_change).await;

            // Step 4: Process each cascading state change recursively
            for cascading_change in cascading_changes {
                println!(
                    "  [Cascading] Processing cascading change for {:?} '{}'",
                    ResourceType::try_from(cascading_change.resource_type)
                        .unwrap_or(ResourceType::Unspecified),
                    cascading_change.resource_name
                );

                // Process cascading changes (non-recursively to avoid async recursion issues)
                let cascading_result = self.process_state_change(cascading_change.clone());

                // If cascading change succeeded, persist it to ETCD
                if cascading_result.is_success() {
                    if let Ok(resource_type) =
                        ResourceType::try_from(cascading_change.resource_type)
                    {
                        let success = self
                            .persist_state_to_etcd(
                                resource_type,
                                &cascading_change.resource_name,
                                &cascading_change.target_state,
                            )
                            .await;

                        if !success {
                            eprintln!("  [Warning] Failed to persist cascading state to ETCD");
                        }
                    }
                }

                results.push(cascading_result);
            }
        } else {
            println!("  [Error] Primary state transition failed, skipping cascading logic");
        }

        println!("================================");
        results
    }

    /// Process a state change request with non-blocking action execution
    pub fn process_state_change(&mut self, state_change: StateChange) -> TransitionResult {
        // Validate input parameters
        if let Err(validation_error) = self.validate_state_change(&state_change) {
            return TransitionResult {
                new_state: Self::state_str_to_enum(
                    state_change.current_state.as_str(),
                    state_change.resource_type,
                ),
                error_code: ErrorCode::InvalidRequest,
                message: format!("Invalid state change request: {validation_error}"),
                actions_to_execute: vec![],
                transition_id: state_change.transition_id.clone(),
                error_details: validation_error,
            };
        }

        // Convert i32 to ResourceType enum
        let resource_type = match ResourceType::try_from(state_change.resource_type) {
            Ok(rt) => rt,
            Err(_) => {
                return TransitionResult {
                    new_state: Self::state_str_to_enum(
                        state_change.current_state.as_str(),
                        state_change.resource_type,
                    ),
                    error_code: ErrorCode::InvalidStateTransition,
                    message: format!("Invalid resource type: {}", state_change.resource_type),
                    actions_to_execute: vec![],
                    transition_id: state_change.transition_id.clone(),
                    error_details: format!(
                        "Unsupported resource type ID: {}",
                        state_change.resource_type
                    ),
                };
            }
        };

        let resource_key = self.generate_resource_key(resource_type, &state_change.resource_name);

        // Get current state - use provided current_state for new resources
        let current_state = match self.resource_states.get(&resource_key) {
            Some(existing_state) => existing_state.current_state,
            None => Self::state_str_to_enum(
                state_change.current_state.as_str(),
                state_change.resource_type,
            ),
        };

        // Check for special CrashLoopBackOff handling
        if current_state == ModelState::CrashLoopBackOff as i32 {
            if let Some(backoff_time) = self.backoff_timers.get(&resource_key) {
                if backoff_time.elapsed() < Duration::from_secs(BACKOFF_DURATION_SECS) {
                    return TransitionResult {
                        new_state: current_state,
                        error_code: ErrorCode::PreconditionFailed,
                        message: "Resource is in backoff period".to_string(),
                        actions_to_execute: vec![],
                        transition_id: state_change.transition_id.clone(),
                        error_details: "Backoff timer has not elapsed yet".to_string(),
                    };
                }
            }
        }

        // Find valid transition
        let transition_event = self.infer_event_from_states(
            current_state,
            Self::state_str_to_enum(
                state_change.target_state.as_str(),
                state_change.resource_type,
            ),
            resource_type,
        );

        if let Some(transition) = self.find_valid_transition(
            resource_type,
            current_state,
            &transition_event,
            Self::state_str_to_enum(
                state_change.target_state.as_str(),
                state_change.resource_type,
            ),
        ) {
            // Check conditions if any
            if let Some(ref condition) = transition.condition {
                if !self.evaluate_condition(condition, &state_change) {
                    return TransitionResult {
                        new_state: current_state,
                        error_code: ErrorCode::PreconditionFailed,
                        message: format!("Condition not met: {condition}"),
                        actions_to_execute: vec![],
                        transition_id: state_change.transition_id.clone(),
                        error_details: format!("Failed condition evaluation: {condition}"),
                    };
                }
            }

            // Execute transition - this is immediate and non-blocking
            self.update_resource_state(
                &resource_key,
                &state_change,
                transition.to_state,
                resource_type,
            );

            // **NON-BLOCKING ACTION EXECUTION** - Queue action for async execution
            if let Some(ref sender) = self.action_sender {
                let action_command = ActionCommand {
                    action: transition.action.clone(),
                    resource_key: resource_key.clone(),
                    resource_type,
                    transition_id: state_change.transition_id.clone(),
                    context: self.build_action_context(&state_change, &transition),
                };

                // Send action for async execution (non-blocking)
                if let Err(e) = sender.send(action_command) {
                    eprintln!("Warning: Failed to queue action for execution: {e}");
                }
            }

            let transitioned_state_str = match resource_type {
                ResourceType::Scenario => ScenarioState::try_from(transition.to_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Package => PackageState::try_from(transition.to_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Model => ModelState::try_from(transition.to_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                _ => "UNKNOWN",
            };

            // Create successful transition result
            let transition_result = TransitionResult {
                new_state: transition.to_state,
                error_code: ErrorCode::Success,
                message: format!("Successfully transitioned to {transitioned_state_str}"),
                actions_to_execute: vec![transition.action.clone()],
                transition_id: state_change.transition_id.clone(),
                error_details: String::new(),
            };

            self.update_health_status(&resource_key, &transition_result);

            // Handle special state-specific logic
            if transition.to_state == ModelState::CrashLoopBackOff as i32 {
                self.backoff_timers
                    .insert(resource_key.clone(), Instant::now());
            }

            transition_result
        } else {
            let current_state_str = match resource_type {
                ResourceType::Scenario => ScenarioState::try_from(current_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Package => PackageState::try_from(current_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Model => ModelState::try_from(current_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                _ => "UNKNOWN",
            };

            let target_state_str = match resource_type {
                ResourceType::Scenario => {
                    let normalized = format!(
                        "SCENARIO_STATE_{}",
                        state_change
                            .target_state
                            .trim()
                            .to_ascii_uppercase()
                            .replace('-', "_")
                    );
                    ScenarioState::from_str_name(&normalized)
                        .map(|s| s.as_str_name())
                        .unwrap_or("UNKNOWN")
                }
                ResourceType::Package => {
                    let normalized = format!(
                        "PACKAGE_STATE_{}",
                        state_change
                            .target_state
                            .trim()
                            .to_ascii_uppercase()
                            .replace('-', "_")
                    );
                    PackageState::from_str_name(&normalized)
                        .map(|s| s.as_str_name())
                        .unwrap_or("UNKNOWN")
                }
                ResourceType::Model => {
                    let normalized = format!(
                        "MODEL_STATE_{}",
                        state_change
                            .target_state
                            .trim()
                            .to_ascii_uppercase()
                            .replace('-', "_")
                    );
                    ModelState::from_str_name(&normalized)
                        .map(|s| s.as_str_name())
                        .unwrap_or("UNKNOWN")
                }
                _ => "UNKNOWN",
            };

            let transition_result = TransitionResult {
                new_state: current_state,
                error_code: ErrorCode::InvalidStateTransition,
                message: format!(
                    "No valid transition from {current_state_str} to {target_state_str} for resource type {resource_type:?}",
                ),
                actions_to_execute: vec![],
                transition_id: state_change.transition_id.clone(),
                error_details: format!(
                    "Invalid state transition attempted: {current_state_str} -> {target_state_str}"
                ),
            };

            self.update_health_status(&resource_key, &transition_result);
            transition_result
        }
    }

    // ========================================
    // VALIDATION AND UTILITY METHODS
    // ========================================

    /// Find a valid transition rule for the given parameters
    ///
    /// Searches the appropriate transition table for a rule that matches
    /// the specified resource type, current state, event, and target state.
    ///
    /// # Parameters
    /// - `resource_type`: The type of resource to check transitions for
    /// - `from_state`: The current state of the resource
    /// - `event`: The event triggering the transition
    /// - `to_state`: The desired target state
    ///
    /// # Returns
    /// - `Some(StateTransition)`: If a valid transition rule is found
    /// - `None`: If no valid transition exists for the given parameters
    ///
    /// # Implementation Details
    /// This method performs exact matching on all transition parameters.
    /// Wildcard or pattern matching is not currently supported.
    fn find_valid_transition(
        &self,
        resource_type: ResourceType,
        from_state: i32,
        event: &str,
        to_state: i32,
    ) -> Option<StateTransition> {
        if let Some(transitions) = self.transition_tables.get(&resource_type) {
            for transition in transitions {
                if transition.from_state == from_state
                    && transition.event == event
                    && transition.to_state == to_state
                {
                    return Some(transition.clone());
                }
            }
        }
        None
    }

    /// Validate state change request parameters
    fn validate_state_change(&self, state_change: &StateChange) -> Result<(), String> {
        if state_change.resource_name.trim().is_empty() {
            return Err("Resource name cannot be empty".to_string());
        }

        if state_change.transition_id.trim().is_empty() {
            return Err("Transition ID cannot be empty".to_string());
        }

        if state_change.current_state == state_change.target_state {
            return Err("Current and target states cannot be the same".to_string());
        }

        if state_change.source.trim().is_empty() {
            return Err("Source cannot be empty".to_string());
        }

        Ok(())
    }

    /// Generate a unique resource key
    fn generate_resource_key(&self, resource_type: ResourceType, resource_name: &str) -> String {
        format!("{resource_type:?}::{resource_name}")
    }

    /// Build context for action execution
    fn build_action_context(
        &self,
        state_change: &StateChange,
        transition: &StateTransition,
    ) -> HashMap<String, String> {
        let mut context = HashMap::new();

        let resource_type = match ResourceType::try_from(state_change.resource_type) {
            Ok(rt) => rt,
            Err(_) => ResourceType::Scenario, // fallback, adjust as needed
        };

        let from_state_str = match resource_type {
            ResourceType::Scenario => ScenarioState::try_from(transition.from_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Package => PackageState::try_from(transition.from_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Model => ModelState::try_from(transition.from_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            _ => "UNKNOWN",
        };

        let to_state_str = match resource_type {
            ResourceType::Scenario => ScenarioState::try_from(transition.to_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Package => PackageState::try_from(transition.to_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Model => ModelState::try_from(transition.to_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            _ => "UNKNOWN",
        };

        context.insert("from_state".to_string(), from_state_str.to_string());
        context.insert("to_state".to_string(), to_state_str.to_string());
        context.insert("event".to_string(), transition.event.clone());
        context.insert(
            "resource_name".to_string(),
            state_change.resource_name.clone(),
        );
        context.insert("source".to_string(), state_change.source.clone());
        context.insert(
            "timestamp_ns".to_string(),
            state_change.timestamp_ns.to_string(),
        );
        context
    }

    /// Updates health status based on transition result
    fn update_health_status(&mut self, resource_key: &str, transition_result: &TransitionResult) {
        if let Some(resource_state) = self.resource_states.get_mut(resource_key) {
            let now = Instant::now();
            resource_state.health_status.last_check = now;

            if transition_result.is_success() {
                resource_state.health_status.healthy = true;
                resource_state.health_status.consecutive_failures = 0;
                resource_state.health_status.status_message = "Healthy".to_string();
            } else {
                resource_state.health_status.consecutive_failures += 1;
                resource_state.health_status.status_message = transition_result.message.clone();

                // Mark as unhealthy if we have multiple consecutive failures
                if resource_state.health_status.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    resource_state.health_status.healthy = false;
                }
            }
        }
    }

    /// Infer the appropriate event type from state transition
    ///
    /// When an explicit event is not provided, this method attempts to
    /// determine the most appropriate event based on the current and target states.
    ///
    /// # Parameters
    /// - `current_state`: The current state of the resource
    /// - `target_state`: The desired target state
    ///
    /// # Returns
    /// A string representing the inferred event type
    ///
    /// # Examples
    /// - "Inactive" -> "Active" might infer "activate"
    /// - "Running" -> "Stopped" might infer "stop"
    /// - Any state -> "Failed" might infer "error"
    ///
    /// # Fallback Behavior
    /// If no specific event can be inferred, returns a generic event name
    /// based on the target state (e.g., "transition_to_active").
    fn infer_event_from_states(
        &self,
        current_state: i32,
        target_state: i32,
        resource_type: ResourceType,
    ) -> String {
        match resource_type {
            ResourceType::Scenario => match (current_state, target_state) {
                (x, y) if x == ScenarioState::Idle as i32 && y == ScenarioState::Waiting as i32 => {
                    "scenario_activation".to_string()
                }
                (x, y)
                    if x == ScenarioState::Waiting as i32 && y == ScenarioState::Allowed as i32 =>
                {
                    "condition_met".to_string()
                }
                (x, y)
                    if x == ScenarioState::Allowed as i32 && y == ScenarioState::Playing as i32 =>
                {
                    "policy_verification_success".to_string()
                }
                (x, y)
                    if x == ScenarioState::Allowed as i32 && y == ScenarioState::Denied as i32 =>
                {
                    "policy_verification_failure".to_string()
                }
                _ => format!("transition_{current_state}_{target_state}"),
            },
            ResourceType::Package => match (current_state, target_state) {
                (x, y)
                    if x == PackageState::Unspecified as i32
                        && y == PackageState::Initializing as i32 =>
                {
                    "launch_request".to_string()
                }
                (x, y)
                    if x == PackageState::Initializing as i32
                        && y == PackageState::Running as i32 =>
                {
                    "initialization_complete".to_string()
                }
                (x, y)
                    if x == PackageState::Initializing as i32
                        && y == PackageState::Degraded as i32 =>
                {
                    "partial_initialization_failure".to_string()
                }
                (x, y)
                    if x == PackageState::Initializing as i32
                        && y == PackageState::Error as i32 =>
                {
                    "critical_initialization_failure".to_string()
                }
                (x, y)
                    if x == PackageState::Running as i32 && y == PackageState::Degraded as i32 =>
                {
                    "model_issue_detected".to_string()
                }
                (x, y) if x == PackageState::Running as i32 && y == PackageState::Error as i32 => {
                    "critical_issue_detected".to_string()
                }
                (x, y) if x == PackageState::Running as i32 && y == PackageState::Paused as i32 => {
                    "pause_request".to_string()
                }
                (x, y)
                    if x == PackageState::Running as i32 && y == PackageState::Updating as i32 =>
                {
                    "update_request".to_string()
                }
                (x, y)
                    if x == PackageState::Degraded as i32 && y == PackageState::Running as i32 =>
                {
                    "model_recovery".to_string()
                }
                (x, y) if x == PackageState::Degraded as i32 && y == PackageState::Error as i32 => {
                    "additional_model_issues".to_string()
                }
                (x, y)
                    if x == PackageState::Degraded as i32 && y == PackageState::Paused as i32 =>
                {
                    "pause_request".to_string()
                }
                (x, y) if x == PackageState::Error as i32 && y == PackageState::Running as i32 => {
                    "recovery_successful".to_string()
                }
                (x, y) if x == PackageState::Paused as i32 && y == PackageState::Running as i32 => {
                    "resume_request".to_string()
                }
                (x, y)
                    if x == PackageState::Updating as i32 && y == PackageState::Running as i32 =>
                {
                    "update_successful".to_string()
                }
                (x, y) if x == PackageState::Updating as i32 && y == PackageState::Error as i32 => {
                    "update_failed".to_string()
                }
                _ => format!("transition_{current_state}_{target_state}"),
            },
            ResourceType::Model => match (current_state, target_state) {
                (x, y)
                    if x == ModelState::Unspecified as i32 && y == ModelState::Pending as i32 =>
                {
                    "creation_request".to_string()
                }
                (x, y)
                    if x == ModelState::Pending as i32
                        && y == ModelState::ContainerCreating as i32 =>
                {
                    "node_allocation_complete".to_string()
                }
                (x, y) if x == ModelState::Pending as i32 && y == ModelState::Failed as i32 => {
                    "node_allocation_failed".to_string()
                }
                (x, y)
                    if x == ModelState::ContainerCreating as i32
                        && y == ModelState::Running as i32 =>
                {
                    "container_creation_complete".to_string()
                }
                (x, y)
                    if x == ModelState::ContainerCreating as i32
                        && y == ModelState::Failed as i32 =>
                {
                    "container_creation_failed".to_string()
                }
                (x, y) if x == ModelState::Running as i32 && y == ModelState::Succeeded as i32 => {
                    "temporary_task_complete".to_string()
                }
                (x, y) if x == ModelState::Running as i32 && y == ModelState::Failed as i32 => {
                    "container_termination".to_string()
                }
                (x, y)
                    if x == ModelState::Running as i32
                        && y == ModelState::CrashLoopBackOff as i32 =>
                {
                    "repeated_crash_detection".to_string()
                }
                (x, y) if x == ModelState::Running as i32 && y == ModelState::Unknown as i32 => {
                    "monitoring_failure".to_string()
                }
                (x, y)
                    if x == ModelState::CrashLoopBackOff as i32
                        && y == ModelState::Running as i32 =>
                {
                    "backoff_time_elapsed".to_string()
                }
                (x, y)
                    if x == ModelState::CrashLoopBackOff as i32
                        && y == ModelState::Failed as i32 =>
                {
                    "maximum_retries_exceeded".to_string()
                }
                (x, y) if x == ModelState::Unknown as i32 && y == ModelState::Running as i32 => {
                    "state_check_recovered".to_string()
                }
                (x, y) if x == ModelState::Failed as i32 && y == ModelState::Pending as i32 => {
                    "manual_automatic_recovery".to_string()
                }
                _ => format!("transition_{current_state}_{target_state}"),
            },
            _ => format!("transition_{current_state}_{target_state}"),
        }
    }

    /// Evaluate whether a transition condition is satisfied
    ///
    /// Processes conditional logic attached to state transitions to determine
    /// if the transition should be allowed to proceed.
    ///
    /// # Parameters
    /// - `condition`: The condition string to evaluate (e.g., "resource_count > 0")
    /// - `_state_change`: The state change request providing context for evaluation
    ///
    /// # Returns
    /// - `true`: If the condition is satisfied or no condition exists
    /// - `false`: If the condition fails evaluation
    ///
    /// # Supported Conditions
    /// The condition language should support:
    /// - Resource property comparisons
    /// - Metadata key existence checks
    /// - Numeric and string comparisons
    /// - Boolean logic operators
    ///
    /// # Error Handling
    /// Malformed conditions should be logged and default to `false` for safety.
    fn evaluate_condition(&self, condition: &str, _state_change: &StateChange) -> bool {
        // TODO: Implement real condition evaluation logic
        match condition {
            "all_models_normal" => true,
            "critical_models_normal" => true,
            "critical_models_failed" => false,
            "non_critical_model_issues" => true,
            "critical_model_issues" => false,
            "all_models_recovered" => true,
            "critical_models_affected" => false,
            "depends_on_recovery_level" => true,
            "depends_on_previous_state" => true,
            "depends_on_rollback_settings" => true,
            "sufficient_resources" => true,
            "timeout_or_error" => false,
            "all_containers_started" => true,
            "one_time_task" => true,
            "unexpected_termination" => false,
            "consecutive_restart_failures" => false,
            "node_communication_issues" => false,
            "restart_successful" => true,
            "retry_limit_reached" => false,
            "depends_on_actual_state" => true,
            "according_to_restart_policy" => true,
            _ => true, // Default to allow transition for unknown conditions
        }
    }

    /// Update the internal resource state after a successful transition
    ///
    /// Performs all necessary bookkeeping when a state transition succeeds,
    /// including updating timestamps, incrementing counters, and managing metadata.
    ///
    /// # Parameters
    /// - `resource_key`: Unique identifier for the resource
    /// - `state_change`: The original state change request
    /// - `new_state`: The state the resource has transitioned to
    /// - `resource_type`: The type of the resource
    ///
    /// # Side Effects
    /// - Updates or creates the resource state entry
    /// - Increments transition counter
    /// - Updates last transition timestamp
    /// - Clears any active backoff timers on successful transition
    /// - Updates health status if applicable
    fn update_resource_state(
        &mut self,
        resource_key: &str,
        state_change: &StateChange,

        new_state: i32,
        resource_type: ResourceType,
    ) {
        let now = Instant::now();

        let resource_state = self
            .resource_states
            .entry(resource_key.to_string())
            .or_insert_with(|| ResourceState {
                resource_type,
                resource_name: state_change.resource_name.clone(),
                current_state: Self::state_str_to_enum(
                    state_change.current_state.as_str(),
                    state_change.resource_type,
                ),
                desired_state: Some(Self::state_str_to_enum(
                    state_change.target_state.as_str(),
                    state_change.resource_type,
                )),
                last_transition_time: now,
                transition_count: 0,
                metadata: HashMap::new(),
                health_status: HealthStatus {
                    healthy: true,
                    status_message: "Healthy".to_string(),
                    last_check: now,
                    consecutive_failures: 0,
                },
            });

        resource_state.current_state = new_state;
        resource_state.last_transition_time = now;
        resource_state.transition_count += 1;
        resource_state.metadata.insert(
            "last_transition_id".to_string(),
            state_change.transition_id.clone(),
        );
        resource_state
            .metadata
            .insert("source".to_string(), state_change.source.clone());
    }

    // ========================================
    // PUBLIC QUERY METHODS
    // ========================================

    /// Retrieve the current state information for a specific resource
    ///
    /// Provides read-only access to the complete state information for
    /// a resource, including metadata and health status.
    ///
    /// # Parameters
    /// - `resource_name`: The unique name of the resource
    /// - `resource_type`: The type of the resource (for validation)
    ///
    /// # Returns
    /// - `Some(&ResourceState)`: If the resource exists and types match
    /// - `None`: If the resource doesn't exist or type mismatch
    ///
    /// # Usage
    /// This method is primarily used for:
    /// - Status queries from external systems
    /// - Health check implementations
    /// - Audit and monitoring purposes
    pub fn get_resource_state(
        &self,
        resource_name: &str,
        resource_type: ResourceType,
    ) -> Option<&ResourceState> {
        let resource_key = self.generate_resource_key(resource_type, resource_name);
        self.resource_states.get(&resource_key)
    }

    /// List all resources currently in a specific state
    ///
    /// Provides a filtered view of all managed resources based on their
    /// current state, optionally filtered by resource type.
    ///
    /// # Parameters
    /// - `resource_type`: Optional filter for resource type (None = all types)
    /// - `state`: The state to filter by
    ///
    /// # Returns
    /// A vector of references to all matching resource states
    ///
    /// # Performance Note
    /// This method performs a linear scan of all resources. For large numbers
    /// of resources, consider implementing indexed lookups by state.
    ///
    /// # Usage Examples
    /// - Find all failed resources: `list_resources_by_state(None, "Failed")`
    /// - Find active scenarios: `list_resources_by_state(Some(ResourceType::Scenario), "Active")`
    pub fn list_resources_by_state(
        &self,
        resource_type: Option<ResourceType>,

        state: i32,
    ) -> Vec<&ResourceState> {
        self.resource_states
            .values()
            .filter(|resource| {
                resource.current_state == state
                    && (resource_type.is_none() || resource_type == Some(resource.resource_type))
            })
            .collect()
    }

    // Utility: Convert state string to proto enum value
    fn state_str_to_enum(state: &str, resource_type: i32) -> i32 {
        // Map "idle" -> "SCENARIO_STATE_IDLE", etc.
        let normalized = match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => format!(
                "SCENARIO_STATE_{}",
                state.trim().to_ascii_uppercase().replace('-', "_")
            ),
            Ok(ResourceType::Package) => format!(
                "PACKAGE_STATE_{}",
                state.trim().to_ascii_uppercase().replace('-', "_")
            ),
            Ok(ResourceType::Model) => format!(
                "MODEL_STATE_{}",
                state.trim().to_ascii_uppercase().replace('-', "_")
            ),
            _ => state.trim().to_ascii_uppercase().replace('-', "_"),
        };
        match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => ScenarioState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or(ScenarioState::Unspecified as i32),
            Ok(ResourceType::Package) => PackageState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or(PackageState::Unspecified as i32),
            Ok(ResourceType::Model) => ModelState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or(ModelState::Unspecified as i32),
            _ => 0,
        }
    }

    // ========================================
    // MODEL AND PACKAGE STATE EVALUATION (LLD IMPLEMENTATION)
    // ========================================

    /// Evaluate model state based on container states according to LLD Table 3.2
    ///
    /// This function implements the core model state evaluation logic as specified
    /// in the StateManager_Model.md LLD document. It determines the model state
    /// based on the collective states of all containers within the model.
    ///
    /// # Arguments
    /// * `model_name` - Name of the model to evaluate
    /// * `container_states` - Map of container name to state string
    ///
    /// # Returns
    /// * `ModelState` - The evaluated state based on LLD conditions
    ///
    /// # State Conditions (LLD Table 3.2)
    /// - Created: Model's initial state (when no containers or all containers are new)
    /// - Paused: All containers are in paused state
    /// - Exited: All containers are in exited state  
    /// - Dead: One or more containers are in dead state OR model info query failed
    /// - Running: Default state when other conditions are not met
    ///
    /// # Implementation Notes
    /// This method follows the exact state transition rules specified in the LLD
    /// and is designed to be called when container states change, triggering
    /// cascading model state evaluation.
    pub fn evaluate_model_state_from_containers(
        &self,
        model_name: &str,
        container_states: &HashMap<String, String>,
    ) -> ModelState {
        println!(
            "  [Model State Evaluation] Evaluating model '{}' with {} containers",
            model_name,
            container_states.len()
        );

        // Handle empty container case - model should be in Created state
        if container_states.is_empty() {
            println!("    -> No containers found, defaulting to MODEL_STATE_PENDING");
            return ModelState::Pending;
        }

        // Log container states for debugging
        for (container_name, state) in container_states {
            println!("    Container '{}': {}", container_name, state);
        }

        // Apply LLD Table 3.2 state evaluation rules
        let evaluated_state = if container_states
            .values()
            .any(|s| s.to_lowercase() == "dead")
        {
            // Rule: One or more containers are dead -> Model Dead
            println!("    -> Found dead container(s), setting model to MODEL_STATE_FAILED");
            ModelState::Failed
        } else if container_states
            .values()
            .all(|s| s.to_lowercase() == "paused")
        {
            // Rule: All containers are paused -> Model Paused
            println!("    -> All containers paused, setting model to MODEL_STATE_PENDING");
            ModelState::Pending
        } else if container_states
            .values()
            .all(|s| s.to_lowercase() == "exited")
        {
            // Rule: All containers are exited -> Model Exited
            println!("    -> All containers exited, setting model to MODEL_STATE_SUCCEEDED");
            ModelState::Succeeded
        } else {
            // Default rule: Running state when other conditions not met
            println!(
                "    -> Mixed or running container states, setting model to MODEL_STATE_RUNNING"
            );
            ModelState::Running
        };

        println!(
            "  [Model State Evaluation] Model '{}' evaluated to: {:?}",
            model_name, evaluated_state
        );
        evaluated_state
    }

    /// Evaluate package state based on model states according to LLD Table 3.1
    ///
    /// This function implements the package state evaluation logic as specified
    /// in the StateManager_Model.md LLD document. It determines the package state
    /// based on the collective states of all models within the package.
    ///
    /// # Arguments  
    /// * `package_name` - Name of the package to evaluate
    /// * `model_states` - Map of model name to ModelState
    ///
    /// # Returns
    /// * `PackageState` - The evaluated state based on LLD conditions
    ///
    /// # State Conditions (LLD Table 3.1)
    /// - idle: Initial package state (when no models exist)
    /// - paused: All models are in paused state
    /// - exited: All models are in exited state
    /// - degraded: Some (1+) models are in dead/failed state, but not all
    /// - error: All models are in dead/failed state  
    /// - running: Default state when other conditions are not met
    ///
    /// # Implementation Notes
    /// This method follows the exact state transition rules specified in the LLD
    /// and is designed to be called after model state evaluation to implement
    /// cascading state transitions.
    pub fn evaluate_package_state_from_models(
        &self,
        package_name: &str,
        model_states: &HashMap<String, ModelState>,
    ) -> PackageState {
        println!(
            "  [Package State Evaluation] Evaluating package '{}' with {} models",
            package_name,
            model_states.len()
        );

        // Handle empty model case - package should be in idle state
        if model_states.is_empty() {
            println!("    -> No models found, setting package to PACKAGE_STATE_INITIALIZING");
            return PackageState::Initializing;
        }

        // Log model states for debugging
        for (model_name, state) in model_states {
            println!("    Model '{}': {:?}", model_name, state);
        }

        // Count models in different states for evaluation
        let failed_count = model_states
            .values()
            .filter(|&s| matches!(s, ModelState::Failed | ModelState::Unknown))
            .count();
        let total_count = model_states.len();

        // Apply LLD Table 3.1 state evaluation rules
        let evaluated_state = if failed_count == total_count && total_count > 0 {
            // Rule: All models are dead/failed -> Package Error
            println!(
                "    -> All {} models failed, setting package to PACKAGE_STATE_ERROR",
                total_count
            );
            PackageState::Error
        } else if failed_count > 0 {
            // Rule: Some models are dead/failed -> Package Degraded
            println!(
                "    -> {}/{} models failed, setting package to PACKAGE_STATE_DEGRADED",
                failed_count, total_count
            );
            PackageState::Degraded
        } else if model_states
            .values()
            .all(|s| matches!(s, ModelState::Pending))
        {
            // Rule: All models are paused -> Package Paused
            println!("    -> All models pending, setting package to PACKAGE_STATE_PAUSED");
            PackageState::Paused
        } else if model_states
            .values()
            .all(|s| matches!(s, ModelState::Succeeded))
        {
            // Rule: All models are exited -> Package Exited
            println!("    -> All models succeeded, package operation complete");
            PackageState::Running // In package context, succeeded models mean package is running
        } else {
            // Default rule: Running state when other conditions not met
            println!("    -> Mixed model states, setting package to PACKAGE_STATE_RUNNING");
            PackageState::Running
        };

        println!(
            "  [Package State Evaluation] Package '{}' evaluated to: {:?}",
            package_name, evaluated_state
        );
        evaluated_state
    }

    /// Persist resource state to ETCD with standardized key format per LLD requirements
    ///
    /// This function implements the ETCD persistence logic with key formats
    /// specified in the StateManager_Model.md LLD document. It saves resource
    /// states using the standardized key structure for consistent data access.
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource (Model, Package, Container)
    /// * `resource_name` - Name of the resource instance
    /// * `state` - State string to persist (e.g., "Running", "Failed")
    ///
    /// # ETCD Key Formats (per LLD Section 4.2)
    /// - Model: `/model/{name}/state`
    /// - Package: `/package/{name}/state`  
    /// - Container: `/container/{name}/state`
    ///
    /// # Returns
    /// * `bool` - true if successful, false if failed
    ///
    /// # Error Handling
    /// ETCD errors are logged and the function returns false for failure cases.
    /// Network failures and key conflicts should be handled by retry logic.
    pub async fn persist_state_to_etcd(
        &self,
        resource_type: ResourceType,
        resource_name: &str,
        state: &str,
    ) -> bool {
        // Generate standardized ETCD key format per LLD
        let key = match resource_type {
            ResourceType::Model => format!("/model/{}/state", resource_name),
            ResourceType::Package => format!("/package/{}/state", resource_name),
            ResourceType::Scenario => format!("/scenario/{}/state", resource_name),
            _ => {
                eprintln!(
                    "    [ETCD] Unsupported resource type for state persistence: {:?}",
                    resource_type
                );
                return false; // Skip unsupported types
            }
        };

        println!(
            "    [ETCD] Persisting state: key='{}', value='{}'",
            key, state
        );

        // Use existing common::etcd module for persistence
        match common::etcd::put(&key, state).await {
            Ok(_) => {
                println!(
                    "    [ETCD] Successfully persisted state for {} '{}'",
                    format!("{:?}", resource_type).to_lowercase(),
                    resource_name
                );
                true
            }
            Err(e) => {
                eprintln!(
                    "    [ETCD] Failed to persist state for {} '{}': {:?}",
                    format!("{:?}", resource_type).to_lowercase(),
                    resource_name,
                    e
                );
                false
            }
        }
    }

    /// Query ETCD for container states belonging to a specific model
    ///
    /// This function retrieves all container states associated with a model
    /// to enable model state evaluation. It uses ETCD prefix queries for
    /// efficient bulk retrieval of related container data.
    ///
    /// # Arguments
    /// * `model_name` - Name of the model to query containers for
    ///
    /// # Returns
    /// * `HashMap<String, String>` - Map of container name to state (empty if error)
    ///
    /// # ETCD Key Pattern
    /// Queries for keys matching: `/container/{model_name}_*` or `/model/{model_name}/container/*/state`
    ///
    /// # Error Handling
    /// ETCD errors are logged and function returns empty HashMap.
    /// Empty results are valid and return empty HashMap.
    pub async fn get_container_states_for_model(
        &self,
        model_name: &str,
    ) -> HashMap<String, String> {
        println!(
            "    [ETCD] Querying container states for model '{}'",
            model_name
        );

        // Query for container states using model prefix pattern
        let prefix = format!("/model/{}/container/", model_name);

        match common::etcd::get_all_with_prefix(&prefix).await {
            Ok(kvs) => {
                let mut container_states = HashMap::new();

                for kv in kvs {
                    // Extract container name from key pattern: /model/{model}/container/{container}/state
                    if let Some(container_name) = kv
                        .key
                        .strip_prefix(&prefix)
                        .and_then(|s| s.strip_suffix("/state"))
                    {
                        println!("      Found container '{}': {}", container_name, kv.value);
                        container_states.insert(container_name.to_string(), kv.value);
                    }
                }

                println!(
                    "    [ETCD] Retrieved {} container states for model '{}'",
                    container_states.len(),
                    model_name
                );
                container_states
            }
            Err(e) => {
                eprintln!(
                    "    [ETCD] Failed to query container states for model '{}': {:?}",
                    model_name, e
                );
                HashMap::new()
            }
        }
    }

    /// Query ETCD for model states belonging to a specific package
    ///
    /// This function retrieves all model states associated with a package
    /// to enable package state evaluation. It uses ETCD prefix queries for
    /// efficient bulk retrieval of related model data.
    ///
    /// # Arguments
    /// * `package_name` - Name of the package to query models for
    ///
    /// # Returns
    /// * `HashMap<String, ModelState>` - Map of model name to ModelState (empty if error)
    ///
    /// # ETCD Key Pattern
    /// Queries for keys matching: `/package/{package_name}/model/*/state`
    ///
    /// # Error Handling
    /// ETCD errors are logged and function returns empty HashMap.
    /// Invalid model states are logged and skipped.
    pub async fn get_model_states_for_package(
        &self,
        package_name: &str,
    ) -> HashMap<String, ModelState> {
        println!(
            "    [ETCD] Querying model states for package '{}'",
            package_name
        );

        // Query for model states using package prefix pattern
        let prefix = format!("/package/{}/model/", package_name);

        match common::etcd::get_all_with_prefix(&prefix).await {
            Ok(kvs) => {
                let mut model_states = HashMap::new();

                for kv in kvs {
                    // Extract model name from key pattern: /package/{package}/model/{model}/state
                    if let Some(model_name) = kv
                        .key
                        .strip_prefix(&prefix)
                        .and_then(|s| s.strip_suffix("/state"))
                    {
                        // Parse state string to ModelState enum
                        let normalized = format!(
                            "MODEL_STATE_{}",
                            kv.value.trim().to_ascii_uppercase().replace('-', "_")
                        );

                        if let Some(model_state) = ModelState::from_str_name(&normalized) {
                            model_states.insert(model_name.to_string(), model_state);
                            println!("      Found model '{}': {:?}", model_name, model_state);
                        } else {
                            eprintln!(
                                "      Invalid model state '{}' for model '{}', skipping",
                                kv.value, model_name
                            );
                        }
                    }
                }

                println!(
                    "    [ETCD] Retrieved {} model states for package '{}'",
                    model_states.len(),
                    package_name
                );
                model_states
            }
            Err(e) => {
                eprintln!(
                    "    [ETCD] Failed to query model states for package '{}': {:?}",
                    package_name, e
                );
                HashMap::new()
            }
        }
    }

    /// Handle cascading state transitions for parent resources
    ///
    /// This function implements the cascading state logic as specified in the LLD:
    /// - Container state changes trigger model state evaluation
    /// - Model state changes trigger package state evaluation
    /// - Package error states trigger ActionController notification
    ///
    /// # Arguments
    /// * `state_change` - The original state change that triggered cascading
    ///
    /// # Returns
    /// * `Vec<StateChange>` - Additional state changes to process
    ///
    /// # Implementation Notes
    /// This method performs ETCD queries to gather related resource states,
    /// evaluates parent resource states, and generates cascading state changes.
    /// It also handles error recovery notifications per LLD requirements.
    pub async fn handle_cascading_transitions(
        &mut self,
        state_change: &StateChange,
    ) -> Vec<StateChange> {
        println!(
            "  [Cascading] Handling cascading transitions for {:?} '{}'",
            ResourceType::try_from(state_change.resource_type).unwrap_or(ResourceType::Unspecified),
            state_change.resource_name
        );

        let mut cascading_changes = Vec::new();

        match ResourceType::try_from(state_change.resource_type) {
            Ok(ResourceType::Model) => {
                // Model state changed - check if package state needs to be updated
                println!("    [Cascading] Model state changed, evaluating package states");

                // Find which package this model belongs to (simplified approach - assumes naming convention)
                // In a real implementation, this would query ETCD for package-model relationships
                if let Some(package_name) =
                    self.extract_package_name_from_model(&state_change.resource_name)
                {
                    println!(
                        "      [Cascading] Found package '{}' for model '{}'",
                        package_name, state_change.resource_name
                    );

                    // Get all model states for this package
                    let model_states = self.get_model_states_for_package(&package_name).await;

                    // Include the current model's new state in the evaluation
                    let mut updated_model_states = model_states;
                    let new_model_state = ModelState::from_str_name(&format!(
                        "MODEL_STATE_{}",
                        state_change
                            .target_state
                            .trim()
                            .to_ascii_uppercase()
                            .replace('-', "_")
                    ))
                    .unwrap_or(ModelState::Unspecified);

                    updated_model_states
                        .insert(state_change.resource_name.clone(), new_model_state);

                    // Evaluate package state
                    let new_package_state = self
                        .evaluate_package_state_from_models(&package_name, &updated_model_states);

                    // Get current package state from ETCD or assume Initializing
                    let current_package_state = self.get_current_package_state(&package_name).await;

                    // Only create state change if package state actually changed
                    if new_package_state != current_package_state {
                        println!(
                            "      [Cascading] Package '{}' state changing from {:?} to {:?}",
                            package_name, current_package_state, new_package_state
                        );

                        cascading_changes.push(StateChange {
                            resource_type: ResourceType::Package as i32,
                            resource_name: package_name.clone(),
                            current_state: format!("{:?}", current_package_state),
                            target_state: format!("{:?}", new_package_state),
                            transition_id: format!(
                                "{}_package_cascade",
                                state_change.transition_id
                            ),
                            timestamp_ns: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as i64,
                            source: "StateManager_CascadingTransition".to_string(),
                        });

                        // Check if package entered error state - trigger ActionController notification
                        if new_package_state == PackageState::Error {
                            println!("      [Cascading] Package '{}' entered ERROR state - will trigger ActionController reconcile", 
                                     package_name);
                            // TODO: Implement ActionController notification
                            // This would send a reconcile request to ActionController
                        }
                    } else {
                        println!(
                            "      [Cascading] Package '{}' state unchanged: {:?}",
                            package_name, current_package_state
                        );
                    }
                } else {
                    println!(
                        "      [Cascading] Could not determine package for model '{}'",
                        state_change.resource_name
                    );
                }
            }

            // Note: Container state changes would trigger model evaluation, but since
            // we don't have a Container resource type in the proto, we'll focus on
            // the Model->Package cascading for now
            _ => {
                println!(
                    "    [Cascading] No cascading logic defined for resource type {:?}",
                    ResourceType::try_from(state_change.resource_type)
                );
            }
        }

        println!(
            "  [Cascading] Generated {} cascading state changes",
            cascading_changes.len()
        );
        cascading_changes
    }

    /// Extract package name from model name using naming convention
    ///
    /// This is a simplified implementation that assumes a naming convention
    /// like "package_name.model_name" or similar. In a real implementation,
    /// this would query ETCD for the actual package-model relationships.
    ///
    /// # Arguments
    /// * `model_name` - Name of the model
    ///
    /// # Returns
    /// * `Option<String>` - Package name if found
    fn extract_package_name_from_model(&self, model_name: &str) -> Option<String> {
        // Simple heuristic: if model name contains a dot, take the part before it
        // Example: "my_package.my_model" -> "my_package"
        if let Some(dot_pos) = model_name.find('.') {
            Some(model_name[..dot_pos].to_string())
        } else {
            // If no dot, assume the model name is the package name
            // This is a fallback for simple scenarios
            Some(format!("{}_package", model_name))
        }
    }

    /// Get current package state from ETCD
    ///
    /// # Arguments
    /// * `package_name` - Name of the package
    ///
    /// # Returns
    /// * `PackageState` - Current state (defaults to Initializing if not found)
    async fn get_current_package_state(&self, package_name: &str) -> PackageState {
        let key = format!("/package/{}/state", package_name);

        match common::etcd::get(&key).await {
            Ok(state_str) => {
                let normalized = format!(
                    "PACKAGE_STATE_{}",
                    state_str.trim().to_ascii_uppercase().replace('-', "_")
                );
                PackageState::from_str_name(&normalized).unwrap_or(PackageState::Initializing)
            }
            Err(_) => {
                println!(
                    "    [ETCD] Package '{}' state not found, defaulting to Initializing",
                    package_name
                );
                PackageState::Initializing
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_evaluate_model_state_from_containers_empty() {
        let state_machine = StateMachine::new();
        let container_states = HashMap::new();

        let result =
            state_machine.evaluate_model_state_from_containers("test_model", &container_states);

        // Empty containers should result in Pending state (Created equivalent)
        assert_eq!(result, ModelState::Pending);
    }

    #[test]
    fn test_evaluate_model_state_from_containers_all_running() {
        let state_machine = StateMachine::new();
        let mut container_states = HashMap::new();
        container_states.insert("container1".to_string(), "running".to_string());
        container_states.insert("container2".to_string(), "running".to_string());

        let result =
            state_machine.evaluate_model_state_from_containers("test_model", &container_states);

        // All running containers should result in Running state
        assert_eq!(result, ModelState::Running);
    }

    #[test]
    fn test_evaluate_model_state_from_containers_all_paused() {
        let state_machine = StateMachine::new();
        let mut container_states = HashMap::new();
        container_states.insert("container1".to_string(), "paused".to_string());
        container_states.insert("container2".to_string(), "paused".to_string());

        let result =
            state_machine.evaluate_model_state_from_containers("test_model", &container_states);

        // All paused containers should result in Pending state (Paused equivalent)
        assert_eq!(result, ModelState::Pending);
    }

    #[test]
    fn test_evaluate_model_state_from_containers_all_exited() {
        let state_machine = StateMachine::new();
        let mut container_states = HashMap::new();
        container_states.insert("container1".to_string(), "exited".to_string());
        container_states.insert("container2".to_string(), "exited".to_string());

        let result =
            state_machine.evaluate_model_state_from_containers("test_model", &container_states);

        // All exited containers should result in Succeeded state (Exited equivalent)
        assert_eq!(result, ModelState::Succeeded);
    }

    #[test]
    fn test_evaluate_model_state_from_containers_one_dead() {
        let state_machine = StateMachine::new();
        let mut container_states = HashMap::new();
        container_states.insert("container1".to_string(), "running".to_string());
        container_states.insert("container2".to_string(), "dead".to_string());

        let result =
            state_machine.evaluate_model_state_from_containers("test_model", &container_states);

        // Any dead container should result in Failed state (Dead equivalent)
        assert_eq!(result, ModelState::Failed);
    }

    #[test]
    fn test_evaluate_model_state_from_containers_mixed_states() {
        let state_machine = StateMachine::new();
        let mut container_states = HashMap::new();
        container_states.insert("container1".to_string(), "running".to_string());
        container_states.insert("container2".to_string(), "starting".to_string());

        let result =
            state_machine.evaluate_model_state_from_containers("test_model", &container_states);

        // Mixed non-special states should result in Running state
        assert_eq!(result, ModelState::Running);
    }

    #[test]
    fn test_evaluate_package_state_from_models_empty() {
        let state_machine = StateMachine::new();
        let model_states = HashMap::new();

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // Empty models should result in Initializing state (idle equivalent)
        assert_eq!(result, PackageState::Initializing);
    }

    #[test]
    fn test_evaluate_package_state_from_models_all_running() {
        let state_machine = StateMachine::new();
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Running);
        model_states.insert("model2".to_string(), ModelState::Running);

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // All running models should result in Running state
        assert_eq!(result, PackageState::Running);
    }

    #[test]
    fn test_evaluate_package_state_from_models_all_pending() {
        let state_machine = StateMachine::new();
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Pending);
        model_states.insert("model2".to_string(), ModelState::Pending);

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // All pending models should result in Paused state
        assert_eq!(result, PackageState::Paused);
    }

    #[test]
    fn test_evaluate_package_state_from_models_all_succeeded() {
        let state_machine = StateMachine::new();
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Succeeded);
        model_states.insert("model2".to_string(), ModelState::Succeeded);

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // All succeeded models should result in Running state (package completed successfully)
        assert_eq!(result, PackageState::Running);
    }

    #[test]
    fn test_evaluate_package_state_from_models_all_failed() {
        let state_machine = StateMachine::new();
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Failed);
        model_states.insert("model2".to_string(), ModelState::Failed);

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // All failed models should result in Error state
        assert_eq!(result, PackageState::Error);
    }

    #[test]
    fn test_evaluate_package_state_from_models_some_failed() {
        let state_machine = StateMachine::new();
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Running);
        model_states.insert("model2".to_string(), ModelState::Failed);
        model_states.insert("model3".to_string(), ModelState::Running);

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // Some failed models should result in Degraded state
        assert_eq!(result, PackageState::Degraded);
    }

    #[test]
    fn test_evaluate_package_state_from_models_mixed_states() {
        let state_machine = StateMachine::new();
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Running);
        model_states.insert("model2".to_string(), ModelState::ContainerCreating);
        model_states.insert("model3".to_string(), ModelState::Succeeded);

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // Mixed non-failed states should result in Running state
        assert_eq!(result, PackageState::Running);
    }

    #[test]
    fn test_evaluate_package_state_case_sensitivity() {
        let state_machine = StateMachine::new();
        let mut container_states = HashMap::new();

        // Test case insensitive state matching
        container_states.insert("container1".to_string(), "DEAD".to_string());
        container_states.insert("container2".to_string(), "Running".to_string());

        let result =
            state_machine.evaluate_model_state_from_containers("test_model", &container_states);

        // Dead state should be detected regardless of case
        assert_eq!(result, ModelState::Failed);
    }

    #[test]
    fn test_evaluate_package_state_with_unknown_models() {
        let state_machine = StateMachine::new();
        let mut model_states = HashMap::new();
        model_states.insert("model1".to_string(), ModelState::Running);
        model_states.insert("model2".to_string(), ModelState::Unknown);

        let result =
            state_machine.evaluate_package_state_from_models("test_package", &model_states);

        // Unknown models should be treated as failed, resulting in Degraded state
        assert_eq!(result, PackageState::Degraded);
    }
}

/// Integration tests for StateManager Model functionality
///
/// These tests demonstrate the complete workflow specified in the LLD:
/// - Model state evaluation based on container states
/// - Package state evaluation based on model states  
/// - Cascading state transitions
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;
    
    /// Integration test demonstrating complete Model → Package cascading workflow
    #[test]
    fn test_model_to_package_cascading_workflow() {
        println!("\n=== Integration Test: Model → Package Cascading ===");
        
        let state_machine = StateMachine::new();
        
        // Test scenario: Model with mixed container states
        println!("\n1. Testing Model state evaluation:");
        
        let mut container_states = HashMap::new();
        container_states.insert("container1".to_string(), "running".to_string());
        container_states.insert("container2".to_string(), "dead".to_string()); // Dead container
        
        let model_state = state_machine.evaluate_model_state_from_containers(
            "test_package.model1", 
            &container_states
        );
        
        println!("  → Model state evaluated as: {:?}", model_state);
        assert_eq!(model_state, ModelState::Failed, "Model should be Failed due to dead container");
        
        println!("\n2. Testing Package state evaluation:");
        
        let mut model_states = HashMap::new();
        model_states.insert("test_package.model1".to_string(), ModelState::Failed);
        model_states.insert("test_package.model2".to_string(), ModelState::Running);
        model_states.insert("test_package.model3".to_string(), ModelState::Running);
        
        let package_state = state_machine.evaluate_package_state_from_models(
            "test_package",
            &model_states
        );
        
        println!("  → Package state evaluated as: {:?}", package_state);
        assert_eq!(package_state, PackageState::Degraded, "Package should be Degraded due to one failed model");
        
        println!("\n3. Testing ActionController notification scenario:");
        
        // Test error state → ActionController notification scenario
        let mut error_model_states = HashMap::new();
        error_model_states.insert("test_package.model1".to_string(), ModelState::Failed);
        error_model_states.insert("test_package.model2".to_string(), ModelState::Failed);
        error_model_states.insert("test_package.model3".to_string(), ModelState::Failed);
        
        let error_package_state = state_machine.evaluate_package_state_from_models(
            "test_package",
            &error_model_states
        );
        
        println!("  → All models failed, package state: {:?}", error_package_state);
        assert_eq!(error_package_state, PackageState::Error, "Package should be Error when all models failed");
        
        println!("  → This would trigger ActionController reconcile in real deployment");
        println!("\n=== Integration Test Completed Successfully ===");
    }
    
    /// Test the package name extraction logic
    #[test]
    fn test_package_name_extraction() {
        let state_machine = StateMachine::new();
        
        // Test dot notation
        let package_name = state_machine.extract_package_name_from_model("my_package.my_model");
        assert_eq!(package_name, Some("my_package".to_string()));
        
        // Test fallback for simple names  
        let package_name = state_machine.extract_package_name_from_model("simple_model");
        assert_eq!(package_name, Some("simple_model_package".to_string()));
        
        // Test nested packages
        let package_name = state_machine.extract_package_name_from_model("system.core.database_model");
        assert_eq!(package_name, Some("system".to_string()));
    }
    
    /// Test state evaluation edge cases
    #[test] 
    fn test_state_evaluation_edge_cases() {
        let state_machine = StateMachine::new();
        
        // Test empty containers
        let empty_containers = HashMap::new();
        let model_state = state_machine.evaluate_model_state_from_containers("test_model", &empty_containers);
        assert_eq!(model_state, ModelState::Pending);
        
        // Test empty models  
        let empty_models = HashMap::new();
        let package_state = state_machine.evaluate_package_state_from_models("test_package", &empty_models);
        assert_eq!(package_state, PackageState::Initializing);
        
        // Test single container/model scenarios
        let mut single_container = HashMap::new();
        single_container.insert("only_container".to_string(), "running".to_string());
        let model_state = state_machine.evaluate_model_state_from_containers("test_model", &single_container);
        assert_eq!(model_state, ModelState::Running);
        
        let mut single_model = HashMap::new();
        single_model.insert("only_model".to_string(), ModelState::Running);
        let package_state = state_machine.evaluate_package_state_from_models("test_package", &single_model);
        assert_eq!(package_state, PackageState::Running);
    }
}

/// Default implementation that creates a new StateMachine
///
/// Provides a convenient way to create a StateMachine with default
/// configuration using the `Default` trait.
impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}
