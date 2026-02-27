use wasmtime_wasi::preview2::{Table, WasiCtx, WasiView};

pub struct HostState {
    pub wasi: WasiCtx,
    pub table: Table,
    // Future expansion:
    // pub llm_provider: Arc<dyn Provider>,
    // pub tool_registry: Arc<dyn ToolRegistry>,
    // pub audit_logger: Arc<dyn AuditLogger>,
}

impl HostState {
    pub fn new(wasi: WasiCtx, table: Table) -> Self {
        Self { wasi, table }
    }
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut Table {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}
