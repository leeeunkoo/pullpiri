/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Command implementations for SettingsCLI

pub mod board;
pub mod container;
pub mod metrics;
pub mod node;
pub mod soc;
pub mod yaml;

use crate::Result;
use colored::Colorize;
use serde_json::Value;

/// Helper function to pretty print JSON output
pub fn print_json(value: &Value) -> Result<()> {
    let pretty = serde_json::to_string_pretty(value)?;
    println!("{}", pretty);
    Ok(())
}

/// Helper function to print success messages
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Helper function to print error messages
pub fn print_error(message: &str) {
    println!("{} {}", "✗".red().bold(), message);
}

/// Helper function to print info messages
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}
