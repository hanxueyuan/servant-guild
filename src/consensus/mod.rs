//! Consensus Engine - Collective Decision Making for ServantGuild
//!
//! This module implements the voting and governance mechanism for multi-agent
//! collaboration. Key concepts:
//!
//! - **Constitution**: Rules defining what actions require voting
//! - **Proposal**: A request for collective decision
//! - **Vote**: Individual agent's vote on a proposal
//! - **Quorum**: Minimum votes needed for a decision
//!
//! ## Voting Rules
//!
//! - Core Servants: 5 members, quorum = 3 (simple majority)
//! - Critical decisions: Require unanimous approval (5/5)
//! - Owner has veto power on any proposal

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use anyhow::{bail, Result};

pub mod constitution;
pub mod engine;
pub mod proposal;
pub mod vote;

// Re-exports
pub use constitution::{Constitution, DecisionType, GovernanceRule};
pub use engine::ConsensusEngine;
pub use proposal::{Proposal, ProposalStatus, VoteRecord};
pub use vote::Vote;

/// Result of a consensus check
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsensusResult {
    /// Proposal passed, execution allowed
    Passed,
    /// Proposal rejected, execution denied
    Rejected,
    /// Voting in progress, waiting for more votes
    Pending,
    /// Proposal expired without reaching quorum
    Expired,
    /// Owner used veto power
    Vetoed,
}

/// Information about a completed vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteTally {
    pub proposal_id: String,
    pub total_votes: usize,
    pub yes_votes: usize,
    pub no_votes: usize,
    pub abstain_votes: usize,
    pub required_quorum: usize,
    pub result: ConsensusResult,
}

/// Configuration for the consensus engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Number of core servants (should be odd for deadlock prevention)
    pub core_servants_count: usize,
    /// Quorum threshold for normal decisions
    pub normal_quorum: usize,
    /// Quorum threshold for critical decisions (usually unanimous)
    pub critical_quorum: usize,
    /// Voting timeout in seconds (0 = no timeout)
    pub voting_timeout_secs: u64,
    /// Enable owner veto capability
    pub owner_veto_enabled: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            core_servants_count: 5,
            normal_quorum: 3,
            critical_quorum: 5,
            voting_timeout_secs: 3600, // 1 hour
            owner_veto_enabled: true,
        }
    }
}
