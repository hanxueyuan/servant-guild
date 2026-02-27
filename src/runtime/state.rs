use wasmtime_wasi::{WasiCtx, WasiView};
use wasmtime::component::ResourceTable;
use std::sync::Arc;
use crate::providers::traits::Provider;
use crate::tools::traits::Tool;
use crate::safety::audit::AuditLogger;
use std::collections::HashMap;

pub struct HostState {
    pub wasi: WasiCtx,
    pub table: ResourceTable,
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<HashMap<String, Arc<dyn Tool>>>,
    pub audit_logger: Arc<AuditLogger>,
}

impl HostState {
    pub fn new(
        wasi: WasiCtx, 
        table: ResourceTable, 
        provider: Arc<dyn Provider>,
        tools: Arc<HashMap<String, Arc<dyn Tool>>>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        Self { wasi, table, provider, tools, audit_logger }
    }
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}
