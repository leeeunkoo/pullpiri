pub mod actioncontroller;
pub mod apiserver;

/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::nodeagent::node_agent_connection_server::NodeAgentConnection;
use common::nodeagent::fromapiserver::{
    ConfigRequest, ConfigResponse, HandleYamlRequest, HandleYamlResponse, HeartbeatRequest,
    HeartbeatResponse, NodeRegistrationRequest, NodeRegistrationResponse, StatusAck, StatusReport,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// NodeAgent gRPC service handler
#[derive(Clone)]
pub struct NodeAgentReceiver {
    pub tx: mpsc::Sender<HandleYamlRequest>,
    // Add node information for clustering
    pub node_id: String,
    pub hostname: String,
    pub ip_address: String,
}

impl NodeAgentReceiver {
    pub fn new(
        tx: mpsc::Sender<HandleYamlRequest>,
        node_id: String,
        hostname: String,
        ip_address: String,
    ) -> Self {
        Self {
            tx,
            node_id,
            hostname,
            ip_address,
        }
    }
}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentReceiver {
    /// Handle a yaml request from API-Server
    ///
    /// Receives a yaml from API-Server and forwards it to the NodeAgent manager for processing.
    async fn handle_yaml(
        &self,
        request: Request<HandleYamlRequest>,
    ) -> Result<Response<HandleYamlResponse>, Status> {
        apiserver::handle_yaml(self.tx.clone(), request).await
    }

    /// Register this node with the API server
    async fn register_node(
        &self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        apiserver::register_node(request).await
    }

    /// Report status to the API server
    async fn report_status(
        &self,
        request: Request<StatusReport>,
    ) -> Result<Response<StatusAck>, Status> {
        apiserver::report_status(request).await
    }

    /// Process heartbeat from API server
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        apiserver::heartbeat(request).await
    }

    /// Receive configuration updates from API server
    async fn receive_config(
        &self,
        request: Request<ConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        apiserver::receive_config(request).await
    }

    async fn handle_workload(
        &self,
        _request: Request<common::nodeagent::fromactioncontroller::HandleWorkloadRequest>,
    ) -> Result<Response<common::nodeagent::fromactioncontroller::HandleWorkloadResponse>, Status> {
        unimplemented!()
    }
}

/*
#[cfg(test)]
mod tests {
    use crate::grpc::receiver::apiserver::{NodeAgentConnection, NodeAgentReceiver};
    use common::nodeagent::fromapiserver::{
        ClusterConfig, ConfigRequest, ConfigResponse, HandleYamlRequest, HandleYamlResponse,
        HeartbeatRequest, HeartbeatResponse, NodeRegistrationRequest, NodeRegistrationResponse,
        StatusAck, StatusReport,
    };
    use tokio::sync::mpsc;
    use tonic::{Request, Status};

    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: hellow
spec:
  condition:
  action: update
  target: hellow
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: hellow
spec:
  pattern:
    - type: plain
  models:
    - name: hellow-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: hellow-core
  annotations:
    io.piccolo.annotations.package-type: hellow-core
    io.piccolo.annotations.package-name: hellow
    io.piccolo.annotations.package-network: default
  labels:
    app: hellow-core
spec:
  hostNetwork: true
  containers:
    - name: hellow
      image: hellow
  terminationGracePeriodSeconds: 0
"#;

    #[tokio::test]
    async fn test_handle_yaml_with_valid_artifact_yaml() {
        let (tx, mut rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
        );

        let request = HandleYamlRequest {
            yaml: VALID_ARTIFACT_YAML.to_string(),
            ..Default::default()
        };
        let tonic_request = Request::new(request.clone());

        let response = receiver.handle_yaml(tonic_request).await.unwrap();
        let response_inner = response.into_inner();

        assert!(response_inner.status);
        assert_eq!(response_inner.desc, "Successfully processed YAML");

        let received = rx.recv().await.unwrap();
        assert_eq!(received.yaml, request.yaml);
    }

    #[tokio::test]
    async fn test_handle_yaml_send_error() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
        );

        let request = HandleYamlRequest {
            yaml: VALID_ARTIFACT_YAML.to_string(),
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let result = receiver.handle_yaml(tonic_request).await;

        assert!(result.is_err());
        let status = result.err().unwrap();
        assert_eq!(status.code(), tonic::Code::Unavailable);
        assert!(status.message().starts_with("cannot send condition:"));
    }

    #[tokio::test]
    async fn test_register_node_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
        );

        let request = NodeRegistrationRequest {
            node_id: "test-node".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .register_node(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.success);
        assert_eq!(response.message, "Node registration processed");
        assert_eq!(response.cluster_token, "node-token");
        assert!(response.cluster_config.is_some());
        let config = response.cluster_config.unwrap();
        assert!(config.master_endpoint.contains("http://"));
        assert_eq!(config.heartbeat_interval, 30);
    }

    #[tokio::test]
    async fn test_report_status_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
        );

        let request = StatusReport {
            node_id: "test-node".to_string(),
            status: 1,
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .report_status(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.received);
        assert_eq!(response.message, "Status report received");
    }

    #[tokio::test]
    async fn test_heartbeat_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
        );

        let request = HeartbeatRequest {
            node_id: "test-node".to_string(),
            timestamp: 123456789,
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .heartbeat(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.ack);
        assert!(response.updated_config.is_some());
        let config = response.updated_config.unwrap();
        assert!(config.master_endpoint.contains("http://"));
        assert_eq!(config.heartbeat_interval, 30);
    }

    #[tokio::test]
    async fn test_receive_config_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
        );

        let mut config_map = std::collections::HashMap::new();
        config_map.insert("key".to_string(), "value".to_string());

        let request = ConfigRequest {
            config: config_map,
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .receive_config(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.applied);
        assert_eq!(response.message, "Configuration applied successfully");
    }
}
*/
