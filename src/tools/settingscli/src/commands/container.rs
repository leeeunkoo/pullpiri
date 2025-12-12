/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use crate::commands::{print_error, print_info, print_success};
use crate::{Result, SettingsClient};
use clap::Subcommand;
use colored::Colorize;

#[derive(Subcommand)]
pub enum ContainerAction {
    /// List all containers
    List,
    /// Get specific container by ID
    Get { id: String },
    /// Get container information in raw JSON format
    Raw,
}

pub async fn handle(client: &SettingsClient, action: ContainerAction) -> Result<()> {
    match action {
        ContainerAction::List => list_containers(client).await,
        ContainerAction::Get { id } => get_container(client, &id).await,
        ContainerAction::Raw => raw_containers(client).await,
    }
}

/// List all containers
async fn list_containers(client: &SettingsClient) -> Result<()> {
    print_info("Fetching containers list...");

    match client.get("/api/v1/containers").await {
        Ok(containers) => {
            println!("\n{}", "Containers".bold());
            println!("{}", "=".repeat(50));

            // If the response is an array, iterate and print each container
            if let Some(containers_array) = containers.as_array() {
                if containers_array.is_empty() {
                    println!("No containers found.");
                } else {
                    for (i, container) in containers_array.iter().enumerate() {
                        println!("{}. Container:", i + 1);
                        if let Some(id) = container.get("id") {
                            println!("   ID: {}", id.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(names) = container.get("names").and_then(|n| n.as_array()) {
                            if let Some(first_name) = names.first().and_then(|n| n.as_str()) {
                                println!("   Name: {}", first_name);
                            }
                        }
                        if let Some(image) = container.get("image") {
                            println!("   Image: {}", image.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(state) = container.get("state") {
                            if let Some(status) = state.get("Status") {
                                println!("   Status: {}", status.as_str().unwrap_or("Unknown"));
                            }
                        }
                        if let Some(config) = container.get("config") {
                            if let Some(hostname) = config.get("Hostname") {
                                println!("   Hostname: {}", hostname.as_str().unwrap_or("Unknown"));
                            }
                        }
                        println!();
                    }
                }
            } else if let Some(containers_obj) =
                containers.get("containers").and_then(|c| c.as_array())
            {
                // Handle wrapped response format
                if containers_obj.is_empty() {
                    println!("No containers found.");
                } else {
                    for (i, container) in containers_obj.iter().enumerate() {
                        println!("{}. Container:", i + 1);
                        if let Some(id) = container.get("id") {
                            println!("   ID: {}", id.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(names) = container.get("names").and_then(|n| n.as_array()) {
                            if let Some(first_name) = names.first().and_then(|n| n.as_str()) {
                                println!("   Name: {}", first_name);
                            }
                        }
                        if let Some(image) = container.get("image") {
                            println!("   Image: {}", image.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(state) = container.get("state") {
                            if let Some(status) = state.get("Status") {
                                println!("   Status: {}", status.as_str().unwrap_or("Unknown"));
                            }
                        }
                        if let Some(config) = container.get("config") {
                            if let Some(hostname) = config.get("Hostname") {
                                println!("   Hostname: {}", hostname.as_str().unwrap_or("Unknown"));
                            }
                        }
                        println!();
                    }
                }
            } else {
                println!("No containers found.");
            }

            print_success("Containers list retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch containers: {}", e));
            return Err(e.into());
        }
    }

    Ok(())
}

/// Get specific container information
async fn get_container(client: &SettingsClient, container_id: &str) -> Result<()> {
    print_info(&format!(
        "Fetching container information for ID: {}",
        container_id
    ));

    let endpoint = format!("/api/v1/containers/{}", container_id);
    match client.get(&endpoint).await {
        Ok(container) => {
            println!("\n{}", format!("Container: {}", container_id).bold());
            println!("{}", "=".repeat(50));

            if let Some(id) = container.get("id") {
                println!("ID: {}", id.as_str().unwrap_or("Unknown"));
            }

            if let Some(names) = container.get("names").and_then(|n| n.as_array()) {
                if let Some(first_name) = names.first().and_then(|n| n.as_str()) {
                    println!("Name: {}", first_name);
                }
            }

            if let Some(image) = container.get("image") {
                println!("Image: {}", image.as_str().unwrap_or("Unknown"));
            }

            if let Some(state) = container.get("state") {
                println!("\nState:");
                if let Some(status) = state.get("Status") {
                    println!("   Status: {}", status.as_str().unwrap_or("Unknown"));
                }
                if let Some(running) = state.get("Running") {
                    println!("   Running: {}", running.as_bool().unwrap_or(false));
                }
                if let Some(pid) = state.get("Pid") {
                    println!("   PID: {}", pid.as_i64().unwrap_or(0));
                }
            }

            if let Some(config) = container.get("config") {
                println!("\nConfiguration:");
                if let Some(hostname) = config.get("Hostname") {
                    println!("   Hostname: {}", hostname.as_str().unwrap_or("Unknown"));
                }
                if let Some(user) = config.get("User") {
                    println!("   User: {}", user.as_str().unwrap_or("Unknown"));
                }
                if let Some(working_dir) = config.get("WorkingDir") {
                    println!(
                        "   Working Dir: {}",
                        working_dir.as_str().unwrap_or("Unknown")
                    );
                }
            }

            if let Some(stats) = container.get("stats") {
                println!("\nStats:");
                if let Some(status) = stats.get("Status") {
                    println!("   Status: {}", status.as_str().unwrap_or("Unknown"));
                }
                if let Some(cpu_usage) = stats.get("CpuTotalUsage") {
                    println!(
                        "   CPU Total Usage: {}",
                        cpu_usage.as_str().unwrap_or("Unknown")
                    );
                }
                if let Some(memory_usage) = stats.get("MemoryUsage") {
                    println!(
                        "   Memory Usage: {}",
                        memory_usage.as_str().unwrap_or("Unknown")
                    );
                }
                if let Some(memory_limit) = stats.get("MemoryLimit") {
                    println!(
                        "   Memory Limit: {}",
                        memory_limit.as_str().unwrap_or("Unknown")
                    );
                }
            }

            print_success("Container information retrieved successfully");
        }
        Err(e) => {
            print_error(&format!(
                "Failed to fetch container {}: {}",
                container_id, e
            ));
            return Err(e.into());
        }
    }

    Ok(())
}

/// Get raw containers data
async fn raw_containers(client: &SettingsClient) -> Result<()> {
    print_info("Fetching raw containers data...");

    match client.get("/api/v1/containers").await {
        Ok(containers) => {
            println!("{}", serde_json::to_string_pretty(&containers)?);
            print_success("Raw containers data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch raw containers data: {}", e));
            return Err(e.into());
        }
    }

    Ok(())
}
