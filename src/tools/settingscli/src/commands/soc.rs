/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! SoC command implementation

use crate::commands::{print_error, print_info, print_json, print_success};
use crate::{Result, SettingsClient};
use clap::Subcommand;
use colored::Colorize;

#[derive(Subcommand)]
pub enum SocAction {
    /// List all SoCs
    List,
    /// Get specific SoC by ID
    Get {
        /// SoC ID
        id: String,
    },
    /// Get SoC information in raw JSON format
    Raw {
        /// SoC ID (optional)
        id: Option<String>,
    },
}

/// Handle SoC commands
pub async fn handle(client: &SettingsClient, action: SocAction) -> Result<()> {
    match action {
        SocAction::List => list_socs(client).await,
        SocAction::Get { id } => get_soc(client, &id).await,
        SocAction::Raw { id } => {
            if let Some(soc_id) = id {
                get_soc_raw(client, &soc_id).await
            } else {
                list_socs_raw(client).await
            }
        }
    }
}

/// List all SoCs
async fn list_socs(client: &SettingsClient) -> Result<()> {
    print_info("Fetching SoCs list...");

    match client.get("/api/v1/socs").await {
        Ok(socs) => {
            println!("\n{}", "SoCs".bold());
            println!("{}", "=".repeat(50));

            // Look for "socs" array in the response
            if let Some(socs_array) = socs.get("socs").and_then(|s| s.as_array()) {
                if socs_array.is_empty() {
                    println!("No SoCs found.");
                } else {
                    for (i, soc) in socs_array.iter().enumerate() {
                        println!("{}. SoC:", i + 1);
                        if let Some(id) = soc.get("soc_id") {
                            println!("   ID: {}", id.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(status) = soc.get("status") {
                            println!("   Status: {}", status.as_str().unwrap_or("Unknown"));
                        }

                        // Show aggregated resource info
                        if let Some(total_cpu_usage) = soc.get("total_cpu_usage") {
                            println!(
                                "   Total CPU Usage: {:.2}%",
                                total_cpu_usage.as_f64().unwrap_or(0.0)
                            );
                        }

                        if let Some(total_mem_usage) = soc.get("total_mem_usage") {
                            println!(
                                "   Total Memory Usage: {:.2}%",
                                total_mem_usage.as_f64().unwrap_or(0.0)
                            );
                        }

                        if let Some(nodes) = soc.get("nodes").and_then(|n| n.as_array()) {
                            println!("   Nodes: {}", nodes.len());
                        }

                        println!();
                    }
                }
            } else if let Some(id) = socs.get("soc_id") {
                // Single SoC response
                println!("SoC ID: {}", id.as_str().unwrap_or("Unknown"));
                if let Some(status) = socs.get("status") {
                    println!("Status: {}", status.as_str().unwrap_or("Unknown"));
                }
            }

            print_success("SoCs list retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch SoCs: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get specific SoC information
async fn get_soc(client: &SettingsClient, soc_id: &str) -> Result<()> {
    print_info(&format!("Fetching SoC information for ID: {}", soc_id));

    let endpoint = format!("/api/v1/socs/{}", soc_id);
    match client.get(&endpoint).await {
        Ok(soc) => {
            println!("\n{}", format!("SoC: {}", soc_id).bold());
            println!("{}", "=".repeat(50));

            if let Some(id) = soc.get("soc_id") {
                println!("ID: {}", id.as_str().unwrap_or("Unknown"));
            }

            if let Some(status) = soc.get("status") {
                println!("Status: {}", status.as_str().unwrap_or("Unknown"));
            }

            // Aggregated resource information
            println!("\nAggregated Resources:");
            if let Some(total_cpu_usage) = soc.get("total_cpu_usage") {
                println!(
                    "  Total CPU Usage: {:.2}%",
                    total_cpu_usage.as_f64().unwrap_or(0.0)
                );
            }

            if let Some(total_cpu_count) = soc.get("total_cpu_count") {
                println!(
                    "  Total CPU Count: {}",
                    total_cpu_count.as_u64().unwrap_or(0)
                );
            }

            if let Some(total_gpu_count) = soc.get("total_gpu_count") {
                println!(
                    "  Total GPU Count: {}",
                    total_gpu_count.as_u64().unwrap_or(0)
                );
            }

            if let Some(total_mem_usage) = soc.get("total_mem_usage") {
                println!(
                    "  Total Memory Usage: {:.2}%",
                    total_mem_usage.as_f64().unwrap_or(0.0)
                );
            }

            if let Some(total_used_memory) = soc.get("total_used_memory") {
                let used_gb =
                    total_used_memory.as_u64().unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
                println!("  Total Used Memory: {:.2} GB", used_gb);
            }

            if let Some(total_memory) = soc.get("total_memory") {
                let total_gb =
                    total_memory.as_u64().unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
                println!("  Total Memory: {:.2} GB", total_gb);
            }

            // Network information
            println!("\nTotal Network Usage:");
            if let Some(total_rx_bytes) = soc.get("total_rx_bytes") {
                println!("  Total RX Bytes: {}", total_rx_bytes.as_u64().unwrap_or(0));
            }

            if let Some(total_tx_bytes) = soc.get("total_tx_bytes") {
                println!("  Total TX Bytes: {}", total_tx_bytes.as_u64().unwrap_or(0));
            }

            // Disk information
            println!("\nTotal Disk Usage:");
            if let Some(total_read_bytes) = soc.get("total_read_bytes") {
                println!(
                    "  Total Read Bytes: {}",
                    total_read_bytes.as_u64().unwrap_or(0)
                );
            }

            if let Some(total_write_bytes) = soc.get("total_write_bytes") {
                println!(
                    "  Total Write Bytes: {}",
                    total_write_bytes.as_u64().unwrap_or(0)
                );
            }

            // Nodes information
            if let Some(nodes) = soc.get("nodes").and_then(|n| n.as_array()) {
                println!("\nNodes ({}):", nodes.len());
                for (i, node) in nodes.iter().enumerate() {
                    if let Some(node_name) = node.get("node_name") {
                        println!("  {}. {}", i + 1, node_name.as_str().unwrap_or("Unknown"));

                        if let Some(ip) = node.get("ip") {
                            println!("     IP: {}", ip.as_str().unwrap_or("Unknown"));
                        }

                        if let Some(cpu_usage) = node.get("cpu_usage") {
                            println!("     CPU Usage: {:.2}%", cpu_usage.as_f64().unwrap_or(0.0));
                        }

                        if let Some(mem_usage) = node.get("mem_usage") {
                            println!(
                                "     Memory Usage: {:.2}%",
                                mem_usage.as_f64().unwrap_or(0.0)
                            );
                        }
                    }
                }
            }

            // Last updated information
            if let Some(last_updated) = soc.get("last_updated") {
                if let Some(secs) = last_updated.get("secs_since_epoch") {
                    println!(
                        "\nLast Updated: {} seconds since epoch",
                        secs.as_u64().unwrap_or(0)
                    );
                }
            }

            print_success("SoC information retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch SoC {}: {}", soc_id, e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get SoCs list in raw JSON format
async fn list_socs_raw(client: &SettingsClient) -> Result<()> {
    print_info("Fetching raw SoCs data...");

    match client.get("/api/v1/socs").await {
        Ok(socs) => {
            print_json(&socs)?;
            print_success("Raw SoCs data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch raw SoCs data: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get specific SoC in raw JSON format
async fn get_soc_raw(client: &SettingsClient, soc_id: &str) -> Result<()> {
    print_info(&format!("Fetching raw SoC data for ID: {}", soc_id));

    let endpoint = format!("/api/v1/socs/{}", soc_id);
    match client.get(&endpoint).await {
        Ok(soc) => {
            print_json(&soc)?;
            print_success("Raw SoC data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!(
                "Failed to fetch raw SoC data for {}: {}",
                soc_id, e
            ));
            return Err(e);
        }
    }

    Ok(())
}
