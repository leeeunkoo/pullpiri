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

    // ========================================
    // CONTAINER-BASED STATE MANAGEMENT
    // ========================================

    /// Process container state information and trigger model state changes
    ///
    /// This method implements the core logic from StateManager_Model.md:
    /// - Analyzes container states for each model
    /// - Determines new model state based on container aggregation rules
    /// - Triggers cascading package state updates
    /// - Stores states in ETCD using specified format
    ///
    /// # Arguments
    /// * `node_name` - Name of the node containing the containers
    /// * `containers` - List of container information from nodeagent
    ///
    /// # State Aggregation Rules (from LLD):
    /// - Created: model's initial state
    /// - Paused: all containers are paused
    /// - Exited: all containers are exited  
    /// - Dead: one or more containers are dead OR model info retrieval failed
    /// - Running: default state when other conditions aren't met
    pub async fn process_container_states(&mut self, node_name: &str, containers: &[common::monitoringserver::ContainerInfo]) {
        println!("=== PROCESSING CONTAINER STATES FOR MODEL STATE MANAGEMENT ===");
        println!("Node: {}, Containers: {}", node_name, containers.len());

        // Group containers by model (using annotation or naming convention)
        let models_containers = self.group_containers_by_model(containers);
        
        // Collect model state changes to process them after analysis
        let mut model_state_changes = Vec::new();

        for (model_name, model_containers) in models_containers {
            // Determine new model state based on container states
            let new_model_state = self.determine_model_state_from_containers(&model_containers);
            
            // Get current model state from ETCD
            let current_model_state = self.get_model_state_from_etcd(&model_name).await;
            
            // Only update if state has changed
            if current_model_state != new_model_state {
                println!("Model '{}' state change: {:?} -> {:?}", 
                    model_name, current_model_state, new_model_state);
                model_state_changes.push((model_name, new_model_state));
            }
        }

        // Process all model state changes
        for (model_name, new_model_state) in model_state_changes {
            // Store new model state in ETCD
            if let Err(e) = self.store_model_state_to_etcd(&model_name, new_model_state).await {
                eprintln!("Failed to store model state: {:?}", e);
                continue;
            }

            // Trigger cascading package state evaluation
            self.evaluate_package_states_for_model(&model_name, new_model_state).await;
        }
        
        println!("=== CONTAINER STATE PROCESSING COMPLETED ===");
    }

    /// Group containers by their associated model
    ///
    /// Uses container annotations or naming conventions to determine which model
    /// each container belongs to. This is crucial for aggregating container states.
    fn group_containers_by_model<'a>(&self, containers: &'a [common::monitoringserver::ContainerInfo]) -> HashMap<String, Vec<&'a common::monitoringserver::ContainerInfo>> {
        let mut models_containers: HashMap<String, Vec<&'a common::monitoringserver::ContainerInfo>> = HashMap::new();
        
        for container in containers {
            // Try to determine model name from container annotations
            let model_name = if let Some(model) = container.annotation.get("model") {
                model.clone()
            } else if let Some(model) = container.annotation.get("pod") {
                model.clone()
            } else if !container.names.is_empty() {
                // Fallback: use first container name as model identifier
                // This is a simplified approach - real implementation might parse naming conventions
                container.names[0].clone()
            } else {
                // Last resort: use container ID
                format!("model_{}", &container.id[..8])
            };

            models_containers.entry(model_name).or_default().push(container);
        }
        
        models_containers
    }

    /// Determine model state based on container states according to LLD rules
    ///
    /// Implements the exact state aggregation logic from StateManager_Model.md:
    /// - Created: model's initial state (handled separately)
    /// - Paused: ALL containers are paused
    /// - Exited: ALL containers are exited
    /// - Dead: ONE OR MORE containers are dead OR model info retrieval failed
    /// - Running: default when other conditions aren't met
    fn determine_model_state_from_containers(&self, containers: &[&common::monitoringserver::ContainerInfo]) -> ModelState {
        if containers.is_empty() {
            return ModelState::Pending; // Use Pending instead of Created
        }

        let mut paused_count = 0;
        let mut exited_count = 0;
        let mut dead_count = 0;
        let mut _running_count = 0; // Track running containers but not used in current logic
        let total_count = containers.len();

        for container in containers {
            // Check container state from the state map
            let container_state = container.state.get("status").unwrap_or(&"unknown".to_string()).to_lowercase();
            
            match container_state.as_str() {
                "paused" => paused_count += 1,
                "exited" | "stopped" => exited_count += 1,
                "dead" | "failed" | "error" => dead_count += 1,
                "running" | "restarting" => _running_count += 1,
                _ => {
                    // Unknown state treated as potentially problematic
                    println!("Warning: Unknown container state '{}' for container {}", 
                        container_state, container.id);
                }
            }
        }

        // Apply state aggregation rules from LLD
        if dead_count > 0 {
            // One or more containers are dead
            ModelState::Failed
        } else if paused_count == total_count {
            // ALL containers are paused
            ModelState::Unknown // Using Unknown for Paused since ModelState doesn't have Paused
        } else if exited_count == total_count {
            // ALL containers are exited
            ModelState::Succeeded
        } else {
            // Default state when other conditions aren't met
            ModelState::Running
        }
    }

    /// Get current model state from ETCD
    async fn get_model_state_from_etcd(&self, model_name: &str) -> ModelState {
        let key = format!("/model/{}/state", model_name);
        match common::etcd::get(&key).await {
            Ok(value) => {
                // Parse state string back to enum
                match value.as_str() {
                    "PENDING" => ModelState::Pending,
                    "RUNNING" => ModelState::Running,
                    "SUCCEEDED" => ModelState::Succeeded,
                    "FAILED" => ModelState::Failed,
                    "UNKNOWN" => ModelState::Unknown,
                    "CONTAINER_CREATING" => ModelState::ContainerCreating,
                    "CRASH_LOOP_BACK_OFF" => ModelState::CrashLoopBackOff,
                    _ => ModelState::Unknown,
                }
            },
            Err(_) => {
                // State not found - assume this is a new model
                ModelState::Pending
            }
        }
    }

    /// Store model state to ETCD using format specified in LLD
    async fn store_model_state_to_etcd(&self, model_name: &str, model_state: ModelState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("/model/{}/state", model_name);
        let value = model_state.as_str_name(); // e.g., "RUNNING"
        
        common::etcd::put(&key, value).await.map_err(|e| {
            eprintln!("Failed to save model state: {:?}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;
        
        println!("Stored model state: {} = {}", key, value);
        Ok(())
    }

    /// Evaluate package states when a model state changes
    ///
    /// This implements the cascading state management from StateManager_Package.md:
    /// - Retrieves all models for packages containing the changed model
    /// - Determines new package state based on model state aggregation
    /// - Updates package state in ETCD
    /// - Triggers ActionController reconcile if package becomes "error"
    async fn evaluate_package_states_for_model(&mut self, model_name: &str, _model_state: ModelState) {
        // Find packages that contain this model
        let packages = self.find_packages_containing_model(model_name).await;
        
        for package_name in packages {
            // Get all models in this package
            let package_models = self.get_models_in_package(&package_name).await;
            
            // Determine new package state based on all model states
            let new_package_state = self.determine_package_state_from_models(&package_models).await;
            
            // Get current package state
            let current_package_state = self.get_package_state_from_etcd(&package_name).await;
            
            // Update if state has changed
            if current_package_state != new_package_state {
                println!("Package '{}' state change: {:?} -> {:?}", 
                    package_name, current_package_state, new_package_state);
                
                // Store new package state in ETCD
                if let Err(e) = self.store_package_state_to_etcd(&package_name, new_package_state).await {
                    eprintln!("Failed to store package state: {:?}", e);
                    continue;
                }

                // Trigger ActionController reconcile if package enters error state
                if new_package_state == PackageState::Error {
                    self.trigger_action_controller_reconcile(&package_name).await;
                }
            }
        }
    }

    /// Find packages that contain the specified model
    async fn find_packages_containing_model(&self, model_name: &str) -> Vec<String> {
        // In a real implementation, this would query ETCD for package definitions
        // that include the specified model. For now, we'll use a simple convention
        // where package names are derived from model names or stored separately.
        
        // Try to get package info from ETCD - simplified approach
        match common::etcd::get_all_with_prefix("/package/").await {
            Ok(kvs) => {
                let mut packages = Vec::new();
                for kv in kvs {
                    if kv.key.contains("/models/") && kv.value.contains(model_name) {
                        // Extract package name from key like "/package/pkg1/models/model1"
                        if let Some(package_name) = kv.key.split('/').nth(2) {
                            if !packages.contains(&package_name.to_string()) {
                                packages.push(package_name.to_string());
                            }
                        }
                    }
                }
                packages
            },
            Err(_) => {
                // Fallback: assume model belongs to a package with similar name
                vec![format!("package_{}", model_name)]
            }
        }
    }

    /// Get all models in a package
    async fn get_models_in_package(&self, package_name: &str) -> Vec<String> {
        // Query ETCD for models in this package
        let prefix = format!("/package/{}/models/", package_name);
        match common::etcd::get_all_with_prefix(&prefix).await {
            Ok(kvs) => {
                kvs.iter()
                    .filter_map(|kv| kv.key.split('/').next_back().map(|s| s.to_string()))
                    .collect()
            },
            Err(_) => Vec::new(),
        }
    }

    /// Determine package state from model states according to LLD rules
    ///
    /// Implements the exact state aggregation logic from StateManager_Package.md:
    /// - idle: initial package state (handled separately)
    /// - paused: ALL models are paused
    /// - exited: ALL models are exited
    /// - degraded: SOME models are dead (but not all)
    /// - error: ALL models are dead
    /// - running: default when other conditions aren't met
    async fn determine_package_state_from_models(&self, model_names: &[String]) -> PackageState {
        if model_names.is_empty() {
            return PackageState::Initializing;
        }

        let mut model_states = Vec::new();
        for model_name in model_names {
            let state = self.get_model_state_from_etcd(model_name).await;
            model_states.push(state);
        }

        let total_count = model_states.len();
        let paused_count = model_states.iter().filter(|&&state| state == ModelState::Unknown).count(); // Using Unknown for paused
        let exited_count = model_states.iter().filter(|&&state| state == ModelState::Succeeded).count(); // Using Succeeded for exited
        let dead_count = model_states.iter().filter(|&&state| state == ModelState::Failed).count();

        // Apply package state aggregation rules from LLD
        if dead_count == total_count {
            // ALL models are dead
            PackageState::Error
        } else if dead_count > 0 {
            // SOME models are dead (but not all)
            PackageState::Degraded
        } else if paused_count == total_count {
            // ALL models are paused
            PackageState::Paused
        } else if exited_count == total_count {
            // ALL models are exited  
            // Note: PackageState doesn't have Exited, using Paused as closest
            PackageState::Paused
        } else {
            // Default when other conditions aren't met
            PackageState::Running
        }
    }

    /// Get current package state from ETCD
    async fn get_package_state_from_etcd(&self, package_name: &str) -> PackageState {
        let key = format!("/package/{}/state", package_name);
        match common::etcd::get(&key).await {
            Ok(value) => {
                // Parse state string back to enum
                match value.as_str() {
                    "INITIALIZING" => PackageState::Initializing,
                    "RUNNING" => PackageState::Running,
                    "DEGRADED" => PackageState::Degraded,
                    "ERROR" => PackageState::Error,
                    "PAUSED" => PackageState::Paused,
                    "UPDATING" => PackageState::Updating,
                    _ => PackageState::Initializing,
                }
            },
            Err(_) => {
                // State not found - assume this is a new package
                PackageState::Initializing
            }
        }
    }

    /// Store package state to ETCD using format specified in LLD
    async fn store_package_state_to_etcd(&self, package_name: &str, package_state: PackageState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("/package/{}/state", package_name);
        let value = package_state.as_str_name(); // e.g., "RUNNING"
        
        common::etcd::put(&key, value).await.map_err(|e| {
            eprintln!("Failed to save package state: {:?}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;
        
        println!("Stored package state: {} = {}", key, value);
        Ok(())
    }

    /// Trigger ActionController reconcile when package enters error state
    ///
    /// This implements the requirement from StateManager_Package.md:
    /// "package dead 상태 시 ActionController에 reconcile 요청"
    async fn trigger_action_controller_reconcile(&self, package_name: &str) {
        println!("TRIGGERING ActionController reconcile for package: {}", package_name);
        
        // Create reconcile request for ActionController
        let reconcile_request = common::actioncontroller::ReconcileRequest {
            scenario_name: package_name.to_string(), // Using package name as scenario name
            current: common::actioncontroller::PodStatus::Failed as i32, // Package is in error state
            desired: common::actioncontroller::PodStatus::Running as i32, // Desired state is running
        };

        // Send reconcile request to ActionController
        match common::actioncontroller::action_controller_connection_client::ActionControllerConnectionClient::connect(
            common::actioncontroller::connect_server()
        ).await {
            Ok(mut client) => {
                println!("  Connected to ActionController successfully");
                
                match client.reconcile(tonic::Request::new(reconcile_request)).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        println!("  ActionController reconcile response:");
                        println!("    Status: {}", resp.status);
                        println!("    Description: {}", resp.desc);
                        
                        if resp.status == 0 {
                            println!("  ✓ Reconcile request successful");
                        } else {
                            println!("  ✗ Reconcile request failed with status: {}", resp.status);
                        }
                    },
                    Err(e) => {
                        eprintln!("  ✗ Failed to send reconcile request: {:?}", e);
                    }
                }
            },
            Err(e) => {
                eprintln!("  ✗ Failed to connect to ActionController: {:?}", e);
                eprintln!("  Make sure ActionController service is running at: {}", 
                    common::actioncontroller::connect_server());
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::monitoringserver::ContainerInfo;
    use std::collections::HashMap;

    /// Test the model state determination logic according to LLD rules
    #[test]
    fn test_determine_model_state_from_containers() {
        let state_machine = StateMachine::new();

        // Test empty container list -> Pending state
        let empty_containers: Vec<&ContainerInfo> = vec![];
        assert_eq!(
            state_machine.determine_model_state_from_containers(&empty_containers),
            ModelState::Pending
        );

        // Create test containers with different states
        let running_container = ContainerInfo {
            id: "container1".to_string(),
            names: vec!["test-container-1".to_string()],
            image: "test-image".to_string(),
            state: {
                let mut map = HashMap::new();
                map.insert("status".to_string(), "running".to_string());
                map
            },
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };

        let failed_container = ContainerInfo {
            id: "container2".to_string(),
            names: vec!["test-container-2".to_string()],
            image: "test-image".to_string(),
            state: {
                let mut map = HashMap::new();
                map.insert("status".to_string(), "failed".to_string());
                map
            },
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };

        let exited_container = ContainerInfo {
            id: "container3".to_string(),
            names: vec!["test-container-3".to_string()],
            image: "test-image".to_string(),
            state: {
                let mut map = HashMap::new();
                map.insert("status".to_string(), "exited".to_string());
                map
            },
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };

        // Test single running container -> Running state
        let running_containers = vec![&running_container];
        assert_eq!(
            state_machine.determine_model_state_from_containers(&running_containers),
            ModelState::Running
        );

        // Test one failed container -> Failed state (even with other running containers)
        let mixed_containers = vec![&running_container, &failed_container];
        assert_eq!(
            state_machine.determine_model_state_from_containers(&mixed_containers),
            ModelState::Failed
        );

        // Test all exited containers -> Succeeded state
        let exited_containers = vec![&exited_container, &exited_container];
        assert_eq!(
            state_machine.determine_model_state_from_containers(&exited_containers),
            ModelState::Succeeded
        );
    }

    /// Test the container grouping logic
    #[test]
    fn test_group_containers_by_model() {
        let state_machine = StateMachine::new();

        // Create test containers with different model annotations
        let container1 = ContainerInfo {
            id: "container1".to_string(),
            names: vec!["test-container-1".to_string()],
            image: "test-image".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: {
                let mut map = HashMap::new();
                map.insert("model".to_string(), "model-a".to_string());
                map
            },
            stats: HashMap::new(),
        };

        let container2 = ContainerInfo {
            id: "container2".to_string(),
            names: vec!["test-container-2".to_string()],
            image: "test-image".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: {
                let mut map = HashMap::new();
                map.insert("model".to_string(), "model-a".to_string());
                map
            },
            stats: HashMap::new(),
        };

        let container3 = ContainerInfo {
            id: "container3".to_string(),
            names: vec!["test-container-3".to_string()],
            image: "test-image".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: {
                let mut map = HashMap::new();
                map.insert("model".to_string(), "model-b".to_string());
                map
            },
            stats: HashMap::new(),
        };

        let containers = vec![container1, container2, container3];
        let grouped = state_machine.group_containers_by_model(&containers);

        // Should have 2 models
        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("model-a"));
        assert!(grouped.contains_key("model-b"));

        // model-a should have 2 containers
        assert_eq!(grouped.get("model-a").unwrap().len(), 2);
        // model-b should have 1 container
        assert_eq!(grouped.get("model-b").unwrap().len(), 1);
    }

    /// Test package state determination from model states
    #[tokio::test]
    async fn test_determine_package_state_from_models() {
        let state_machine = StateMachine::new();

        // Test empty model list -> Initializing state
        let empty_models: Vec<String> = vec![];
        assert_eq!(
            state_machine.determine_package_state_from_models(&empty_models).await,
            PackageState::Initializing
        );

        // Note: Full testing of this function requires ETCD integration
        // which would need a test environment with ETCD running
        // For now, we test the empty case and structure
    }

    /// Test ETCD key format consistency
    #[test]
    fn test_etcd_key_formats() {
        // Test model key format
        let model_name = "test_model";
        let expected_model_key = format!("/model/{}/state", model_name);
        assert_eq!(expected_model_key, "/model/test_model/state");

        // Test package key format
        let package_name = "test_package";
        let expected_package_key = format!("/package/{}/state", package_name);
        assert_eq!(expected_package_key, "/package/test_package/state");
    }

    /// Test ActionController reconcile request creation
    #[test]
    fn test_action_controller_reconcile_request() {
        use common::actioncontroller::{ReconcileRequest, PodStatus};
        
        // Test that we can create a proper reconcile request
        let package_name = "test_package";
        let reconcile_request = ReconcileRequest {
            scenario_name: package_name.to_string(),
            current: PodStatus::Failed as i32,
            desired: PodStatus::Running as i32,
        };

        assert_eq!(reconcile_request.scenario_name, "test_package");
        assert_eq!(reconcile_request.current, PodStatus::Failed as i32);
        assert_eq!(reconcile_request.desired, PodStatus::Running as i32);
    }
}
