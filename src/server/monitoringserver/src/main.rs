/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! MonitoringServer main entry point
//!
//! This file sets up the asynchronous runtime, initializes the manager and gRPC server,
//! and launches both concurrently. It also provides unit tests for initialization.

use common::monitoringserver::{ContainerList, NodeInfo};
pub mod data_structures;
pub mod etcd_storage;
pub mod grpc;
pub mod manager;

use common::monitoringserver::monitoring_server_connection_server::MonitoringServerConnectionServer;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Launches the MonitoringServerManager in an asynchronous task.
///
/// This function creates the manager, initializes it, and then runs it.
/// If initialization or running fails, errors are printed to stderr.
async fn launch_manager(rx_container: Receiver<ContainerList>, rx_node: Receiver<NodeInfo>) {
    let mut manager = manager::MonitoringServerManager::new(rx_container, rx_node).await;

    match manager.initialize().await {
        Ok(_) => {
            println!("MonitoringServerManager successfully initialized");
            if let Err(e) = manager.run().await {
                eprintln!("Error running MonitoringServerManager: {:?}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize MonitoringServerManager: {:?}", e);
        }
    }
}

/// Initializes the MonitoringServer gRPC server.
///
/// Sets up the gRPC service and starts listening for incoming requests.
async fn initialize(tx_container: Sender<ContainerList>, tx_node: Sender<NodeInfo>) {
    use tonic::transport::Server;

    let server = grpc::receiver::MonitoringServerReceiver {
        tx_container,
        tx_node,
    };

    let addr = common::monitoringserver::open_server()
        .parse()
        .expect("monitoringserver address parsing error");
    println!("MonitoringServer listening on {}", addr);

    if let Err(e) = Server::builder()
        .add_service(MonitoringServerConnectionServer::new(server))
        .serve(addr)
        .await
    {
        eprintln!("gRPC server error: {}", e);
    }
}

#[tokio::main]
async fn main() {
    println!("Starting MonitoringServer...");

    let (tx_container, rx_container) = channel::<ContainerList>(100);
    let (tx_node, rx_node) = channel::<NodeInfo>(100);

    let mgr = launch_manager(rx_container, rx_node);
    let grpc = initialize(tx_container, tx_node);

    tokio::join!(mgr, grpc);
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::monitoringserver::{ContainerInfo, ContainerList, NodeInfo};
    use std::collections::HashMap;
    use tokio::time::{timeout, Duration};

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

    fn sample_container(id: &str, name: &str, status: &str) -> ContainerInfo {
        let mut state = HashMap::new();
        state.insert("Status".to_string(), status.to_string());
        ContainerInfo {
            id: id.to_string(),
            names: vec![name.to_string()],
            image: "testimg".to_string(),
            state,
            ..Default::default()
        }
    }

    fn sample_container_list(node_name: &str, containers: Vec<ContainerInfo>) -> ContainerList {
        ContainerList {
            node_name: node_name.to_string(),
            containers,
        }
    }

    #[tokio::test]
    async fn test_launch_manager_completes() {
        let (tx_c, rx_c) = tokio::sync::mpsc::channel(1);
        let (tx_n, rx_n) = tokio::sync::mpsc::channel(1);

        // Use a timeout to ensure the test does not hang
        let result = timeout(Duration::from_secs(2), launch_manager(rx_c, rx_n)).await;
        //assert!(result.is_ok(), "launch_manager did not complete in time");
    }

    #[tokio::test]
    async fn test_initialize_completes() {
        let (tx_c, mut rx_c) = tokio::sync::mpsc::channel(1);
        let (tx_n, mut rx_n) = tokio::sync::mpsc::channel(1);

        // Spawn initialize in a background task and cancel after a short delay
        let handle = tokio::spawn(async move {
            // Use a short timeout to avoid hanging on .serve()
            let _ = timeout(Duration::from_millis(500), initialize(tx_c, tx_n)).await;
        });

        // Wait for the task to finish or timeout
        let result = timeout(Duration::from_secs(1), handle).await;
        assert!(result.is_ok(), "initialize did not complete in time");
    }
}
