/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager Module Exports
//!
//! This module provides the public interface for the StateManager component

pub mod grpc;
pub mod manager;
pub mod model;
pub mod package;
pub mod state_machine;
pub mod storage;
pub mod types;
pub mod utils;

// Re-export main types for easier access
pub use manager::StateManagerManager;
pub use model::ModelStateManager;
pub use package::PackageStateManager;
pub use state_machine::{ResourceState, StateMachine, TransitionResult};
pub use storage::{EtcdStateStorage, StateStorage};
pub use utils::{ActionControllerClient, ActionControllerService};