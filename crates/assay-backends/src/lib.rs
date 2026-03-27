//! Backend factory for dispatching [`StateBackendConfig`] variants
//! to concrete [`StateBackend`] implementations.

pub mod factory;

#[cfg(feature = "linear")]
pub mod linear;
