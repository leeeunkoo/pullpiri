use common::statemanager::{ErrorCode, ModelState, PackageState, ResourceType};
use std::collections::HashMap;
use tokio::time::Instant;
// ========================================
// CORE DATA STRUCTURES
// ========================================

/// Action execution command for async processing
#[derive(Debug, Clone)]
pub struct ActionCommand {
    pub action: String,
    pub resource_key: String,
    pub resource_type: ResourceType,
    pub transition_id: String,
    pub context: HashMap<String, String>,
}

/// Represents a state transition in the state machine
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransition {
    pub from_state: i32,
    pub event: String,
    pub to_state: i32,
    pub condition: Option<String>,
    pub action: String,
}

/// Health status tracking for resources
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub status_message: String,
    pub last_check: Instant,
    pub consecutive_failures: u32,
}

/// Represents the current state of a resource with metadata
#[derive(Debug, Clone)]
pub struct ResourceState {
    pub resource_type: ResourceType,
    pub resource_name: String,
    pub current_state: i32,
    pub desired_state: Option<i32>,
    pub last_transition_time: Instant,
    pub transition_count: u64,
    pub metadata: HashMap<String, String>,
    pub health_status: HealthStatus,
}

/// Result of a state transition attempt - aligned with proto StateChangeResponse
#[derive(Debug, Clone)]
pub struct TransitionResult {
    pub new_state: i32,
    pub error_code: ErrorCode,
    pub message: String,
    pub actions_to_execute: Vec<String>,
    pub transition_id: String,
    pub error_details: String,
}

// ========================================
// MODEL/PACKAGE STATE MANAGEMENT TYPES
// ========================================

/// Container state mapping based on LLD requirements
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Created,
    Running,
    Stopped,
    Exited,
    Dead,
    Paused,
}

/// Container update processing result
#[derive(Debug, Clone)]
pub struct ContainerUpdateResult {
    pub affected_models: Vec<String>,
    pub affected_packages: Vec<String>,
    pub reconcile_requests: Vec<String>, // Package names requiring reconcile
}

/// Model state aggregation info
#[derive(Debug, Clone)]
pub struct ModelStateInfo {
    pub model_name: String,
    pub previous_state: ModelState,
    pub current_state: ModelState,
    pub container_count: usize,
    pub container_states: HashMap<String, ContainerState>,
}

/// Package state aggregation info  
#[derive(Debug, Clone)]
pub struct PackageStateInfo {
    pub package_name: String,
    pub previous_state: PackageState,
    pub current_state: PackageState,
    pub model_count: usize,
    pub model_states: HashMap<String, ModelState>,
}
