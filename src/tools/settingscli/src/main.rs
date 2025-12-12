/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! SettingsCLI - Command Line Interface for Pullpiri SettingsService
//!
//! This CLI tool provides a convenient way to interact with the Pullpiri SettingsService
//! via REST APIs. It supports various operations for managing boards, nodes, and SoCs.

use clap::{Parser, Subcommand};
use colored::Colorize;
use settingscli::commands::{board, container, metrics, node, soc, yaml};
use settingscli::{Result, SettingsClient};

#[derive(Parser)]
#[command(name = "settingscli")]
#[command(about = "CLI tool for Pullpiri SettingsService")]
#[command(version)]
#[command(long_about = None)]
struct Cli {
    /// SettingsService URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    url: String,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get system metrics
    Metrics {
        #[command(subcommand)]
        action: metrics::MetricsAction,
    },
    /// Board-related operations
    Board {
        #[command(subcommand)]
        action: board::BoardAction,
    },
    /// Node-related operations
    Node {
        #[command(subcommand)]
        action: node::NodeAction,
    },
    /// SoC-related operations
    Soc {
        #[command(subcommand)]
        action: soc::SocAction,
    },
    /// Container-related operations
    Container {
        #[command(subcommand)]
        action: container::ContainerAction,
    },
    /// YAML artifact management
    Yaml {
        #[command(subcommand)]
        action: yaml::YamlAction,
    },
    /// Test connection to SettingsService
    Health,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        println!(
            "{} Connecting to SettingsService at: {}",
            "ℹ".blue().bold(),
            cli.url
        );
    }

    // Create client
    let client = match SettingsClient::new(&cli.url, cli.timeout) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("{} Failed to create client: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    };

    // Execute command
    let result = match cli.command {
        Commands::Metrics { action } => metrics::handle(&client, action).await,
        Commands::Board { action } => board::handle(&client, action).await,
        Commands::Node { action } => node::handle(&client, action).await,
        Commands::Soc { action } => soc::handle(&client, action).await,
        Commands::Container { action } => container::handle(&client, action).await,
        Commands::Yaml { action } => yaml::handle(&client, action).await,
        Commands::Health => health_check(&client).await,
    };

    match result {
        Ok(_) => {
            if cli.verbose {
                println!("{} Command completed successfully", "✓".green().bold());
            }
        }
        Err(e) => {
            eprintln!("{} Command failed: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Perform a health check on the SettingsService
async fn health_check(client: &SettingsClient) -> Result<()> {
    println!("{} Checking SettingsService health...", "ℹ".blue().bold());

    match client.health_check().await {
        Ok(true) => {
            println!(
                "{} SettingsService is healthy and reachable",
                "✓".green().bold()
            );
        }
        Ok(false) => {
            println!("{} SettingsService is not reachable", "✗".red().bold());
            return Err(settingscli::error::CliError::Custom(
                "Health check failed".to_string(),
            ));
        }
        Err(e) => {
            println!("{} Health check failed: {}", "✗".red().bold(), e);
            return Err(e);
        }
    }

    Ok(())
}
