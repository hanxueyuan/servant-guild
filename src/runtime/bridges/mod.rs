//! Runtime bridges module
//!
//! This module provides bridges to external systems for the ServantGuild,
//! enabling integration with GitHub and other services.

pub mod github;

pub use github::GitHubBridge;
