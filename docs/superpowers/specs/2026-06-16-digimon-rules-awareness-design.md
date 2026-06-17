# Design: Deeply rules-aware-by-default workflow

- **Date:** 2026-06-16
- **Status:** Approved (brainstorming) — pending implementation plan
- **Topic:** Make Claude reliably knowledgeable about Digimon TCG rules every session
- **Author:** brainstorming session with james

## Problem

The official rules manual is the project's canonical source of truth for rules,
keyword semantics, and timing (CLAUDE.md "Source priority", rule #1). Three PDFs
exist in the base repo's `Digimon TCG resources/`:

- `general_rule.pdf` — Comprehensive Rules Manual Ver.3.6 (2025/12/25), ~41 pp.
  text, the authority for timing/keyword/effect rules.
- `glossary.pdf` — keyword definitions.
- `manual.pdf` — Official Rule Manual for Web Ver.5.0, ~52 MB, image-heavy
  (good for UI/visual reference).

Despite CLAUDE.md telling the assistant to read it, the assistant rarely does.
This is **structural, not a discipline problem**:

1. **Invisible from worktrees.** All three PDFs are git-ignored (deliberately —
   the user does not want a 52 MB binary copied into every worktree). CLAUDE.md
   instructs `Read "Digimon TCG resources/general_rule.pdf"`, but in a worktree
   that relative path does not resolve, the read fails, and the assistant falls
   back to lower-trust sources. DCGO has an explicit rule-29 base-repo-resolution
   convention; the rules PDFs have **no equivalent**.
2. **Nothing nudges toward it.** The `digimon_card_image_hint.py`
   UserPromptSubmit hook fires on card IDs and injects image paths — a proven
   pattern — but there is no analogous hook for *rules/keyword* vocabulary.
3. **The cheap fallback is explicitly distrusted.** `docs/RULES_CONTEXT.md`
   (779 lines) is the LLM-generated decomposition. CLAUDE.md demotes it ("has
   been wrong, convenience index only"), yet the file's own header still calls
   itself "canonical" — a stale internal contradiction. The one cheap-to-read
   artifact is one the assistant is told not to trust; the trustworthy one is
   unreachable.
4. **No targeting.** Even when reachable, a PDF read is page-limited and
   expensive, and there is no topic→page-range index pointing at *which* pages
   answer a given question.

## Goals

- The assistant has **light baseline rules awareness from turn one** of every
  session, with **no required tool call**.
- The assistant can **reliably reach the authoritative PDF from any worktree**
  and read the **right pages** cheaply.
- The assistant is **proactively reminded** of the authoritative source when a
  prompt genuinely touches rules — without relying on it remembering.
- The user can **opt into a deep "thinking-partner" mode** that loads fuller
  game knowledge into context on demand.
- There is **one trusted rules decomposition**, verified against the PDF.

## Non-goals

- Copying any PDF into worktrees, or un-git-ignoring the binaries.
- Forcing the full deep digest into context every session (recurring token cost
  the user explicitly does not want).
- Replacing DCGO / `general_rule.pdf` as the *behavioral* / *rules* authority.
  The new artifacts are derived, cited, verified pointers — not a new authority.

## Constraints

- PDFs stay base-only and git-ignored.
- Solution must work identically from any linked worktree (rule 29 environment).
- Per-session passive cost must stay small (light baseline only).
- **No Pinecone / vector-DB / MCP dependency.** The hooks and the skill read the
  committed `docs/digimon-rules/*` artifacts and the base-repo PDF *directly* via
  the `Read` tool. Nothing here queries Pinecone (contrast `/implement-archetype`,
  which does). The workflow must function with `PINECONE_API_KEY` unset.

## Core principle — separate the *authoritative* from the *reachable*

Keep the heavy PDFs base-only. Derive **small, hand-verified text artifacts**
from them and **commit those**, so they exist in every worktree. Day-to-day the
assistant reads the committed derivations (fast, always reachable); for
ground-truth it reads the base-repo PDF via a documented resolution path. This
fixes "invisible in worktrees" without moving binaries.

```
Authoritative (base-only, git-ignored)        Reachable (committed, in every worktree)
  Digimon TCG resources/general_rule.pdf  ──►  docs/digimon-rules/keyword-semantics.md
  Digimon TCG resources/glossary.pdf      ──►  docs/digimon-rules/rules-index.json
  Digimon TCG resources/manual.pdf        ──►  docs/digimon-rules/digest.md
        ▲ read for ground-truth via $BASE path           ▲ read day-to-day
```

## Component 1 — Committed artifacts (built once, verified against the PDF)

All three carry a **version stamp** in their header (source PDF name + version +
"Last Updated" date) so staleness is visible when the manual revises (e.g.
Ver.3.7). **Verification requirement:** content is produced by *reading the
actual PDF pages*, not from memory or from `RULES_CONTEXT.md`. Every assertion
cites a rule number that can be checked against the PDF; index page ranges are
spot-checked by opening each cited range and confirming the section header.

1. **`docs/digimon-rules/keyword-semantics.md`** — the compact spine. One row per
   keyword: name, **optional/mandatory** processing kind, what it targets,
   one-line semantics, citing rule § (e.g. `16-36`). Source: §16 "Keyword
   Effects" (pp. 33–40) + `glossary.pdf`. Sized to be cheap to inject every
   session (~50–80 lines).
2. **`docs/digimon-rules/rules-index.json`** — topic / keyword / rule-number →
   `{ pdf, pages, section }`. Seeded from the verified TOC of `general_rule.pdf`
   (§15 Effect Rules pp. 22–32, §16 Keyword Effects pp. 33–40, §8 Digivolution
   p. 16, §11 Attacking pp. 19–20, §13 Security Checks p. 21, etc.), then drilled
   for per-keyword page offsets within §16. This is what makes every read
   *targeted* instead of "scan 41 pages."
3. **`docs/digimon-rules/digest.md`** — the fuller deep-knowledge doc: turn/phase
   structure, memory mechanics, digivolution & cost rules, security/checking
   flow, battle resolution, and the timing windows + interaction gotchas that
   bite most often. This is the "deep thinking-partner" content loaded on demand.
   It **supersedes `docs/RULES_CONTEXT.md`** (see Component 4).

## Component 2 — Delivery layer: SessionStart baseline (automatic, light)

A `SessionStart` hook injects, every session:

- the compact keyword table from artifact #1, and
- a short banner: "Deeper Digimon TCG rules knowledge is available — invoke
  `/digimon-rules` to look up a specific rule, or `/digimon-rules deep` to load
  the full digest and act as a deep TCG thinking partner. Authoritative PDFs
  resolve at `$BASE/Digimon TCG resources/`."

This gives turn-one baseline awareness and tells the assistant how to go deeper.
It does **not** inject the full digest (per the user's preference: baseline light,
depth opt-in).

## Component 3 — Delivery layer: UserPromptSubmit hook (automatic, JIT depth)

`.claude/hooks/digimon_rules_hint.py`, modeled directly on
`digimon_card_image_hint.py` (read JSON payload from stdin, scan the prompt,
print a context block, stay silent otherwise). **Conservative tuning** (chosen):
fires only on high-signal tokens — explicit keyword names (Blocker, Piercing,
Jamming, Save, Fragment, Progress, Alliance, Collision, …) and rule-number
patterns (`\b\d{1,2}-\d(?:-\d+)*\b`, e.g. `16-36`). It does **not** fire on bare
common verbs like "digivolve"/"attack". On a match it injects, for each hit: the
matching `rules-index` entry (which PDF, which pages, base-repo-resolved absolute
path, section name) + "read this before reasoning about the rule." Keyword token
list and trigger regex are easy to widen later.

## Component 4 — Delivery layer: `/digimon-rules` skill (on-demand)

A new skill under `.claude/skills/digimon-rules/`, sibling to
`digimon-card-lookup`. Two modes (one skill, to keep the surface small and match
the card-lookup pattern):

- **Lookup mode** (default): given a keyword / rule number / topic, resolve via
  `rules-index.json`, then `Read` the exact PDF pages (base-repo path) and the
  relevant `digest.md` section. Returns the cited rule text.
- **Deep / "coach" mode** (`/digimon-rules deep`): load the full `digest.md` into
  context to prime the assistant as a sparring/thinking partner who knows how the
  game is actually played. This is the user's opt-in "deeply aware thinking
  partner."

Both modes use **only** the `Read` tool against the committed artifacts and the
base-repo PDF — **no Pinecone**, no MCP, no network. The skill works with
`PINECONE_API_KEY` unset.

## Component 5 — Retire `RULES_CONTEXT.md`; reachability convention

- **Retire `docs/RULES_CONTEXT.md`.** Once `digest.md` exists and is verified,
  replace `RULES_CONTEXT.md` with a one-line pointer to the new artifacts so
  there is a single trusted decomposition. Update CLAUDE.md's "Source priority"
  list (item #5) accordingly. **Pinecone is not part of this workflow** — if the
  optional `rules-docs` namespace is used elsewhere it may later want a
  re-ingest, but that is out of scope and nothing here depends on it.
- **New CLAUDE.md rule (~rule 32), parallel to rule 29 (DCGO).** Document that
  the rules PDFs resolve from a worktree to
  `"$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/Digimon TCG resources/<file>.pdf"`,
  with routing: `general_rule.pdf` for timing/keyword/effect/engine work,
  `glossary.pdf` for keyword definitions, `manual.pdf` for visual/UI reference.
  Note that the committed `docs/digimon-rules/*` artifacts are the cheap
  first-stop and the PDF is the ground-truth deep-dive.

## Decisions made (with rationale)

- **Combination of all layers**, not a single mechanism — the user wants both a
  baseline and on-demand depth.
- **Baseline light, depth opt-in** — user: "you don't need a full digest every
  session start but I'd like the option of a thinking partner who is deeply
  aware." SessionStart injects only the compact table; the digest loads on demand.
- **Conservative hook tuning** — rules vocabulary is fuzzier than card IDs;
  start low-noise and widen later if coverage feels thin.
- **One skill with two modes**, not two skills — smaller surface, matches
  `/digimon-card-lookup`.
- **Retire `RULES_CONTEXT.md`** rather than keep two overlapping decompositions —
  eliminates the "trusted vs distrusted" ambiguity.

## Implementation notes

- **Committed vs local:** the artifacts (`docs/digimon-rules/*`), the hook script
  (`.claude/hooks/digimon_rules_hint.py`), and the skill
  (`.claude/skills/digimon-rules/`) are tracked → committed → propagate to all
  worktrees via main. The **hook *wiring*** lives in `.claude/settings.local.json`
  (base repo, git-ignored) — registering the SessionStart + UserPromptSubmit
  hooks there is a local config step the plan must call out separately from the
  committed code.
- **Hook robustness:** the rules hook must resolve `$BASE` the same way rule 29
  does and degrade silently (print nothing) on any error, exactly like
  `digimon_card_image_hint.py`.
- **Encoding:** card text / rule text contains fullwidth chars — the hook must
  `reconfigure(encoding="utf-8")` as the card-image hook does.

## Risks & mitigations

- **Artifact drift when the manual revises.** Mitigation: version stamp in each
  artifact header; the index records the source PDF version. A future manual
  update is a re-verification task, surfaced by the stamp.
- **Hook noise.** Mitigation: conservative trigger set; widen only if coverage
  proves thin in practice.
- **Verification cost.** Building the index/digest requires careful PDF reading.
  This is one-time and is the whole point (trustworthiness). The TOC makes it
  tractable.

## Out of scope / future

- Auto-detecting manual version bumps and regenerating artifacts.
- Wiring `manual.pdf` page images into UI-design workflows beyond the routing note.
- Folding the digest into the Pinecone `rules-docs` namespace. Pinecone is
  explicitly **out of scope** — this workflow neither requires nor touches it.

## Verification / testing approach

- Index page ranges spot-checked against the live PDF section headers.
- Hook unit-tested with sample prompts (keyword hit, rule-number hit, no-match
  silence), mirroring how the card-image hook behaves.
- A manual session smoke check: open a fresh session, confirm the SessionStart
  baseline appears; submit a keyword prompt, confirm the JIT hint fires;
  invoke `/digimon-rules deep`, confirm the digest loads.
