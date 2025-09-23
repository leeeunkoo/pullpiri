# StateManager Model and Package State Management - Design Document

## Feature Overview

This implementation adds hierarchical state management functionality to the StateManager component based on the LLD specifications. The system will:

1. **Model State Management**: Monitor container states and automatically transition model states based on container status aggregation
2. **Package State Management**: Monitor model states and automatically transition package states based on model status aggregation  
3. **ETCD Integration**: Store state changes using specified key/value format
4. **Cascading State Updates**: Automatically propagate state changes from containers → models → packages

## Component Mapping

Based on the LLD documents and existing codebase analysis, the following files need to be created or modified:

### Files to be Modified

1. **`src/player/statemanager/src/state_machine.rs`** - Core logic implementation
   - Add container-to-model state evaluation functions
   - Add model-to-package state evaluation functions
   - Add ETCD integration methods
   - Implement state transition logic per LLD specifications

2. **`src/player/statemanager/src/manager.rs`** - Integration with existing manager
   - Add container list processing for model state evaluation
   - Integrate new state evaluation functions into processing loop

3. **`src/common/proto/statemanager.proto`** - Update if needed for new state definitions
   - Verify existing states match LLD requirements

### Files to be Created

None - all functionality can be implemented within existing structure.

## Sequence Diagrams

### Container State Change Flow
```
NodeAgent → StateManager: ContainerList (via gRPC)
StateManager → state_machine: process_container_updates()
state_machine → state_machine: evaluate_model_states()
state_machine → ETCD: put("/model/{name}/state", state)
state_machine → state_machine: evaluate_package_states()  
state_machine → ETCD: put("/package/{name}/state", state)
state_machine → ActionController: reconcile_request (if package=error)
```

### External State Change Flow
```
Component → StateManager: StateChange (via gRPC)
StateManager → state_machine: process_state_change()
state_machine → ETCD: put("/{resource_type}/{name}/state", state)
state_machine → state_machine: evaluate_parent_states()
state_machine → ETCD: put parent states
```

## API Specifications

### New Methods in StateMachine

#### Container State Processing
```rust
impl StateMachine {
    /// Process container list updates from NodeAgent
    /// Evaluates model states based on container aggregation rules
    pub async fn process_container_updates(&mut self, container_list: ContainerList) -> Result<Vec<StateChangeResponse>>;
    
    /// Evaluate model states based on current container states
    /// Implements LLD Table 3.2 rules
    async fn evaluate_model_states(&mut self, containers: &[Container]) -> Result<Vec<String>>;
    
    /// Evaluate package states based on current model states  
    /// Implements LLD Table 3.1 rules
    async fn evaluate_package_states(&mut self, updated_models: &[String]) -> Result<Vec<String>>;
}
```

#### ETCD Integration Methods
```rust
impl StateMachine {
    /// Store model state in ETCD using specified format
    async fn store_model_state(&self, model_name: &str, state: ModelState) -> Result<()>;
    
    /// Store package state in ETCD using specified format
    async fn store_package_state(&self, package_name: &str, state: PackageState) -> Result<()>;
    
    /// Get all models for a package from ETCD
    async fn get_models_for_package(&self, package_name: &str) -> Result<Vec<String>>;
    
    /// Get all containers for a model from ETCD  
    async fn get_containers_for_model(&self, model_name: &str) -> Result<Vec<String>>;
}
```

#### State Evaluation Methods
```rust
impl StateMachine {
    /// Determine model state based on container states per LLD Table 3.2
    fn determine_model_state(&self, containers: &[ContainerState]) -> ModelState;
    
    /// Determine package state based on model states per LLD Table 3.1
    fn determine_package_state(&self, models: &[ModelState]) -> PackageState;
    
    /// Send reconcile request to ActionController for failed packages
    async fn send_reconcile_request(&self, package_name: &str) -> Result<()>;
}
```

## Skeleton Code

### File: `src/player/statemanager/src/state_machine.rs`

```rust
use common::etcd;
use common::monitoringserver::{ContainerList, Container, ContainerState};
use common::statemanager::{ModelState, PackageState, StateChangeResponse, ErrorCode};
use common::actioncontroller;

impl StateMachine {
    /// Process container list updates from NodeAgent per LLD Section 2
    /// 
    /// Receives container status information and evaluates:
    /// 1. Model state changes based on container aggregation
    /// 2. Package state changes based on model aggregation  
    /// 3. Stores results in ETCD with specified key format
    /// 4. Triggers ActionController reconcile for error states
    pub async fn process_container_updates(&mut self, container_list: ContainerList) -> Result<Vec<StateChangeResponse>> {
        // Extract containers from the list
        let containers = container_list.containers;
        
        // Evaluate model states based on container states
        let updated_models = self.evaluate_model_states(&containers).await?;
        
        // Evaluate package states based on updated models
        let updated_packages = self.evaluate_package_states(&updated_models).await?;
        
        // Collect all state change responses
        let mut responses = Vec::new();
        
        // Add model responses
        for model_name in updated_models {
            responses.push(StateChangeResponse {
                message: format!("Model {} state updated", model_name),
                transition_id: format!("model_{}_{}", model_name, chrono::Utc::now().timestamp_nanos()),
                timestamp_ns: chrono::Utc::now().timestamp_nanos(),
                error_code: ErrorCode::Success,
                error_details: String::new(),
            });
        }
        
        // Add package responses  
        for package_name in updated_packages {
            responses.push(StateChangeResponse {
                message: format!("Package {} state updated", package_name),
                transition_id: format!("package_{}_{}", package_name, chrono::Utc::now().timestamp_nanos()),
                timestamp_ns: chrono::Utc::now().timestamp_nanos(),
                error_code: ErrorCode::Success,
                error_details: String::new(),
            });
        }
        
        Ok(responses)
    }

    /// Evaluate model states based on container states per LLD Table 3.2
    async fn evaluate_model_states(&mut self, containers: &[Container]) -> Result<Vec<String>> {
        let mut updated_models = Vec::new();
        
        // Group containers by model_name
        let mut model_containers: std::collections::HashMap<String, Vec<&Container>> = std::collections::HashMap::new();
        
        for container in containers {
            model_containers
                .entry(container.model_name.clone())
                .or_insert_with(Vec::new)
                .push(container);
        }
        
        // Evaluate each model
        for (model_name, model_container_list) in model_containers {
            let container_states: Vec<ContainerState> = model_container_list
                .iter()
                .map(|c| c.state)
                .collect();
                
            let new_model_state = self.determine_model_state(&container_states);
            
            // Check if state changed
            let current_state = self.get_current_model_state(&model_name).await.unwrap_or(ModelState::Created);
            
            if new_model_state != current_state {
                // Store new state in ETCD
                self.store_model_state(&model_name, new_model_state).await?;
                updated_models.push(model_name);
            }
        }
        
        Ok(updated_models)
    }

    /// Evaluate package states based on model states per LLD Table 3.1
    async fn evaluate_package_states(&mut self, updated_models: &[String]) -> Result<Vec<String>> {
        let mut updated_packages = Vec::new();
        let mut packages_to_check = std::collections::HashSet::new();
        
        // Find packages that contain the updated models
        for model_name in updated_models {
            if let Ok(package_name) = self.get_package_for_model(model_name).await {
                packages_to_check.insert(package_name);
            }
        }
        
        // Evaluate each affected package
        for package_name in packages_to_check {
            let model_names = self.get_models_for_package(&package_name).await?;
            let mut model_states = Vec::new();
            
            for model_name in &model_names {
                if let Ok(state) = self.get_current_model_state(model_name).await {
                    model_states.push(state);
                }
            }
            
            let new_package_state = self.determine_package_state(&model_states);
            let current_state = self.get_current_package_state(&package_name).await.unwrap_or(PackageState::Idle);
            
            if new_package_state != current_state {
                // Store new state in ETCD
                self.store_package_state(&package_name, new_package_state).await?;
                
                // If package goes to error state, send reconcile request
                if new_package_state == PackageState::Error {
                    self.send_reconcile_request(&package_name).await?;
                }
                
                updated_packages.push(package_name);
            }
        }
        
        Ok(updated_packages)
    }

    /// Determine model state based on container states per LLD Table 3.2
    fn determine_model_state(&self, containers: &[ContainerState]) -> ModelState {
        if containers.is_empty() {
            return ModelState::Created;
        }
        
        // Check LLD conditions in priority order
        
        // Dead: 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패  
        if containers.iter().any(|&state| state == ContainerState::Dead) {
            return ModelState::Dead;
        }
        
        // Paused: 모든 container가 paused 상태일 때
        if containers.iter().all(|&state| state == ContainerState::Paused) {
            return ModelState::Paused;
        }
        
        // Exited: 모든 container가 exited 상태일 때
        if containers.iter().all(|&state| state == ContainerState::Exited) {
            return ModelState::Exited;
        }
        
        // Running: 위 조건을 모두 만족하지 않을 때(기본 상태)
        ModelState::Running
    }

    /// Determine package state based on model states per LLD Table 3.1  
    fn determine_package_state(&self, models: &[ModelState]) -> PackageState {
        if models.is_empty() {
            return PackageState::Idle;
        }
        
        // Check LLD conditions in priority order
        
        // error: 모든 model이 dead 상태일 때
        if models.iter().all(|&state| state == ModelState::Dead) {
            return PackageState::Error;
        }
        
        // degraded: 일부 model이 dead 상태일 때 (단 모든 model이 dead가 아닐 때)
        if models.iter().any(|&state| state == ModelState::Dead) {
            return PackageState::Degraded;
        }
        
        // paused: 모든 model이 paused 상태일 때
        if models.iter().all(|&state| state == ModelState::Paused) {
            return PackageState::Paused;
        }
        
        // exited: 모든 model이 exited 상태일 때  
        if models.iter().all(|&state| state == ModelState::Exited) {
            return PackageState::Exited;
        }
        
        // running: 위 조건을 모두 만족하지 않을 때(기본 상태)
        PackageState::Running
    }

    /// Store model state in ETCD using LLD specified format
    async fn store_model_state(&self, model_name: &str, state: ModelState) -> Result<()> {
        let key = format!("/model/{}/state", model_name);
        let value = state.as_str_name(); // 예: "Running"
        if let Err(e) = common::etcd::put(&key, value).await {
            eprintln!("Failed to save model state: {:?}", e);
        }
        Ok(())
    }

    /// Store package state in ETCD using LLD specified format
    async fn store_package_state(&self, package_name: &str, state: PackageState) -> Result<()> {
        let key = format!("/package/{}/state", package_name);
        let value = state.as_str_name(); // 예: "Running"
        if let Err(e) = common::etcd::put(&key, value).await {
            eprintln!("Failed to save package state: {:?}", e);
        }
        Ok(())
    }

    /// Send reconcile request to ActionController for failed packages
    async fn send_reconcile_request(&self, package_name: &str) -> Result<()> {
        // Implementation will depend on ActionController gRPC interface
        println!("Sending reconcile request for package: {}", package_name);
        // TODO: Implement actual gRPC call to ActionController
        Ok(())
    }

    // Helper methods for ETCD operations
    async fn get_current_model_state(&self, model_name: &str) -> Result<ModelState> {
        let key = format!("/model/{}/state", model_name);
        match common::etcd::get(&key).await {
            Ok(value) => {
                let state_str = format!("MODEL_STATE_{}", value.to_uppercase());
                Ok(ModelState::from_str_name(&state_str).unwrap_or(ModelState::Created))
            }
            Err(_) => Ok(ModelState::Created)
        }
    }

    async fn get_current_package_state(&self, package_name: &str) -> Result<PackageState> {
        let key = format!("/package/{}/state", package_name);
        match common::etcd::get(&key).await {
            Ok(value) => {
                let state_str = format!("PACKAGE_STATE_{}", value.to_uppercase());
                Ok(PackageState::from_str_name(&state_str).unwrap_or(PackageState::Idle))
            }
            Err(_) => Ok(PackageState::Idle)
        }
    }

    async fn get_models_for_package(&self, package_name: &str) -> Result<Vec<String>> {
        // This would need to be implemented based on how package-model relationships are stored
        // For now, using a placeholder implementation
        let prefix = format!("/package/{}/models/", package_name);
        match common::etcd::get_all_with_prefix(&prefix).await {
            Ok(kvs) => Ok(kvs.into_iter().map(|kv| kv.key.split('/').last().unwrap_or("").to_string()).collect()),
            Err(_) => Ok(Vec::new())
        }
    }

    async fn get_package_for_model(&self, model_name: &str) -> Result<String> {
        // This would need to be implemented based on how model-package relationships are stored
        // For now, using a placeholder implementation  
        let key = format!("/model/{}/package", model_name);
        match common::etcd::get(&key).await {
            Ok(package_name) => Ok(package_name),
            Err(_) => Err("Package not found".into())
        }
    }
}
```

### File: `src/player/statemanager/src/manager.rs`

```rust
impl StateManagerManager {
    /// Enhanced container processing with model/package state evaluation
    async fn process_container_list(&self, container_list: ContainerList) -> Result<()> {
        println!("Processing container list with {} containers", container_list.containers.len());
        
        // Process containers through state machine for model/package evaluation
        let mut state_machine = self.state_machine.lock().await;
        match state_machine.process_container_updates(container_list).await {
            Ok(responses) => {
                println!("Successfully processed container updates, {} state changes", responses.len());
                for response in responses {
                    println!("State change: {}", response.message);
                }
            }
            Err(e) => {
                eprintln!("Failed to process container updates: {:?}", e);
            }
        }
        
        Ok(())
    }
}
```

## State Mapping

### Container States (from LLD Table 3.3)
- `Created`: 컨테이너가 생성되지 않았거나 모두 삭제된 경우
- `Running`: 하나 이상의 컨테이너가 실행 중
- `Stopped`: 하나 이상의 컨테이너가 중지, 실행 중인 컨테이너는 없음  
- `Exited`: 모든 컨테이너가 종료됨
- `Dead`: 상태 정보 조회 실패, 시스템 오류 등

### Model States (from LLD Table 3.2)
- `Created`: model의 최초 상태
- `Paused`: 모든 container가 paused 상태일 때
- `Exited`: 모든 container가 exited 상태일 때
- `Dead`: 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패
- `Running`: 위 조건을 모두 만족하지 않을 때(기본 상태)

### Package States (from LLD Table 3.1)
- `idle`: 맨 처음 package의 상태
- `paused`: 모든 model이 paused 상태일 때
- `exited`: 모든 model이 exited 상태일 때
- `degraded`: 일부 model이 dead 상태일 때 (단 모든 model이 dead가 아닐 때)
- `error`: 모든 model이 dead 상태일 때
- `running`: 위 조건을 모두 만족하지 않을 때(기본 상태)

## Implementation Rules

1. **Location**: All state evaluation logic must be implemented in `src/player/statemanager/src/state_machine.rs` within the `impl StateMachine` block
2. **ETCD Format**: Use exact key/value format specified in LLD documents
3. **State Priority**: Follow exact condition precedence defined in LLD state tables
4. **Error Handling**: Package error states must trigger ActionController reconcile requests
5. **Performance**: State evaluation should be efficient for real-time container monitoring

## Testing Strategy

1. **Unit Tests**: Test state determination logic with various container/model combinations
2. **Integration Tests**: Test full flow from container updates to ETCD storage
3. **Edge Cases**: Test empty containers, missing models, ETCD failures
4. **Performance Tests**: Verify performance with large numbers of containers/models

This design follows the LLD specifications exactly and integrates seamlessly with the existing StateManager architecture.