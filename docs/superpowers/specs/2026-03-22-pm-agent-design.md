# PM Agent Design Spec

**Date:** 2026-03-22
**Status:** Implemented
**Scope:** Product management agent for Digimon TCG Simulator project

## Overview

A conversational product management agent built as Claude Code skills + hooks. It reads/writes the Notion Project Management board, reads git history and codebase signals, and reasons about dev priorities. Scoped to the Digimon project, with a reusable template in global `~/.claude/` for other projects.

## Deliverables

| Piece | Purpose | Location |
|-------|---------|----------|
| `/pm` skill | Conversational PM — triage, advise, sync | `.claude/skills/pm/SKILL.md` |
| `/session-end` skill | Snapshot session state before wrapping up | `.claude/skills/session-end/SKILL.md` |
| Startup hook | Inject briefing context on first message | `settings.local.json` + `.claude/pm/startup-briefing.sh` |
| Notion schema upgrade | Add Priority, Category, Effort to board | One-time migration via Notion MCP |
| Global template | Reusable PM pattern for other projects | `~/.claude/skills/pm/` |

## 1. `/pm` Skill

### Modes

The skill instructs Claude to act as PM for the Digimon TCG Simulator with three modes:

- **Triage** — "create a task for X", "move Y to in progress", "what's blocked?"
  - Reads/writes Notion directly, no confirmation needed
  - Can create tasks with Name, Status, Priority, Category, Effort
  - Can update task Status, Priority, Assign
  - No deletes — flags stale tasks and asks user

- **Advise** — "what should I work on next?", "poke holes in my plan"
  - Gathers context: board state, recent git, engine gaps, QA reports
  - Reasons about priorities, trade-offs, sequencing
  - Factors in what's blocked, what's in progress, what's high priority

- **Sync** — "sync me up", "what's the status?"
  - Pulls board state + recent git log
  - Gives concise brief of current state

### Context Sources

| Source | How accessed | What it provides |
|--------|-------------|-----------------|
| Notion Project Management board | MCP `notion-search` + `notion-fetch` | Task list, statuses, priorities |
| Git history | `git log --oneline -20` | Recent activity, what changed |
| Engine gaps | Read `qa/archetype-qa/engine-gaps.md` | Known blockers, missing mechanics |
| QA reports | Read `qa/qa-reports/INDEX.md` | Validation status, open failures |
| Session logs | Read `.claude/pm/sessions/*.md` | Prior session context, continuity |

### Notion Database IDs

- **Project Management database:** `31f97972-7634-8009-a1a0-ef0b0ece4b18`
- **Data source:** `collection://31f97972-7634-80d0-97eb-000b817cdae1`

### Write Operations (Autonomous)

- **Create task:** `notion-create-pages` with parent `data_source_id: 31f97972-7634-80d0-97eb-000b817cdae1`
- **Update task:** Lookup-then-update pattern:
  1. `notion-search` with query = task name and `data_source_url: "collection://31f97972-7634-80d0-97eb-000b817cdae1"` to find the task
  2. Extract the page ID from the search result's `id` field
  3. `notion-update-page` with that `page_id` and `command: update_properties`
- **No deletes** without user confirmation

### Error Handling

- If Notion MCP is unreachable, inform the user and offer to work from git history and local files only
- If a search returns no results for a task name, ask the user to clarify before creating a duplicate

## 2. Notion Schema Upgrade

### Current Schema

| Property | Type | Values |
|----------|------|--------|
| Name | Title | — |
| Status | Status | Not started, In progress, Done |
| Assign | Person | — |

### New Properties

| Property | Type | Values |
|----------|------|--------|
| Priority | Select | `Critical` (red), `High` (orange), `Medium` (yellow), `Low` (green) |
| Category | Select | `Engine` (blue), `Frontend` (purple), `RL` (green), `AI Pipeline` (brown), `Desktop` (red), `Infra` (gray), `QA` (pink) |
| Effort | Select | `S` (green), `M` (yellow), `L` (orange), `XL` (red) |

### DDL Migration

```sql
ADD COLUMN "Priority" SELECT('Critical':red, 'High':orange, 'Medium':yellow, 'Low':green);
ADD COLUMN "Category" SELECT('Engine':blue, 'Frontend':purple, 'RL':green, 'AI Pipeline':brown, 'Desktop':red, 'Infra':gray, 'QA':pink);
ADD COLUMN "Effort" SELECT('S':green, 'M':yellow, 'L':orange, 'XL':red)
```

Data source ID: `31f97972-7634-80d0-97eb-000b817cdae1`

### New Views

- **Priority Board** — `notion-create-view` with type `board`, config: `GROUP BY "Priority"; SHOW "Name", "Status", "Category", "Effort", "Assign"`

## 3. `/session-end` Skill

Invoked manually before wrapping up a session. It:

1. Reads git diff — commits since session start (or last N commits)
2. Reads Notion board — current snapshot of all tasks
3. Summarizes from conversation context — work done, decisions, blockers
4. Writes session log to `.claude/pm/sessions/YYYY-MM-DD-HHmm.md`

### Session Log Format

```markdown
---
date: YYYY-MM-DDTHH:MM:SS
---

## What was done
- Summary of work completed

## Commits
- hash message
- hash message

## Notion changes
- Created/Moved/Updated tasks

## Decisions & context
- Key decisions and reasoning

## Suggested next
- What to pick up next session
```

## 4. Startup Hook

### Hook Script: `.claude/pm/startup-briefing.sh`

```bash
#!/usr/bin/env bash
# Gathers context for PM briefing on session start
# Uses a lock file so it only fires once per session

LOCK_FILE=".claude/pm/.briefing-lock"
mkdir -p .claude/pm/sessions

# Guard: only fire once per session (lock file cleared on new session)
if [ -f "$LOCK_FILE" ]; then
  exit 0
fi
touch "$LOCK_FILE"

echo "=== PM BRIEFING ==="

# Last session summary
LATEST=$(ls -t .claude/pm/sessions/*.md 2>/dev/null | head -1)
if [ -n "$LATEST" ]; then
  echo "## Last Session"
  cat "$LATEST"
fi

# Recent git activity (time-based fallback for shallow clones)
echo ""
echo "## Recent Git Activity"
git log --oneline -15 2>/dev/null

echo ""
echo "## Files Changed Recently"
git log --oneline --stat --since="3 days ago" 2>/dev/null | head -30
```

### Hook Configuration

Added to `settings.local.json` (merged with existing `permissions` block):

```json
{
  "permissions": { "allow": [ "...existing..." ] },
  "hooks": {
    "userPromptSubmit": [{
      "matcher": "",
      "command": "bash .claude/pm/startup-briefing.sh 2>/dev/null"
    }]
  }
}
```

The lock file (`.claude/pm/.briefing-lock`) ensures the script only produces output on the first message of each session. Subsequent messages get an early `exit 0`. The lock file should be deleted by `/session-end` or manually between sessions. Add `.claude/pm/.briefing-lock` to `.gitignore`.

Notion board state is fetched live by Claude (not cached by the script) since it requires MCP access. When the `/pm` skill sees `=== PM BRIEFING ===` in hook output, it uses that context to deliver a briefing. The skill should deliver the briefing when the user's first message is a greeting, asks for status, or invokes `/pm`. If the first message is task-specific (e.g., "fix the bug in X"), skip the briefing and get to work.

## 5. Global Template

Location: `~/.claude/skills/pm/`

Contains:
- `SKILL.md` — template `/pm` skill with `TODO: customize` markers for:
  - Project name
  - Notion database ID / data source ID
  - Category values
  - Context sources (project-specific files to check)
- `session-end/SKILL.md` — template `/session-end` skill
- `startup-briefing.sh` — template startup script
- `README.md` — setup instructions

To add PM to a new project: copy templates into project `.claude/skills/`, fill in project-specific values, add hook to project `settings.local.json`.

## 6. File Structure

```
.claude/
├── skills/
│   ├── pm/
│   │   └── SKILL.md              # /pm skill (Digimon-specific)
│   └── session-end/
│       └── SKILL.md              # /session-end skill
├── pm/
│   ├── startup-briefing.sh       # Hook script
│   ├── .briefing-lock            # Session guard (gitignored)
│   └── sessions/                 # Session logs (gitignored)
│       └── YYYY-MM-DD-HHmm.md
└── settings.local.json           # Hook configuration (merged with existing permissions)

~/.claude/
└── skills/
    └── pm/
        ├── SKILL.md              # Global template
        ├── session-end/
        │   └── SKILL.md          # Template
        ├── startup-briefing.sh   # Template
        └── README.md             # Setup guide
```

## 7. Gitignore Additions

Add to `.gitignore`:
```
.claude/pm/sessions/
.claude/pm/.briefing-lock
```

## Non-Goals

- No custom MCP server
- No external Python scripts for Notion access
- No due dates on tasks
- No delete operations without confirmation
- No cross-project priority reasoning (Digimon-scoped)
