# Coordinator API Reference

## Overview

The Coordinator is the "brain" of the guild, responsible for:
- Receiving and parsing user requests
- Breaking down complex tasks into sub-tasks
- Distributing work to appropriate servants
- Tracking task progress and dependencies
- Aggregating results

## Table of Contents

- [Core Types](#core-types)
- [Initialization](#initialization)
- [Task Processing](#task-processing)
- [Task Decomposition](#task-decomposition)
- [Task Assignment](#task-assignment)
- [Result Aggregation](#result-aggregation)
- [Status Management](#status-management)
- [Consensus Integration](#consensus-integration)
- [Error Handling](#error-handling)
- [Usage Examples](#usage-examples)

---

## Core Types

### Coordinator

The main Coordinator servant structure.

```rust
pub struct Coordinator {
    id: ServantId,
    status: RwLock<ServantStatus>,
    consensus: Option<Arc<ConsensusEngine>>,
    active_tasks: RwLock<HashMap<String, CoordinatedTask>>,
    task_history: RwLock<Vec<CompletedTask>>,
}
```

**Fields:**
- `id`: Unique identifier for the coordinator
- `status`: Current operational status (Starting, Ready, Busy, etc.)
- `consensus`: Optional reference to the consensus engine
- `active_tasks`: Map of currently coordinated tasks
- `task_history`: Historical record of completed tasks

### CoordinatedTask

Represents a task being coordinated by the Coordinator.

```rust
pub struct CoordinatedTask {
    pub id: String,
    pub request: String,
    pub sub_tasks: Vec<SubTask>,
    pub status: CoordinationStatus,
    pub started_at: DateTime<Utc>,
    pub results: HashMap<String, serde_json::Value>,
}
```

**Fields:**
- `id`: Unique task identifier
- `request`: Original user request
- `sub_tasks`: List of generated sub-tasks
- `status`: Current coordination status
- `started_at`: When coordination began
- `results`: Results from completed sub-tasks

### SubTask

A sub-task assigned to a worker servant.

```rust
pub struct SubTask {
    pub id: String,
    pub parent_id: String,
    pub work_type: String,
    pub instructions: String,
    pub assignee: Option<String>,
    pub status: SubTaskStatus,
    pub dependencies: Vec<String>,
}
```

**Fields:**
- `id`: Unique sub-task identifier
- `parent_id`: Parent task ID
- `work_type`: Type of work (e.g., "code", "analysis", "file_ops")
- `instructions`: Instructions for the worker
- `assignee`: Assigned servant (if any)
- `status`: Current sub-task status
- `dependencies`: List of sub-task IDs this depends on

### CoordinationStatus

Status of a coordinated task.

```rust
pub enum CoordinationStatus {
    Analyzing,
    Decomposing,
    Assigning,
    Waiting,
    Aggregating,
    Completed,
    Failed,
}
```

**Variants:**
- `Analyzing`: Analyzing the user request
- `Decomposing`: Breaking down into sub-tasks
- `Assigning`: Assigning sub-tasks to workers
- `Waiting`: Waiting for sub-task results
- `Aggregating`: Combining results
- `Completed`: Successfully completed
- `Failed`: Task failed

### SubTaskStatus

Status of a sub-task.

```rust
pub enum SubTaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Blocked,
}
```

**Variants:**
- `Pending`: Not yet assigned
- `Assigned`: Assigned but not started
- `InProgress`: Currently executing
- `Completed`: Successfully completed
- `Failed`: Execution failed
- `Blocked`: Blocked by dependencies

---

## Initialization

### new()

Creates a new Coordinator instance.

```rust
pub fn new() -> Self
```

**Returns:** A new Coordinator instance

**Example:**
```rust
let coordinator = Coordinator::new();
```

### with_consensus()

Sets the consensus engine for the coordinator.

```rust
pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self
```

**Parameters:**
- `consensus`: Shared reference to the consensus engine

**Returns:** Self for builder pattern chaining

**Example:**
```rust
let coordinator = Coordinator::new()
    .with_consensus(consensus_engine);
```

---

## Task Processing

### process_request()

Processes a user request with full coordination workflow.

```rust
pub async fn process_request(&self, request: String) -> Result<String, ServantError>
```

**Parameters:**
- `request`: User request string

**Returns:**
- `Ok(String)`: Result message
- `Err(ServantError)`: Error if processing fails

**Workflow:**
1. Validates coordinator is ready
2. Decomposes task into sub-tasks
3. Assigns sub-tasks to workers
4. Aggregates results
5. Returns final result

**Example:**
```rust
let result = coordinator.process_request(
    "Update the README to include a new feature".to_string()
).await?;
```

---

## Task Decomposition

### decompose_task()

Decomposes a task into sub-tasks based on intelligent analysis.

```rust
async fn decompose_task(&self, task_id: &str) -> Result<Vec<SubTask>, ServantError>
```

**Parameters:**
- `task_id`: ID of the task to decompose

**Returns:**
- `Ok(Vec<SubTask>)`: List of generated sub-tasks
- `Err(ServantError)`: Error if decomposition fails

**Decomposition Modes:**
- **Sequential**: Tasks executed in order
- **Parallel**: Tasks executed simultaneously
- **Hybrid**: Mixed sequential and parallel execution

**Example:**
```rust
let sub_tasks = coordinator.decompose_task(&task_id).await?;
for sub_task in sub_tasks {
    println!("Sub-task: {} - {}", sub_task.work_type, sub_task.instructions);
}
```

---

## Task Assignment

### assign_subtasks()

Assigns sub-tasks to appropriate workers.

```rust
async fn assign_subtasks(&self, task_id: &str) -> Result<(), ServantError>
```

**Parameters:**
- `task_id`: ID of the coordinated task

**Returns:**
- `Ok(())`: Assignment successful
- `Err(ServantError)`: Error if assignment fails

**Assignment Logic:**
1. Checks dependencies
2. Matches sub-tasks to worker capabilities
3. Assigns tasks based on worker availability
4. Handles blocked tasks

**Example:**
```rust
coordinator.assign_subtasks(&task_id).await?;
```

---

## Result Aggregation

### aggregate_results()

Aggregates results from completed sub-tasks.

```rust
async fn aggregate_results(&self, task_id: &str) -> Result<String, ServantError>
```

**Parameters:**
- `task_id`: ID of the coordinated task

**Returns:**
- `Ok(String)`: Aggregated result message
- `Err(ServantError)`: Error if aggregation fails

**Aggregation Logic:**
1. Collects all sub-task results
2. Combines results into coherent output
3. Handles partial failures
4. Formats final response

**Example:**
```rust
let result = coordinator.aggregate_results(&task_id).await?;
println!("Final result: {}", result);
```

---

## Status Management

### get_active_tasks()

Gets all currently active tasks.

```rust
pub fn get_active_tasks(&self) -> Vec<CoordinatedTask>
```

**Returns:** List of active coordinated tasks

**Example:**
```rust
let active = coordinator.get_active_tasks();
for task in active {
    println!("Task {}: {}", task.id, task.status);
}
```

### get_task_history()

Gets historical task records.

```rust
pub fn get_task_history(&self) -> Vec<CompletedTask>
```

**Returns:** List of completed tasks

**Example:**
```rust
let history = coordinator.get_task_history();
for task in history {
    println!("Completed: {} in {}ms", task.request, task.duration_ms);
}
```

---

## Consensus Integration

### propose_vote()

Proposes a vote for decision-making.

```rust
pub async fn propose_vote(&self, proposal: String) -> Result<String, ServantError>
```

**Parameters:**
- `proposal`: Proposal content

**Returns:**
- `Ok(String)`: Vote proposal ID
- `Err(ServantError)`: Error if proposal fails

**Use Cases:**
- Task execution approval
- Resource allocation
- System changes

**Example:**
```rust
let vote_id = coordinator.propose_vote(
    "Approve task: Update README".to_string()
).await?;
```

---

## Error Handling

### ServantError

Error types for coordinator operations.

```rust
pub enum ServantError {
    NotReady(String),
    TaskNotFound(String),
    DecompositionFailed(String),
    AssignmentFailed(String),
    AggregationFailed(String),
    ConsensusError(String),
    InvalidTask(String),
}
```

**Error Handling Example:**
```rust
match coordinator.process_request(request).await {
    Ok(result) => println!("Success: {}", result),
    Err(ServantError::NotReady(msg)) => {
        eprintln!("Coordinator not ready: {}", msg);
    },
    Err(ServantError::DecompositionFailed(msg)) => {
        eprintln!("Failed to decompose task: {}", msg);
    },
    Err(e) => {
        eprintln!("Error: {:?}", e);
    }
}
```

---

## Usage Examples

### Example 1: Basic Task Processing

```rust
use servant_guild::servants::Coordinator;
use servant_guild::consensus::ConsensusEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create coordinator
    let coordinator = Coordinator::new();
    
    // Process a simple request
    let result = coordinator.process_request(
        "Create a new user endpoint".to_string()
    ).await?;
    
    println!("Result: {}", result);
    Ok(())
}
```

### Example 2: With Consensus Integration

```rust
use servant_guild::servants::Coordinator;
use servant_guild::consensus::ConsensusEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create consensus engine
    let consensus = ConsensusEngine::new();
    
    // Create coordinator with consensus
    let coordinator = Coordinator::new()
        .with_consensus(Arc::new(consensus));
    
    // Process request that requires voting
    let result = coordinator.process_request(
        "Update system configuration".to_string()
    ).await?;
    
    println!("Result: {}", result);
    Ok(())
}
```

### Example 3: Monitoring Active Tasks

```rust
use servant_guild::servants::Coordinator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let coordinator = Coordinator::new();
    
    // Start some tasks
    coordinator.process_request("Task 1".to_string()).await?;
    coordinator.process_request("Task 2".to_string()).await?;
    
    // Monitor active tasks
    let active_tasks = coordinator.get_active_tasks();
    println!("Active tasks: {}", active_tasks.len());
    
    for task in active_tasks {
        println!("Task {}: {:?}", task.id, task.status);
    }
    
    Ok(())
}
```

### Example 4: Task History Analysis

```rust
use servant_guild::servants::Coordinator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let coordinator = Coordinator::new();
    
    // Process some tasks
    for i in 1..=5 {
        coordinator.process_request(format!("Task {}", i)).await?;
    }
    
    // Analyze history
    let history = coordinator.get_task_history();
    let total_duration: u64 = history.iter()
        .map(|t| t.duration_ms)
        .sum();
    
    println!("Completed {} tasks", history.len());
    println!("Total duration: {}ms", total_duration);
    println!("Average duration: {}ms", total_duration / history.len() as u64);
    
    Ok(())
}
```

---

## Best Practices

### 1. Error Handling
Always handle errors appropriately:

```rust
match coordinator.process_request(request).await {
    Ok(result) => { /* handle success */ },
    Err(e) => {
        eprintln!("Coordinator error: {:?}", e);
        // Implement recovery logic
    }
}
```

### 2. Task Decomposition
Design tasks that can be effectively decomposed:

```rust
// Good: Specific, actionable task
"Create user authentication endpoint with JWT"

// Avoid: Too vague or complex
"Make the system better"
```

### 3. Status Monitoring
Monitor coordinator status before processing:

```rust
let status = coordinator.status();
if status != ServantStatus::Ready {
    println!("Coordinator busy, retry later...");
    return;
}
```

### 4. Result Validation
Always validate aggregated results:

```rust
let result = coordinator.aggregate_results(&task_id).await?;
if result.contains("error") {
    // Handle partial failures
}
```

---

## Performance Considerations

- **Concurrent Tasks**: Coordinator can handle multiple tasks concurrently
- **Sub-task Parallelism**: Independent sub-tasks execute in parallel
- **Memory Management**: Task history can grow large; implement cleanup
- **Status Polling**: Use event-driven updates instead of polling

---

## Limitations

- Currently simulates worker execution (real worker integration in progress)
- LLM integration for intelligent decomposition (Phase 3)
- Limited dependency resolution (basic implementation)
- No persistent task storage (in-memory only)

---

## Future Enhancements

- **LLM Integration**: Use LLM for intelligent task decomposition
- **Adaptive Assignment**: Learn from past assignments to optimize
- **Retry Logic**: Automatically retry failed sub-tasks
- **Task Prioritization**: Implement priority queues
- **Persistent Storage**: Save tasks to database
- **Real-time Updates**: Push notifications for task progress

---

## See Also

- [Worker API Reference](worker_api_reference.md)
- [Consensus Engine](../../consensus/README.md)
- [Architecture Overview](../../architecture/servant_guild_architecture_v1.0.md)
