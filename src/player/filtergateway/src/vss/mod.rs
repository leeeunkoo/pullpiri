/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! VSS (Vehicle Signal Specification) integration module
//!
//! This module provides integration with Kuksa.val Databroker for subscribing to
//! and receiving vehicle signal updates via gRPC.
//!
//! # Feature Flag
//!
//! This module is only compiled when the `vss` feature is enabled:
//!
//! ```toml
//! [dependencies.filtergateway]
//! features = ["vss"]
//! ```
//!
//! # Environment Variables
//!
//! - `KUKSA_DATABROKER_URI`: URI of the Kuksa.val Databroker (e.g., "http://databroker:55556")
//!   - If not set, VSS integration will be disabled at runtime
//!
//! # Reference
//!
//! Implementation patterns follow:
//! - fms-forwarder/src/vehicle_abstraction.rs
//! - fms-consumer usage patterns

#[cfg(feature = "vss")]
mod kuksa_subscriber;
#[cfg(feature = "vss")]
mod types;

#[cfg(feature = "vss")]
pub use kuksa_subscriber::VssSubscriber;
#[cfg(feature = "vss")]
pub use types::{VssError, VssTrigger, VssValue};
