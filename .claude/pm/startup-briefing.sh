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
