---
name: archetype-campaign
description: Run a full archetype campaign on an oracle node — resolve the archetype to its card pool, implement the cards that have no YAML spec, exam the clauses that have no verdict against the DCGO oracle, triage divergences, and leave a ledger entry so no other node repeats the work. Triggers on "run a campaign on <archetype>", "do <archetype> end to end", "implement and exam <archetype>", dispatching an archetype as a job, or resuming a crashed campaign. Composes /batch-implement-cards-rust-dsl and /dcgo-exam; re-implements neither.
argument-hint: <archetype> [--exam-only] [--core-fraction 0.7]
---

# Archetype Campaign

You take **one archetype** and drive it to a stated finish line on a node that
has its own DCGO oracle. The output is a per-clause verdict table over the full
denominator plus a ledger entry — never "archetype done".

This is the dispatch unit the fleet is built around. Everything it needs is
resumable from the ledger, so a crashed campaign is re-dispatched, not restarted.

## Non-negotiables — read before acting

- **Preflight first.** Run `node_health` before authoring anything. Authoring
  costs real money; discovering afterwards that the oracle was never going to
  answer wastes all of it. A NO-GO stops the campaign — report it, do not
  author "in the meantime".
- **`--sim-only` is not confirmation.** It proves a line is legal in our engine
  and says nothing about DCGO's prompt sequence. Six sim-green scenarios were
  put to the oracle in the first campaign and **all six failed, every one on
  prompt sequence.** Only an oracle pass moves a clause to `confirmed`.
- **Always print the full denominator.** An archetype reads as
  `Hunters — 166 clauses: 107 confirmed, 0 diverged, 5 unreachable, 54 unmeasured`,
  never "Hunters passed". If anything is `unmeasured`, `unavailable`, or
  `unreachable`, say so in the **first sentence** of your summary.
- **`diverged` is a finding to TRIAGE, not proof our engine is wrong.**
  `general_rule.pdf` outranks DCGO. Read the printed card text and the rule
  before concluding anything.
- **Claim before you author.** Archetype pools overlap — one archetype's cards
  can be a strict subset of another's. Claiming is what stops two nodes doing
  the same card twice.
- **Never re-implement a card that already has a spec.** The plan's `implement`
  list is the authority; if it looks wrong, stop and check the resolver rather
  than authoring over existing work.

## Phase 0 — Preflight

```
node_health(build=<player dir>)
```

Every check must be `ok` or `warn`. On any `fail`, report the check and its
remedy and **stop**. The most common `fail` is `action_space`: the player
encodes against a dead action space and its recordings would read as engine
divergence. That needs a rebuild on the build machine, not a retry here.

## Phase 1 — Resolve and bind

```bash
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "<NAME>" --json
```

Read four things before doing anything else:

- `core` — `{cards, threshold, list_count}`. **This is your finish line.**
- `implement` — cards with no YAML spec. They cannot be examined yet.
- `exam` — outstanding clauses, core-first.
- `skipped` — confirmed or `unavailable`, with reasons. Report these; never
  silently drop them.

Two verified worked examples, so you know what the output actually looks like:

- `--archetype "Toho Braves"` → 42 cards, core 18 (≥32 of 45 lists), `implement`
  0, `exam` 56, `skipped` 110.
- `--archetype "Hunters"` → 65 cards, core 16 (≥19 of 27 lists), `implement` 42,
  `exam` 88, `skipped` 3.

An unknown archetype raises with near-misses. Use one of them; do not invent a
name.

## Phase 2 — Claim

```
claim(cards=[...the plan's implement + examinable cards...], job_id="<archetype>-<n>",
      archetype="<NAME>", node="<node name>")
```

Work only what was `granted`. For anything `held_by_others`, say who holds it in
your report — that is the fleet coordinating, not an error.

Claims are **advisory**: simultaneous pushes can both claim. If you find a
duplicate at merge, that is the known trade, not a bug.

## Phase 3 — Implement the missing cards

For the `implement` list, use **`/batch-implement-cards-rust-dsl`**. Do not
author cards inline here — that skill owns the TDD flow, the DSL-first rule, and
the verdict tracking, and duplicating it would drift.

Gaps route to their existing trackers: DSL vocabulary to `qa/dsl-vocab-gaps.md`,
engine primitives to `docs/RUST_ENGINE_GAPS.md`. **Widen the substrate rather
than routing around it** (CLAUDE.md rule 28) — a card implemented by
approximation fails the exam anyway, and now silently.

## Phase 4 — Exam, with the oracle in the loop

For each outstanding clause, core first:

1. `exam_keyword_brief(keyword)` for each keyword the clause's text prints. The
   **kind predicts the prompt shape**: `Opt-cost→Mand` means DCGO asks (the line
   needs an `expect:` row); `Mandatory` means no prompt at all (an `expect:` row
   there desynchronizes everything after it).
2. Compose the line. `exam_authoring_guide(topic)` for the part you need —
   `format`, `steps`, `prompts`, `decks`, `assert`, `verdicts`.
3. `exam_validate(yaml)` — milliseconds, catches the orphan clause id, an
   unstacked card, a prompt kind outside the 13.
4. `exam_probe(yaml, sim_only=true)` — does it lower? This is the only working
   mode today. **`exam_probe(sim_only=false)` is not currently possible** — it
   returns a clear, actionable error rather than an oracle answer, because an
   oracle result needs a DCGO state sidecar written next to a real recording,
   and a scratch scenario has no Unity trace behind it. This is logged as
   `G-TOOLING-EXAM-PROBE-NO-ORACLE-MODE` in `docs/RUST_ENGINE_GAPS.md`. Do not
   call it expecting an oracle answer — it will error, not confirm, and the
   error is not a sign your scenario is wrong.
5. Fix and repeat until the scenario lowers cleanly. A clean sim-only result is
   **not** confirmation — it says nothing about DCGO's prompt sequence, which is
   where lines actually break.
6. Commit the scenario file (`qa/dcgo-exams/<SET>/<CARD-ID>.yaml`) and get the
   oracle answer through the **existing route**: submit it through the harness
   queue and run **`/dcgo-exam`**, which drives the run and records the verdict.
   That composition — not an in-loop probe — is how a clause reaches
   `confirmed` today.

## Phase 5 — Triage divergences

For each `diverged`, in this order:

1. The **printed card text** — the image / official bundle (`/digimon-card-lookup`).
2. The governing rule in `general_rule.pdf` (`/digimon-rules`). **The PDF
   outranks DCGO.**
3. The DCGO C# at `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`
   (underscored filename).

Classify: **our bug** / **DCGO quirk** / **rules-ambiguous**.

A fix may land autonomously **only** with all three of: a citation to the rule §
or DCGO C# it rests on; a test that fails before and passes after; and
`cards_behavioral` green. **Card/YAML fixes** proceed under that gate. **Engine
fixes** proceed under it but land on their own branch and are flagged for human
review. Anything you cannot justify by citation is a **logged finding, not a
fix**.

## Phase 6 — Done, reported, and released

The finish line is **not** "every clause confirmed":

> **Every core clause is adjudicated** — `confirmed`, or carrying a named,
> *measured* reason — **and zero untriaged `diverged`.**

Pool coverage is reported, not gated. The tail of 1-of tech cards is real work
but it is not this campaign's gate; grinding it at $8+/clause is how a campaign
never ends. (Toho Braves landed at core 69/74 with each remaining clause
carrying a measured cause — that is a finished campaign, not an incomplete one.)

Then:
- Append one line per attempt to `qa/qa-reports/exam-log.jsonl`.
- Regenerate the index: `python -m tools.clause_coverage.exam_index`.
- `release(cards=[...], job_id=...)`.
- Report the table, denominator first.

## Red flags — STOP

- About to author before `node_health` returns GO → STOP.
- About to treat a sim-green scenario as confirmation → STOP. Only the oracle confirms.
- About to call `exam_probe(sim_only=false)` expecting an oracle answer → STOP.
  It errors today; use the harness queue + `/dcgo-exam` route instead.
- About to report the archetype without printing `unmeasured` → STOP.
- About to call a `diverged` clause an engine bug before reading `general_rule.pdf` → STOP.
- About to re-implement a card that already has a YAML spec → STOP; check the resolver.
- About to fix the engine without a rule citation and a failing-then-passing test → STOP.
- About to grind the support-card tail while a core clause is unmeasured → STOP. Core first.

## Reference

- Fleet design: `docs/superpowers/specs/2026-08-27-archetype-campaign-fleet-design.md`
- Exam manual: `docs/DCGO_EXAM.md`; node runbook: `docs/runbooks/oracle-node.md`
- Tooling gap: `docs/RUST_ENGINE_GAPS.md` (`G-TOOLING-EXAM-PROBE-NO-ORACLE-MODE`)
- Composes: `/batch-implement-cards-rust-dsl`, `/dcgo-exam`
- What to dispatch next: `qa/qa-reports/exam-index.md`, and the ranked shortlist
  in `docs/superpowers/specs/2026-08-22-unimplemented-winning-decks.md`
