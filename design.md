# StateManager Model/Package State Management Design Document

## 1. Feature Overview

This document outlines the implementation of Model and Package state management functionality in the StateManager service based on the provided Low-Level Design (LLD) specifications. The implementation will enable the StateManager to:

1. **Model State Management**: Monitor container states and automatically transition model states based on the aggregated container state conditions
2. **Package State Management**: Monitor model states within packages and automatically transition package states based on the aggregated model state conditions  
3. **Dead Package Recovery**: Trigger ActionController reconcile requests when packages transition to error/dead state
4. **Persistent State Storage**: Store all state changes in etcd with proper key/value formatting

## 2. Component Mapping

### Files to be Modified:
- `/home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/state_machine.rs` - Core state transition logic
- `/home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/manager.rs` - Container update processing integration  
- `/home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/types.rs` - Type definitions for new functionality

### Files to be Created:
- None (utilizing existing structure)

### Dependencies:
- `common::etcd` - For state persistence (already available)
- `common::statemanager` - For proto definitions (already available)
- `common::monitoringserver::ContainerList` - For container status input (already available)
- ActionController gRPC communication (sender module exists)

## 3. Sequence Diagrams

### Model State Transition Flow:
```
NodeAgent -> StateManager: ContainerList (containers with states)
StateManager -> StateMachine: process_container_updates()
StateMachine -> StateMachine: aggregate_container_states_to_model()
StateMachine -> ETCD: put(/model/{model_name}/state, new_state)
```

### Package State Transition Flow:
```
StateMachine -> StateMachine: detect_model_state_change()
StateMachine -> StateMachine: aggregate_model_states_to_package()  
StateMachine -> ETCD: put(/package/{package_name}/state, new_state)
StateMachine -> ActionController: reconcile_request() [if package becomes dead]
```

## 4. API Specifications

### StateMachine New Methods:

#### Container Processing
```rust
// File: /home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/state_machine.rs
impl StateMachine {
    /// Process container updates and trigger model state changes
    pub async fn process_container_updates(&mut self, container_list: ContainerList) -> Result<Vec<String>>;
    
    /// Aggregate container states to determine model state
    fn aggregate_container_states_to_model(&self, model_name: &str, containers: &[ContainerInfo]) -> ModelState;
    
    /// Aggregate model states to determine package state  
    fn aggregate_model_states_to_package(&self, package_name: &str) -> PackageState;
    
    /// Save model state to etcd
    async fn save_model_state(&self, model_name: &str, state: ModelState) -> Result<()>;
    
    /// Save package state to etcd
    async fn save_package_state(&self, package_name: &str, state: PackageState) -> Result<()>;
    
    /// Get all models belonging to a package
    async fn get_package_models(&self, package_name: &str) -> Result<Vec<String>>;
    
    /// Trigger ActionController reconcile for dead packages
    async fn trigger_package_reconcile(&self, package_name: &str) -> Result<()>;
}
```

#### Container State Mapping:
```rust
// File: /home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/state_machine.rs  
impl StateMachine {
    /// Map container state string to standardized enum
    fn map_container_state(&self, state: &str) -> ContainerState;
}

#[derive(Debug, Clone, PartialEq)]
enum ContainerState {
    Created,
    Running, 
    Stopped,
    Exited,
    Dead,
    Paused,
}
```

### StateManagerManager Integration:

#### Container Update Processing
```rust
// File: /home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/manager.rs
impl StateManagerManager {
    /// Process container updates from NodeAgent
    async fn process_container_updates(&self, container_list: ContainerList) -> Result<()>;
}
```

## 5. Skeleton Code

### 5.1 StateMachine Container Processing (state_machine.rs)

```rust
// File: /home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/state_machine.rs

use common::monitoringserver::{ContainerList, ContainerInfo};
use common::statemanager::{ModelState, PackageState};

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Created,
    Running,
    Stopped, 
    Exited,
    Dead,
    Paused,
}

impl StateMachine {
    /// Process container updates and trigger cascading state changes
    pub async fn process_container_updates(&mut self, container_list: ContainerList) -> Result<Vec<String>> {
        // TODO: Group containers by model
        // TODO: For each model, aggregate container states
        // TODO: If model state changes, save to etcd
        // TODO: Check if package state needs updating
        // TODO: If package becomes dead/error, trigger reconcile
        todo!()
    }

    /// Aggregate container states to determine model state per LLD rules
    fn aggregate_container_states_to_model(&self, model_name: &str, containers: &[ContainerInfo]) -> ModelState {
        // TODO: Implement LLD state transition rules:
        // - Created: model's initial state  
        // - Paused: all containers are paused
        // - Exited: all containers are exited
        // - Dead: one or more containers are dead OR model info retrieval failed
        // - Running: default state when above conditions not met
        todo!()
    }

    /// Aggregate model states to determine package state per LLD rules  
    fn aggregate_model_states_to_package(&self, package_name: &str) -> PackageState {
        // TODO: Implement LLD state transition rules:
        // - Idle: initial package state
        // - Paused: all models are paused
        // - Exited: all models are exited  
        // - Degraded: some (1+) models are dead, but not all
        // - Error: all models are dead
        // - Running: default state when above conditions not met
        todo!()
    }

    /// Save model state to etcd using specified format
    async fn save_model_state(&self, model_name: &str, state: ModelState) -> Result<()> {
        let key = format!("/model/{}/state", model_name);
        let value = state.as_str_name();
        common::etcd::put(&key, value).await
            .map_err(|e| format!("Failed to save model state: {:?}", e).into())
    }

    /// Save package state to etcd using specified format
    async fn save_package_state(&self, package_name: &str, state: PackageState) -> Result<()> {
        let key = format!("/package/{}/state", package_name);
        let value = state.as_str_name(); 
        common::etcd::put(&key, value).await
            .map_err(|e| format!("Failed to save package state: {:?}", e).into())
    }

    /// Get all models belonging to a package from etcd
    async fn get_package_models(&self, package_name: &str) -> Result<Vec<String>> {
        let prefix = format!("/package/{}/models/", package_name);
        // TODO: Query etcd for models in this package
        todo!()
    }

    /// Trigger ActionController reconcile for dead packages
    async fn trigger_package_reconcile(&self, package_name: &str) -> Result<()> {
        // TODO: Send gRPC request to ActionController for reconcile
        todo!()
    }

    /// Map container state string to standardized enum
    fn map_container_state(&self, state: &str) -> ContainerState {
        match state.to_lowercase().as_str() {
            "created" => ContainerState::Created,
            "running" => ContainerState::Running,
            "stopped" => ContainerState::Stopped,
            "exited" => ContainerState::Exited,
            "dead" => ContainerState::Dead,
            "paused" => ContainerState::Paused,
            _ => ContainerState::Dead // Default to dead for unknown states
        }
    }
}
```

### 5.2 StateManagerManager Integration (manager.rs)

```rust
// File: /home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/manager.rs

impl StateManagerManager {
    /// Process container updates from NodeAgent
    async fn process_container_updates(&self, container_list: ContainerList) -> Result<()> {
        let mut state_machine = self.state_machine.lock().await;
        let changed_resources = state_machine.process_container_updates(container_list).await?;
        
        // Log the changes
        for resource in changed_resources {
            println!("State changed for resource: {}", resource);
        }
        
        Ok(())
    }

    /// Enhanced gRPC processing to handle container updates
    pub async fn process_grpc_requests(&self) -> Result<()> {
        // TODO: Add container update processing branch to existing implementation
        // TODO: Integrate with existing state change processing
        todo!()
    }
}
```

### 5.3 Enhanced Types (types.rs)

```rust
// File: /home/runner/work/pullpiri/pullpiri/src/player/statemanager/src/types.rs

use common::monitoringserver::ContainerList;

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
```

## 6. Implementation Rules from LLD

### 6.1 Model State Transition Rules:
| State | Condition |
|-------|-----------|
| Created | Initial model state |
| Paused | All containers are paused |
| Exited | All containers are exited |
| Dead | One or more containers are dead OR model info retrieval failed |
| Running | Default when above conditions not met |

### 6.2 Package State Transition Rules:
| State | Condition |
|-------|-----------|
| Idle | Initial package state |
| Paused | All models are paused |
| Exited | All models are exited |
| Degraded | Some (1+) models are dead, but not all models |
| Error | All models are dead |
| Running | Default when above conditions not met |

### 6.3 Container State Definitions:
| State | Description |
|-------|-------------|
| Created | No containers running yet, containers not created or all deleted |
| Running | One or more containers are running |
| Stopped | One or more containers stopped, no running containers |
| Exited | All containers in pod have exited |
| Dead | Failed to get pod state info (metadata corruption, system errors) |

### 6.4 etcd Key/Value Format:
- Model states: `/model/{model_name}/state` → `ModelState.as_str_name()`
- Package states: `/package/{package_name}/state` → `PackageState.as_str_name()`

### 6.5 ActionController Integration:
- When package state transitions to `Error` (all models dead), send reconcile request to ActionController
- Use existing gRPC sender pattern from StateManager to ActionController

This design ensures minimal changes to existing code while implementing the required functionality exactly as specified in the LLD documents.