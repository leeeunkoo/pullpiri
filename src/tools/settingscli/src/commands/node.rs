/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Node command implementation

use crate::commands::{print_error, print_info, print_json, print_success};
use crate::{Result, SettingsClient};
use clap::Subcommand;
use colored::Colorize;

#[derive(Subcommand)]
pub enum NodeAction {
    /// List all nodes
    List,
    /// Get specific node by ID
    Get {
        /// Node ID
        id: String,
    },
    /// Get node information in raw JSON format
    Raw {
        /// Node ID (optional)
        id: Option<String>,
    },
}

/// Handle node commands
pub async fn handle(client: &SettingsClient, action: NodeAction) -> Result<()> {
    match action {
        NodeAction::List => list_nodes(client).await,
        NodeAction::Get { id } => get_node(client, &id).await,
        NodeAction::Raw { id } => {
            if let Some(node_id) = id {
                get_node_raw(client, &node_id).await
            } else {
                list_nodes_raw(client).await
            }
        }
    }
}

/// List all nodes
async fn list_nodes(client: &SettingsClient) -> Result<()> {
    print_info("Fetching nodes list...");

    match client.get("/api/v1/nodes").await {
        Ok(nodes) => {
            println!("\n{}", "Nodes".bold());
            println!("{}", "=".repeat(50));

            // Look for "nodes" array in the response
            if let Some(nodes_array) = nodes.get("nodes").and_then(|n| n.as_array()) {
                if nodes_array.is_empty() {
                    println!("No nodes found.");
                } else {
                    for (i, node) in nodes_array.iter().enumerate() {
                        println!("{}. Node:", i + 1);
                        if let Some(name) = node.get("node_name") {
                            println!("   Name: {}", name.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(ip) = node.get("ip") {
                            println!("   IP: {}", ip.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(arch) = node.get("arch") {
                            println!("   Architecture: {}", arch.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(os) = node.get("os") {
                            println!("   OS: {}", os.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(cpu_count) = node.get("cpu_count") {
                            println!("   CPU Count: {}", cpu_count.as_u64().unwrap_or(0));
                        }
                        if let Some(cpu_usage) = node.get("cpu_usage") {
                            println!("   CPU Usage: {:.2}%", cpu_usage.as_f64().unwrap_or(0.0));
                        }
                        if let Some(mem_usage) = node.get("mem_usage") {
                            println!("   Memory Usage: {:.2}%", mem_usage.as_f64().unwrap_or(0.0));
                        }
                        println!();
                    }
                }
            } else if let Some(name) = nodes.get("node_name") {
                // Single node response
                println!("Node Name: {}", name.as_str().unwrap_or("Unknown"));
                if let Some(ip) = nodes.get("ip") {
                    println!("IP: {}", ip.as_str().unwrap_or("Unknown"));
                }
            } else {
                println!("No nodes found.");
            }

            print_success("Nodes list retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch nodes: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get specific node information
async fn get_node(client: &SettingsClient, node_id: &str) -> Result<()> {
    print_info(&format!("Fetching node information for ID: {}", node_id));

    let endpoint = format!("/api/v1/nodes/{}", node_id);
    match client.get(&endpoint).await {
        Ok(node) => {
            println!("\n{}", format!("Node: {}", node_id).bold());
            println!("{}", "=".repeat(50));

            if let Some(id) = node.get("id") {
                println!("ID: {}", id.as_str().unwrap_or("Unknown"));
            }

            if let Some(name) = node.get("node_name") {
                println!("Name: {}", name.as_str().unwrap_or("Unknown"));
            }

            if let Some(ip) = node.get("ip") {
                println!("IP Address: {}", ip.as_str().unwrap_or("Unknown"));
            }

            if let Some(os) = node.get("os") {
                println!("OS: {}", os.as_str().unwrap_or("Unknown"));
            }

            if let Some(arch) = node.get("arch") {
                println!("Architecture: {}", arch.as_str().unwrap_or("Unknown"));
            }

            // Resource information
            println!("\nResource Usage:");
            if let Some(cpu_usage) = node.get("cpu_usage") {
                println!("  CPU Usage: {:.2}%", cpu_usage.as_f64().unwrap_or(0.0));
            }

            if let Some(cpu_count) = node.get("cpu_count") {
                println!("  CPU Count: {}", cpu_count.as_u64().unwrap_or(0));
            }

            if let Some(gpu_count) = node.get("gpu_count") {
                println!("  GPU Count: {}", gpu_count.as_u64().unwrap_or(0));
            }

            if let Some(mem_usage) = node.get("mem_usage") {
                println!("  Memory Usage: {:.2}%", mem_usage.as_f64().unwrap_or(0.0));
            }

            if let Some(used_memory) = node.get("used_memory") {
                let used_gb = used_memory.as_u64().unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
                println!("  Used Memory: {:.2} GB", used_gb);
            }

            if let Some(total_memory) = node.get("total_memory") {
                let total_gb =
                    total_memory.as_u64().unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
                println!("  Total Memory: {:.2} GB", total_gb);
            }

            // Network information
            println!("\nNetwork Usage:");
            if let Some(rx_bytes) = node.get("rx_bytes") {
                println!("  RX Bytes: {}", rx_bytes.as_u64().unwrap_or(0));
            }

            if let Some(tx_bytes) = node.get("tx_bytes") {
                println!("  TX Bytes: {}", tx_bytes.as_u64().unwrap_or(0));
            }

            // Disk information
            println!("\nDisk Usage:");
            if let Some(read_bytes) = node.get("read_bytes") {
                println!("  Read Bytes: {}", read_bytes.as_u64().unwrap_or(0));
            }

            if let Some(write_bytes) = node.get("write_bytes") {
                println!("  Write Bytes: {}", write_bytes.as_u64().unwrap_or(0));
            }

            print_success("Node information retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch node {}: {}", node_id, e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get nodes list in raw JSON format
async fn list_nodes_raw(client: &SettingsClient) -> Result<()> {
    print_info("Fetching raw nodes data...");

    match client.get("/api/v1/nodes").await {
        Ok(nodes) => {
            print_json(&nodes)?;
            print_success("Raw nodes data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch raw nodes data: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get specific node in raw JSON format
async fn get_node_raw(client: &SettingsClient, node_id: &str) -> Result<()> {
    print_info(&format!("Fetching raw node data for ID: {}", node_id));

    let endpoint = format!("/api/v1/nodes/{}", node_id);
    match client.get(&endpoint).await {
        Ok(node) => {
            print_json(&node)?;
            print_success("Raw node data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!(
                "Failed to fetch raw node data for {}: {}",
                node_id, e
            ));
            return Err(e);
        }
    }

    Ok(())
}
