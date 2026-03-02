//! Tools module
//!
//! This module provides the tools available to the agent.

pub mod agents_ipc;
pub mod apply_patch;
pub mod browser;
pub mod browser_open;
pub mod build;
pub mod cli_discovery;
pub mod composio;
pub mod content_search;
pub mod cron_add;
pub mod cron_list;
pub mod cron_remove;
pub mod cron_run;
pub mod cron_runs;
pub mod cron_update;
pub mod delegate;
pub mod delegate_coordination_status;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod git_operations;
pub mod glob_search;
pub mod hardware_board_info;
pub mod hardware_memory_map;
pub mod hardware_memory_read;
pub mod http_request;
pub mod image_info;
pub mod memory_forget;
pub mod memory_recall;
pub mod memory_store;
pub mod model_routing_config;
pub mod pdf_read;
pub mod process;
pub mod proxy_config;
pub mod pushover;
pub mod schedule;
pub mod schema;
pub mod screenshot;
pub mod shell;
#[cfg(feature = "sop")]
pub mod sop_advance;
#[cfg(feature = "sop")]
pub mod sop_approve;
#[cfg(feature = "sop")]
pub mod sop_execute;
#[cfg(feature = "sop")]
pub mod sop_list;
#[cfg(feature = "sop")]
pub mod sop_status;
pub mod subagent_list;
pub mod subagent_manage;
pub mod subagent_registry;
pub mod subagent_spawn;
pub mod task_plan;
pub mod traits;
pub mod url_validation;
pub mod wasm_module;
pub mod web_fetch;
pub mod web_search_tool;

pub use build::{BuildBuilder, BuildConfig, BuildResult, BuildTools};
pub use traits::{Tool, ToolResult, ToolSpec};

use crate::config::{BrowserConfig, Config, DelegateAgentConfig, HttpRequestConfig, WebFetchConfig};
use crate::memory::Memory;
use crate::runtime::RuntimeAdapter;
use crate::security::SecurityPolicy;
use crate::tools::subagent_registry::SubAgentRegistry;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

struct ArcTool {
    inner: Arc<dyn Tool>,
}

#[async_trait]
impl Tool for ArcTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.inner.execute(args).await
    }
}

#[allow(clippy::too_many_arguments)]
pub fn all_tools_with_runtime(
    config: Arc<Config>,
    security: &Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    memory: Arc<dyn Memory>,
    composio_key: Option<&str>,
    composio_entity_id: Option<&str>,
    browser: &BrowserConfig,
    http_request: &HttpRequestConfig,
    web_fetch: &WebFetchConfig,
    workspace_dir: &Path,
    agents: &HashMap<String, DelegateAgentConfig>,
    api_key: Option<&str>,
    full_config: &Config,
) -> Vec<Box<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

    tools.push(Arc::new(file_read::FileReadTool::new(Arc::clone(security))));
    tools.push(Arc::new(file_write::FileWriteTool::new(Arc::clone(security))));
    tools.push(Arc::new(file_edit::FileEditTool::new(Arc::clone(security))));
    tools.push(Arc::new(glob_search::GlobSearchTool::new(Arc::clone(security))));
    tools.push(Arc::new(content_search::ContentSearchTool::new(Arc::clone(security))));
    tools.push(Arc::new(shell::ShellTool::new(
        Arc::clone(security),
        Arc::clone(&runtime),
    )));
    tools.push(Arc::new(process::ProcessTool::new(
        Arc::clone(security),
        Arc::clone(&runtime),
    )));

    tools.push(Arc::new(memory_recall::MemoryRecallTool::new(Arc::clone(
        &memory,
    ))));
    tools.push(Arc::new(memory_store::MemoryStoreTool::new(
        Arc::clone(&memory),
        Arc::clone(security),
    )));
    tools.push(Arc::new(memory_forget::MemoryForgetTool::new(
        Arc::clone(&memory),
        Arc::clone(security),
    )));

    tools.push(Arc::new(git_operations::GitOperationsTool::new(
        Arc::clone(security),
        workspace_dir.to_path_buf(),
    )));

    tools.push(Arc::new(proxy_config::ProxyConfigTool::new(
        Arc::clone(&config),
        Arc::clone(security),
    )));
    tools.push(Arc::new(model_routing_config::ModelRoutingConfigTool::new(
        Arc::clone(&config),
        Arc::clone(security),
    )));

    if full_config.cron.enabled {
        tools.push(Arc::new(cron_list::CronListTool::new(Arc::clone(&config))));
        tools.push(Arc::new(cron_add::CronAddTool::new(
            Arc::clone(&config),
            Arc::clone(security),
        )));
        tools.push(Arc::new(cron_remove::CronRemoveTool::new(
            Arc::clone(&config),
            Arc::clone(security),
        )));
        tools.push(Arc::new(cron_update::CronUpdateTool::new(
            Arc::clone(&config),
            Arc::clone(security),
        )));
        tools.push(Arc::new(cron_runs::CronRunsTool::new(Arc::clone(&config))));
        tools.push(Arc::new(cron_run::CronRunTool::new(
            Arc::clone(&config),
            Arc::clone(security),
        )));
    }

    if browser.enabled {
        tools.push(Arc::new(browser_open::BrowserOpenTool::new(
            Arc::clone(security),
            browser.allowed_domains.clone(),
        )));
        tools.push(Arc::new(browser::BrowserTool::new_with_backend(
            Arc::clone(security),
            browser.allowed_domains.clone(),
            browser.session_name.clone(),
            browser.backend.clone(),
            browser.native_headless,
            browser.native_webdriver_url.clone(),
            browser.native_chrome_path.clone(),
            crate::tools::browser::ComputerUseConfig {
                endpoint: browser.computer_use.endpoint.clone(),
                api_key: browser.computer_use.api_key.clone(),
                timeout_ms: browser.computer_use.timeout_ms,
                allow_remote_endpoint: browser.computer_use.allow_remote_endpoint,
                window_allowlist: browser.computer_use.window_allowlist.clone(),
                max_coordinate_x: browser.computer_use.max_coordinate_x,
                max_coordinate_y: browser.computer_use.max_coordinate_y,
            },
        )));
    }

    if http_request.enabled {
        tools.push(Arc::new(http_request::HttpRequestTool::new(
            Arc::clone(security),
            http_request.allowed_domains.clone(),
            http_request.max_response_size,
            http_request.timeout_secs,
        )));
    }

    if web_fetch.enabled {
        tools.push(Arc::new(web_fetch::WebFetchTool::new(
            Arc::clone(security),
            web_fetch.allowed_domains.clone(),
            web_fetch.blocked_domains.clone(),
            web_fetch.max_response_size,
            web_fetch.timeout_secs,
        )));
    }

    if full_config.web_search.enabled {
        let user_agent = format!("zeroclaw/{}", env!("CARGO_PKG_VERSION"));
        tools.push(Arc::new(web_search_tool::WebSearchTool::new(
            Arc::clone(security),
            full_config.web_search.provider.clone(),
            full_config.web_search.brave_api_key.clone(),
            None,
            full_config.web_search.max_results,
            full_config.web_search.timeout_secs,
            user_agent,
        )));
    }

    tools.push(Arc::new(delegate::DelegateTool::new_with_options(
        agents.clone(),
        api_key.map(|s| s.to_string()),
        Arc::clone(security),
        crate::providers::ProviderRuntimeOptions {
            auth_profile_override: None,
            provider_api_url: full_config.api_url.clone(),
            zeroclaw_dir: full_config.config_path.parent().map(std::path::PathBuf::from),
            secrets_encrypt: full_config.secrets.encrypt,
            reasoning_enabled: full_config.runtime.reasoning_enabled,
        },
    )));

    if let Some(key) = composio_key {
        tools.push(Arc::new(composio::ComposioTool::new(
            key,
            composio_entity_id,
            Arc::clone(security),
        )));
    }

    let subagent_registry = Arc::new(SubAgentRegistry::new());
    tools.push(Arc::new(subagent_list::SubAgentListTool::new(Arc::clone(
        &subagent_registry,
    ))));
    tools.push(Arc::new(subagent_manage::SubAgentManageTool::new(
        Arc::clone(&subagent_registry),
        Arc::clone(security),
    )));

    let parent_tools = Arc::new(tools.clone());
    tools.push(Arc::new(subagent_spawn::SubAgentSpawnTool::new(
        agents.clone(),
        api_key.map(|s| s.to_string()),
        Arc::clone(security),
        crate::providers::ProviderRuntimeOptions {
            auth_profile_override: None,
            provider_api_url: full_config.api_url.clone(),
            zeroclaw_dir: full_config.config_path.parent().map(std::path::PathBuf::from),
            secrets_encrypt: full_config.secrets.encrypt,
            reasoning_enabled: full_config.runtime.reasoning_enabled,
        },
        Arc::clone(&subagent_registry),
        parent_tools,
        full_config.multimodal.clone(),
    )));

    tools
        .into_iter()
        .map(|inner| Box::new(ArcTool { inner }) as Box<dyn Tool>)
        .collect()
}

pub fn default_tools(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> {
    let runtime: Arc<dyn RuntimeAdapter> = Arc::new(crate::runtime::NativeRuntime::new());
    vec![
        Box::new(file_read::FileReadTool::new(Arc::clone(&security))),
        Box::new(file_write::FileWriteTool::new(Arc::clone(&security))),
        Box::new(shell::ShellTool::new(Arc::clone(&security), runtime)),
    ]
}
