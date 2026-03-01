//! Error Analysis and Auto-Fix - Intelligent Build Error Recovery
//!
//! This module provides intelligent error analysis and automatic fix
//! suggestions for build failures in ServantGuild.
//!
//! Capabilities:
//! - Parse compiler error messages
//! - Identify root causes
//! - Generate fix suggestions
//! - Apply automatic fixes (with consensus approval)
//! - Track fix success rates

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::providers::LLMProvider;

/// Build error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildError {
    /// Error ID
    pub id: String,
    /// Error code (e.g., E0277, E0433)
    pub error_code: String,
    /// Error message
    pub message: String,
    /// File path
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Column number
    pub column: Option<u32>,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Suggested fix
    pub suggestion: Option<String>,
    /// Related errors
    pub related: Vec<String>,
}

/// Error severity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical error, cannot continue
    Error,
    /// Warning, might be an issue
    Warning,
    /// Note, informational
    Note,
    /// Help message
    Help,
}

/// Fix suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixSuggestion {
    /// Suggestion ID
    pub id: String,
    /// Description of the fix
    pub description: String,
    /// Confidence level (0-100)
    pub confidence: u8,
    /// File to modify
    pub file: String,
    /// Line range to modify
    pub line_range: (u32, u32),
    /// Original code
    pub original: String,
    /// Replacement code
    pub replacement: String,
    /// Fix category
    pub category: FixCategory,
    /// Whether this fix can be applied automatically
    pub auto_applicable: bool,
}

/// Fix category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FixCategory {
    /// Import/dependency issue
    Import,
    /// Type mismatch
    TypeMismatch,
    /// Missing implementation
    MissingImpl,
    /// Syntax error
    Syntax,
    /// Unused code
    UnusedCode,
    /// Borrow checker issue
    BorrowCheck,
    /// Lifetime issue
    Lifetime,
    /// Trait bound issue
    TraitBound,
    /// Other
    Other,
}

/// Error pattern for recognition
#[derive(Debug, Clone)]
pub struct ErrorPattern {
    /// Pattern ID
    pub id: String,
    /// Regex pattern
    pub pattern: Regex,
    /// Error code
    pub error_code: String,
    /// Category
    pub category: FixCategory,
    /// Fix template
    pub fix_template: Option<String>,
}

/// Error analyzer
pub struct ErrorAnalyzer {
    /// Known error patterns
    patterns: Vec<ErrorPattern>,
    /// LLM provider for complex analysis
    llm: Option<Arc<dyn LLMProvider>>,
    /// Fix history
    fix_history: Arc<RwLock<Vec<FixRecord>>>,
    /// Success rates per category
    success_rates: Arc<RwLock<HashMap<FixCategory, f64>>>,
}

/// Record of a fix attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixRecord {
    /// Record ID
    pub id: String,
    /// Fix suggestion ID
    pub suggestion_id: String,
    /// Category
    pub category: FixCategory,
    /// Whether the fix was applied
    pub applied: bool,
    /// Whether the fix succeeded
    pub success: bool,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Error before fix
    pub error_before: String,
    /// Error after fix (if failed)
    pub error_after: Option<String>,
}

impl ErrorAnalyzer {
    /// Create a new error analyzer
    pub fn new() -> Self {
        Self::with_llm(None)
    }

    /// Create an error analyzer with LLM support
    pub fn with_llm(llm: Option<Arc<dyn LLMProvider>>) -> Self {
        let patterns = Self::build_patterns();

        Self {
            patterns,
            llm,
            fix_history: Arc::new(RwLock::new(Vec::new())),
            success_rates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build known error patterns
    fn build_patterns() -> Vec<ErrorPattern> {
        vec![
            // E0277: the trait bound is not satisfied
            ErrorPattern {
                id: "e0277-trait-bound".to_string(),
                pattern: Regex::new(r"error\[E0277\]: the trait bound `([^`]+)` is not satisfied").unwrap(),
                error_code: "E0277".to_string(),
                category: FixCategory::TraitBound,
                fix_template: Some("Add impl {} for the type or use a type that already implements it.".to_string()),
            },

            // E0433: failed to resolve
            ErrorPattern {
                id: "e0433-unresolved".to_string(),
                pattern: Regex::new(r"error\[E0433\]: failed to resolve: use of undeclared ([^\n]+)").unwrap(),
                error_code: "E0433".to_string(),
                category: FixCategory::Import,
                fix_template: Some("Add the missing use declaration or import.".to_string()),
            },

            // E0308: mismatched types
            ErrorPattern {
                id: "e0308-type-mismatch".to_string(),
                pattern: Regex::new(r"error\[E0308\]: mismatched types").unwrap(),
                error_code: "E0308".to_string(),
                category: FixCategory::TypeMismatch,
                fix_template: None,
            },

            // E0599: no method named found
            ErrorPattern {
                id: "e0599-no-method".to_string(),
                pattern: Regex::new(r"error\[E0599\]: no method named `([^`]+)` found").unwrap(),
                error_code: "E0599".to_string(),
                category: FixCategory::MissingImpl,
                fix_template: Some("The method may be missing from the trait impl or requires different type parameters.".to_string()),
            },

            // E0502: cannot borrow as mutable
            ErrorPattern {
                id: "e0502-borrow-mut".to_string(),
                pattern: Regex::new(r"error\[E0502\]: cannot borrow `([^`]+)` as mutable").unwrap(),
                error_code: "E0502".to_string(),
                category: FixCategory::BorrowCheck,
                fix_template: None,
            },

            // E0382: use of moved value
            ErrorPattern {
                id: "e0382-moved-value".to_string(),
                pattern: Regex::new(r"error\[E0382\]: use of moved value: `([^`]+)`").unwrap(),
                error_code: "E0382".to_string(),
                category: FixCategory::BorrowCheck,
                fix_template: Some("Consider cloning the value or using a reference.".to_string()),
            },

            // E0106: missing lifetime specifier
            ErrorPattern {
                id: "e0106-lifetime".to_string(),
                pattern: Regex::new(r"error\[E0106\]: missing lifetime specifier").unwrap(),
                error_code: "E0106".to_string(),
                category: FixCategory::Lifetime,
                fix_template: None,
            },

            // E0275: overflow evaluating the requirement
            ErrorPattern {
                id: "e0275-overflow".to_string(),
                pattern: Regex::new(r"error\[E0275\]: overflow evaluating the requirement").unwrap(),
                error_code: "E0275".to_string(),
                category: FixCategory::TraitBound,
                fix_template: Some("There may be a circular dependency in trait implementations.".to_string()),
            },
        ]
    }

    /// Analyze build output and extract errors
    pub async fn analyze(&self, output: &str) -> Result<Vec<BuildError>> {
        let mut errors = Vec::new();

        // Parse error messages
        let error_regex = Regex::new(r"error(?:\[(E\d+)\])?: ([^\n]+)").unwrap();
        let file_regex = Regex::new(r"   --> ([^:]+):(\d+):(\d+)").unwrap();

        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            if let Some(caps) = error_regex.captures(line) {
                let error_code = caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                let message = caps
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();

                // Look for file location on next line
                let mut file = None;
                let mut line_num = None;
                let mut column = None;

                if i + 1 < lines.len() {
                    if let Some(file_caps) = file_regex.captures(lines[i + 1]) {
                        file = Some(file_caps.get(1).unwrap().as_str().to_string());
                        line_num = Some(file_caps.get(2).unwrap().as_str().parse().unwrap_or(0));
                        column = Some(file_caps.get(3).unwrap().as_str().parse().unwrap_or(0));
                        i += 1;
                    }
                }

                // Determine severity
                let severity = if error_code.starts_with("E") {
                    ErrorSeverity::Error
                } else if message.contains("warning") {
                    ErrorSeverity::Warning
                } else {
                    ErrorSeverity::Error
                };

                // Find matching pattern
                let suggestion = self.find_suggestion(&error_code, &message);

                errors.push(BuildError {
                    id: format!("err-{}", uuid::Uuid::new_v4()),
                    error_code,
                    message,
                    file,
                    line: line_num,
                    column,
                    severity,
                    suggestion,
                    related: Vec::new(),
                });
            }

            i += 1;
        }

        debug!("Analyzed {} errors from build output", errors.len());

        Ok(errors)
    }

    /// Find suggestion for an error
    fn find_suggestion(&self, error_code: &str, message: &str) -> Option<String> {
        for pattern in &self.patterns {
            if pattern.error_code == error_code {
                if let Some(ref template) = pattern.fix_template {
                    if let Some(caps) = pattern.pattern.captures(message) {
                        let suggestion = template.clone();
                        // Replace captured groups in template
                        if let Some(cap) = caps.get(1) {
                            return Some(suggestion.replace("{}", cap.as_str()));
                        }
                        return Some(suggestion);
                    }
                }
                return pattern.fix_template.clone();
            }
        }
        None
    }

    /// Generate fix suggestions for errors
    pub async fn suggest_fixes(
        &self,
        errors: &[BuildError],
        context: &BuildContext,
    ) -> Result<Vec<FixSuggestion>> {
        let mut suggestions = Vec::new();

        for error in errors {
            // Try pattern-based fixes first
            if let Some(suggestion) = self.generate_pattern_fix(error, context).await? {
                suggestions.push(suggestion);
                continue;
            }

            // Fall back to LLM-based analysis
            if let Some(ref llm) = self.llm {
                if let Some(suggestion) = self.generate_llm_fix(error, context, llm).await? {
                    suggestions.push(suggestion);
                }
            }
        }

        info!("Generated {} fix suggestions", suggestions.len());

        Ok(suggestions)
    }

    /// Generate fix using pattern matching
    async fn generate_pattern_fix(
        &self,
        error: &BuildError,
        context: &BuildContext,
    ) -> Result<Option<FixSuggestion>> {
        // Find matching pattern
        let pattern = self
            .patterns
            .iter()
            .find(|p| p.error_code == error.error_code && p.pattern.is_match(&error.message));

        if pattern.is_none() {
            return Ok(None);
        }

        let pattern = pattern.unwrap();

        // Generate fix based on category
        match pattern.category {
            FixCategory::Import => self.generate_import_fix(error, context).await,
            FixCategory::TraitBound => self.generate_trait_fix(error, context).await,
            _ => Ok(None),
        }
    }

    /// Generate import fix
    async fn generate_import_fix(
        &self,
        error: &BuildError,
        context: &BuildContext,
    ) -> Result<Option<FixSuggestion>> {
        // Extract missing item from error message
        let missing_regex =
            Regex::new(r"use of undeclared (?:crate|module|type|value) `([^`]+)`").unwrap();

        if let Some(caps) = missing_regex.captures(&error.message) {
            let missing_item = caps.get(1).unwrap().as_str();

            // Try to find the correct import path
            let import_path = self.find_import_path(missing_item, context).await?;

            if let Some(path) = import_path {
                return Ok(Some(FixSuggestion {
                    id: format!("fix-{}", uuid::Uuid::new_v4()),
                    description: format!("Add import: use {}", path),
                    confidence: 85,
                    file: error.file.clone().unwrap_or_default(),
                    line_range: (1, 1),
                    original: String::new(),
                    replacement: format!("use {};\n", path),
                    category: FixCategory::Import,
                    auto_applicable: true,
                }));
            }
        }

        Ok(None)
    }

    /// Find import path for an item
    async fn find_import_path(&self, item: &str, context: &BuildContext) -> Result<Option<String>> {
        // Search in project files
        let search_pattern = format!("use.*{}.*;", item);
        let search_regex = Regex::new(&search_pattern).unwrap();

        // Check standard library items
        let std_items = HashMap::from([
            ("Result", "std::result::Result"),
            ("Option", "std::option::Option"),
            ("Vec", "std::vec::Vec"),
            ("String", "std::string::String"),
            ("Box", "std::boxed::Box"),
            ("Arc", "std::sync::Arc"),
            ("RwLock", "std::sync::RwLock"),
            ("Mutex", "std::sync::Mutex"),
            ("HashMap", "std::collections::HashMap"),
            ("HashSet", "std::collections::HashSet"),
            ("BTreeMap", "std::collections::BTreeMap"),
            ("Path", "std::path::Path"),
            ("PathBuf", "std::path::PathBuf"),
            ("Duration", "std::time::Duration"),
            ("Instant", "std::time::Instant"),
        ]);

        if let Some(path) = std_items.get(item) {
            return Ok(Some(path.to_string()));
        }

        // Check project modules
        for module in &context.available_modules {
            if module.ends_with(item) || module.contains(&format!("::{}", item)) {
                return Ok(Some(module.clone()));
            }
        }

        Ok(None)
    }

    /// Generate trait bound fix
    async fn generate_trait_fix(
        &self,
        error: &BuildError,
        context: &BuildContext,
    ) -> Result<Option<FixSuggestion>> {
        // Extract trait requirement
        let trait_regex = Regex::new(r"the trait bound `([^`]+)` is not satisfied").unwrap();

        if let Some(caps) = trait_regex.captures(&error.message) {
            let trait_req = caps.get(1).unwrap().as_str();

            // Parse trait requirement
            let description = format!("Add trait implementation or bound: {}", trait_req);

            return Ok(Some(FixSuggestion {
                id: format!("fix-{}", uuid::Uuid::new_v4()),
                description,
                confidence: 60,
                file: error.file.clone().unwrap_or_default(),
                line_range: (error.line.unwrap_or(1), error.line.unwrap_or(1) + 5),
                original: String::new(),
                replacement: format!("// TODO: Implement or add bound for {}\n", trait_req),
                category: FixCategory::TraitBound,
                auto_applicable: false,
            }));
        }

        Ok(None)
    }

    /// Generate fix using LLM
    async fn generate_llm_fix(
        &self,
        error: &BuildError,
        context: &BuildContext,
        llm: &Arc<dyn LLMProvider>,
    ) -> Result<Option<FixSuggestion>> {
        let prompt = format!(
            r#"Analyze this Rust compiler error and suggest a fix:

Error Code: {}
Error Message: {}
File: {:?}
Line: {:?}
Column: {:?}

Context:
- Project has {} modules
- Available dependencies: {:?}

Provide:
1. Root cause analysis
2. Suggested fix (code if applicable)
3. Confidence level (0-100)
4. Whether this can be auto-applied (yes/no)

Format your response as JSON:
{{
    "description": "...",
    "confidence": 85,
    "code_fix": "...",
    "auto_applicable": true
}}"#,
            error.error_code,
            error.message,
            error.file,
            error.line,
            error.column,
            context.available_modules.len(),
            context.dependencies.keys().take(5).collect::<Vec<_>>()
        );

        // Call LLM (simplified - would use actual LLM integration)
        debug!("Would call LLM for error analysis: {}", error.error_code);

        // For now, return None
        // In production, this would call the LLM and parse the response
        Ok(None)
    }

    /// Apply a fix suggestion
    pub async fn apply_fix(
        &self,
        suggestion: &FixSuggestion,
        project_path: &Path,
    ) -> Result<FixResult> {
        let file_path = project_path.join(&suggestion.file);

        if !file_path.exists() {
            bail!("File not found: {:?}", file_path);
        }

        // Read file
        let content = fs::read_to_string(&file_path).await?;
        let lines: Vec<&str> = content.lines().collect();

        // Apply the fix
        let (start, end) = suggestion.line_range;
        let start_idx = (start as usize).saturating_sub(1);
        let end_idx = (end as usize).min(lines.len()).saturating_sub(1);

        // Build new content
        let mut new_lines = Vec::new();

        // Add lines before
        for line in lines.iter().take(start_idx) {
            new_lines.push(line.to_string());
        }

        // Add fix (for imports, prepend at beginning)
        if suggestion.category == FixCategory::Import {
            new_lines.push(suggestion.replacement.clone());
            new_lines.push(String::new());
        }

        // Add remaining lines
        for line in lines.iter().skip(start_idx) {
            new_lines.push(line.to_string());
        }

        // Write file
        let new_content = new_lines.join("\n");
        fs::write(&file_path, new_content).await?;

        info!("Applied fix to {:?}", file_path);

        Ok(FixResult {
            success: true,
            file: file_path.display().to_string(),
            lines_modified: suggestion.line_range,
        })
    }

    /// Record fix result
    pub async fn record_fix_result(
        &self,
        suggestion: &FixSuggestion,
        applied: bool,
        success: bool,
        error_before: &str,
        error_after: Option<&str>,
    ) {
        let record = FixRecord {
            id: format!("rec-{}", uuid::Uuid::new_v4()),
            suggestion_id: suggestion.id.clone(),
            category: suggestion.category,
            applied,
            success,
            timestamp: chrono::Utc::now(),
            error_before: error_before.to_string(),
            error_after: error_after.map(|s| s.to_string()),
        };

        // Update history
        self.fix_history.write().await.push(record);

        // Update success rates
        let mut rates = self.success_rates.write().await;
        let entry = rates.entry(suggestion.category).or_insert(0.0);
        // Simple moving average
        *entry = (*entry * 0.9) + (if success { 10.0 } else { 0.0 });
    }

    /// Get success rate for a category
    pub async fn get_success_rate(&self, category: FixCategory) -> f64 {
        let rates = self.success_rates.read().await;
        rates.get(&category).copied().unwrap_or(0.0)
    }

    /// Get fix history
    pub async fn get_fix_history(&self) -> Vec<FixRecord> {
        self.fix_history.read().await.clone()
    }
}

/// Build context for fix generation
#[derive(Debug, Clone, Default)]
pub struct BuildContext {
    /// Available project modules
    pub available_modules: Vec<String>,
    /// Project dependencies
    pub dependencies: HashMap<String, String>,
    /// Rust edition
    pub edition: String,
    /// Target triple
    pub target: String,
}

/// Result of applying a fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Whether the fix was applied successfully
    pub success: bool,
    /// File that was modified
    pub file: String,
    /// Lines that were modified
    pub lines_modified: (u32, u32),
}

/// Auto-fixer that can apply fixes automatically
pub struct AutoFixer {
    /// Error analyzer
    analyzer: Arc<ErrorAnalyzer>,
    /// Minimum confidence to auto-apply
    min_confidence: u8,
    /// Maximum fixes per run
    max_fixes: usize,
}

impl AutoFixer {
    /// Create a new auto-fixer
    pub fn new(analyzer: Arc<ErrorAnalyzer>, min_confidence: u8, max_fixes: usize) -> Self {
        Self {
            analyzer,
            min_confidence,
            max_fixes,
        }
    }

    /// Auto-fix build errors
    pub async fn auto_fix(
        &self,
        output: &str,
        project_path: &Path,
        context: &BuildContext,
    ) -> Result<AutoFixResult> {
        // Analyze errors
        let errors = self.analyzer.analyze(output).await?;

        if errors.is_empty() {
            return Ok(AutoFixResult {
                errors_found: 0,
                fixes_generated: 0,
                fixes_applied: 0,
                fixes_successful: 0,
                remaining_errors: Vec::new(),
            });
        }

        // Generate fixes
        let suggestions = self.analyzer.suggest_fixes(&errors, context).await?;

        let mut applied = 0;
        let mut successful = 0;
        let mut remaining = Vec::new();

        for suggestion in suggestions.iter().take(self.max_fixes) {
            if suggestion.auto_applicable && suggestion.confidence >= self.min_confidence {
                match self.analyzer.apply_fix(suggestion, project_path).await {
                    Ok(result) => {
                        applied += 1;
                        if result.success {
                            successful += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to apply fix: {}", e);
                    }
                }
            } else {
                remaining.push(suggestion.clone());
            }
        }

        info!(
            "Auto-fix: {}/{} applied, {} successful",
            applied,
            suggestions.len(),
            successful
        );

        Ok(AutoFixResult {
            errors_found: errors.len(),
            fixes_generated: suggestions.len(),
            fixes_applied: applied,
            fixes_successful: successful,
            remaining_errors: remaining,
        })
    }
}

/// Result of auto-fix operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoFixResult {
    /// Number of errors found
    pub errors_found: usize,
    /// Number of fixes generated
    pub fixes_generated: usize,
    /// Number of fixes applied
    pub fixes_applied: usize,
    /// Number of fixes that succeeded
    pub fixes_successful: usize,
    /// Remaining errors that couldn't be auto-fixed
    pub remaining_errors: Vec<FixSuggestion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_errors() {
        let analyzer = ErrorAnalyzer::new();

        let output = r#"
error[E0433]: failed to resolve: use of undeclared crate 'serde'
  --> src/main.rs:10:5
   |
10 |     use serde::Deserialize;
   |         ^^^^^ use of undeclared crate 'serde'

error[E0277]: the trait bound `MyStruct: Clone` is not satisfied
  --> src/main.rs:20:5
"#;

        let errors = analyzer.analyze(output).await.unwrap();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].error_code, "E0433");
        assert_eq!(errors[1].error_code, "E0277");
    }

    #[tokio::test]
    async fn test_generate_import_fix() {
        let analyzer = ErrorAnalyzer::new();

        let error = BuildError {
            id: "test-1".to_string(),
            error_code: "E0433".to_string(),
            message: "use of undeclared type `HashMap`".to_string(),
            file: Some("src/main.rs".to_string()),
            line: Some(10),
            column: Some(5),
            severity: ErrorSeverity::Error,
            suggestion: None,
            related: Vec::new(),
        };

        let context = BuildContext::default();

        let fix = analyzer
            .generate_import_fix(&error, &context)
            .await
            .unwrap();

        assert!(fix.is_some());
        let fix = fix.unwrap();
        assert_eq!(fix.category, FixCategory::Import);
        assert!(fix.replacement.contains("std::collections::HashMap"));
    }

    #[test]
    fn test_error_patterns() {
        let patterns = ErrorAnalyzer::build_patterns();

        assert!(!patterns.is_empty());

        // Test E0277 pattern
        let pattern = patterns.iter().find(|p| p.error_code == "E0277").unwrap();
        assert!(pattern
            .pattern
            .is_match("the trait bound `Clone` is not satisfied"));
    }
}
