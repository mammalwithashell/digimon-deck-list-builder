# Archetype Campaigns on an Oracle Fleet

Turns the Toho Braves exam — a hand-driven, human-attended campaign — into a
**dispatchable job**: pick an archetype, hand it to a warm node running its own
DCGO oracle, and get back implemented cards, confirmed clauses, and a ledger
entry that stops anyone repeating the work.

Builds on:
- `docs/superpowers/specs/2026-08-21-dcgo-scripted-scenario-exam-design.md` (the exam itself)
- `docs/superpowers/specs/2026-08-20-dcgo-agent-puppet-design.md` (Layer A: build, manifest, daemon)
- `docs/superpowers/specs/2026-08-22-unimplemented-winning-decks.md` (what to dispatch next)

Operating manuals: `docs/DCGO_EXAM.md`, `docs/DCGO_HARNESS.md`.

## Problem

The exam works. Toho Braves went from nothing to **107 of 166 clauses confirmed,
0 diverged, and a competitive core at 69/74 (93%)** — while surfacing a dozen
real faithfulness bugs (`<Decode>` offering its optional processing twice,
`<Retaliation>` as a phantom branch on non-battle deletions, five EX12 clauses
firing cross-permanent, 13 auto-paid optional costs that never offered the
decline, `<Delay>` auto-trashing a window owned elsewhere, mass auras installing
a granted keyword's flag but not its triggers).

But it ran as one continuous human-attended session on one machine. Three things
block repeating it on the next seven archetypes:

1. **No unit of dispatch.** "Do Hudiemon" is not a thing you can hand to a node.
2. **No fleet-safe ledger.** `qa/qa-reports/dcgo_exam_verdicts.json` is a single
   JSON blob; two nodes writing it collide on every clause.
3. **No node.** The oracle is one Unity install on one desktop, started by hand.

## What already exists

| Piece | State |
|---|---|
| `dcgo-harness` queue, `build`, manifest + action-space hash gate, `up`/`down`/`watch` | Built |
| Scripted input driver in DCGO — both seats, deck stacking, state sidecar | Built |
| `exam/` — scenario, lower, adapter, differ, projection, verdict, backfill, drafter | Built |
| `clause_coverage.exam_binding.bind()` — clauses × scenarios × verdicts, five classes | Built |
| `/dcgo-exam` skill, `docs/DCGO_EXAM.md`, sim-only PR gate | Built |
| 145 scenarios, 148 verdict rows across 35 cards | Built |
| `exam/mcp.rs` (Task 7 of the workflow plan) | **Never built** |
| Fleet-safe ledger, claims, job dispatch, node bootstrap | **This spec** |

## Findings that drive the design

Each of these was measured during this design pass, not assumed.

**F1 — The built player is 492 MB and needs no Unity license to run.**
`D:\dcgo-build\scripted-v7\` is `DCGO.exe` + `DCGO_Data` + `UnityPlayer.dll`.
The "multi-GB licensed LFS checkout" is the *project* (4.3 GB), not the artifact.
Building requires a license; running does not. So: build once locally, ship the
artifact, run anywhere.

**F2 — DCGO's C# source is 53 MB; the rules PDFs are 1 MB.**
`Assets/Scripts` is 53 MB (of which `CardEffect` is 48 MB) out of 4.3 GB.
`general_rule.pdf` is 975 KB, `glossary.pdf` 53 KB (`manual.pdf` is 52 MB and is
UI reference — not needed on a node). A node can therefore carry everything
source-priority #1 and #2 require for ~54 MB.

**F3 — Windows is pinned in exactly three shallow places.**
`HarnessBuild.cs` (`BuildTarget.StandaloneWindows64`, `ExecutableName = "DCGO.exe"`);
`daemon.rs` (`tasklist` / `taskkill`); and the player is launched with only
`-logFile <path>` — no `-batchmode -nographics` — so today it wants a real
graphics device and an attached desktop session. None is architectural.

**F4 — `--sim-only` is not a proxy for the oracle.** From the 2026-08-26 report:
the corpus lowers 144/144 in our engine alone, and when six sim-green scenarios
were put to the oracle **all six failed — none on state divergence, every one on
prompt sequence.** Authoring without a live oracle produces corpora that are
100% sim-green and substantially oracle-red, discovered only in batch.
*This is the finding that forces oracle colocation.*

**F5 — Agent tokens dominate infrastructure by ~20–50×.** The full campaign
(2026-08-21 → 08-27, session `0bc53a23`) cost **$4,210 API-equivalent** —
11.3 M output tokens over 18,565 messages, priced at list (Opus 5 $5/$25 per
MTok, Fable 5 $10/$50, cache write 1.25× input, cache read 0.10× input). A
long-lived VM is $30–150/month. Infrastructure is a rounding error; **token
efficiency and oracle availability are the only things worth optimizing.**

Corollary: cold starts are paid in tokens (re-deriving the environment, waiting
on builds, re-reading runbooks), so **warm nodes are the cheap option**, not the
expensive one.

**F6 — Model choice is a 2× lever on authoring.** `toho-coverage-authoring` ran
all-Opus: 843 k output for **$180**. `toho-selection-wave` ran all-Fable:
850 k output for **$365**. Near-identical volume, double the price.

**F7 — Orchestration was the largest single line item: $704**, driven by 540 M
cache reads on the main thread — an agent shelling out, reading large outputs,
re-establishing context. This is what the MCP exists to cut.

**F8 — Substrate widening compounds.** Daily burn fell steadily as vocabulary
landed: $1,468 → $907 → $679 → $456 → $458.

**F9 — Archetype card pools overlap.** Beelstar's missing EX7 cards are a strict
subset of Three Musketeers'. Claims must be at *card* granularity; claiming
whole archetypes would either block legitimate work or duplicate shared cards.

## Architecture

```
        your machine                          a warm node (× N, as you spin them up)
  ┌───────────────────────┐            ┌──────────────────────────────────────────┐
  │ Unity + DCGO project  │  build     │  repo  +  player(492M)  +  C# mirror(53M) │
  │ (4.3 GB, licensed)    │ ─────────► │        +  rules PDFs(1M)  +  harness      │
  │ dcgo-harness build    │  artifact  │                                           │
  └───────────────────────┘            │   agent ──MCP──► dcgo-harness ──► oracle  │
                                       │     │                  ▲   local queue    │
                                       │     └── author/probe ───┘   (serial)      │
                                       └───────────────┬──────────────────────────┘
                                                       │ git branch + ledger
                                                       ▼
                                              ledger on main (merge point)
```

Two clock speeds, one merge point. Inside a node the oracle is a **single
serialized resource** — one scenario at a time, ~40 s each — fed by parallel
authoring. That is exactly what `<root>/{jobs,claimed,done,failed}` with
atomic-rename claiming already is, so the harness queue stays as-is and becomes
the node's internal scheduler. **It is never distributed.** The only thing shared
between nodes is the git-merged ledger.

### Component boundaries

| Component | Owns | Depends on |
|---|---|---|
| **Ledger** (`qa/qa-reports/exam-*`) | What is confirmed, what was attempted, what is claimed | Nothing — plain files |
| **`dcgo-harness exam` + MCP** | Running scenarios, verdicts, serving the authoring contract | Ledger, engine, local oracle |
| **`dcgo-harness node`** | Oracle lifecycle + health on one box | Manifest, player artifact |
| **`/archetype-campaign` skill** | The job: plan → implement → exam → triage → report | MCP, existing card skills |

Each is testable alone: the ledger is file I/O, the MCP is a thin projection over
`bind()` + the differ, `node` is process management, and the skill is prose over
the other three.

## 1. The ledger

Three files, each shaped by how it merges.

### 1.1 Verdicts — one file per card

`qa/qa-reports/exam-verdicts/<CARD-ID>.json`, replacing the single blob. Same
`ClauseVerdict` records, so `VerdictStore` gains a *directory loader*, not a new
schema. `bind()` already tolerates a missing store, so a fresh checkout is fine.

Per **card**, not per clause, because a card is the unit an author works on — its
clauses share a deck and a pool — and it keeps the tree near ~800 files rather
than ~4,000. Two nodes on different archetypes never touch the same file; two
nodes on overlapping archetypes touch the same file only for genuinely shared
cards (F9).

Migration is mechanical: split the existing 148 rows by `card_id`, delete the
blob, keep `version: 1` per file.

### 1.2 Attempt log — append-only

`qa/qa-reports/exam-log.jsonl`, one line per attempt:

```json
{"ts":"2026-08-27T14:02:11Z","job_id":"hunters-01","node":"oracle-a",
 "archetype":"Hunters","card":"BT12-042","clause":"BT12-042#effect#0",
 "verdict_before":"unmeasured","verdict_after":"confirmed",
 "scenario":"qa/dcgo-exams/BT12/BT12-042-effect0.yaml",
 "dcgo_build":"638f4070","outcome":"oracle_clean"}
```

Configured with a union merge driver (`.gitattributes`), so concurrent nodes
concatenate rather than conflict. The verdict store is *current state*; this is
*history*, and it answers the question `unmeasured` cannot: **has anyone already
tried this and given up, and why.** That is the difference between "nobody looked"
and "three nodes each burned $40 discovering the same dead end".

### 1.3 Claims — advisory leases at card granularity

`qa/qa-reports/exam-claims/<CARD-ID>.claim`:

```json
{"job_id":"hunters-01","node":"oracle-a","archetype":"Hunters",
 "claimed_at":"2026-08-27T13:40:00Z","expires_at":"2026-08-28T13:40:00Z"}
```

Written and pushed at job start, removed at job end, ignored once expired.

**Stated limitation:** a git-based claim is *advisory*. Two nodes pushing in the
same instant can both claim a card. We accept this deliberately — at ~$8 per
authored clause an occasional duplicate is far cheaper than a coordination
server, and the duplicate is *detectable* at merge (two verdicts for one clause,
normally agreeing). If it proves painful, the MCP is the natural place to front a
real lease later. It is not in scope now.

### 1.4 Human index — generated, never hand-edited

`qa/qa-reports/exam-index.md`, regenerated from ledger + `bind()`: a row per
archetype (total clauses, the five counts, % core adjudicated, last touched, open
findings) and per card beneath it. This is what you read to decide what to
dispatch next. A test asserts it is reproducible from the ledger, so a
hand-edit is a failing build rather than silent drift.

## 2. `/archetype-campaign <name>` — the job

**Phase 0 — Preflight.** Player present; `manifest.action_space_hash` equals the
engine's; harness enabled; one throwaway job drains. GO/NO-GO *before* a token is
spent on authoring. A stale player must refuse rather than silently encode against
a dead action space.

**Phase 1 — Resolve + bind.** Archetype → card pool (`deck_library.json`,
`archetype_aliases.json`) → `clause_coverage.extract` → `bind()` against the
ledger. Produces a work plan with **two kinds of work**:

- cards with **no YAML spec** → *implement*
- cards with YAML but unconfirmed clauses → *exam*

Skipping is by construction: `confirmed` with a matching `text_sha256` is not in
the plan; `unavailable` is excluded with its reason; `unreachable` is excluded
unless `--retry-unreachable`. Re-dispatching a crashed archetype re-binds and
resumes exactly the outstanding work, because **the ledger is the state**.

Each outstanding clause is tagged with the keywords its printed text carries, and
carries the keyword brief (§3.3) with it.

**Phase 2 — Claim** the plan's cards; push; drop any already leased.

**Phase 3 — Implement wave.** DSL-first TDD per rule 28, the
`/batch-implement-cards-rust-dsl` shape (scout → implement/audit → review).
Verdicts land in `validated_cards_dsl.json` as today. DSL gaps route to
`qa/dsl-vocab-gaps.md`, engine gaps to `docs/RUST_ENGINE_GAPS.md`.

**Phase 4 — Exam wave.** Author scenarios with the oracle *in the loop* (F4):
compose → `exam_validate` → `exam_probe` against DCGO → fix prompt sequence →
commit the file → sim-only gate → oracle pass → verdict → backfill confirmed
asserts. Parallel authors; the node's queue serializes the oracle.

**Phase 5 — Triage** each divergence: printed card text, then `general_rule.pdf`,
then the DCGO C#. Classify **our bug / DCGO quirk / rules-ambiguous**. Fixes are
governed by §4.

**Phase 6 — Report, log, regenerate index, push.**

### 2.1 Done is defined on the core

An archetype is never 100% — support 1-ofs and structurally unreachable clauses
guarantee a tail, and without a stopping rule an agent grinds it at $8+/clause.
The exit condition is:

> **Every core clause is adjudicated** — either `confirmed`, or carrying a named,
> *measured* reason — **and zero untriaged `diverged`.**

"Core" is a **fraction of the archetype's recorded tournament lists, not a raw
count**: cards appearing in ≥70% of them. Toho's report expresses this as
"≥33 of 45 lists", which is that fraction for a 45-list corpus; hardcoding 33
would silently redefine the core for archetypes with more or fewer lists. The
threshold is a job parameter defaulting to 0.7, and the resolved absolute count
is printed in every report so the denominator is never implicit.

Pool coverage is *reported*, never gated. Toho landed exactly here: core 69/74
confirmed with each of the remaining 5 blocked on a measured cause, pool 107/166.

## 3. The MCP surface

`dcgo-harness mcp` — the never-built Task 7, scoped at F7 and F4.

### 3.1 Work and execution

| Tool | Returns |
|---|---|
| `exam_plan(archetype)` | Outstanding clauses only, keyword-tagged. Small payload. |
| `exam_status(card\|archetype)` | The five-class summary, `unmeasured` always present. |
| `exam_probe(scenario_yaml)` | Runs a line against the oracle **without committing a file** — the prompt sequence, step by step. |
| `run_scenario(path, sim_only)` | The structured `DiffReport`. |
| `node_health()` | Phase-0 preflight as one call. |
| `claim(cards)` / `release(cards)` | Lease management. |

`exam_probe` is the centre of gravity, not a convenience. F4 says prompt sequence
is where lines break and sim-only cannot see it; probing while composing is the
only way to author a line that will survive the oracle.

### 3.2 The authoring contract, served in pieces

The scenario format currently lives across `docs/DCGO_EXAM.md` (802 lines), the
scenarios README (193) and `SKILL.md` (237). Every author pays to rediscover it.

`exam_authoring_guide(topic?)` serves it in targeted chunks. Topics map to the
things the campaign actually got wrong:

| Topic | Covers |
|---|---|
| `format` | The six top-level keys; clause-id grammar `{card}#{zone}#{idx}`; rejection of unknown ids |
| `steps` | `hatch`/`pass`/`move`/`play`/`digivolve`/`attack`/`main`/`select`; `main: {on: field.0}` vs `play:` |
| `prompts` | The 13 prompt kinds; the EndOfTurnAction↔OptionalSkill fold; the Raid-family OptionalSkill+pick fold |
| `decks` | `stack:` is a prefix over the **initial shuffle only**; the sim-only-deals-a-different-opening trap — *stack every card the line names* |
| `assert` | Backfilled from the oracle, never hand-guessed; never over security **contents** |
| `verdicts` | The five classes; when `unreachable` is the honest answer |

Bare call returns a compact overview plus the topic list.

`exam_validate(yaml)` lints a draft *before* sim-only, with errors that teach —
unknown clause id (the orphan class), unknown verb, prompt kind outside the 13, a
`stack:` missing a card the line references, an `assert` reaching into security
contents. Each message names the rule and the guide topic.

**Anti-drift:** the guide is *generated* from `docs/DCGO_EXAM.md` (which remains
the human operating manual), with a test asserting the two match. Two prose
copies of one contract will diverge; a projection cannot.

### 3.3 Rules wired in mechanically

`exam_keyword_brief(keyword)` returns the row from the committed
`docs/digimon-rules/keyword-semantics.md` — kind, rule §, and page pointers from
`rules-index.json`.

This is load-bearing because **the keyword's kind predicts the prompt shape**:

- `Opt-cost→Mand` — `<Evade>`, `<Barrier>`, `<Alliance>`, `<Fragment>`,
  `<Decoy>`, `<Armor Purge>`, `<Digisorption>`, `<Overclock>`, `<Training>` —
  DCGO **asks**, then resolves mandatorily. The line needs an `expect:` row.
- `Mandatory` — `<Piercing>`, `<Draw>`, `<De-Digivolve>`, `<Retaliation>`,
  `<Fortitude>`, `<Mind Link>`, `<Recovery>` — **no prompt at all**. An `expect:`
  row here desynchronizes the remainder of the line.

Getting this backwards is the prompt-shape asymmetry family. The brief is
*pushed* with the work item by `exam_plan` rather than waiting to be asked.

Scope discipline: `dcgo-harness` is dev/test tooling. The MCP writes **only** to
the ledger, the scenario directory, and the node's local job queue — never to
game state, a database, or any hosted surface. It is never imported by
`server.*` or `digimon_gym.*`, and never bundled into a production build. (It is
therefore not "read-only" in the sense the engine and training MCPs are; it is
closer to `digimon-scenario-mcp`, the documented write-capable dev/test
exception.)

## 4. The fix gate

`/dcgo-exam` says *triage, then ask before changing engine or card code*. On an
unattended node "ask" stalls the job — and triage-and-fix is where the value was
(82 → 107 confirmed, and every faithfulness bug listed above).

A fix may land autonomously **only** with all three:

1. a **citation** — `general_rule.pdf` § or the DCGO C# it rests on;
2. a test that **fails before and passes after**;
3. `cards_behavioral` green.

**Card/YAML fixes** proceed under the gate. **Engine fixes** proceed under it but
land on their own branch and are flagged in the report for human review, because
they are cross-cutting. Anything not justifiable by citation becomes a **logged
finding, not a fix**.

This encodes discipline the campaign already practised — commits such as
`engine: a <Delay> whose window is owned elsewhere must NOT be auto-trashed
(16-16-1)` cite the rule in the subject line. The change is habit → gate.

Unchanged: DCGO is source-priority #2. `general_rule.pdf` outranks it, and
`diverged` is never presumed to be our bug.

## 5. The node

`dcgo-harness node up` — verify the artifact against the manifest digest, launch,
health-check, print GO. `node down`, `node status`.

**Image (~550 MB):** repo + player (492 MB) + DCGO `Assets/Scripts` mirror
(53 MB, for the adversarial pre-review — *the cheapest Unity time is the run you
never start*) + `general_rule.pdf` and `glossary.pdf` (1 MB) + harness binary.

The rules PDFs are git-ignored by rule 32 and stay that way: they go in the
**image**, not the repo. With them present, `/digimon-rules` works fully on a
remote node including PDF drill-down.

Also ship the build profile fix that landed with the campaign — `opt-level = 2`
for dev/test plus mimalloc in the test binary took `cards_behavioral` from
56.5 min to 5.2. On a node that runs the suite as a fix gate (§4), that is the
difference between a usable gate and an unusable one.

**Fleet version rule:** changing `code/digimon-engine/src/action/space.rs`
invalidates every node's player at once. `up` already refuses to launch a player
whose `action_space_hash` mismatches, so the failure is loud. Operationally:
rebuild locally, redistribute, restart. This is a chore, not a hazard.

## Testing

| Area | How |
|---|---|
| Ledger | Round-trip per-card files; union-merge of two divergent logs; expired-claim behaviour; migration from the blob preserves all 148 rows |
| Index | Regeneration is deterministic and reproducible from the ledger |
| MCP | Each tool unit-tested against fixtures with no MCP client, as the harness already does |
| Guide | Generated guide matches `docs/DCGO_EXAM.md` (drift test) |
| `exam_validate` | One case per historical failure family (orphan id, unknown verb, bad prompt kind, unstacked card, security-contents assert) |
| Node | `node_health` against a manifest with a deliberately stale action-space hash must refuse |
| Job | Bind-skip: a confirmed clause with matching hash never enters a plan; with a drifted hash it does |

## Build order

Three pieces are prerequisites regardless of the Windows-vs-Linux question:

1. **Ledger reshape** — per-card verdicts, append-only log, claims, generated
   index. Without it, two nodes corrupt each other.
2. **The MCP** — biggest lever on F7, and `exam_probe` is what makes
   prompt-sequence authoring viable at all (F4).
3. **`node up` + the image recipe** — so a fresh box is GO in one command.

Then `/archetype-campaign`, then the Linux spike if cheaper nodes are wanted.

## Risks and open questions

| Risk | Status |
|---|---|
| **Headless display.** The player launches with only `-logFile`; whether it tolerates `-batchmode -nographics` is **untested**. A Windows node may need an attached desktop session (and `tscon`-style redirection if driven over RDP). | Must be spiked before the first remote node |
| **Linux port.** Needs `StandaloneLinux64` in `HarnessBuild.cs`, the Linux Build Support module locally, the two `tasklist`/`taskkill` calls ported, and Xvfb or `-nographics`. AssetRipper-derived projects can carry Windows-only native plugins. Photon supports Linux, so odds are fair — but it is a genuine spike. | Timeboxed, optional, changes node economics |
| **Photon CCU.** Each node holds a connection and a private one-seat room against the app id baked into the build. N nodes = N concurrent CCU. Unmeasured. | Check before N > ~4 |
| **Advisory claim races.** Accepted deliberately (§1.3). | Revisit only if observed |
| **Artifact redistribution.** The player embeds card art. Copying to infrastructure you control is your call; this spec does not put it in the repo or anywhere public. | Noted, your decision |

## Non-goals

- A distributed job queue. The harness queue stays node-local (§Architecture).
- A lease server. Advisory claims until proven insufficient.
- Running DCGO in CI. Unchanged: Photon, licensing, and card art rule it out;
  the PR gate remains `--sim-only`, which **cannot find a new divergence** and
  must never be described as if it could.
- Auto-committing drafted `cards_behavioral` tests. Evidence, not truth — a
  human still reads them.
