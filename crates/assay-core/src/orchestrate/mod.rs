//! Session dependency orchestration.
//!
//! This module provides DAG construction and validation for multi-session
//! manifests where sessions declare `depends_on` relationships. It is gated
//! behind the `orchestrate` Cargo feature.

pub mod dag;
