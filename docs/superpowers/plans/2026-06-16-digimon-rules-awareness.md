# Deeply Rules-Aware-by-Default Workflow — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the assistant reliably knowledgeable about Digimon TCG rules every session by committing small *verified* derivations of the base-only rules PDF and surfacing them via a light SessionStart baseline, a conservative prompt-time hook, and an on-demand `/digimon-rules` skill — all with zero Pinecone/network dependency.

**Architecture:** Keep the heavy PDFs base-only and git-ignored; derive three small hand-verified text artifacts (`keyword-semantics.md`, `rules-index.json`, `digest.md`) and **commit** them so they reach every worktree. Three delivery layers consume them: a SessionStart Python hook injects the compact keyword table; a conservative UserPromptSubmit Python hook injects targeted PDF page pointers on rules/keyword vocabulary; and a `/digimon-rules` skill does lookup + deep "thinking-partner" loads. Wiring lives in a new committed `.claude/settings.json`.

**Tech Stack:** Python 3.11 (stdlib only — `json`, `re`, `subprocess`, `pathlib`), Markdown/JSON data artifacts, Claude Code hooks + skills, the base-repo rules PDFs read via the `Read` tool.

**Spec:** `docs/superpowers/specs/2026-06-16-digimon-rules-awareness-design.md`

---

## File Structure

**Committed artifacts (reachable in every worktree):**
- `docs/digimon-rules/keyword-semantics.md` — compact keyword table (SessionStart baseline content + spine).
- `docs/digimon-rules/rules-index.json` — keyword/topic/rule-# → `{pdf, pages, section}` map; powers the prompt hook + skill.
- `docs/digimon-rules/digest.md` — fuller verified deep-knowledge doc (the on-demand thinking-partner content).

**Hooks (committed scripts, modeled on `.claude/hooks/digimon_card_image_hint.py`):**
- `.claude/hooks/digimon_rules_hint.py` — UserPromptSubmit; conservative rules/keyword detector → targeted PDF pointers.
- `.claude/hooks/digimon_rules_baseline.py` — SessionStart; prints the compact keyword table + banner.
- `.claude/hooks/test_digimon_rules_hint.py` — pytest for the hint hook (explicit-invocation; not in default testpaths).

**Skill:**
- `.claude/skills/digimon-rules/SKILL.md` — lookup + deep modes.

**Wiring + docs:**
- `.claude/settings.json` — **new committed** file registering both hooks (merges with base `settings.local.json`).
- `CLAUDE.md` — add rule 32 (base-repo PDF resolution + committed artifacts); update "Source priority" item #5.
- `docs/RULES_CONTEXT.md` — replaced with a one-line pointer to the new artifacts.

**Authoritative sources (base-only, git-ignored, read for verification — never copied):**
- `$BASE/Digimon TCG resources/general_rule.pdf` (Comprehensive Rules Manual Ver.3.6, 2025/12/25), `glossary.pdf`, `manual.pdf`, where `$BASE = $(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")`.

**Verified TOC (from `general_rule.pdf`, used to seed the index):**
`1 Game Overview p.1 · 2 Card Information p.3 · 3 Game Areas p.5 · 4 Basic Game Terminology p.7 · 5 Game Preparation p.11 · 6 Game Procedures p.11 · 7 Playing a Card p.13 · 8 Digivolution p.16 · 9 Using Cards p.18 · 10 Link p.19 · 11 Attacking p.19 · 12 Blocking p.21 · 13 Security Checks p.21 · 14 Battles p.21 · 15 Effect Rules p.22 · 16 Keyword Effects p.33 · 17 Rule Checks p.40 · 18 Other Information p.40`

---

## Task 1: Build `keyword-semantics.md` (verified from §16 + glossary)

**Files:**
- Create: `docs/digimon-rules/keyword-semantics.md`
- Read (verify against): `$BASE/Digimon TCG resources/general_rule.pdf` pp.33–40 (§16), `$BASE/Digimon TCG resources/glossary.pdf`

- [ ] **Step 1: Resolve the base path and read §16**

Run:
```bash
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"; echo "$BASE"
```
Then use the `Read` tool with `pages: "33-40"` on `$BASE/Digimon TCG resources/general_rule.pdf` and read all of `glossary.pdf`. Do **not** transcribe from memory or from `RULES_CONTEXT.md` — read the actual pages.

- [ ] **Step 2: Write the file in this exact format**

Header carries a version stamp; one table row per keyword. Use the printed processing kind ("Mandatory"/"Optional") exactly as §16 states it.

```markdown
# Digimon TCG — Keyword Semantics (compact baseline)

> Source: Comprehensive Rules Manual `general_rule.pdf` Ver.3.6 (2025/12/25) §16 (pp.33–40) + `glossary.pdf`.
> Verified by reading the PDF pages directly. This is the cheap baseline; for full
> rule text use `/digimon-rules <keyword>` (reads the cited PDF pages) or `/digimon-rules deep`.
> When the manual revises (e.g. Ver.3.7), re-verify and bump this stamp.

| Keyword | Kind | Targets / when | One-line semantics | Rule § |
|---|---|---|---|---|
| Blocker | Mandatory option | Opponent's attack vs you/your Digimon | May suspend to redirect the attack to this Digimon | 16-x |
| Security Attack +N / −N | — | While attacking the player | Checks N more/fewer security cards | 16-x |
| Piercing | Mandatory | When this attacks & deletes a Digimon in battle | Deals the excess as damage to security/player | 16-x |
| ... (one row PER keyword found in §16; do not omit any) ... |
```

Fill `Kind`, `Rule §`, and semantics from the actual §16 text. Include every keyword §16 lists (Blocker, Security Attack, Piercing, Jamming, Reboot, Blitz, Decoy, Save, Recovery, Draw, De-Digivolve, Digisorption, Material Save, Fragment, Progress, Alliance, Collision, Link, Raid, Vortex, Retaliation, Delay, Rush, Evade, Barrier, Armor Purge, Blast Digivolve, etc. — whatever §16 actually contains).

- [ ] **Step 3: Verify coverage**

Run:
```bash
grep -c '^| ' docs/digimon-rules/keyword-semantics.md   # row count (incl. header+separator)
```
Expected: row count ≥ number of keyword subsections in §16. Spot-check 3 rows by re-opening their cited pages and confirming Kind + semantics match the printed text.

- [ ] **Step 4: Commit**

```bash
git add docs/digimon-rules/keyword-semantics.md
git commit -m "Add verified Digimon TCG keyword-semantics baseline table"
```

---

## Task 2: Build `rules-index.json` (verified topic→page index)

**Files:**
- Create: `docs/digimon-rules/rules-index.json`
- Read (verify against): `$BASE/Digimon TCG resources/general_rule.pdf` (TOC + §16 drill)

- [ ] **Step 1: Write the index using this exact schema**

`names` lists the literal strings the prompt hook matches (display names + common variants). `pages` is a string ("33" or "33-40"). Every keyword's `pages` points to its subsection inside §16; drill §16 (pp.33–40) to get per-keyword pages.

```json
{
  "source": { "pdf": "general_rule.pdf", "version": "Ver.3.6", "last_updated": "2025/12/25" },
  "pdfs": {
    "general_rule.pdf": "Comprehensive Rules Manual — timing/keyword/effect rules (authoritative text)",
    "glossary.pdf": "Keyword definitions",
    "manual.pdf": "Official Rule Manual for Web — image-heavy, visual/UI reference"
  },
  "sections": {
    "1":  { "title": "Game Overview",          "pdf": "general_rule.pdf", "pages": "1-2" },
    "8":  { "title": "Digivolution",           "pdf": "general_rule.pdf", "pages": "16-17" },
    "11": { "title": "Attacking",              "pdf": "general_rule.pdf", "pages": "19-20" },
    "12": { "title": "Blocking",               "pdf": "general_rule.pdf", "pages": "21" },
    "13": { "title": "Security Checks",        "pdf": "general_rule.pdf", "pages": "21" },
    "14": { "title": "Battles",                "pdf": "general_rule.pdf", "pages": "21" },
    "15": { "title": "Effect Rules",           "pdf": "general_rule.pdf", "pages": "22-32" },
    "16": { "title": "Keyword Effects",        "pdf": "general_rule.pdf", "pages": "33-40" },
    "17": { "title": "Rule Checks",            "pdf": "general_rule.pdf", "pages": "40" }
  },
  "keywords": {
    "blocker":  { "names": ["Blocker"],                          "section": "16-x", "pdf": "general_rule.pdf", "pages": "33" },
    "piercing": { "names": ["Piercing"],                         "section": "16-x", "pdf": "general_rule.pdf", "pages": "34" },
    "security_attack": { "names": ["Security Attack", "Security A."], "section": "16-x", "pdf": "general_rule.pdf", "pages": "33" }
    /* ... one entry per keyword from Task 1, with real section/pages ... */
  },
  "topics": {
    "digivolution": { "names": ["digivolve", "digivolution", "digivolving"], "section": "8",  "pdf": "general_rule.pdf", "pages": "16-17" },
    "security check": { "names": ["security check"],            "section": "13", "pdf": "general_rule.pdf", "pages": "21" },
    "blocking":     { "names": ["blocking"],                     "section": "12", "pdf": "general_rule.pdf", "pages": "21" }
  }
}
```

Keep `topics` deliberately small (conservative hook). `keywords` must mirror Task 1's rows exactly (same set).

- [ ] **Step 2: Write a validity + cross-consistency test**

Create `docs/digimon-rules/_validate_index.py` (a throwaway verifier; delete after — or keep, your call):
```python
import json, re, sys
from pathlib import Path
idx = json.loads(Path("docs/digimon-rules/rules-index.json").read_text(encoding="utf-8"))
assert idx["source"]["pdf"] == "general_rule.pdf"
for grp in ("keywords", "topics"):
    for key, v in idx[grp].items():
        assert v["names"] and all(isinstance(n, str) for n in v["names"]), key
        assert re.fullmatch(r"\d+(-\d+)?", v["pages"]), (key, v["pages"])
        assert v["pdf"] in idx["pdfs"], (key, v["pdf"])
# every keyword in the index appears as a row in the semantics table
table = Path("docs/digimon-rules/keyword-semantics.md").read_text(encoding="utf-8").lower()
missing = [k for k,v in idx["keywords"].items() if v["names"][0].lower() not in table]
assert not missing, f"keywords missing from semantics table: {missing}"
print("rules-index.json OK")
```

- [ ] **Step 3: Run the validator**

Run: `python docs/digimon-rules/_validate_index.py`
Expected: `rules-index.json OK` (fix the JSON until it passes).

- [ ] **Step 4: Spot-check page ranges against the PDF**

Pick 3 keyword/topic entries, `Read` their cited pages on `$BASE/Digimon TCG resources/general_rule.pdf`, and confirm the cited section header appears there. Fix any wrong page numbers.

- [ ] **Step 5: Commit**

```bash
rm -f docs/digimon-rules/_validate_index.py
git add docs/digimon-rules/rules-index.json
git commit -m "Add verified rules-index.json (topic/keyword -> PDF page map)"
```

---

## Task 3: Build `digest.md` (verified deep-knowledge doc)

**Files:**
- Create: `docs/digimon-rules/digest.md`
- Read (verify against): `$BASE/Digimon TCG resources/general_rule.pdf` (esp. §§1, 6–8, 11–15) + `glossary.pdf`

- [ ] **Step 1: Read the relevant sections**

Use `Read` with the page ranges from the index: §6 Game Procedures (turn structure), §7 Playing a Card, §8 Digivolution, §11–14 (attack/block/security/battle), §15 Effect Rules (timing). Verify, don't recall.

- [ ] **Step 2: Write the digest in these sections**

```markdown
# Digimon TCG — Deep Rules Digest (verified)

> Source: `general_rule.pdf` Ver.3.6 (2025/12/25) + `glossary.pdf`. Each claim cites a rule §.
> Loaded on demand via `/digimon-rules deep` to act as a deep TCG thinking partner.
> Supersedes the retired `docs/RULES_CONTEXT.md`. Re-verify on manual revisions.

## 1. Turn structure & phases (§6)
## 2. Memory gauge & the memory rule (§1-4-2, §6)
## 3. Playing cards & costs (§7, §1-3-11)
## 4. Digivolution, costs & inherited effects (§8, §15-3)
## 5. Attacking, blocking, security checks, battles (§11–14)
## 6. Effect rules: timing, triggers, processing order, optional vs mandatory (§15)
## 7. Common interaction gotchas (each with the citing rule §)
```

Fill each section with verified prose, citing rule numbers inline (e.g. "(11-5)"). Aim for a genuinely useful thinking-partner reference, not a transcription of the whole PDF.

- [ ] **Step 3: Verify citations**

Spot-check 5 cited rule numbers by re-opening the relevant pages and confirming the rule says what the digest claims. Fix mismatches.

- [ ] **Step 4: Commit**

```bash
git add docs/digimon-rules/digest.md
git commit -m "Add verified deep rules digest (thinking-partner reference)"
```

---

## Task 4: UserPromptSubmit hook `digimon_rules_hint.py` (TDD)

**Files:**
- Create: `.claude/hooks/digimon_rules_hint.py`
- Test: `.claude/hooks/test_digimon_rules_hint.py`

- [ ] **Step 1: Write the failing test**

```python
# .claude/hooks/test_digimon_rules_hint.py
import json, os, subprocess, sys
from pathlib import Path

HOOK = Path(__file__).resolve().parent / "digimon_rules_hint.py"

def _run(prompt: str) -> str:
    env = dict(os.environ, DIGIMON_RULES_PDF_DIR=r"C:/FAKE/Digimon TCG resources")
    p = subprocess.run([sys.executable, str(HOOK)], input=json.dumps({"prompt": prompt}),
                       capture_output=True, text=True, env=env)
    assert p.returncode == 0, p.stderr
    return p.stdout

def test_keyword_match_emits_pointer():
    out = _run("How does Blocker interact with an attack?")
    assert "general_rule.pdf" in out
    assert "Blocker" in out
    assert "digimon-rules" in out

def test_rule_number_match_emits_pointer():
    out = _run("Explain rule 16-36 please")
    assert "general_rule.pdf" in out
    assert "16" in out

def test_no_rules_vocab_is_silent():
    assert _run("Refactor the deck builder pagination component").strip() == ""

def test_bare_common_verb_does_not_fire():
    # conservative: 'attack' alone (no keyword/rule-number) stays silent
    assert _run("make the attack button bigger in the UI").strip() == ""
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `python -m pytest .claude/hooks/test_digimon_rules_hint.py -v`
Expected: FAIL (hook file does not exist yet).

- [ ] **Step 3: Implement the hook**

```python
#!/usr/bin/env python3
"""UserPromptSubmit hook: when a prompt mentions Digimon rules/keyword vocabulary
or a rule number, point the assistant at the authoritative rules PDF (resolved in
the BASE repo, rule 29) + the exact pages, using the committed rules-index.json.

Conservative: fires only on explicit keyword/topic names from the index and on
rule-number patterns (e.g. 16-36). Silent otherwise. No Pinecone / network — reads
docs/digimon-rules/rules-index.json and resolves the base PDF path via git.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

RULE_NUM_RE = re.compile(r"\b\d{1,2}-\d{1,2}(?:-\d{1,3}){0,3}\b")


def repo_root() -> Path:
    # .claude/hooks/<this>.py -> repo root (the worktree, where the index is committed)
    return Path(__file__).resolve().parents[2]


def base_resources_dir() -> str | None:
    env = os.environ.get("DIGIMON_RULES_PDF_DIR")
    if env:
        return env
    try:
        common = subprocess.check_output(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            cwd=str(repo_root()), stderr=subprocess.DEVNULL, text=True,
        ).strip()
        return str(Path(common).parent / "Digimon TCG resources")
    except Exception:
        return None


def load_index() -> dict | None:
    try:
        return json.loads((repo_root() / "docs/digimon-rules/rules-index.json").read_text(encoding="utf-8"))
    except Exception:
        return None


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    try:
        payload = json.load(sys.stdin)
    except Exception:
        return 0
    prompt = str(payload.get("prompt", "") or "")
    if not prompt:
        return 0
    idx = load_index()
    if not idx:
        return 0

    base = base_resources_dir()
    hits: list[str] = []
    seen: set[str] = set()

    # Keyword + topic name matches (word-boundary, case-insensitive).
    for grp in ("keywords", "topics"):
        for key, entry in idx.get(grp, {}).items():
            for name in entry.get("names", []):
                if re.search(rf"(?<![\w]){re.escape(name)}(?![\w])", prompt, re.IGNORECASE):
                    if key in seen:
                        continue
                    seen.add(key)
                    pdf, pages = entry.get("pdf", "general_rule.pdf"), entry.get("pages", "")
                    path = f"{base}/{pdf}" if base else f"(base repo)/Digimon TCG resources/{pdf}"
                    hits.append(f'  - "{name}" (§{entry.get("section","?")}): {pdf} p.{pages} -> {path}')
                    break

    # Rule-number matches -> resolve to the enclosing section.
    sections = idx.get("sections", {})
    for m in RULE_NUM_RE.finditer(prompt):
        num = m.group(0)
        sec = num.split("-", 1)[0]
        if sec in sections and f"rule:{num}" not in seen:
            seen.add(f"rule:{num}")
            s = sections[sec]
            pdf, pages = s.get("pdf", "general_rule.pdf"), s.get("pages", "")
            path = f"{base}/{pdf}" if base else f"(base repo)/Digimon TCG resources/{pdf}"
            hits.append(f'  - Rule {num}: §{sec} {s.get("title","")}, {pdf} pp.{pages} -> {path}')

    if not hits:
        return 0

    print(
        "[digimon-rules] This prompt touches Digimon TCG rules. The authoritative source "
        "is the Comprehensive Rules Manual — read the cited pages with the Read tool before "
        "reasoning. Cheap quick-reference: docs/digimon-rules/keyword-semantics.md; full "
        "verified digest via `/digimon-rules deep`."
    )
    print("\n".join(hits))
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `python -m pytest .claude/hooks/test_digimon_rules_hint.py -v`
Expected: PASS (4 passed). If `test_keyword_match_emits_pointer` fails because `rules-index.json` lacks a `blocker` keyword, fix Task 2's index (it must contain Blocker).

- [ ] **Step 5: Make the hook executable + commit**

```bash
chmod +x .claude/hooks/digimon_rules_hint.py 2>/dev/null || true
git add .claude/hooks/digimon_rules_hint.py .claude/hooks/test_digimon_rules_hint.py
git commit -m "Add conservative UserPromptSubmit rules-hint hook + tests"
```

---

## Task 5: SessionStart hook `digimon_rules_baseline.py`

**Files:**
- Create: `.claude/hooks/digimon_rules_baseline.py`

- [ ] **Step 1: Implement the hook**

```python
#!/usr/bin/env python3
"""SessionStart hook: inject a compact Digimon TCG keyword-semantics baseline plus
a pointer to deeper resources, so the assistant has light rules awareness from turn
one. Reads the committed docs/digimon-rules/keyword-semantics.md (single source of
truth) and prints it with a short banner. Silent if absent. No Pinecone / network.
"""
from __future__ import annotations

import sys
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    try:
        text = (repo_root() / "docs/digimon-rules/keyword-semantics.md").read_text(encoding="utf-8")
    except Exception:
        return 0  # artifact not present (e.g. older branch) -> stay silent
    print(
        "[digimon-rules] Baseline Digimon TCG rules awareness (compact keyword table below). "
        "For a specific rule/keyword invoke `/digimon-rules <query>` (reads the exact PDF pages); "
        "to load the full deep digest and act as a TCG thinking partner invoke `/digimon-rules deep`. "
        "Authoritative PDFs resolve under the base repo's `Digimon TCG resources/` (rule 29-style base path)."
    )
    print()
    print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 2: Smoke-test the hook**

Run: `echo '{}' | python .claude/hooks/digimon_rules_baseline.py`
Expected: the banner line, a blank line, then the contents of `keyword-semantics.md`.

- [ ] **Step 3: Commit**

```bash
chmod +x .claude/hooks/digimon_rules_baseline.py 2>/dev/null || true
git add .claude/hooks/digimon_rules_baseline.py
git commit -m "Add SessionStart rules-baseline hook (compact keyword table)"
```

---

## Task 6: `/digimon-rules` skill

**Files:**
- Create: `.claude/skills/digimon-rules/SKILL.md`

- [ ] **Step 1: Write the skill**

```markdown
---
name: digimon-rules
description: Use when reasoning about, implementing, debugging, or QA'ing Digimon TCG RULES, KEYWORDS, or TIMING — or when the user wants a deep TCG thinking partner. Resolves a keyword / rule number / topic to the exact authoritative rules-manual pages and reads them; `deep` mode loads the full verified rules digest. Triggers on keyword semantics ("is Save optional?", "how does Piercing resolve?"), rule numbers (16-36), timing/processing-order questions, or "be my rules thinking partner". Reads local files + the base-repo PDF only — no Pinecone/network.
---

# Digimon TCG Rules Lookup

Authoritative source: the Comprehensive Rules Manual. PDFs live **base-only** (rule 29):

    BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
    # general_rule.pdf  -> timing / keyword / effect rules (authoritative text)
    # glossary.pdf      -> keyword definitions
    # manual.pdf        -> image-heavy, visual / UI reference

Committed quick-reference (in every worktree): `docs/digimon-rules/keyword-semantics.md`,
`docs/digimon-rules/rules-index.json`, `docs/digimon-rules/digest.md`.

## Mode A — Lookup (default): `/digimon-rules <keyword | rule-# | topic>`

1. Open `docs/digimon-rules/rules-index.json`. Find the entry whose `names` (keywords/topics)
   match the query, or — for a rule number like `16-36` — the enclosing `sections` entry.
2. Resolve `BASE` (command above). `Read` the entry's `pdf` at its `pages` (use the Read
   tool's `pages` arg, e.g. `pages: "33-40"`).
3. Answer from the **printed rule text**, citing the rule number. Cross-check `keyword-semantics.md`
   for the optional/mandatory kind. If the printed text is terse, also read `glossary.pdf`.
4. Behavior questions about a specific *card* still defer to DCGO C# / the card image (CLAUDE.md
   source priority) — this skill is for *rules*, not card-specific resolution.

## Mode B — Deep / thinking partner: `/digimon-rules deep`

`Read` the whole of `docs/digimon-rules/digest.md` into context, then act as a sparring/thinking
partner who knows how the game is actually played. For any specific rule you cite, open the
underlying PDF pages (via `rules-index.json`) to confirm before asserting.

## Rules
- Read files + the base-repo PDF only. Never query Pinecone or the network.
- Never assert a rule from memory — cite the page you read.
- If an artifact is missing (older branch), fall back to reading `general_rule.pdf` directly via `BASE`.
```

- [ ] **Step 2: Sanity-check the frontmatter parses**

Run: `python -c "import re,sys; t=open('.claude/skills/digimon-rules/SKILL.md',encoding='utf-8').read(); assert t.startswith('---') and t.count('---')>=2; print('frontmatter OK')"`
Expected: `frontmatter OK`

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/digimon-rules/SKILL.md
git commit -m "Add /digimon-rules skill (lookup + deep thinking-partner modes)"
```

---

## Task 7: Wire the hooks into committed `.claude/settings.json`

**Files:**
- Create: `.claude/settings.json`

- [ ] **Step 1: Create the committed settings file**

Note: this MERGES with the base-only `.claude/settings.local.json` (pm briefing + card-image hooks) — it does not replace them. Putting our hooks in the committed file is what makes them fire in every worktree.

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "python .claude/hooks/digimon_rules_baseline.py" }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "python .claude/hooks/digimon_rules_hint.py" }
        ]
      }
    ]
  }
}
```

- [ ] **Step 2: Validate JSON + echo-test both hook commands exactly as configured**

Run:
```bash
python -c "import json; json.load(open('.claude/settings.json')); print('settings.json OK')"
echo '{"prompt":"How does Blocker work with rule 16-36?"}' | python .claude/hooks/digimon_rules_hint.py
echo '{}' | python .claude/hooks/digimon_rules_baseline.py | head -5
```
Expected: `settings.json OK`; the hint hook prints a `[digimon-rules]` block citing `general_rule.pdf`; the baseline hook prints its banner + table head.

- [ ] **Step 3: Confirm firing in a real session (manual)**

Open a fresh Claude Code session in this worktree. Confirm the SessionStart baseline appears in the first turn's context, then submit a prompt containing "Blocker" and confirm the JIT hint fires. (If they don't fire, the project may be resolving only `settings.local.json`; in that case also add the same two hook entries to the base repo's `.claude/settings.local.json` as a fallback.)

- [ ] **Step 4: Commit**

```bash
git add .claude/settings.json
git commit -m "Wire SessionStart + UserPromptSubmit rules hooks (committed settings)"
```

---

## Task 8: Retire `RULES_CONTEXT.md` + update CLAUDE.md

**Files:**
- Modify: `docs/RULES_CONTEXT.md` (replace body with a pointer)
- Modify: `CLAUDE.md` (add rule 32; update Source-priority item #5)

- [ ] **Step 1: Replace `docs/RULES_CONTEXT.md` with a pointer**

Overwrite the whole file with:
```markdown
# Digimon TCG Rules Reference — MOVED

This LLM-generated decomposition has been **retired** in favor of verified,
PDF-derived artifacts (2026-06-16):

- `docs/digimon-rules/keyword-semantics.md` — compact keyword table (verified §16).
- `docs/digimon-rules/rules-index.json` — keyword/topic/rule-# → exact PDF pages.
- `docs/digimon-rules/digest.md` — deep rules digest (verified, cited).

Authoritative source remains `Digimon TCG resources/general_rule.pdf` (base repo).
Use `/digimon-rules <query>` to read the right pages, or `/digimon-rules deep` to
load the full digest. See CLAUDE.md "Source priority" and rule 32.
```

- [ ] **Step 2: Append rule 32 to CLAUDE.md**

Find the end of rule 31 (the block beginning `31. **Rust build isolation`) and insert immediately after it:
```markdown
32. **Rules manual lives in the base repo; worktrees read verified derivations + resolve the PDF from base (2026-06-16).** The official rules PDFs (`Digimon TCG resources/general_rule.pdf`, `glossary.pdf`, `manual.pdf`) are git-ignored and exist **only in the base repo** (like DCGO, rule 29) — they are NOT in worktrees. Resolve them via `BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"` then `Read "$BASE/Digimon TCG resources/<file>.pdf"`. Routing: `general_rule.pdf` for timing/keyword/effect/engine work, `glossary.pdf` for keyword definitions, `manual.pdf` for visual/UI reference. For the cheap first stop, read the **committed** verified derivations in `docs/digimon-rules/` (`keyword-semantics.md`, `rules-index.json`, `digest.md`) — present in every worktree — and drill into the PDF for ground truth. A SessionStart hook injects the compact keyword table each session; a conservative UserPromptSubmit hook (`.claude/hooks/digimon_rules_hint.py`) points at the exact PDF pages when a prompt names a keyword or rule number; and the `/digimon-rules` skill does targeted lookups + a deep thinking-partner load. None of this uses Pinecone.
```

- [ ] **Step 3: Update the Source-priority list (item #5)**

In CLAUDE.md's "Source priority for card / keyword / rules questions" section, replace the `5. **docs/RULES_CONTEXT.md**` bullet with:
```markdown
5. **`docs/digimon-rules/`** (committed, verified) — `keyword-semantics.md` / `digest.md` / `rules-index.json` are PDF-derived and cited; trustworthy as a cheap first stop and as page-pointers into the canonical PDF (#1). They **replace** the retired LLM-generated `docs/RULES_CONTEXT.md`. Still verify anything load-bearing against the PDF (#1) and DCGO (#2).
```

- [ ] **Step 4: Verify the edits**

Run:
```bash
grep -n "^32\." CLAUDE.md
grep -n "docs/digimon-rules" CLAUDE.md
grep -c "RULES_CONTEXT.md is canonical" docs/RULES_CONTEXT.md   # expect 0 (old claim gone)
```
Expected: rule 32 present; `docs/digimon-rules` referenced in CLAUDE.md; the stale "canonical" claim is gone from the pointer file.

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md docs/RULES_CONTEXT.md
git commit -m "Retire RULES_CONTEXT.md for verified docs/digimon-rules; add CLAUDE.md rule 32"
```

---

## Self-Review (completed by plan author)

- **Spec coverage:** Artifacts (Tasks 1–3) ✓; SessionStart baseline (Task 5) ✓; conservative prompt hook (Task 4) ✓; `/digimon-rules` lookup+deep skill (Task 6) ✓; reachability convention / CLAUDE.md rule 32 (Task 8) ✓; retire RULES_CONTEXT.md (Task 8) ✓; no-Pinecone (asserted in hook/skill code + skill frontmatter) ✓; wiring in committed settings (Task 7) ✓.
- **Placeholder scan:** Data-artifact tasks (1–3) intentionally specify *process + exact schema/format + verification* rather than pre-baked content, because the content IS the verification work (reading the PDF) and must not be hallucinated. All *code* steps contain complete code.
- **Type consistency:** `rules-index.json` schema (`source`, `pdfs`, `sections`, `keywords`, `topics`, each entry with `names`/`section`/`pdf`/`pages`) is defined in Task 2 and consumed identically by the hook in Task 4 and the skill in Task 6. `repo_root()`/`base_resources_dir()` names are consistent across both hooks. `DIGIMON_RULES_PDF_DIR` env override used by the hook (Task 4) is the same var the test (Task 4 Step 1) sets.

## Notes / risks carried from the spec
- Hook firing in worktrees depends on Claude Code reading the committed `.claude/settings.json` for the project; Task 7 Step 3 verifies this and gives a `settings.local.json` fallback.
- Artifacts carry a version stamp; a future manual revision (Ver.3.7) is a re-verification task.
- Building the index/digest requires careful PDF reading — this is one-time and is the point (trustworthiness).
