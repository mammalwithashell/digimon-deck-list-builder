---
name: session-end
description: Snapshot the current session state (git changes, Notion board, decisions made) into a session log before wrapping up. Run this before ending a work session.
---

# Session End — Save Session Summary

You are ending a work session. Create a snapshot of what happened so the next session can pick up where you left off.

## Steps

### 1. Gather git activity

Run:
```bash
git log --oneline -20
```

Identify which commits were made during this session (use timestamps and context from the conversation).

### 2. Gather Notion board state

Use `notion-search` with `data_source_url: "collection://31f97972-7634-80d0-97eb-000b817cdae1"` to get all current tasks. Note any tasks that were created or moved during this session.

### 3. Summarize from conversation context

Review the conversation and extract:
- What was worked on
- Key decisions made and why
- Blockers encountered
- What should be picked up next

### 4. Write the session log

Create the sessions directory if needed:
```bash
mkdir -p .claude/pm/sessions
```

Write to `.claude/pm/sessions/YYYY-MM-DD-HHmm.md` (use current date/time):

```markdown
---
date: {ISO 8601 timestamp}
---

## What was done
- {Summary of work completed this session}

## Commits
- {hash} {message}

## Notion changes
- {Created/Moved/Updated tasks, or "No Notion changes"}

## Decisions & context
- {Key decisions and reasoning, or "No major decisions"}

## Suggested next
- {What to pick up next session, based on priorities and momentum}
```

### 5. Clear the briefing lock

Delete the lock file so the next session gets a fresh briefing:
```bash
rm -f .claude/pm/.briefing-lock
```

### 6. Confirm

Tell the user: "Session log saved to `.claude/pm/sessions/{filename}`. Next session will pick up from here."
