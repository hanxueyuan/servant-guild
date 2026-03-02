//! Network Isolation Module
//!
//! Provides network segmentation and isolation for secure communication

use crate::security::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Network policy for isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Policy name
    pub name: String,
    /// Ingress rules
    pub ingress: Vec<IngressRule>,
    /// Egress rules
    pub egress: Vec<EgressRule>,
    /// Applied to agents/services
    pub selectors: Vec<String>,
    /// Policy priority
    pub priority: u32,
    /// Whether policy is enabled
    pub enabled: bool,
}

/// Ingress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    /// Source CIDR or agent
    pub from: String,
    /// Allowed ports
    pub ports: Vec<u16>,
    /// Protocol
    pub protocol: Protocol,
}

/// Egress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressRule {
    /// Destination CIDR or service
    pub to: String,
    /// Allowed ports
    pub ports: Vec<u16>,
    /// Protocol
    pub protocol: Protocol,
}

/// Network protocol
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Protocol {
    TCP,
    UDP,
    Any,
}

/// Network isolation manager
pub struct NetworkIsolation {
    policies: HashMap<String, NetworkPolicy>,
    connections: HashMap<String, HashSet<String>>,
}

impl NetworkIsolation {
    /// Create new network isolation manager
    pub fn new() -> Self {
        let mut policies = HashMap::new();

        // Default deny-all policy
        policies.insert(
            "deny-all".to_string(),
            NetworkPolicy {
                name: "deny-all".to_string(),
                ingress: vec![],
                egress: vec![],
                selectors: vec![],
                priority: 0,
                enabled: true,
            },
        );

        Self {
            policies,
            connections: HashMap::new(),
        }
    }

    /// Add network policy
    pub fn add_policy(&mut self, policy: NetworkPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    /// Remove network policy
    pub fn remove_policy(&mut self, name: &str) -> bool {
        self.policies.remove(name).is_some()
    }

    /// Check if connection is allowed
    pub fn is_connection_allowed(
        &self,
        source: &str,
        destination: &str,
        port: u16,
        protocol: Protocol,
    ) -> bool {
        // Get applicable policies for source
        let source_policies: Vec<_> = self
            .policies
            .values()
            .filter(|p| p.enabled && p.selectors.iter().any(|s| source.starts_with(s)))
            .collect();

        // Check egress from source
        let egress_allowed = source_policies.iter().any(|policy| {
            policy.egress.iter().any(|rule| {
                let dest_matches = rule.to == "*" || destination.starts_with(&rule.to);
                let port_matches = rule.ports.is_empty() || rule.ports.contains(&port);
                let protocol_matches =
                    matches!(rule.protocol, Protocol::Any) || rule.protocol == protocol;

                dest_matches && port_matches && protocol_matches
            })
        });

        if !egress_allowed {
            return false;
        }

        // Get applicable policies for destination
        let dest_policies: Vec<_> = self
            .policies
            .values()
            .filter(|p| p.enabled && p.selectors.iter().any(|s| destination.starts_with(s)))
            .collect();

        // Check ingress to destination
        let ingress_allowed = dest_policies.iter().any(|policy| {
            policy.ingress.iter().any(|rule| {
                let source_matches = rule.from == "*" || source.starts_with(&rule.from);
                let port_matches = rule.ports.is_empty() || rule.ports.contains(&port);
                let protocol_matches =
                    matches!(rule.protocol, Protocol::Any) || rule.protocol == protocol;

                source_matches && port_matches && protocol_matches
            })
        });

        ingress_allowed
    }

    /// Get allowed destinations for source
    pub fn allowed_destinations(&self, source: &str) -> Vec<String> {
        let mut destinations = Vec::new();

        for policy in self.policies.values() {
            if policy.enabled && policy.selectors.iter().any(|s| source.starts_with(s)) {
                for rule in &policy.egress {
                    if rule.to != "*" {
                        destinations.push(rule.to.clone());
                    }
                }
            }
        }

        destinations.sort();
        destinations.dedup();
        destinations
    }

    /// Record active connection
    pub fn record_connection(&mut self, source: &str, destination: &str) {
        self.connections
            .entry(source.to_string())
            .or_default()
            .insert(destination.to_string());
    }

    /// Get active connections for source
    pub fn get_connections(&self, source: &str) -> Option<&HashSet<String>> {
        self.connections.get(source)
    }

    /// Clear all connections
    pub fn clear_connections(&mut self) {
        self.connections.clear();
    }
}

impl Default for NetworkIsolation {
    fn default() -> Self {
        Self::new()
    }
}

/// Network zone for segmentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkZone {
    /// Zone name
    pub name: String,
    /// CIDR ranges in zone
    pub cidrs: Vec<String>,
    /// Allowed outbound zones
    pub allowed_outbound: Vec<String>,
    /// Allowed inbound zones
    pub allowed_inbound: Vec<String>,
}

impl NetworkZone {
    /// Create new network zone
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            cidrs: vec![],
            allowed_outbound: vec![],
            allowed_inbound: vec![],
        }
    }

    /// Add CIDR range
    pub fn add_cidr(&mut self, cidr: &str) {
        self.cidrs.push(cidr.to_string());
    }

    /// Allow outbound to zone
    pub fn allow_outbound(&mut self, zone: &str) {
        if !self.allowed_outbound.contains(&zone.to_string()) {
            self.allowed_outbound.push(zone.to_string());
        }
    }

    /// Allow inbound from zone
    pub fn allow_inbound(&mut self, zone: &str) {
        if !self.allowed_inbound.contains(&zone.to_string()) {
            self.allowed_inbound.push(zone.to_string());
        }
    }
}

/// Create default network policies for ServantGuild
pub fn create_default_policies() -> Vec<NetworkPolicy> {
    vec![
        // Allow internal communication between agents
        NetworkPolicy {
            name: "agent-internal".to_string(),
            ingress: vec![IngressRule {
                from: "agent:".to_string(),
                ports: vec![8080, 9090],
                protocol: Protocol::TCP,
            }],
            egress: vec![EgressRule {
                to: "agent:".to_string(),
                ports: vec![8080, 9090],
                protocol: Protocol::TCP,
            }],
            selectors: vec!["agent:".to_string()],
            priority: 100,
            enabled: true,
        },
        // Allow database access from agents
        NetworkPolicy {
            name: "agent-database".to_string(),
            ingress: vec![],
            egress: vec![EgressRule {
                to: "database:".to_string(),
                ports: vec![5432],
                protocol: Protocol::TCP,
            }],
            selectors: vec!["agent:".to_string()],
            priority: 90,
            enabled: true,
        },
        // Allow Redis access from agents
        NetworkPolicy {
            name: "agent-redis".to_string(),
            ingress: vec![],
            egress: vec![EgressRule {
                to: "redis:".to_string(),
                ports: vec![6379],
                protocol: Protocol::TCP,
            }],
            selectors: vec!["agent:".to_string()],
            priority: 90,
            enabled: true,
        },
        // Allow external API calls from agents
        NetworkPolicy {
            name: "agent-external-api".to_string(),
            ingress: vec![],
            egress: vec![EgressRule {
                to: "0.0.0.0/0".to_string(),
                ports: vec![443],
                protocol: Protocol::TCP,
            }],
            selectors: vec!["agent:".to_string()],
            priority: 80,
            enabled: true,
        },
        // Deny database access from external
        NetworkPolicy {
            name: "database-isolation".to_string(),
            ingress: vec![IngressRule {
                from: "agent:".to_string(),
                ports: vec![5432],
                protocol: Protocol::TCP,
            }],
            egress: vec![],
            selectors: vec!["database:".to_string()],
            priority: 100,
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_isolation() {
        let mut isolation = NetworkIsolation::new();

        // Add policy allowing agent to database
        isolation.add_policy(NetworkPolicy {
            name: "test-policy".to_string(),
            ingress: vec![],
            egress: vec![EgressRule {
                to: "database:".to_string(),
                ports: vec![5432],
                protocol: Protocol::TCP,
            }],
            selectors: vec!["agent:".to_string()],
            priority: 100,
            enabled: true,
        });

        // Should allow agent to database
        assert!(isolation.is_connection_allowed(
            "agent:coordinator",
            "database:postgres",
            5432,
            Protocol::TCP
        ));

        // Should deny agent to external
        assert!(!isolation.is_connection_allowed(
            "agent:coordinator",
            "external:example.com",
            443,
            Protocol::TCP
        ));
    }

    #[test]
    fn test_network_zone() {
        let mut zone = NetworkZone::new("production");
        zone.add_cidr("10.0.0.0/16");
        zone.allow_outbound("monitoring");

        assert_eq!(zone.cidrs.len(), 1);
        assert!(zone.allowed_outbound.contains(&"monitoring".to_string()));
    }
}
