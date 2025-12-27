---
description: Orchestrates feature development by delegating work to sub-agents (branching, implementation, testing, PR creation).
allowed-tools: Read, Task, TaskOutput, TodoWrite, Glob, Grep, AskUserQuestion
argument-hint: [path to spec file or description of work]
---

# Role

Act as a development manager. Delegate all work to sub-agents.

# Input

```
$1
```

# Workflow

```
Phase 0: Input validation  → AskUserQuestion if empty
Phase 1: Branch check      → managing-branches skill
Phase 2: Planning          → TodoWrite → ⏸️ User approval
Phase 3: Branch creation   → managing-branches skill (if needed)
Phase 4: Investigation     → Explore/Plan sub-agent (if needed)
Phase 5: Implementation    → implementing-code skill
Phase 6: Verification      → running-tests skill → ⏸️ User approval
Phase 7: Acceptance        → Feedback loop
Done:    PR creation       → creating-pull-requests skill (optional)
```

# Phase Details

## Phase 0: Input validation

- Empty → AskUserQuestion for details
- File path → Read the file
- Text → Use as spec directly

## Phase 1: Branch check

Delegate to sub-agent:
> Use managing-branches skill to investigate current branch status

## Phase 2: Planning

1. Break down spec into independent tasks
2. Register all tasks with TodoWrite
3. Propose branch name (refer to CLAUDE.md for conventions)

**⏸️ User approval**: Present task list, branch status, proposed branch name

## Phase 3: Branch creation

Delegate to sub-agent:
> Use managing-branches skill to create <branch-name> from <base-branch>

## Phase 4: Investigation

Delegate investigation if needed:
- `subagent_type: Explore` - Codebase investigation
- `subagent_type: Plan` - Implementation design

## Phase 5: Implementation

Delegate each task to sub-agent:
> Use implementing-code skill to implement:
> - Purpose: [task purpose]
> - Deliverable: [completion criteria]

Independent tasks can run with `run_in_background: true`.

## Phase 6: Verification

Delegate to sub-agent:
> Use running-tests skill to run tests

**⏸️ User approval**: Report implementation summary, commits, test results

## Phase 7: Acceptance

1. User reviews and tests
2. If feedback, delegate fixes to implementing-code skill
3. Complete upon approval

## Post-completion

- **PR creation**: Delegate to creating-pull-requests skill
