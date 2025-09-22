# StateManager Model Feature Usage Examples

This document provides practical examples of how to use the newly implemented StateManager Model functionality for cascading state transitions.

## Overview

The StateManager Model feature implements the LLD requirements for:
- Model state evaluation based on container states
- Package state evaluation based on model states
- Cascading state transitions (Model → Package)
- ETCD persistence with standardized key formats
- Error recovery notifications

## Usage Examples

### 1. Basic Model State Evaluation

```rust
use statemanager::state_machine::StateMachine;
use std::collections::HashMap;

// Create StateManager instance
let state_machine = StateMachine::new();

// Prepare container states for a model
let mut container_states = HashMap::new();
container_states.insert("web_server".to_string(), "running".to_string());
container_states.insert("database".to_string(), "running".to_string());
container_states.insert("cache".to_string(), "paused".to_string());

// Evaluate model state based on containers
let model_state = state_machine.evaluate_model_state_from_containers(
    "payment_service.backend_model", 
    &container_states
);

// Result: ModelState::Running (mixed states default to Running)
println!("Model state: {:?}", model_state);
```

### 2. Package State Evaluation

```rust
use common::statemanager::ModelState;

// Prepare model states for a package
let mut model_states = HashMap::new();
model_states.insert("payment_service.backend_model".to_string(), ModelState::Running);
model_states.insert("payment_service.frontend_model".to_string(), ModelState::Running);
model_states.insert("payment_service.auth_model".to_string(), ModelState::Failed);

// Evaluate package state based on models
let package_state = state_machine.evaluate_package_state_from_models(
    "payment_service",
    &model_states
);

// Result: PackageState::Degraded (some models failed)
println!("Package state: {:?}", package_state);
```

### 3. Complete Cascading State Change Processing

```rust
use common::statemanager::{StateChange, ResourceType};

// Create a StateChange for a model transition
let state_change = StateChange {
    resource_type: ResourceType::Model as i32,
    resource_name: "payment_service.auth_model".to_string(),
    current_state: "Running".to_string(),
    target_state: "Failed".to_string(),
    transition_id: "transition_001".to_string(),
    timestamp_ns: std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64,
    source: "NodeAgent".to_string(),
};

// Process with cascading (async method)
let results = state_manager.process_state_change_with_cascading(state_change).await;

// Results will include:
// 1. Primary model state transition result
// 2. Cascading package state transition result (if package state changed)
for result in results {
    println!("Transition: {} - {}", result.transition_id, result.message);
    if result.is_success() {
        println!("  ✓ Success");
    } else {
        println!("  ✗ Failed: {}", result.error_details);
    }
}
```

### 4. ETCD Persistence Integration

```rust
// The StateManager automatically persists states to ETCD using standardized keys:

// Model states: /model/{model_name}/state
// Example: /model/payment_service.auth_model/state = "Failed"

// Package states: /package/{package_name}/state  
// Example: /package/payment_service/state = "Degraded"

// Manual ETCD persistence (if needed)
let success = state_machine.persist_state_to_etcd(
    ResourceType::Model,
    "payment_service.auth_model",
    "Failed"
).await;

if success {
    println!("State persisted to ETCD");
} else {
    println!("Failed to persist state");
}
```

### 5. Error Recovery Scenario

```rust
// When all models in a package fail, the package enters Error state
// This triggers ActionController reconcile (as per LLD requirements)

let mut all_failed_models = HashMap::new();
all_failed_models.insert("payment_service.backend_model".to_string(), ModelState::Failed);
all_failed_models.insert("payment_service.frontend_model".to_string(), ModelState::Failed);
all_failed_models.insert("payment_service.auth_model".to_string(), ModelState::Failed);

let package_state = state_machine.evaluate_package_state_from_models(
    "payment_service",
    &all_failed_models
);

if package_state == PackageState::Error {
    println!("Package in Error state - ActionController reconcile will be triggered");
    // TODO: Implement actual ActionController notification
}
```

## State Transition Rules

### Model State Rules (LLD Table 3.2)

| Condition | Resulting Model State |
|-----------|----------------------|
| No containers | `Pending` |
| All containers paused | `Pending` |
| All containers exited | `Succeeded` |
| Any container dead | `Failed` |
| Mixed/running states | `Running` |

### Package State Rules (LLD Table 3.1)

| Condition | Resulting Package State |
|-----------|------------------------|
| No models | `Initializing` |
| All models paused/pending | `Paused` |
| All models succeeded | `Running` |
| All models failed | `Error` |
| Some models failed | `Degraded` |
| Mixed states | `Running` |

## Integration with StateManager Service

The StateManager service (manager.rs) uses the new cascading functionality:

```rust
// In StateManagerManager::process_state_change()
let results = {
    let mut state_machine = self.state_machine.lock().await;
    state_machine.process_state_change_with_cascading(state_change).await
};

// Process all results (primary + cascading)
for result in results {
    if result.is_success() {
        println!("✓ Transition succeeded: {}", result.message);
    } else {
        println!("✗ Transition failed: {}", result.error_details);
    }
}
```

## Testing Examples

The implementation includes comprehensive tests:

```bash
# Run all StateManager tests (15 unit + 3 integration tests)
cd src/player/statemanager
cargo test

# Run specific test categories
cargo test test_evaluate_model_state    # Model evaluation tests
cargo test test_evaluate_package_state  # Package evaluation tests  
cargo test integration_tests           # Integration tests
```

## Error Handling

The implementation provides robust error handling:

```rust
// ETCD failures are logged but don't block state transitions
let success = state_machine.persist_state_to_etcd(resource_type, name, state).await;
if !success {
    eprintln!("Warning: Failed to persist to ETCD, but transition completed");
}

// Invalid state transitions return detailed error information
let result = state_machine.process_state_change(invalid_change);
if result.is_failure() {
    println!("Error: {} - {}", result.error_code, result.error_details);
}
```

## Performance Considerations

- **Non-blocking**: State transitions are processed asynchronously
- **Non-recursive**: Cascading uses iterative approach to avoid stack overflow  
- **Efficient queries**: ETCD prefix queries for bulk operations
- **Minimal changes**: Only processes actual state changes, not duplicates

## Next Steps

1. **ActionController Integration**: Implement actual reconcile requests
2. **Container Events**: Add container state change event processing
3. **Metrics**: Add state transition metrics and monitoring
4. **Advanced Queries**: Implement more sophisticated ETCD query patterns

For more details, see the design document at `doc/design/StateManager_Model_Implementation_Design.md`.