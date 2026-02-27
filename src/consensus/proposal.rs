use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub proposer: String,
    pub created_at: i64,
    pub status: ProposalStatus,
    pub votes: HashMap<String, VoteRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRecord {
    pub voter: String,
    pub vote: crate::consensus::vote::Vote,
    pub reason: String,
    pub timestamp: i64,
}

impl Proposal {
    pub fn new(id: String, title: String, description: String, proposer: String) -> Self {
        Self {
            id,
            title,
            description,
            proposer,
            created_at: chrono::Utc::now().timestamp(),
            status: ProposalStatus::Active,
            votes: HashMap::new(),
        }
    }
}
