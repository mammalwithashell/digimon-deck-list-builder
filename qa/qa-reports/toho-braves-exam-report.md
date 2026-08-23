# Toho Braves — DCGO card-clause exam report

**Date:** 2026-08-22 · **Campaign:** `toho-braves-exam` · **Oracle:** DCGO player
`scripted-v7` (`dcgo_commit f6f726088`) · **Verdict store:**
`qa/qa-reports/dcgo_exam_verdicts.json` · **Scenarios:** `qa/dcgo-exams/EX12/`
(64 files) + the ST1 selection gate.

## The denominator, always first

**239 clauses across the 44-card tournament pool: 37 confirmed · 12 diverged · 46 unreachable · 144 unmeasured.**

Never read this as "Toho Braves passed". Read it per clause: every `confirmed`
is a scripted line that ran identically in both engines under a full per-step
state diff; everything else is exactly as unproven as it says.

The archetype's **18-card competitive core** (cards in ≥33 of 45 tournament
lists) accounts for 102 of these clauses and nearly all of the measured ones.
The 144 unmeasured are dominated by low-play support cards (1-of techs) and
core clauses awaiting the tooling noted below.

| Card | Name | Clauses | confirmed | diverged | unreachable | unmeasured | tier |
|---|---|---|---|---|---|---|---|
| EX1-066 | Analog Youth | 3 | 1 | 1 |  | 1 | core |
| EX12-004 | Onibimon | 5 |  | 2 | 3 |  | core |
| EX12-009 | Wankomon | 4 | 2 |  | 2 |  | core |
| EX12-011 | Seasarmon | 5 | 2 | 1 | 1 | 1 | core |
| EX12-020 | Gasamon | 5 | 4 |  | 1 |  | core |
| EX12-026 | Shellmon | 6 | 3 | 1 | 1 | 1 | core |
| EX12-031 | MarineBullmon | 5 | 1 |  | 4 |  | core |
| EX12-036 | Ryugumon | 9 | 2 | 1 | 6 |  | core |
| EX12-046 | Shishimamon | 6 | 3 | 1 | 2 |  | core |
| EX12-047 | Amaterasumon | 8 | 1 | 1 | 5 | 1 | core |
| EX12-061 | Hanimon | 6 | 3 | 2 | 1 |  | core |
| EX12-062 | Kokeshimon | 5 | 1 |  | 4 |  | core |
| EX12-063 | Karakurumon | 5 | 1 | 1 | 3 |  | core |
| EX12-065 | Kaguyamon | 7 | 3 |  | 1 | 3 | core |
| EX12-070 | Sanmyojin Arrival | 6 | 4 | 1 | 1 |  | core |
| EX12-074 | Genshi Continent & Ashin | 4 | 3 |  | 1 |  | core |
| EX12-075 | Kunlun's Imperial Decree | 4 | 2 |  | 1 | 1 | core |
| EX12-076 | Susanoomon | 9 |  |  | 9 |  | core |
| BT11-089 | Akiho Rindou | 4 |  |  |  | 4 | support |
| BT20-037 | Chaosmon: Valdur Arm | 5 |  |  |  | 5 | support |
| BT8-084 | Kimeramon | 3 |  |  |  | 3 | support |
| BT8-097 | Crimson Blaze | 4 |  |  |  | 4 | support |
| EX12-002 | Mococomon | 4 |  |  |  | 4 | support |
| EX12-006 | Kakamon | 5 |  |  |  | 5 | support |
| EX12-012 | Apemon | 6 |  |  |  | 6 | support |
| EX12-015 | Gokuumon | 7 |  |  |  | 7 | support |
| EX12-019 | Nezhamon | 11 |  |  |  | 11 | support |
| EX12-022 | Kamemon | 5 |  |  |  | 5 | support |
| EX12-025 | Gawappamon | 6 |  |  |  | 6 | support |
| EX12-029 | Sagomon | 7 |  |  |  | 7 | support |
| EX12-034 | Erlangmon | 7 |  |  |  | 7 | support |
| EX12-039 | Takinmon | 4 |  |  |  | 4 | support |
| EX12-043 | Hakubamon | 4 |  |  |  | 4 | support |
| EX12-045 | Sanzomon | 6 |  |  |  | 6 | support |
| EX12-048 | SeitenGokuumon | 8 |  |  |  | 8 | support |
| EX12-056 | Cho-Hakkaimon | 8 |  |  |  | 8 | support |
| EX12-057 | Takutoumon | 8 |  |  |  | 8 | support |
| EX12-071 | Saneiketsu Invitation | 6 |  |  |  | 6 | support |
| EX4-074 | ShineGreymon: Ruin Mode | 4 |  |  |  | 4 | support |
| P-130 | Lui Ohwada | 3 |  |  |  | 3 | support |
| ST1-12 | Tai Kamiya | 2 |  |  |  | 2 | support |
| ST1-15 | Giga Destroyer | 3 | 1 |  |  | 2 | support |
| ST16-14 | Matt Ishida | 3 |  |  |  | 3 | support |
| ST19-14 | Arisa Kinosaki | 4 |  |  |  | 4 | support |

## What "confirmed" means here

A hand-authored scenario (`qa/dcgo-exams/EX12/`) drives BOTH engines through the
same legal line from game start — same stacked deck, same seed, same actor
sequence, every prompt asserted before it is answered — and a normalized
per-step state diff (board, effective DP, suspension, hands, trash, security
count, memory) came back CLEAN. Selections are answered by card identity on
both sides. This is stronger evidence than any per-card unit test we have: it
is two independent implementations agreeing about what actually happened.

## The 2 diverged — findings to triage, not verdicts of guilt

`diverged` means both engines ran the line and disagreed about the state.
`general_rule.pdf` outranks DCGO; neither engine is presumed right.

1. **EX12-070#effect#0 (Sanmyojin Arrival)** — declining the optional
   "by trashing 1 [TB] card" cost: our engine **auto-resolved the optional
   cost prompt** (the exam CLI's own note: "select answered no live prompt --
   our engine auto-resolved it") and put the option in the trash;
   DCGO asked, was declined, and left the option elsewhere at the same step.
   Two suspicions, both ours: an auto-resolution (rule 17 violation candidate)
   and possibly the wrong placement on decline. **Highest-priority triage.**
2. **EX12-046#effect#2 (Shishimamon)** — trash-timing divergence around a
   digivolve our sim rejects; predicted by the adversarial review
   ("diverged-by-design", note in the scenario header).

## The 56 unreachable — five named causes, none silent

| Family | Count | Cause |
|---|---|---|
| Optional-trigger gate asymmetry | ~20 | DCGO parks an `OptionalSkill`/`MultipleSkills` gate at `isOptional` trigger boundaries; our engine surfaces no prompt there. The scripted line desyncs at the gate. **Triage task `task_69f10a66`** — the rules manual decides which engine is right. |
| Recon: structurally unmeasurable | 26 | `[Security]` trigger contexts and Susanoomon's (EX12-076) execution-context clauses — no line vocabulary reaches them yet. |
| MultipleSkills value-space | 3 | Our TriggerOrder slot values vs DCGO's 0-based `skillInfos` index; out-of-range silently clears DCGO's trigger stack. |
| 1-card OrderedPermutation | 2 | DCGO auto-places a single leftover; we park an ordering prompt; the strict cursor aborts. |
| Engine/DSL gaps | 4 | `grant_keyword` keywords never reach the would-be-deleted replacement window (`<Barrier>`/`<Evade>` never fire from grants — **task `task_8f063aa6`**); Material-source prompts are an indexes-payload class outside select-support scope; a spurious Decode leave-window our sim parks. |

## Engine + data findings surfaced by the campaign

- **DSL `grant_keyword` → no replacement window** (task_8f063aa6): granted
  `<Barrier>`/`<Evade>` never offer their prevention. Printed-keyword paths
  work; the grant path does not. Found by an authoring agent probing a real
  line, which refused to enshrine the wrong outcome in CI asserts.
- **Optional-trigger prompt asymmetry** (task_69f10a66): if our engine is
  auto-resolving optional triggers it is a no-approximations violation; if it
  surfaces them elsewhere, the exam needs a mapping; if DCGO over-asks, we
  document the quirk. §15-6/15-7 decide.
- **EX12-070 optional-cost auto-resolution** — see diverged #1.
- **BT8-084 Kimeramon** implemented (the pool's one missing card), honestly
  **PARTIAL**: the color-scaled DP-minus needs a source-stack∪top color-count
  formula the DSL cannot express (`G-DSL-SOURCE-STACK-UNION-COLOR-COUNT`,
  logged in `qa/dsl-vocab-gaps.md` with tripwire tests that fail on any
  future approximation).
- **[TB] trait data hole (corrected 2026-08-22):** EX12-036 and EX12-047 print
  `[TB]` on their faces (EX12-036 also carries a `Rule: Trait: Has [Aquatic]
  Type.` grant) but `cards.json` had dropped both — the scenario author's
  original claim was right, and an earlier campaign note calling it false was
  itself wrong (a truncated diagnostic hid exactly these two long trait
  lines). Fixed durably in `card_overrides.json` + patched in `cards.json`.

## Tooling shipped by this campaign (all exam-general, not Toho-specific)

- `select:` steps end to end — five symbolic forms, identities on the wire,
  each engine resolving against its own candidate list; proven on ST1-15
  ("delete up to 2") with a CLEAN 14/14 oracle diff.
- OptionalSkill decline bridge (cancel IS "no" at a yes/no prompt).
- `move` verb (breeding → battle); `--verdicts` store wiring with clause-text
  hashes + orphan refusal; `--emit-job`; phase normalization for selection and
  combat-interrupt windows (representation, never semantics — state fields
  still compare).
- Adversarial pre-Unity review of every authored scenario against DCGO C#
  (24 VERIFIED / 7 FIXED / 13 REJECTED with named systemic causes) — the
  cheapest Unity time is the run you never start.

## How to extend this

- Re-run a card: `/dcgo-exam`, or
  `dcgo-harness exam --scenario qa/dcgo-exams/EX12/<file> --sim-only ...`
  (CI-safe); oracle pass per `docs/DCGO_EXAM.md`.
- The three spawned tasks (`task_8f063aa6`, `task_69f10a66`, plus the
  alt-trait digivolve chip) unlock ~24 more clauses between them.
- MultipleSkills index translation + 1-card ordering skip tolerance are the
  next two harness investments (5 clauses).

## Addendum (2026-08-23): post-fix oracle pass

After the six engine fixes (commits 7a0837871 + 013b7fe10), the re-run moved the
store from 37/2/56 to **37 confirmed / 12 diverged / 46 unreachable**. The 10
new diverged are an evidence-quality upgrade, not a regression: those clauses
previously desynced at unanswerable gates and measured NOTHING; now the granted
<Execute> triggers actually fire on both sides, the full traces compare
end-to-end, and each records a concrete state divergence at the Execute
boundary. One probable root cause -- our new granted-Execute semantics (or the
exam's OptionalSkill<->end-of-turn-attack mapping) differs from DCGO at that
step -- to be triaged with the recorded diffs in hand. Remaining oracle-failed
scenarios sit in the already-documented families (MultipleSkills value-space,
prompt-shape asymmetries, EX12-047 T1 actor parity).
