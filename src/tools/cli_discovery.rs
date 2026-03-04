//! CLI Discovery Module
//!
//! Provides utilities for discovering and managing CLI tools
//! that may be available in the system environment.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Information about a discovered CLI tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTool {
    /// Tool name
    pub name: String,
    /// Path to the executable
    pub path: PathBuf,
    /// Version string (if available)
    pub version: Option<String>,
    /// Whether the tool is available
    pub available: bool,
}

/// CLI tool discovery manager
pub struct CliDiscovery {
    /// Cache of discovered tools
    cache: HashMap<String, DiscoveredTool>,
}

impl CliDiscovery {
    /// Create a new CLI discovery manager
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Discover a tool by name
    pub fn discover(&mut self, tool_name: &str) -> Result<DiscoveredTool> {
        // Check cache first
        if let Some(cached) = self.cache.get(tool_name) {
            return Ok(cached.clone());
        }

        // Try to find the tool in PATH
        let result = which::which(tool_name);
        
        let tool = match result {
            Ok(path) => {
                let version = self.get_version(tool_name, &path).ok();
                DiscoveredTool {
                    name: tool_name.to_string(),
                    path,
                    version,
                    available: true,
                }
            }
            Err(_) => {
                DiscoveredTool {
                    name: tool_name.to_string(),
                    path: PathBuf::new(),
                    version: None,
                    available: false,
                }
            }
        };

        self.cache.insert(tool_name.to_string(), tool.clone());
        Ok(tool)
    }

    /// Get version of a tool
    fn get_version(&self, tool_name: &str, _path: &PathBuf) -> Result<String> {
        // Common version flags
        let version_flags = match tool_name {
            "node" => vec!["--version"],
            "npm" => vec!["--version"],
            "python" => vec!["--version"],
            "python3" => vec!["--version"],
            "rustc" => vec!["--version"],
            "cargo" => vec!["--version"],
            "git" => vec!["--version"],
            "docker" => vec!["--version"],
            _ => vec!["--version", "-v", "-V"],
        };

        // Try to get version
        for flag in version_flags {
            let output = std::process::Command::new(tool_name)
                .arg(flag)
                .output()
                .ok();

            if let Some(output) = output {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    return Ok(version.trim().to_string());
                }
            }
        }

        anyhow::bail!("Could not determine version for {}", tool_name)
    }

    /// Check if a tool is available
    pub fn is_available(&mut self, tool_name: &str) -> bool {
        self.discover(tool_name)
            .map(|t| t.available)
            .unwrap_or(false)
    }

    /// Get path to a tool
    pub fn get_path(&mut self, tool_name: &str) -> Option<PathBuf> {
        self.discover(tool_name)
            .ok()
            .filter(|t| t.available)
            .map(|t| t.path)
    }

    /// Discover multiple tools at once
    pub fn discover_tools(&mut self, tool_names: &[&str]) -> HashMap<String, DiscoveredTool> {
        let mut results = HashMap::new();
        for name in tool_names {
            if let Ok(tool) = self.discover(name) {
                results.insert(name.to_string(), tool);
            }
        }
        results
    }

    /// Get all cached tools
    pub fn cached_tools(&self) -> &HashMap<String, DiscoveredTool> {
        &self.cache
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for CliDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Discover CLI tools (convenience function)
pub fn discover_cli_tools(tools: &[&str]) -> HashMap<String, DiscoveredTool> {
    let mut discovery = CliDiscovery::new();
    discovery.discover_tools(tools)
}

/// Check if a CLI tool is available in PATH
pub fn is_tool_available(tool_name: &str) -> bool {
    which::which(tool_name).is_ok()
}

/// Get the path to a CLI tool
pub fn get_tool_path(tool_name: &str) -> Option<PathBuf> {
    which::which(tool_name).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_creation() {
        let discovery = CliDiscovery::new();
        assert!(discovery.cache.is_empty());
    }

    #[test]
    fn test_discover_tool() {
        let mut discovery = CliDiscovery::new();
        
        // Test with a tool that should be available (sh on Unix)
        #[cfg(unix)]
        {
            let result = discovery.discover("sh");
            assert!(result.is_ok());
            let tool = result.unwrap();
            assert_eq!(tool.name, "sh");
            assert!(tool.available);
        }
    }

    #[test]
    fn test_discover_cli_tools() {
        let tools = discover_cli_tools(&["sh", "bash"]);
        #[cfg(unix)]
        assert!(tools.contains_key("sh"));
    }
}
