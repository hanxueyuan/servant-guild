//! Tools module
//!
//! This module provides build and compilation tools for ServantGuild,
//! enabling automated Wasm compilation and release generation.

pub mod build;
pub mod cli_discovery;
pub mod traits;

use std::collections::HashMap;
use std::sync::Arc;

pub use build::{BuildBuilder, BuildConfig, BuildResult, BuildTools};
pub use cli_discovery::{discover_cli_tools, CliDiscovery, DiscoveredTool};
pub use traits::{Tool, ToolResult, ToolSpec};

use crate::config::schema::{Config, DelegateAgentConfig, BrowserConfig, HttpRequestConfig, WebFetchConfig};
use crate::memory::Memory;
use crate::runtime::traits::RuntimeAdapter;
use crate::safety::policy::SafetyPolicy;

/// Get all tools with runtime adapter and configuration
#[allow(clippy::too_many_arguments)]
pub fn all_tools_with_runtime(
    _config: Arc<Config>,
    _security: &SafetyPolicy,
    _runtime: Arc<dyn RuntimeAdapter>,
    _memory: Arc<dyn Memory>,
    _composio_key: Option<&str>,
    _composio_entity_id: Option<&str>,
    _browser_config: &BrowserConfig,
    _http_config: &HttpRequestConfig,
    _web_fetch_config: &WebFetchConfig,
    _workspace_dir: &std::path::Path,
    _agents_config: &HashMap<String, DelegateAgentConfig>,
    _api_key: Option<&str>,
    _full_config: &Config,
) -> Vec<Box<dyn Tool>> {
    // Return a list of available tools based on the runtime
    // For now, return an empty vector as the actual tools are implemented elsewhere
    Vec::new()
}

/// Get all available tools
pub fn all_tools() -> Vec<Box<dyn Tool>> {
    // Return a list of all available tools
    Vec::new()
}
