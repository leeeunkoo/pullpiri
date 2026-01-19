/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Board command implementation

use crate::commands::{print_error, print_info, print_json, print_success};
use crate::{Result, SettingsClient};
use clap::Subcommand;
use colored::Colorize;

#[derive(Subcommand)]
pub enum BoardAction {
    /// List all boards
    List,
    /// Get specific board by ID
    Get {
        /// Board ID
        id: String,
    },
    /// Get board information in raw JSON format
    Raw {
        /// Board ID (optional)
        id: Option<String>,
    },
}

/// Handle board commands
pub async fn handle(client: &SettingsClient, action: BoardAction) -> Result<()> {
    match action {
        BoardAction::List => list_boards(client).await,
        BoardAction::Get { id } => get_board(client, &id).await,
        BoardAction::Raw { id } => {
            if let Some(board_id) = id {
                get_board_raw(client, &board_id).await
            } else {
                list_boards_raw(client).await
            }
        }
    }
}

/// List all boards
async fn list_boards(client: &SettingsClient) -> Result<()> {
    print_info("Fetching boards list...");

    match client.get("/api/v1/boards").await {
        Ok(boards) => {
            println!("\n{}", "Boards".bold());
            println!("{}", "=".repeat(50));

            // Look for "boards" array in the response
            if let Some(boards_array) = boards.get("boards").and_then(|b| b.as_array()) {
                if boards_array.is_empty() {
                    println!("No boards found.");
                } else {
                    for (i, board) in boards_array.iter().enumerate() {
                        println!("{}. Board:", i + 1);
                        if let Some(id) = board.get("board_id") {
                            println!("   ID: {}", id.as_str().unwrap_or("Unknown"));
                        }
                        // Name and status may not exist in your schema, so print if present
                        if let Some(name) = board.get("name") {
                            println!("   Name: {}", name.as_str().unwrap_or("Unknown"));
                        }
                        if let Some(status) = board.get("status") {
                            println!("   Status: {}", status.as_str().unwrap_or("Unknown"));
                        }
                        println!();
                    }
                }
            } else if let Some(id) = boards.get("board_id") {
                // Single board response
                println!("Board ID: {}", id.as_str().unwrap_or("Unknown"));
                if let Some(name) = boards.get("name") {
                    println!("Name: {}", name.as_str().unwrap_or("Unknown"));
                }
            } else {
                println!("No boards found.");
            }

            print_success("Boards list retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch boards: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get specific board information
async fn get_board(client: &SettingsClient, board_id: &str) -> Result<()> {
    print_info(&format!("Fetching board information for ID: {}", board_id));

    let endpoint = format!("/api/v1/boards/{}", board_id);
    match client.get(&endpoint).await {
        Ok(board) => {
            println!("\n{}", format!("Board: {}", board_id).bold());
            println!("{}", "=".repeat(50));

            if let Some(id) = board.get("id") {
                println!("ID: {}", id.as_str().unwrap_or("Unknown"));
            }

            if let Some(name) = board.get("name") {
                println!("Name: {}", name.as_str().unwrap_or("Unknown"));
            }

            if let Some(status) = board.get("status") {
                println!("Status: {}", status.as_str().unwrap_or("Unknown"));
            }

            if let Some(nodes) = board.get("nodes").and_then(|n| n.as_array()) {
                println!("\nNodes ({}):", nodes.len());
                for (i, node) in nodes.iter().enumerate() {
                    if let Some(node_name) = node.get("node_name") {
                        println!("  {}. {}", i + 1, node_name.as_str().unwrap_or("Unknown"));
                    }
                }
            }

            if let Some(socs) = board.get("socs").and_then(|s| s.as_array()) {
                println!("\nSoCs ({}):", socs.len());
                for (i, soc) in socs.iter().enumerate() {
                    if let Some(soc_id) = soc.get("soc_id") {
                        println!("  {}. {}", i + 1, soc_id.as_str().unwrap_or("Unknown"));
                    }
                }
            }

            print_success("Board information retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch board {}: {}", board_id, e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get boards list in raw JSON format
async fn list_boards_raw(client: &SettingsClient) -> Result<()> {
    print_info("Fetching raw boards data...");

    match client.get("/api/v1/boards").await {
        Ok(boards) => {
            print_json(&boards)?;
            print_success("Raw boards data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch raw boards data: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get specific board in raw JSON format
async fn get_board_raw(client: &SettingsClient, board_id: &str) -> Result<()> {
    print_info(&format!("Fetching raw board data for ID: {}", board_id));

    let endpoint = format!("/api/v1/boards/{}", board_id);
    match client.get(&endpoint).await {
        Ok(board) => {
            print_json(&board)?;
            print_success("Raw board data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!(
                "Failed to fetch raw board data for {}: {}",
                board_id, e
            ));
            return Err(e);
        }
    }

    Ok(())
}
