use crate::consensus::ConsensusEngine;
use crate::memory::Memory;
use crate::providers::traits::Provider;
use crate::safety::audit::AuditLogger;
use crate::safety::TransactionManager;
use crate::tools::traits::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiView};

pub struct HostState {
    pub wasi: WasiCtx,
    pub table: ResourceTable,
    pub servant_id: String,
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<HashMap<String, Arc<dyn Tool>>>,
    pub audit_logger: Arc<AuditLogger>,
    pub consensus_engine: Option<Arc<ConsensusEngine>>,
    pub memory: Option<Arc<dyn Memory>>,
    pub rollback_manager: Option<Arc<TransactionManager>>,
}

impl HostState {
    pub fn new(
        wasi: WasiCtx,
        table: ResourceTable,
        servant_id: String,
        provider: Arc<dyn Provider>,
        tools: Arc<HashMap<String, Arc<dyn Tool>>>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        Self {
            wasi,
            table,
            servant_id,
            provider,
            tools,
            audit_logger,
            consensus_engine: None,
            memory: None,
            rollback_manager: None,
        }
    }

    /// Set the consensus engine
    pub fn with_consensus_engine(mut self, engine: Arc<ConsensusEngine>) -> Self {
        self.consensus_engine = Some(engine);
        self
    }

    /// Set the memory backend
    pub fn with_memory(mut self, memory: Arc<dyn Memory>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Set the rollback manager
    pub fn with_rollback_manager(mut self, manager: Arc<TransactionManager>) -> Self {
        self.rollback_manager = Some(manager);
        self
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
