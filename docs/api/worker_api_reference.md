# Worker Servant API Reference

## Overview

The Worker servant is the "hands" of the ServantGuild, responsible for executing tools and operations. It provides a comprehensive set of Host Tools for file system operations, command execution, network requests, and code analysis.

## Table of Contents

- [Initialization](#initialization)
- [Host Tools](#host-tools)
  - [File System Tools](#file-system-tools)
  - [Shell/Execution Tools](#shelexecution-tools)
  - [Network Tools](#network-tools)
  - [Code Analysis Tools](#code-analysis-tools)
- [ReAct Pattern](#react-pattern)
- [Error Handling & Retry](#error-handling--retry)
- [Examples](#examples)

## Initialization

### Creating a Worker

```rust
use servant_guild::servants::worker::Worker;
use std::sync::Arc;

// Create a new worker
let worker = Worker::new();

// Create a worker with consensus engine
let worker = Worker::new().with_consensus(consensus_engine);

// Start the worker
worker.start().await?;
```

## Host Tools

### File System Tools

#### `read_file`

Read a file from the filesystem.

**Risk Level:** 2 (Low)

**Parameters:**
```json
{
  "path": "string (required) - File path to read"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "path": "README.md",
    "content": "file content...",
    "size": 1024
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "read_file",
    serde_json::json!({
        "path": "README.md"
    })
).await?;
```

---

#### `write_file`

Write content to a file.

**Risk Level:** 5 (Medium)
**Requires Approval:** Yes

**Parameters:**
```json
{
  "path": "string (required) - File path to write",
  "content": "string (required) - Content to write"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "path": "output.txt",
    "bytes_written": 512
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "write_file",
    serde_json::json!({
        "path": "output.txt",
        "content": "Hello, World!"
    })
).await?;
```

---

#### `delete_file`

Delete a file from the filesystem.

**Risk Level:** 8 (High)
**Requires Approval:** Yes

**Parameters:**
```json
{
  "path": "string (required) - File path to delete"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "path": "temp.txt",
    "deleted": true
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "delete_file",
    serde_json::json!({
        "path": "temp.txt"
    })
).await?;
```

---

#### `list_files`

List files in a directory.

**Risk Level:** 1 (Low)

**Parameters:**
```json
{
  "path": "string (optional) - Directory path (default: current)",
  "recursive": "boolean (optional) - Recursive listing (default: false)"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "path": ".",
    "recursive": false,
    "files": ["Cargo.toml", "src", "README.md"],
    "count": 3
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "list_files",
    serde_json::json!({
        "path": "src",
        "recursive": true
    })
).await?;
```

---

#### `search_files`

Search for text in files.

**Risk Level:** 1 (Low)

**Parameters:**
```json
{
  "pattern": "string (required) - Search pattern",
  "path": "string (optional) - Directory path (default: current)"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "pattern": "TODO",
    "path": ".",
    "matches": [
      {
        "file": "src/main.rs",
        "line": 42,
        "content": "// TODO: implement this"
      }
    ],
    "match_count": 1
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "search_files",
    serde_json::json!({
        "pattern": "TODO",
        "path": "src"
    })
).await?;
```

---

#### `file_info`

Get file metadata.

**Risk Level:** 1 (Low)

**Parameters:**
```json
{
  "path": "string (required) - File path"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "path": "Cargo.toml",
    "is_dir": false,
    "is_file": true,
    "size": 1024,
    "modified": 1709251200,
    "permissions": "644"
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "file_info",
    serde_json::json!({
        "path": "Cargo.toml"
    })
).await?;
```

### Shell/Execution Tools

#### `run_command`

Execute a shell command.

**Risk Level:** 7 (High)
**Requires Approval:** Yes

**Parameters:**
```json
{
  "command": "string (required) - Command to execute",
  "args": "array<string> (optional) - Command arguments"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "command": "ls",
    "args": ["-la"],
    "exit_code": 0,
    "stdout": "file1.txt\nfile2.txt",
    "stderr": ""
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "run_command",
    serde_json::json!({
        "command": "ls",
        "args": ["-la"]
    })
).await?;
```

### Network Tools

#### `http_request`

Make an HTTP request.

**Risk Level:** 4 (Low-Medium)

**Parameters:**
```json
{
  "url": "string (required) - Request URL",
  "method": "string (optional) - HTTP method (default: GET)"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "url": "https://api.example.com",
    "method": "GET",
    "status": 200,
    "message": "Request successful",
    "note": "Actual HTTP implementation requires reqwest crate"
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "http_request",
    serde_json::json!({
        "url": "https://api.example.com",
        "method": "GET"
    })
).await?;
```

### Code Analysis Tools

#### `analyze_code`

Analyze code for issues.

**Risk Level:** 1 (Low)

**Parameters:**
```json
{
  "code": "string (required) - Code to analyze"
}
```

**Returns:**
```json
{
  "success": true,
  "output": {
    "code_length": 256,
    "lines": 10,
    "issues": [
      "Potential panic: using unwrap()",
      "Incomplete code (TODO/FIXME)"
    ],
    "issues_count": 2
  }
}
```

**Example:**
```rust
let result = worker.execute_tool(
    "analyze_code",
    serde_json::json!({
        "code": "fn main() {\n    let x = Some(1);\n    let y = x.unwrap();\n}"
    })
).await?;
```

## ReAct Pattern

The Worker implements the ReAct (Reasoning + Acting) pattern for intelligent task execution.

### Basic Usage

```rust
let result = worker.react_execute(
    "Read the README.md file and summarize it",
    5  // max_iterations
).await?;

if result.success {
    println!("Task completed: {:?}", result.output);
}
```

### ReAct Flow

1. **Think**: Analyze the task and decide the next action
2. **Act**: Execute the chosen tool
3. **Observe**: Capture the result
4. **Iterate**: Repeat until task is complete

### Example ReAct Execution

```rust
// Task: "Find all TODO comments in the codebase"
let result = worker.react_execute("Find all TODO comments", 10).await?;

// ReAct loop will:
// 1. Think: "I should search for TODO pattern"
// 2. Act: search_files("TODO")
// 3. Observe: Get matches
// 4. Think: "Task complete"
// 5. Return result
```

## Error Handling & Retry

### Automatic Retry

The Worker automatically retries transient failures with exponential backoff.

**Default Retry Configuration:**
- Max retries: 3
- Initial backoff: 100ms
- Backoff multiplier: 2.0
- Max backoff: 5000ms

### Retryable Errors

Errors containing the following keywords are automatically retried:
- `timeout`
- `temporary`
- `connection`
- `network`
- `IO`

### Example

```rust
let result = worker.execute_tool(
    "http_request",
    serde_json::json!({
        "url": "https://example.com/api"
    })
).await?;

if result.success {
    println!("Success after {} retries", result.retry_count);
} else {
    println!("Failed: {:?}", result.error);
}
```

## Examples

### Example 1: Read and Analyze a File

```rust
use servant_guild::servants::worker::Worker;

let worker = Worker::new();
worker.start().await?;

// Read file
let read_result = worker.execute_tool(
    "read_file",
    serde_json::json!({
        "path": "src/main.rs"
    })
).await?;

if read_result.success {
    let content = read_result.output["content"].as_str().unwrap();
    
    // Analyze code
    let analyze_result = worker.execute_tool(
        "analyze_code",
        serde_json::json!({
            "code": content
        })
    ).await?;
    
    println!("Found {} issues", analyze_result.output["issues_count"]);
}
```

### Example 2: Search and Replace (using ReAct)

```rust
let worker = Worker::new();
worker.start().await?;

// Use ReAct pattern for complex tasks
let result = worker.react_execute(
    "Find all TODO comments in src/ and create a list",
    10
).await?;

println!("Result: {}", serde_json::to_string_pretty(&result.output).unwrap());
```

### Example 3: Safe File Operations

```rust
let worker = Worker::new();
worker.start().await?;

// Check if file exists
let info_result = worker.execute_tool(
    "file_info",
    serde_json::json!({
        "path": "data.txt"
    })
).await?;

if info_result.success {
    // Read the file
    let read_result = worker.execute_tool(
        "read_file",
        serde_json::json!({
            "path": "data.txt"
        })
    ).await?;
    
    // Process and write backup
    if let Some(content) = read_result.output["content"].as_str() {
        let write_result = worker.execute_tool(
            "write_file",
            serde_json::json!({
                "path": "data.txt.backup",
                "content": content
            })
        ).await?;
        
        if write_result.success {
            println!("Backup created successfully");
        }
    }
}
```

## Best Practices

1. **Always Check Success**: Always check the `success` field before accessing the output.
2. **Handle Errors Gracefully**: Use the `error` field to diagnose failures.
3. **Use ReAct for Complex Tasks**: For multi-step tasks, use `react_execute` instead of manual tool calls.
4. **Leverage Retry**: The automatic retry mechanism handles transient failures.
5. **Respect Approval**: Tools marked with `requires_approval` will trigger consensus checks.

## Notes

- All file operations currently use `std::fs`. In production, they should use the safety module.
- Shell commands are not sandboxed. Use with caution in production.
- HTTP requests currently return mock results. Real implementation requires `reqwest` crate.
- ReAct pattern currently uses simple rule-based reasoning. LLM integration will enhance this.

## See Also

- [Coordinator API](coordinator_api_reference.md)
- [Warden API](warden_api_reference.md)
- [Safety Module](../safety/README.md)
