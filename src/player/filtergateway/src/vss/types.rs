/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! VSS (Vehicle Signal Specification) type definitions
//!
//! This module defines the core types used for VSS integration with Kuksa.val Databroker.

use std::time::SystemTime;

/// VSS signal change trigger
///
/// Represents a change notification for a specific VSS signal path.
#[derive(Debug, Clone)]
pub struct VssTrigger {
    /// VSS path (e.g., "Vehicle.Chassis.ParkingBrake.IsEngaged")
    pub path: String,
    /// Changed value
    pub value: VssValue,
    /// Timestamp of the change
    pub timestamp: SystemTime,
}

/// VSS value types
///
/// Represents the various data types supported by VSS signals.
#[derive(Debug, Clone, PartialEq)]
pub enum VssValue {
    /// Boolean value
    Bool(bool),
    /// 32-bit signed integer
    Int32(i32),
    /// 64-bit signed integer
    Int64(i64),
    /// 32-bit floating point
    Float(f32),
    /// 64-bit floating point
    Double(f64),
    /// String value
    String(String),
    /// Unknown or unsupported type
    Unknown,
}

impl VssValue {
    /// Convert to string representation for scenario condition evaluation
    ///
    /// # Returns
    ///
    /// String representation of the value
    pub fn to_string_value(&self) -> String {
        match self {
            VssValue::Bool(v) => v.to_string(),
            VssValue::Int32(v) => v.to_string(),
            VssValue::Int64(v) => v.to_string(),
            VssValue::Float(v) => v.to_string(),
            VssValue::Double(v) => v.to_string(),
            VssValue::String(v) => v.clone(),
            VssValue::Unknown => "unknown".to_string(),
        }
    }

    /// Try to convert to boolean value
    ///
    /// # Returns
    ///
    /// `Some(bool)` if conversion is possible, `None` otherwise
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            VssValue::Bool(v) => Some(*v),
            VssValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

/// VSS-related error types
#[derive(Debug, thiserror::Error)]
pub enum VssError {
    /// Connection error to Kuksa.val Databroker
    #[error("Connection error: {0}")]
    Connection(String),
    /// Subscription error
    #[error("Subscribe error: {0}")]
    Subscribe(String),
    /// Invalid URI format
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vss_value_to_string() {
        assert_eq!(VssValue::Bool(true).to_string_value(), "true");
        assert_eq!(VssValue::Int32(42).to_string_value(), "42");
        assert_eq!(VssValue::Int64(1000).to_string_value(), "1000");
        assert_eq!(VssValue::Float(3.14).to_string_value(), "3.14");
        assert_eq!(VssValue::Double(2.71828).to_string_value(), "2.71828");
        assert_eq!(
            VssValue::String("test".to_string()).to_string_value(),
            "test"
        );
        assert_eq!(VssValue::Unknown.to_string_value(), "unknown");
    }

    #[test]
    fn test_vss_value_as_bool() {
        assert_eq!(VssValue::Bool(true).as_bool(), Some(true));
        assert_eq!(VssValue::Bool(false).as_bool(), Some(false));
        assert_eq!(
            VssValue::String("true".to_string()).as_bool(),
            Some(true)
        );
        assert_eq!(
            VssValue::String("false".to_string()).as_bool(),
            Some(false)
        );
        assert_eq!(VssValue::Int32(1).as_bool(), None);
    }

    #[test]
    fn test_vss_trigger_creation() {
        let trigger = VssTrigger {
            path: "Vehicle.Speed".to_string(),
            value: VssValue::Float(60.5),
            timestamp: SystemTime::now(),
        };

        assert_eq!(trigger.path, "Vehicle.Speed");
        assert_eq!(trigger.value, VssValue::Float(60.5));
    }
}
