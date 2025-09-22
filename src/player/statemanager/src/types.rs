use common::statemanager::{ErrorCode, ResourceType};
use std::collections::HashMap;
use tokio::time::Instant;

// ========================================
// CONTAINER STATE DEFINITIONS
// ========================================

/// Container states as defined in the problem statement
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Created,
    Running,
    Stopped,
    Exited,
    Dead,
}

impl ContainerState {
    /// Convert string representation to ContainerState
    pub fn from_str(state: &str) -> Option<Self> {
        match state.to_lowercase().as_str() {
            "created" => Some(ContainerState::Created),
            "running" => Some(ContainerState::Running),
            "stopped" => Some(ContainerState::Stopped),
            "exited" => Some(ContainerState::Exited),
            "dead" => Some(ContainerState::Dead),
            _ => None,
        }
    }

    /// Convert ContainerState to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerState::Created => "created",
            ContainerState::Running => "running",
            ContainerState::Stopped => "stopped",
            ContainerState::Exited => "exited",
            ContainerState::Dead => "dead",
        }
    }
}
// ========================================
// CORE DATA STRUCTURES
// ========================================

/// Mapping between containers and their parent models
#[derive(Debug, Clone)]
pub struct ContainerModelMapping {
    /// Map from container_id to model_id
    pub container_to_model: HashMap<String, String>,
    /// Map from model_id to list of container_ids
    pub model_to_containers: HashMap<String, Vec<String>>,
}

/// Mapping between models and their parent packages
#[derive(Debug, Clone)]
pub struct ModelPackageMapping {
    /// Map from model_id to package_id
    pub model_to_package: HashMap<String, String>,
    /// Map from package_id to list of model_ids
    pub package_to_models: HashMap<String, Vec<String>>,
}

/// Container state update request
#[derive(Debug, Clone)]
pub struct ContainerStateUpdate {
    pub container_id: String,
    pub new_state: ContainerState,
    pub timestamp: Instant,
    pub node_name: String,
}

/// State determination context for hierarchical updates
#[derive(Debug, Clone)]
pub struct StateContext {
    pub container_mappings: ContainerModelMapping,
    pub model_mappings: ModelPackageMapping,
    pub current_container_states: HashMap<String, ContainerState>,
    pub current_model_states: HashMap<String, String>,
    pub current_package_states: HashMap<String, String>,
}

impl Default for StateContext {
    fn default() -> Self {
        Self {
            container_mappings: ContainerModelMapping {
                container_to_model: HashMap::new(),
                model_to_containers: HashMap::new(),
            },
            model_mappings: ModelPackageMapping {
                model_to_package: HashMap::new(),
                package_to_models: HashMap::new(),
            },
            current_container_states: HashMap::new(),
            current_model_states: HashMap::new(),
            current_package_states: HashMap::new(),
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_state_enum() {
        // Test ContainerState string conversion
        assert_eq!(ContainerState::from_str("running"), Some(ContainerState::Running));
        assert_eq!(ContainerState::from_str("dead"), Some(ContainerState::Dead));
        assert_eq!(ContainerState::from_str("unknown"), None);
        
        // Test to string conversion
        assert_eq!(ContainerState::Running.as_str(), "running");
        assert_eq!(ContainerState::Dead.as_str(), "dead");
    }

    #[test]
    fn test_state_context_default() {
        let context = StateContext::default();
        assert!(context.container_mappings.container_to_model.is_empty());
        assert!(context.model_mappings.model_to_package.is_empty());
        assert!(context.current_container_states.is_empty());
    }

    #[test]
    fn test_container_state_update() {
        let update = ContainerStateUpdate {
            container_id: "test-container".to_string(),
            new_state: ContainerState::Running,
            timestamp: tokio::time::Instant::now(),
            node_name: "test-node".to_string(),
        };
        
        assert_eq!(update.container_id, "test-container");
        assert_eq!(update.new_state, ContainerState::Running);
        assert_eq!(update.node_name, "test-node");
    }
}
