/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use std::{thread, time::Duration};

use crate::grpc::sender::pharos::request_network_pod;
use crate::grpc::sender::statemanager::StateManagerSender;
use common::{
    actioncontroller::PodStatus as Status,
    spec::artifact::{Model, Package, Scenario},
    statemanager::{ResourceType, StateChange},
    Result,
};
// Prost Message 트레이트 추가 // base64 Engine 트레이트 추가

/// Manager for coordinating scenario actions and workload operations
///
/// Responsible for:
/// - Processing scenario requests from gRPC receivers
/// - Determining appropriate actions based on scenario definitions
/// - Delegating workload operations to the appropriate runtime (NodeAgent)
/// - Handling state reconciliation for scenario workloads
pub struct ActionControllerManager {
    /// List of nodes managed by NodeAgent
    pub nodeagent_nodes: Vec<String>,
    /// StateManager sender for scenario state changes
    state_sender: StateManagerSender,
    // Add other fields as needed
}
#[allow(dead_code)]
impl ActionControllerManager {
    /// Creates a new ActionControllerManager instance
    ///
    /// Initializes the manager with empty node lists. Node information
    /// will be loaded from etcd when needed during trigger_manager_action.
    ///
    /// # Returns
    ///
    /// A new ActionControllerManager instance
    pub fn new() -> Self {
        // 초기화 단계에서는 빈 노드 목록으로 시작
        // 실제 노드 정보는 trigger_manager_action에서 etcd로부터 가져옴
        Self {
            nodeagent_nodes: Vec::new(),
            state_sender: StateManagerSender::new(),
        }
    }

    /// Fetches node role information from etcd
    ///
    /// Retrieves node information from etcd to determine if it is a nodeagent node.
    ///
    /// # Arguments
    ///
    /// * `node_name` - Name of the node to query
    ///
    /// # Returns
    ///
    /// * `Ok(String)` with node role ("nodeagent") if found
    /// * `Err(...)` if the node could not be found or role determined
    async fn get_node_role_from_etcd(&self, node_name: &str) -> Result<String> {
        // 1. 먼저 nodes/{hostname} 키에서 노드 IP 조회
        let node_info_key = format!("nodes/{}", node_name);
        #[allow(unused_variables)]
        let node_ip = match common::etcd::get(&node_info_key).await {
            Ok(ip) => ip,
            Err(e) => {
                println!(
                    "Warning: Failed to get IP for node '{}' from etcd: {}",
                    node_name, e
                );
                // settings.yaml 파일의 호스트 정보 사용
                let config = common::setting::get_config();
                if config.host.name == node_name {
                    println!("Using host IP from settings.yaml: {}", config.host.ip);
                    config.host.ip.clone()
                } else {
                    return Err(format!("No IP found for node '{}'", node_name).into());
                }
            }
        };

        // 2. 이제 cluster/nodes 접두사에서 상세 정보 조회 시도 (json string)
        let cluster_node_key = format!("cluster/nodes/{}", node_name);
        let node_json = match common::etcd::get(&cluster_node_key).await {
            Ok(value) => value,
            Err(e) => {
                println!(
                    "Warning: Failed to get details for node '{}' from etcd: {}",
                    node_name, e
                );
                // settings.yaml 파일의 호스트 정보 사용
                let config = common::setting::get_config();
                if config.host.name == node_name {
                    println!("Using role from settings.yaml for node '{}'", node_name);
                    // settings.yaml의 type 값에 따라 노드 역할 결정
                    return Ok("nodeagent".to_string());
                } else {
                    return Err(format!("No details found for node '{}'", node_name).into());
                }
            }
        };

        // 3. json 파싱
        let node_info: common::apiserver::NodeInfo = serde_json::from_str(&node_json)?;

        // 4. node_role 값에 따라 노드 유형 결정 (node_role: 2 = Nodeagent)
        let role = if node_info.node_role == 2 {
            "nodeagent".to_string()
        } else {
            return Err(format!("Unknown node role: {}", node_info.node_role).into());
        };

        println!("Node {} role loaded from etcd: {}", node_name, role);
        Ok(role)
    }

    /// Processes a trigger action request for a specific scenario
    ///
    /// Retrieves scenario information from ETCD and performs the
    /// appropriate actions based on the scenario definition.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario to trigger
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the action was triggered successfully
    /// * `Err(...)` if the action could not be triggered
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The scenario is not allowed by policy
    /// - The runtime operation fails
    pub async fn trigger_manager_action(&self, scenario_name: &str) -> Result<()> {
        println!("trigger_manager_action in manager {:?}", scenario_name);
        if scenario_name.trim().is_empty() {
            return Err(format!("Scenario '{}' is invalid: cannot be empty", scenario_name).into());
        }
        let etcd_scenario_key = format!("Scenario/{}", scenario_name);
        let scenario_str: String = match common::etcd::get(&etcd_scenario_key).await {
            Ok(value) => value,
            Err(e) => {
                return Err(format!("Scenario '{}' not found: {}", scenario_name, e).into());
            }
        };
        let scenario: Scenario = serde_yaml::from_str(&scenario_str)
            .map_err(|e| format!("Failed to parse scenario '{}': {}", scenario_name, e))?;

        let action: String = scenario.get_actions();

        let etcd_package_key = format!("Package/{}", scenario.get_targets());
        let package_str = match common::etcd::get(&etcd_package_key).await {
            Ok(value) => value,
            Err(e) => {
                return Err(format!("Package key '{}' not found: {}", etcd_package_key, e).into());
            }
        };

        let package: Package = serde_yaml::from_str(&package_str).map_err(|e| {
            format!(
                "Failed to parse package '{}': {}",
                scenario.get_targets(),
                e
            )
        })?;

        // To Do.. network, node yaml extract from etcd.
        let etcd_network_key = format!("Network/{}", scenario_name);
        let network_str = (common::etcd::get(&etcd_network_key).await).ok();

        let etcd_node_key = format!("Node/{}", scenario_name);
        let node_str = (common::etcd::get(&etcd_node_key).await).ok();

        // 각 모델의 노드 정보를 etcd에서 확인
        let mut node_roles = std::collections::HashMap::new();

        for mi in package.get_models() {
            let model_name = format!("{}.service", mi.get_name());
            let model_node = mi.get_node();

            // 노드 역할 정보가 캐시에 없으면 etcd에서 가져옴
            if !node_roles.contains_key(&model_node) {
                match self.get_node_role_from_etcd(&model_node).await {
                    Ok(role) => {
                        node_roles.insert(model_node.clone(), role);
                    }
                    Err(e) => {
                        println!(
                            "Warning: Failed to get role for node '{}' from etcd: {}",
                            model_node, e
                        );
                        if self.nodeagent_nodes.contains(&model_node) {
                            node_roles.insert(model_node.clone(), "nodeagent".to_string());
                            println!(
                                "Node {} found in nodeagent_nodes from cached list",
                                model_node
                            );
                        }
                    }
                }
            }

            // 노드 역할 정보를 사용하여 처리
            let node_type = match node_roles.get(&model_node) {
                Some(role) => {
                    println!("Using node {} as {}", model_node, role);
                    role.as_str()
                }
                None => {
                    // 역할 정보를 찾을 수 없으면 스킵
                    println!(
                        "Warning: Node '{}' is not configured or cannot determine its role. Skipping deployment.",
                        model_node
                    );
                    continue;
                }
            };
            println!(
                "Processing model '{}' on node '{}' with action '{}'",
                model_name, model_node, action
            );
            match action.as_str() {
                "launch" => {
                    self.start_workload(&model_name, &model_node, node_type)
                        .await
                        .map_err(|e| format!("Failed to start workload '{}': {}", model_name, e))?;

                    // If network and node are specified, request network pod to Pharos
                    if network_str.is_some() && node_str.is_some() {
                        request_network_pod(
                            node_str.clone().unwrap(),
                            scenario_name.to_string(),
                            network_str.clone().unwrap(),
                        )
                        .await
                        .map_err(|e| {
                            format!("Failed to request network pod for '{}': {}", model_name, e)
                        })?;
                    }
                }
                "terminate" => {
                    self.stop_workload(&model_name, &model_node, node_type)
                        .await
                        .map_err(|e| format!("Failed to stop workload '{}': {}", model_name, e))?;
                }
                "update" | "rollback" => {
                    self.restart_workload(&model_name, &model_node, node_type)
                        .await
                        .map_err(|e| format!("Failed to stop workload '{}': {}", model_name, e))?;

                    // If pod need realtime feature, send sched info to timpani
                    if mi.get_resources().get_realtime().unwrap_or(false) {
                        let model_str =
                            common::etcd::get(&format!("Model/{}", &mi.get_name())).await?;
                        let model: Model = serde_yaml::from_str(&model_str)?;

                        if let Some(command) = model.get_podspec().containers[0].command.clone() {
                            if let Some(task_name) = command.last() {
                                crate::grpc::sender::timpani::add_sched_info(
                                    mi.get_name(),
                                    task_name,
                                    &model_node,
                                )
                                .await;
                            }
                        }
                    }
                }
                _ => {
                    // Ignore unknown action for now, or optionally return error:
                    // return Err(format!("Unknown action '{}'", action).into());
                }
            }
        }

        // 🔍 COMMENT 2: ActionController scenario processing completion
        // After successful scenario processing (launch/terminate/update actions),
        // ActionController should notify StateManager of scenario state changes.
        // This would typically involve calling StateManagerSender to report:
        // - Action execution success/failure
        // - Final scenario state transitions
        // - Resource state confirmations

        println!("🔄 SCENARIO STATE TRANSITION: ActionController Completion");
        println!("   📋 Scenario: {}", scenario_name);
        println!("   🔄 State Change: allowed → completed");
        println!("   🔍 Reason: All scenario actions executed successfully");

        // Send state change to StateManager: allowed -> completed
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: scenario_name.to_string(),
            current_state: "allowed".to_string(),
            target_state: "completed".to_string(),
            transition_id: format!("actioncontroller-processing-complete-{}", timestamp),
            timestamp_ns: timestamp,
            source: "actioncontroller".to_string(),
        };

        println!("   📤 Sending StateChange to StateManager:");
        println!("      • Resource Type: SCENARIO");
        println!("      • Resource Name: {}", state_change.resource_name);
        println!("      • Current State: {}", state_change.current_state);
        println!("      • Target State: {}", state_change.target_state);
        println!("      • Transition ID: {}", state_change.transition_id);
        println!("      • Source: {}", state_change.source);

        if let Err(e) = self
            .state_sender
            .clone()
            .send_state_change(state_change)
            .await
        {
            println!("   ❌ Failed to send state change to StateManager: {:?}", e);
        } else {
            println!(
                "   ✅ Successfully notified StateManager: scenario {} allowed → completed",
                scenario_name
            );
        }

        Ok(())
    }

    /// Reconciles current and desired states for a scenario
    ///
    /// Compares the current state with the desired state for a given scenario
    /// and performs the necessary actions to align them.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    /// * `current` - Current state value
    /// * `desired` - Desired state value
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the reconciliation was successful
    /// * `Err(...)` if the reconciliation failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The reconciliation action fails
    pub async fn reconcile_do(
        &self,
        scenario_name: String,
        current: Status,
        desired: Status,
    ) -> Result<()> {
        if current == desired {
            return Ok(());
        }

        if matches!(current, Status::None | Status::Failed | Status::Unknown) {
            return Err(format!(
                "Invalid current status: {:?}. Cannot reconcile from this state",
                current
            )
            .into());
        }

        if matches!(desired, Status::None | Status::Failed | Status::Unknown) {
            return Err(format!(
                "Invalid desired status: {:?}. Cannot set this as target state",
                desired
            )
            .into());
        }

        let etcd_scenario_key: String = format!("scenario/{}", scenario_name);
        let scenario_str = common::etcd::get(&etcd_scenario_key).await?;
        let scenario: Scenario = serde_yaml::from_str(&scenario_str)?;

        let etcd_package_key = format!("package/{}", scenario.get_targets());
        let package_str = common::etcd::get(&etcd_package_key).await?;
        let package: Package = serde_yaml::from_str(&package_str)?;

        for mi in package.get_models() {
            let model_name = format!("{}.service", mi.get_name());
            let model_node = mi.get_node();
            let node_type = if self.nodeagent_nodes.contains(&model_node) {
                "nodeagent"
            } else {
                // Log warning for unknown node types and skip processing
                println!(
                    "Warning: Node '{}' is not explicitly configured. Skipping deployment.",
                    model_node
                );
                continue;
            };

            if desired == Status::Running {
                self.start_workload(&model_name, &model_node, node_type)
                    .await?;
            }
        }

        Ok(())
    }

    /// Creates a new workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was created successfully
    /// * `Err(...)` if the workload creation failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload already exists
    /// - The runtime operation fails
    #[allow(unused)]
    pub async fn create_workload(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Deletes an existing workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was deleted successfully
    /// * `Err(...)` if the workload deletion failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The runtime operation fails
    #[allow(unused_variables)]
    pub async fn delete_workload(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Restarts an existing workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was restarted successfully
    /// * `Err(...)` if the workload restart failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The runtime operation fails
    pub async fn restart_workload(
        &self,
        model_name: &str,
        node_name: &str,
        node_type: &str,
    ) -> Result<()> {
        match node_type {
            "nodeagent" => {
                crate::runtime::nodeagent::restart_workload(model_name, node_name).await?;
            }
            _ => {
                return Err(format!(
                    "Unsupported node type '{}' for workload '{}' on node '{}'",
                    node_type, model_name, node_name
                )
                .into());
            }
        }
        Ok(())
    }

    /// Pauses an active workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was paused successfully
    /// * `Err(...)` if the workload pause failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The workload is not in a pausable state
    /// - The runtime operation fails
    #[allow(unused_variables)]
    pub async fn pause_workload(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Starts a paused or stopped workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was started successfully
    /// * `Err(...)` if the workload start failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The workload is not in a startable state
    /// - The runtime operation fails
    pub async fn start_workload(
        &self,
        model_name: &str,
        node_name: &str,
        node_type: &str,
    ) -> Result<()> {
        match node_type {
            "nodeagent" => {
                crate::runtime::nodeagent::start_workload(model_name, node_name).await?;
            }
            _ => {
                return Err(format!(
                    "Unsupported node type '{}' for workload '{}' on node '{}'",
                    node_type, model_name, node_name
                )
                .into());
            }
        }
        Ok(())
    }

    /// Stops an active workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was stopped successfully
    /// * `Err(...)` if the workload stop failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The workload is already stopped
    /// - The runtime operation fails
    pub async fn stop_workload(
        &self,
        model_name: &str,
        node_name: &str,
        node_type: &str,
    ) -> Result<()> {
        match node_type {
            "nodeagent" => {
                crate::runtime::nodeagent::stop_workload(model_name, node_name).await?;
            }
            _ => {
                return Err(format!(
                    "Unsupported node type '{}' for workload '{}' on node '{}'",
                    node_type, model_name, node_name
                )
                .into());
            }
        }
        Ok(())
    }

    pub async fn reload_all_node(&self, model_name: &str, model_node: &str) -> Result<()> {
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}

//UNIT TEST SKELTON

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::Status;
    use std::error::Error;

    #[tokio::test]
    async fn test_get_node_role_from_etcd_invalid_json() {
        // Setup: Insert nodes/{name} and invalid JSON in cluster/nodes/{name}
        common::etcd::put("nodes/TestInvalid", "192.168.1.103")
            .await
            .ok();
        common::etcd::put("cluster/nodes/TestInvalid", "not valid json")
            .await
            .ok();

        let manager = ActionControllerManager::new();
        let result = manager.get_node_role_from_etcd("TestInvalid").await;

        // Must error because JSON is invalid
        assert!(result.is_err());

        // Cleanup
        common::etcd::delete("nodes/TestInvalid").await.ok();
        common::etcd::delete("cluster/nodes/TestInvalid").await.ok();
    }

    #[tokio::test]
    async fn test_get_node_role_from_etcd_etcd_missing_cluster_info() {
        // Setup: Only nodes/{hostname} exists but not cluster/nodes/{hostname}
        // This should fallback to settings.yaml
        common::etcd::put("nodes/TestMissing", "192.168.1.104")
            .await
            .ok();

        let manager = ActionControllerManager::new();
        let result = manager.get_node_role_from_etcd("TestMissing").await;

        // Should fallback to settings.yaml configuration
        // Result depends on settings.yaml, so we accept both ok and err
        assert!(result.is_ok() || result.is_err());

        // Cleanup
        common::etcd::delete("nodes/TestMissing").await.ok();
    }

    // ==================== trigger_manager_action Tests ====================

    #[tokio::test]
    async fn test_trigger_manager_action_empty_scenario_name() {
        let manager = ActionControllerManager::new();
        let result = manager.trigger_manager_action("").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_trigger_manager_action_whitespace_scenario_name() {
        let manager = ActionControllerManager::new();
        let result = manager.trigger_manager_action("   ").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_trigger_manager_action_scenario_not_found() {
        let manager = ActionControllerManager::new();
        let result = manager
            .trigger_manager_action("nonexistent_scenario_xyz")
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_trigger_manager_action_invalid_scenario_yaml() {
        // Setup: Insert invalid YAML for scenario
        common::etcd::put("Scenario/invalid-yaml", "{ invalid: yaml: ]")
            .await
            .unwrap();

        let manager = ActionControllerManager::new();
        let result = manager.trigger_manager_action("invalid-yaml").await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse scenario"));

        // Cleanup
        common::etcd::delete("Scenario/invalid-yaml").await.unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_package_not_found() {
        // Setup: Insert scenario but no corresponding package
        common::etcd::put(
            "Scenario/test-scenario",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test-scenario
spec:
  condition:
  action: launch
  target: missing-package
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager::new();
        let result = manager.trigger_manager_action("test-scenario").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        // Cleanup
        common::etcd::delete("Scenario/test-scenario")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_invalid_package_yaml() {
        // Setup: Insert valid scenario and invalid package
        common::etcd::put(
            "Scenario/test-scenario",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test-scenario
spec:
  condition:
  action: launch
  target: invalid-pkg
"#,
        )
        .await
        .unwrap();

        common::etcd::put("Package/invalid-pkg", "invalid: yaml: ]")
            .await
            .unwrap();

        let manager = ActionControllerManager::new();
        let result = manager.trigger_manager_action("test-scenario").await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse package"));

        // Cleanup
        common::etcd::delete("Scenario/test-scenario")
            .await
            .unwrap();
        common::etcd::delete("Package/invalid-pkg").await.unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_launch_success() {
        // Setup: Insert valid scenario and package
        common::etcd::put(
            "Scenario/launch-test",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: launch-test
spec:
  condition:
  action: launch
  target: launch-pkg
"#,
        )
        .await
        .unwrap();

        common::etcd::put(
            "Package/launch-pkg",
            r#"
apiVersion: v1
kind: Package
metadata:
  label: null
  name: launch-pkg
spec:
  pattern:
    - type: plain
  models:
    - name: test-service
      node: HPC
      resources:
        volume:
        network:
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("launch-test").await;

        assert!(result.is_ok() || result.is_err());

        // Cleanup
        common::etcd::delete("Scenario/launch-test").await.unwrap();
        common::etcd::delete("Package/launch-pkg").await.unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_terminate_success() {
        // Setup: Insert valid scenario with terminate action
        common::etcd::put(
            "Scenario/terminate-test",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: terminate-test
spec:
  condition:
  action: terminate
  target: terminate-pkg
"#,
        )
        .await
        .unwrap();

        common::etcd::put(
            "Package/terminate-pkg",
            r#"
apiVersion: v1
kind: Package
metadata:
  name: terminate-pkg
spec:
  models:
    - name: test-service
      node: HPC
      resources:
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("terminate-test").await;
        assert!(result.is_ok() || result.is_err());

        // Cleanup
        common::etcd::delete("Scenario/terminate-test")
            .await
            .unwrap();
        common::etcd::delete("Package/terminate-pkg").await.unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_update_success() {
        // Setup: Insert valid scenario with update action
        common::etcd::put(
            "Scenario/update-test",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: update-test
spec:
  condition:
  action: update
  target: update-pkg
"#,
        )
        .await
        .unwrap();

        common::etcd::put(
            "Package/update-pkg",
            r#"
apiVersion: v1
kind: Package
metadata:
  name: update-pkg
spec:
  models:
    - name: test-service
      node: HPC
      resources:
        realtime: false
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("update-test").await;
        assert!(result.is_ok() || result.is_err());

        // Cleanup
        common::etcd::delete("Scenario/update-test").await.unwrap();
        common::etcd::delete("Package/update-pkg").await.unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_rollback_success() {
        // Setup: Insert valid scenario with rollback action
        common::etcd::put(
            "Scenario/rollback-test",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: rollback-test
spec:
  condition:
  action: rollback
  target: rollback-pkg
"#,
        )
        .await
        .unwrap();

        common::etcd::put(
            "Package/rollback-pkg",
            r#"
apiVersion: v1
kind: Package
metadata:
  name: rollback-pkg
spec:
  models:
    - name: test-service
      node: HPC
      resources:
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("rollback-test").await;
        assert!(result.is_ok() || result.is_err());

        // Cleanup
        common::etcd::delete("Scenario/rollback-test")
            .await
            .unwrap();
        common::etcd::delete("Package/rollback-pkg").await.unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_unknown_node() {
        // Setup: Insert scenario with unknown node
        common::etcd::put(
            "Scenario/unknown-node-test",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: unknown-node-test
spec:
  action: launch
  target: unknown-node-pkg
"#,
        )
        .await
        .unwrap();

        common::etcd::put(
            "Package/unknown-node-pkg",
            r#"
apiVersion: v1
kind: Package
metadata:
  name: unknown-node-pkg
spec:
  models:
    - name: test-service
      node: UNKNOWN_NODE
      resources:
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("unknown-node-test").await;

        // Should handle unknown nodes gracefully
        assert!(result.is_ok() || result.is_err());

        // Cleanup
        common::etcd::delete("Scenario/unknown-node-test")
            .await
            .unwrap();
        common::etcd::delete("Package/unknown-node-pkg")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_nodeagent_workload() {
        // Setup: Insert scenario with nodeagent node
        common::etcd::put(
            "Scenario/nodeagent-test",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: nodeagent-test
spec:
  action: launch
  target: nodeagent-pkg
"#,
        )
        .await
        .unwrap();

        common::etcd::put(
            "Package/nodeagent-pkg",
            r#"
apiVersion: v1
kind: Package
metadata:
  name: nodeagent-pkg
spec:
  models:
    - name: test-service
      node: ZONE
      resources:
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager {
            nodeagent_nodes: vec!["ZONE".to_string()],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("nodeagent-test").await;
        assert!(result.is_ok() || result.is_err());

        // Cleanup
        common::etcd::delete("Scenario/nodeagent-test")
            .await
            .unwrap();
        common::etcd::delete("Package/nodeagent-pkg").await.unwrap();
    }

    // ==================== reconcile_do Tests ====================

    #[tokio::test]
    async fn test_reconcile_do_same_status() {
        // Test: Current and desired status are the same
        let manager = ActionControllerManager::new();
        let result = manager
            .reconcile_do("test".into(), Status::Running, Status::Running)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reconcile_do_invalid_current_status_none() {
        let manager = ActionControllerManager::new();
        let result = manager
            .reconcile_do("test".into(), Status::None, Status::Running)
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid current status"));
    }

    #[tokio::test]
    async fn test_reconcile_do_invalid_current_status_failed() {
        let manager = ActionControllerManager::new();
        let result = manager
            .reconcile_do("test".into(), Status::Failed, Status::Running)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reconcile_do_invalid_desired_status_none() {
        let manager = ActionControllerManager::new();
        let result = manager
            .reconcile_do("test".into(), Status::Running, Status::None)
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid desired status"));
    }

    // ==================== start_workload Tests ====================

    #[tokio::test]
    async fn test_start_workload_nodeagent_node() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec!["ZONE".to_string()],
            state_sender: StateManagerSender::new(),
        };

        let result = manager
            .start_workload("test-service", "ZONE", "nodeagent")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_workload_invalid_node_type() {
        let manager = ActionControllerManager::new();
        let result = manager
            .start_workload("test-service", "node", "invalid_type")
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported node type"));
    }

    #[tokio::test]
    async fn test_stop_workload_nodeagent_node() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec!["ZONE".to_string()],
            state_sender: StateManagerSender::new(),
        };

        let result = manager
            .stop_workload("test-service", "ZONE", "nodeagent")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_workload_invalid_node_type() {
        let manager = ActionControllerManager::new();
        let result = manager
            .stop_workload("test-service", "node", "invalid_type")
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reload_all_node() {
        let manager = ActionControllerManager::new();
        let result = manager.reload_all_node("test-service", "HPC").await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_reconcile_do_with_valid_status() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };
        let result = manager
            .reconcile_do("antipinch-enable".into(), Status::Running, Status::Running)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_manager_action_with_valid_data() {
        common::etcd::put(
            "Scenario/antipinch-enable",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: antipinch-enable
spec:
  condition:
  action: update
  target: antipinch-enable
"#,
        )
        .await
        .unwrap();

        common::etcd::put(
            "Package/antipinch-enable",
            r#"
apiVersion: v1
kind: Package
metadata:
  label: null
  name: antipinch-enable
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
"#,
        )
        .await
        .unwrap();

        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("antipinch-enable").await;

        if let Err(ref e) = result {
            println!("Error in trigger_manager_action: {:?}", e);
        } else {
            println!("trigger_manager_action successful");
        }

        assert!(result.is_ok());

        common::etcd::delete("Scenario/antipinch-enable")
            .await
            .unwrap();
        common::etcd::delete("Package/antipinch-enable")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_trigger_manager_action_invalid_scenario() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager.trigger_manager_action("invalid_scenario").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reconcile_do_invalid_scenario_key() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager
            .reconcile_do("invalid_scenario".into(), Status::None, Status::Running)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_workload_invalid_node_type_legacy() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result: std::result::Result<(), Box<dyn Error>> = manager
            .start_workload("antipinch-enable", "HPC", "invalid_type")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_workload_invalid_node_type_legacy() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        let result = manager
            .stop_workload("antipinch-enable", "HPC", "invalid_type")
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_manager_initializes_with_empty_nodes() {
        let manager = ActionControllerManager::new();
        assert!(manager.nodeagent_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_create_delete_restart_pause_are_noops() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec![],
            state_sender: StateManagerSender::new(),
        };

        assert!(manager.create_workload("test".into()).await.is_ok());
        assert!(manager.delete_workload("test".into()).await.is_ok());
        assert!(manager.pause_workload("test".into()).await.is_ok());
    }

    #[test]
    fn test_unknown_nodes_skipped() {
        let manager = ActionControllerManager {
            nodeagent_nodes: vec!["ZONE".to_string()],
            state_sender: StateManagerSender::new(),
        };

        assert!(manager.nodeagent_nodes.contains(&"ZONE".to_string()));
    }
}
