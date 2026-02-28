//! Input Validation Module
//!
//! Provides input validation and sanitization for security

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Input validator
pub struct InputValidator {
    max_size: usize,
    patterns: Vec<ValidationPattern>,
}

/// Validation pattern
#[derive(Debug, Clone)]
pub struct ValidationPattern {
    /// Pattern name
    pub name: String,
    /// Regex pattern
    pub pattern: Regex,
    /// Whether pattern is required
    pub required: bool,
    /// Whether to reject matches (blocklist)
    pub reject_match: bool,
    /// Error message
    pub error_message: String,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error type
    pub error_type: ValidationErrorType,
    /// Error message
    pub message: String,
    /// Field that failed validation
    pub field: Option<String>,
}

/// Types of validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    /// Input too large
    TooLarge,
    /// Invalid format
    InvalidFormat,
    /// Forbidden content
    ForbiddenContent,
    /// Missing required field
    MissingRequired,
    /// Invalid characters
    InvalidCharacters,
    /// Pattern mismatch
    PatternMismatch,
    /// SQL injection detected
    SqlInjection,
    /// XSS detected
    XssAttack,
    /// Command injection detected
    CommandInjection,
    /// Path traversal detected
    PathTraversal,
}

impl InputValidator {
    /// Create new input validator
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            patterns: Self::default_patterns(),
        }
    }
    
    /// Get default validation patterns (security-focused)
    fn default_patterns() -> Vec<ValidationPattern> {
        vec![
            // SQL Injection patterns
            ValidationPattern {
                name: "sql_injection".to_string(),
                pattern: Regex::new(r"(?i)(union\s+select|insert\s+into|delete\s+from|drop\s+table|or\s+1\s*=\s*1|;\s*--|'\s*or\s*'|\bexec\s*\(|\bxp_cmdshell)")
                    .expect("Invalid SQL injection pattern"),
                required: false,
                reject_match: true,
                error_message: "Potential SQL injection detected".to_string(),
            },
            
            // XSS patterns
            ValidationPattern {
                name: "xss".to_string(),
                pattern: Regex::new(r"(?i)(<script|javascript:|on\w+\s*=|<iframe|<object|<embed|<svg|<math|expression\s*\()")
                    .expect("Invalid XSS pattern"),
                required: false,
                reject_match: true,
                error_message: "Potential XSS attack detected".to_string(),
            },
            
            // Command injection patterns
            ValidationPattern {
                name: "command_injection".to_string(),
                pattern: Regex::new(r"[;&|`$(){}\\[\]]|(?i)(rm\s+-rf|chmod\s+777|wget\s+|curl\s+|nc\s+-l|/bin/sh|/bin/bash|sudo\s+)")
                    .expect("Invalid command injection pattern"),
                required: false,
                reject_match: true,
                error_message: "Potential command injection detected".to_string(),
            },
            
            // Path traversal patterns
            ValidationPattern {
                name: "path_traversal".to_string(),
                pattern: Regex::new(r"\.\./|\.\.\\|%2e%2e%2f|%2e%2e/")
                    .expect("Invalid path traversal pattern"),
                required: false,
                reject_match: true,
                error_message: "Potential path traversal detected".to_string(),
            },
        ]
    }
    
    /// Validate input string
    pub fn validate(&self, input: &str) -> Result<(), ValidationError> {
        // Check size
        if input.len() > self.max_size {
            return Err(ValidationError {
                error_type: ValidationErrorType::TooLarge,
                message: format!(
                    "Input size {} exceeds maximum {}",
                    input.len(),
                    self.max_size
                ),
                field: None,
            });
        }
        
        // Check patterns
        for pattern in &self.patterns {
            if pattern.reject_match && pattern.pattern.is_match(input) {
                return Err(ValidationError {
                    error_type: match pattern.name.as_str() {
                        "sql_injection" => ValidationErrorType::SqlInjection,
                        "xss" => ValidationErrorType::XssAttack,
                        "command_injection" => ValidationErrorType::CommandInjection,
                        "path_traversal" => ValidationErrorType::PathTraversal,
                        _ => ValidationErrorType::ForbiddenContent,
                    },
                    message: pattern.error_message.clone(),
                    field: None,
                });
            }
        }
        
        Ok(())
    }
    
    /// Validate with field name
    pub fn validate_field(&self, field: &str, input: &str) -> Result<(), ValidationError> {
        self.validate(input).map_err(|mut e| {
            e.field = Some(field.to_string());
            e
        })
    }
    
    /// Sanitize input (remove dangerous content)
    pub fn sanitize(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        // Remove HTML tags
        result = html_escape::encode_text(&result).to_string();
        
        // Remove control characters except newline and tab
        result = result
            .chars()
            .filter(|&c| c >= ' ' || c == '\n' || c == '\t')
            .collect();
        
        result
    }
    
    /// Add custom pattern
    pub fn add_pattern(&mut self, pattern: ValidationPattern) {
        self.patterns.push(pattern);
    }
    
    /// Validate email
    pub fn validate_email(&self, email: &str) -> Result<(), ValidationError> {
        self.validate(email)?;
        
        let email_pattern = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .expect("Invalid email pattern");
        
        if !email_pattern.is_match(email) {
            return Err(ValidationError {
                error_type: ValidationErrorType::InvalidFormat,
                message: "Invalid email format".to_string(),
                field: Some("email".to_string()),
            });
        }
        
        Ok(())
    }
    
    /// Validate URL
    pub fn validate_url(&self, url: &str) -> Result<(), ValidationError> {
        self.validate(url)?;
        
        // Only allow http and https
        let url_pattern = Regex::new(r"^https?://[a-zA-Z0-9.-]+(:[0-9]+)?(/.*)?$")
            .expect("Invalid URL pattern");
        
        if !url_pattern.is_match(url) {
            return Err(ValidationError {
                error_type: ValidationErrorType::InvalidFormat,
                message: "Invalid URL format (only http/https allowed)".to_string(),
                field: Some("url".to_string()),
            });
        }
        
        Ok(())
    }
    
    /// Validate identifier (alphanumeric with underscores/dashes)
    pub fn validate_identifier(&self, id: &str) -> Result<(), ValidationError> {
        self.validate(id)?;
        
        let id_pattern = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
            .expect("Invalid identifier pattern");
        
        if !id_pattern.is_match(id) {
            return Err(ValidationError {
                error_type: ValidationErrorType::InvalidFormat,
                message: "Invalid identifier format (must start with letter, contain only alphanumeric, underscore, or dash)".to_string(),
                field: Some("identifier".to_string()),
            });
        }
        
        Ok(())
    }
    
    /// Validate JSON string
    pub fn validate_json(&self, json: &str) -> Result<(), ValidationError> {
        self.validate(json)?;
        
        serde_json::from_str::<serde_json::Value>(json).map_err(|_| ValidationError {
            error_type: ValidationErrorType::InvalidFormat,
            message: "Invalid JSON format".to_string(),
            field: Some("json".to_string()),
        })?;
        
        Ok(())
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new(10 * 1024 * 1024) // 10MB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_safe_input() {
        let validator = InputValidator::new(1000);
        
        assert!(validator.validate("Hello, World!").is_ok());
        assert!(validator.validate("Normal text with numbers 123").is_ok());
    }
    
    #[test]
    fn test_reject_sql_injection() {
        let validator = InputValidator::new(1000);
        
        let result = validator.validate("'; DROP TABLE users; --");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().error_type,
            ValidationErrorType::SqlInjection
        ));
    }
    
    #[test]
    fn test_reject_xss() {
        let validator = InputValidator::new(1000);
        
        let result = validator.validate("<script>alert('xss')</script>");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().error_type,
            ValidationErrorType::XssAttack
        ));
    }
    
    #[test]
    fn test_sanitize() {
        let validator = InputValidator::new(1000);
        
        let input = "<script>alert('xss')</script>";
        let sanitized = validator.sanitize(input);
        
        assert!(!sanitized.contains("<script>"));
    }
    
    #[test]
    fn test_validate_email() {
        let validator = InputValidator::new(1000);
        
        assert!(validator.validate_email("test@example.com").is_ok());
        assert!(validator.validate_email("invalid-email").is_err());
    }
    
    #[test]
    fn test_validate_url() {
        let validator = InputValidator::new(1000);
        
        assert!(validator.validate_url("https://example.com").is_ok());
        assert!(validator.validate_url("ftp://example.com").is_err());
    }
}
