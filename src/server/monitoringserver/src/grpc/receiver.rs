/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::monitoringserver::monitoring_server_connection_server::MonitoringServerConnection;
use common::monitoringserver::{
    ContainerList, NodeInfo, SendContainerListResponse, SendNodeInfoResponse,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// MonitoringServer gRPC service handler
#[derive(Clone)]
pub struct MonitoringServerReceiver {
    pub tx_container: mpsc::Sender<ContainerList>,
    pub tx_node: mpsc::Sender<NodeInfo>,
}

#[tonic::async_trait]
impl MonitoringServerConnection for MonitoringServerReceiver {
    /// Handle a ContainerList message from nodeagent
    ///
    /// Receives a ContainerList from nodeagent and forwards it to the MonitoringServer manager for processing.
    async fn send_container_list<'life>(
        &'life self,
        request: Request<ContainerList>,
    ) -> Result<Response<SendContainerListResponse>, Status> {
        let req: ContainerList = request.into_inner();

        match self.tx_container.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendContainerListResponse {
                resp: "Successfully processed ContainerList".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send container list: {}", e),
            )),
        }
    }

    /// Handle a NodeInfo message from nodeagent
    ///
    /// Receives a NodeInfo from nodeagent and forwards it to the MonitoringServer manager for processing.
    async fn send_node_info<'life>(
        &'life self,
        request: Request<NodeInfo>,
    ) -> Result<Response<SendNodeInfoResponse>, Status> {
        let req: NodeInfo = request.into_inner();

        match self.tx_node.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendNodeInfoResponse {
                resp: "Successfully processed NodeInfo".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send node info: {}", e),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::monitoringserver::{
        ContainerList, NodeInfo, SendContainerListResponse, SendNodeInfoResponse,
    };
    use tokio::sync::mpsc;
    use tokio::time::{timeout, Duration};
    use tonic::{Code, Request, Status};

    fn sample_node(name: &str, ip: &str) -> NodeInfo {
        NodeInfo {
            node_name: name.to_string(),
            ip: ip.to_string(),
            cpu_usage: 42.0,
            cpu_count: 2,
            gpu_count: 1,
            used_memory: 1024,
            total_memory: 2048,
            mem_usage: 50.0,
            rx_bytes: 100,
            tx_bytes: 200,
            read_bytes: 300,
            write_bytes: 400,
            arch: "x86_64".to_string(),
            os: "linux".to_string(),
        }
    }

    fn sample_container_list(node_name: &str) -> ContainerList {
        ContainerList {
            node_name: node_name.to_string(),
            containers: vec![],
        }
    }

    #[tokio::test]
    async fn test_send_container_list_success() {
        let (tx, mut rx) = mpsc::channel(1);
        let dummy_tx = mpsc::channel(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: tx,
            tx_node: dummy_tx,
        };
        let req = Request::new(sample_container_list("node1"));
        let resp = receiver.send_container_list(req).await.unwrap();
        assert_eq!(resp.get_ref().resp, "Successfully processed ContainerList");
        // Ensure the message was sent
        let received = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_send_container_list_failure() {
        // Drop the receiver so send will fail
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let dummy_tx = mpsc::channel(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: tx,
            tx_node: dummy_tx,
        };
        let req = Request::new(sample_container_list("node1"));
        let resp = receiver.send_container_list(req).await;
        assert!(resp.is_err());
        let status = resp.err().unwrap();
        assert_eq!(status.code(), Code::Unavailable);
    }

    #[tokio::test]
    async fn test_send_node_info_success() {
        let (tx, mut rx) = mpsc::channel(1);
        let dummy_tx = mpsc::channel(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: dummy_tx,
            tx_node: tx,
        };
        let req = Request::new(sample_node("node1", "192.168.10.201"));
        let resp = receiver.send_node_info(req).await.unwrap();
        assert_eq!(resp.get_ref().resp, "Successfully processed NodeInfo");
        // Ensure the message was sent
        let received = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_send_node_info_failure() {
        // Drop the receiver so send will fail
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let dummy_tx = mpsc::channel(1).0;
        let receiver = MonitoringServerReceiver {
            tx_container: dummy_tx,
            tx_node: tx,
        };
        let req = Request::new(sample_node("node1", "192.168.10.201"));
        let resp = receiver.send_node_info(req).await;
        assert!(resp.is_err());
        let status = resp.err().unwrap();
        assert_eq!(status.code(), Code::Unavailable);
    }
}
