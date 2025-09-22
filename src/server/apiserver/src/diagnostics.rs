/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Diagnostic utilities for service connectivity

use std::time::Duration;
use tokio::net::TcpStream;

/// Check if a service is reachable at the given IP and port
pub async fn check_service_connectivity(ip: &str, port: u16) -> bool {
    let addr = format!("{}:{}", ip, port);
    println!("Checking connectivity to {}", addr);

    match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => {
            println!("Successfully connected to {}", addr);
            true
        }
        Ok(Err(e)) => {
            println!("Failed to connect to {}: {}", addr, e);
            false
        }
        Err(_) => {
            println!("Connection timeout to {}", addr);
            false
        }
    }
}

/// Check if NodeAgent is reachable at the given IP
pub async fn check_node_agent_connectivity(ip: &str) -> bool {
    check_service_connectivity(ip, 47004).await
}
