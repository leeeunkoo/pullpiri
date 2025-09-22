/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node lookup utilities for finding nodes in the cluster

use base64::Engine;
use common::apiserver::NodeInfo;
use common::etcd;
use prost::Message;
use std::error::Error;

/// Find a node by IP address from simplified node keys
pub async fn find_node_by_simple_key() -> Option<String> {
    println!("Checking simplified node keys in etcd...");
    match etcd::get_all_with_prefix("nodes/").await {
        Ok(kvs) => {
            println!("Found {} simplified node keys", kvs.len());
            for kv in &kvs {
                println!("Node key: {}", kv.key);
                let ip_address = kv.key.trim_start_matches("nodes/");
                println!("Found node IP directly from key: {}", ip_address);
                return Some(ip_address.to_string());
            }
            None
        }
        Err(e) => {
            println!("Error checking simplified nodes: {}", e);
            None
        }
    }
}

/// Find a node directly from etcd using cluster/nodes/ prefix
pub async fn find_node_from_etcd() -> Option<String> {
    println!("Checking cluster/nodes/ prefix in etcd...");
    match etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => {
            println!("Found {} node entries", kvs.len());
            for kv in &kvs {
                println!("Node entry: {}", kv.key);
            }

            if !kvs.is_empty() {
                match base64::engine::general_purpose::STANDARD.decode(&kvs[0].value) {
                    Ok(buf) => match NodeInfo::decode(&buf[..]) {
                        Ok(node) => {
                            println!(
                                "Decoded node: {} ({}), status: {}",
                                node.node_id, node.ip_address, node.status
                            );
                            return Some(node.ip_address);
                        }
                        Err(e) => println!("Failed to decode node data: {}", e),
                    },
                    Err(e) => println!("Failed to decode base64: {}", e),
                }
            }
            None
        }
        Err(e) => {
            println!("Error getting nodes: {}", e);
            None
        }
    }
}

/// Find a node using the NodeManager
pub async fn find_node_from_manager() -> Option<String> {
    let node_manager = match crate::node::NodeManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to create NodeManager: {}", e);
            return None;
        }
    };

    match node_manager.get_nodes().await {
        Ok(nodes) => {
            println!("Node manager found {} nodes", nodes.len());

            if !nodes.is_empty() {
                let node = &nodes[0];
                println!(
                    "Node manager found: {} ({}), status: {}",
                    node.node_id, node.ip_address, node.status
                );
                return Some(node.ip_address.clone());
            } else {
                println!("Node manager found no nodes");
                None
            }
        }
        Err(e) => {
            eprintln!("Node manager error: {}", e);
            None
        }
    }
}

/// Find a node by hostname
pub async fn find_node_by_hostname(hostname: &str) -> Option<common::apiserver::NodeInfo> {
    println!("Looking for node with hostname: {}", hostname);
    match common::etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => {
            for kv in kvs {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&kv.value) {
                    if let Ok(node_info) = common::apiserver::NodeInfo::decode(&decoded[..]) {
                        if node_info.hostname == hostname {
                            println!(
                                "Found node with hostname {}: {}",
                                hostname, node_info.ip_address
                            );
                            return Some(node_info);
                        }
                    }
                }
            }
            println!("No node found with hostname: {}", hostname);
            None
        }
        Err(e) => {
            println!("Error searching for hostname {}: {}", hostname, e);
            None
        }
    }
}

/// Get node IP using all available methods
pub async fn get_node_ip() -> String {
    // Try all methods to find a node IP
    if let Some(ip) = find_node_by_simple_key().await {
        return ip;
    }

    if let Some(ip) = find_node_from_etcd().await {
        return ip;
    }

    if let Some(ip) = find_node_from_manager().await {
        return ip;
    }

    // Fallback to settings file if no nodes are found
    let config = common::setting::get_config();
    let node_ip = config.host.ip.clone();
    println!(
        "All node lookups failed. Falling back to settings IP: {}",
        node_ip
    );

    node_ip
}

/// Add a node IP to the simplified keys for quick lookup
pub async fn add_node_to_simple_keys(ip_address: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let key = format!("nodes/{}", ip_address);
    etcd::put(&key, ip_address).await?;
    println!("Added node IP to simple keys: {}", ip_address);
    Ok(())
}

/// 게스트 노드 정보를 etcd에서 검색하는 함수
pub async fn find_guest_nodes() -> Vec<NodeInfo> {
    println!("Finding guest nodes from etcd...");
    match etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => {
            println!("Found {} node entries for guest search", kvs.len());
            let mut guest_nodes = Vec::new();

            for kv in kvs {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&kv.value) {
                    if let Ok(node_info) = NodeInfo::decode(&decoded[..]) {
                        // 마스터 노드가 아닌 경우에만 게스트 노드로 간주
                        if node_info.node_role != common::nodeagent::NodeRole::Master as i32 {
                            println!(
                                "Found guest node: {} ({}) with role: {}",
                                node_info.node_id, node_info.ip_address, node_info.node_role
                            );
                            guest_nodes.push(node_info);
                        }
                    }
                }
            }

            println!("Found {} guest nodes", guest_nodes.len());
            guest_nodes
        }
        Err(e) => {
            println!("Error searching for guest nodes: {}", e);
            Vec::new()
        }
    }
}
