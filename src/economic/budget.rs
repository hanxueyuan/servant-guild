//! Budget Management Module
//!
//! Tracks spending against budget limits and provides throttling

use crate::economic::*;
use chrono::{DateTime, Datelike, Timelike, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Budget manager - tracks and enforces budget limits
pub struct BudgetManager {
    config: BudgetConfig,
    state: Arc<RwLock<BudgetState>>,
}

/// Internal budget state
struct BudgetState {
    /// Spending by hour (hour -> amount)
    hourly_spending: HashMap<u32, f64>,
    /// Spending by day (date string -> amount)
    daily_spending: HashMap<String, f64>,
    /// Spending by agent
    agent_spending: HashMap<String, f64>,
    /// Spending by task
    task_spending: HashMap<uuid::Uuid, f64>,
    /// Current hour
    current_hour: u32,
    /// Current day
    current_day: String,
    /// Throttle multiplier (1.0 = no throttle)
    throttle_multiplier: f64,
    /// Last reset time
    last_reset: DateTime<Utc>,
}

impl BudgetState {
    fn new() -> Self {
        let now = Utc::now();
        Self {
            hourly_spending: HashMap::new(),
            daily_spending: HashMap::new(),
            agent_spending: HashMap::new(),
            task_spending: HashMap::new(),
            current_hour: now.hour(),
            current_day: now.format("%Y-%m-%d").to_string(),
            throttle_multiplier: 1.0,
            last_reset: now,
        }
    }

    /// Get current hour spending
    fn hourly_spent(&self) -> f64 {
        self.hourly_spending
            .get(&self.current_hour)
            .copied()
            .unwrap_or(0.0)
    }

    /// Get current day spending
    fn daily_spent(&self) -> f64 {
        self.daily_spending
            .get(&self.current_day)
            .copied()
            .unwrap_or(0.0)
    }

    /// Get agent spending
    fn agent_spent(&self, agent: &str) -> f64 {
        self.agent_spending.get(agent).copied().unwrap_or(0.0)
    }
}

impl BudgetManager {
    /// Create new budget manager
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(BudgetState::new())),
        }
    }

    /// Check if spending is allowed
    pub async fn can_spend(&self, amount: f64, agent: &str) -> bool {
        let state = self.state.read().await;

        // Check daily limit
        if state.daily_spent() + amount > self.config.daily_limit_usd {
            return false;
        }

        // Check hourly limit
        if state.hourly_spent() + amount > self.config.hourly_limit_usd {
            return false;
        }

        // Check per-agent limit
        if state.agent_spent(agent) + amount > self.config.per_agent_limit_usd {
            return false;
        }

        true
    }

    /// Record spending
    pub async fn spend(&self, amount: f64, agent: &str) {
        let mut state = self.state.write().await;

        // Check for day/hour rollover
        let now = Utc::now();
        if now.hour() != state.current_hour {
            state.current_hour = now.hour();
        }

        let today = now.format("%Y-%m-%d").to_string();
        if today != state.current_day {
            state.current_day = today;
            // Reset daily counters
            state.agent_spending.clear();
            state.task_spending.clear();
        }

        // Update counters
        let current_hour = state.current_hour;
        let current_day = state.current_day.clone();
        *state
            .hourly_spending
            .entry(current_hour)
            .or_insert(0.0) += amount;
        *state
            .daily_spending
            .entry(current_day)
            .or_insert(0.0) += amount;
        *state.agent_spending.entry(agent.to_string()).or_insert(0.0) += amount;

        // Check if throttling needed
        if self.config.auto_throttle {
            let daily_ratio = state.daily_spent() / self.config.daily_limit_usd;
            if daily_ratio > self.config.critical_threshold {
                state.throttle_multiplier = self.config.throttle_factor;
            } else if daily_ratio < self.config.warning_threshold {
                state.throttle_multiplier = 1.0;
            }
        }
    }

    /// Record task spending
    pub async fn spend_on_task(&self, amount: f64, agent: &str, task_id: uuid::Uuid) -> bool {
        // Check per-task limit
        {
            let state = self.state.read().await;
            let task_spent = state.task_spending.get(&task_id).copied().unwrap_or(0.0);
            if task_spent + amount > self.config.per_task_limit_usd {
                return false;
            }
        }

        // Record spending
        self.spend(amount, agent).await;

        // Update task spending
        {
            let mut state = self.state.write().await;
            *state.task_spending.entry(task_id).or_insert(0.0) += amount;
        }

        true
    }

    /// Get current budget status
    pub async fn status(&self) -> BudgetStatus {
        let state = self.state.read().await;
        let daily_spent = state.daily_spent();
        let hourly_spent = state.hourly_spent();
        let daily_percentage = (daily_spent / self.config.daily_limit_usd) * 100.0;

        BudgetStatus {
            daily_spent,
            daily_limit: self.config.daily_limit_usd,
            daily_percentage,
            hourly_spent,
            hourly_limit: self.config.hourly_limit_usd,
            exceeded: daily_spent > self.config.daily_limit_usd,
            warning: daily_percentage > self.config.warning_threshold * 100.0,
            throttle_factor: state.throttle_multiplier,
        }
    }

    /// Get throttle multiplier (1.0 = no throttle, lower = more throttled)
    pub async fn throttle_multiplier(&self) -> f64 {
        let state = self.state.read().await;
        state.throttle_multiplier
    }

    /// Reset budget counters (for testing)
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = BudgetState::new();
    }

    /// Update budget configuration
    pub fn update_config(&mut self, config: BudgetConfig) {
        self.config = config;
    }
}

/// Budget alert types
#[derive(Debug, Clone)]
pub enum BudgetAlert {
    /// Warning threshold reached
    Warning {
        percentage: f64,
        spent: f64,
        limit: f64,
    },
    /// Critical threshold reached
    Critical {
        percentage: f64,
        spent: f64,
        limit: f64,
    },
    /// Budget exceeded
    Exceeded { spent: f64, limit: f64 },
    /// Hourly limit exceeded
    HourlyExceeded { spent: f64, limit: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_can_spend_within_limits() {
        let config = BudgetConfig {
            daily_limit_usd: 100.0,
            hourly_limit_usd: 20.0,
            per_agent_limit_usd: 50.0,
            ..Default::default()
        };
        let manager = BudgetManager::new(config);

        assert!(manager.can_spend(10.0, "agent1").await);
    }

    #[tokio::test]
    async fn test_cannot_exceed_daily_limit() {
        let config = BudgetConfig {
            daily_limit_usd: 10.0,
            ..Default::default()
        };
        let manager = BudgetManager::new(config);

        manager.spend(10.0, "agent1").await;
        assert!(!manager.can_spend(1.0, "agent1").await);
    }

    #[tokio::test]
    async fn test_budget_status() {
        let manager = BudgetManager::new(BudgetConfig::default());
        manager.spend(10.0, "agent1").await;

        let status = manager.status().await;
        assert_eq!(status.daily_spent, 10.0);
        assert!(!status.exceeded);
    }
}
