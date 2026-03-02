//! Runtime bridges module
//!
//! This module provides bridges to external systems for the ServantGuild,
//! enabling integration with GitHub and other services.

pub mod github;
pub mod llm;
pub mod tools;
pub mod safety;
pub mod memory;
pub mod consensus;

pub use github::GitHubBridge;
