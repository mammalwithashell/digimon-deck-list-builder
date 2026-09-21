# Three Musketeers — DCGO card-clause exam report

**Date:** 2026-09-21 — **Campaign:** `three-musketeers-1` + close-out job
`three-musketeers-2` (node `james-desktop`) — **Oracle:** DCGO player build
`D:/dcgo-build/scripted-v16` (base-repo DCGO `fc67f9ae6`; `scripted-v15` produced the
sidecars job 1 recorded, `scripted-v14` is NO-GO on the action-space digest) —
**Store:** `qa/qa-reports/exam-verdicts/<CARD-ID>.json` — **Attempt log:**
`qa/qa-reports/exam-log.jsonl` — **Scenarios:** 220 under `qa/dcgo-exams/` for 61 of
the 64 pool cards (369 in the whole tree) — **Meta source:** `data/deck_library.json`,
102 tournament lists (EX12 format).

## The denominator, always first

**224 clauses across the 64-card Three Musketeers pool: 216 confirmed — 4 diverged
— 1 unreachable — 1 unavailable — 2 unmeasured.**

**The 15-card competitive core (in ≥72 of 102 lists) is at 54 of 54 clauses
adjudicated — 53 confirmed, 1 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**
Every core clause either ran clean against the oracle or carries a named, *measured*
reason. **Untriaged `diverged`: 0** — all four carry a triage class and a citation.

Recomputed from disk at close-out, not carried over from any stage report:

```
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "Three Musketeers" --json
  → denominator {total_cards: 64, total_clauses: 224,
                 confirmed 216, diverged 4, unreachable 1, unavailable 1, unmeasured 2}
```

The five classes sum to the denominator per card and in total (checked per card at
close-out: 0 mismatches). `exam_binding.bind()` appends exactly one class per clause,
and the index refuses to render a row whose counts do not sum.

Never read this as "Three Musketeers passed". Read it per clause: a `confirmed` is a
scripted line that ran identically in our engine and in DCGO under a full per-step
state diff; everything else is exactly as unproven as it says. And see
"One `confirmed` that is not currently backed by a clean re-diff" below — the store
holds one clause whose doubt lives only in a note beside it.

### Where this report and the stage reports disagree, the disk wins

The close-out stages handed this job a verdict snapshot taken mid-flight. It listed a
live `diverged` on **EX7-073, BT25-028, BT25-093, BT25-100, BT19-075 and BT24-091**.
None of those survive on disk: each was triaged and flipped later inside the same job
(five to `confirmed` after an engine fix, one after a tooling fix). The snapshot also
predates the last four commits. Likewise, job 1's published denominator
(194 / 4 / 8 / 1 / 17) is superseded: the numbers above are what
`clause_coverage.campaign` reads out of `qa/qa-reports/exam-verdicts/` today.
Every figure below was recomputed from the store, and where a stage `RESULT` line and
the store disagreed, the store is what is printed.

The engine-gap stages also quote full-suite counts of 8199 / 8202 / 8206 / 8209 /
8212 / 8214 / 8221 as they landed. The number measured at HEAD for this report is
**8227**.

## Per-card denominator

| Card | Name | In lists | Clauses | confirmed | diverged | unreachable | unavailable | unmeasured |
|---|---|---|---|---|---|---|---|---|
| BT21-071 **(core)** | Scopemon | 102/102 | 5 | 5 |  |  |  |  |
| EX7-051 **(core)** | Sparrowmon | 102/102 | 3 | 3 |  |  |  |  |
| EX7-071 **(core)** | Hurricane Screw Shot | 102/102 | 3 | 3 |  |  |  |  |
| P-180 **(core)** | Bind Red Trigger | 102/102 | 3 | 3 |  |  |  |  |
| BT21-074 **(core)** | Satellamon | 100/102 | 5 | 5 |  |  |  |  |
| EX7-073 **(core)** | BeelStarmon (X Antibody) | 100/102 | 3 | 3 |  |  |  |  |
| BT24-088 **(core)** | Asuna Shiroki | 99/102 | 3 | 3 |  |  |  |  |
| EX7-070 **(core)** | Der Blitz | 97/102 | 3 | 3 |  |  |  |  |
| BT25-005 **(core)** | Pagumon | 96/102 | 1 | 1 |  |  |  |  |
| BT25-078 **(core)** | Gazimon | 96/102 | 4 | 4 |  |  |  |  |
| BT25-082 **(core)** | BlackGatomon | 96/102 | 4 | 4 |  |  |  |  |
| BT25-083 **(core)** | LadyDevimon | 96/102 | 4 | 4 |  |  |  |  |
| BT25-085 **(core)** | BeelStarmon | 96/102 | 7 | 7 |  |  |  |  |
| BT25-092 **(core)** | Asuna Shiroki | 95/102 | 3 | 3 |  |  |  |  |
| EX7-008 **(core)** | ToyAgumon | 94/102 | 3 | 2 | 1 |  |  |  |
| BT25-058 | Callismon | 50/102 | 6 | 6 |  |  |  |  |
| LM-056 | Image Training | 38/102 | 4 | 4 |  |  |  |  |
| BT24-081 | Titamon + SkullBaluchimon | 37/102 | 6 | 6 |  |  |  |  |
| EX4-074 | ShineGreymon: Ruin Mode | 36/102 | 3 | 3 |  |  |  |  |
| BT25-028 | Dianamon | 24/102 | 5 | 5 |  |  |  |  |
| EX7-048 | Gundramon | 24/102 | 4 | 4 |  |  |  |  |
| P-130 | Lui Ohwada | 20/102 | 3 | 3 |  |  |  |  |
| LM-032 | Purple Scramble | 18/102 | 3 | 3 |  |  |  |  |
| BT18-093 | Violet Inboots | 14/102 | 3 | 3 |  |  |  |  |
| BT7-073 | KaiserLeomon | 13/102 | 2 | 2 |  |  |  |  |
| EX7-043 | Tankmon | 11/102 | 3 | 3 |  |  |  |  |
| EX7-059 | BeelStarmon | 11/102 | 5 | 5 |  |  |  |  |
| BT24-091 | Tidal Stream | 10/102 | 6 | 6 |  |  |  |  |
| EX7-066 | Chaos Triangular | 10/102 | 3 | 3 |  |  |  |  |
| EX7-005 | Kapurimon | 7/102 | 1 | 1 |  |  |  |  |
| BT8-084 | Kimeramon | 6/102 | 3 | 3 |  |  |  |  |
| EX7-040 | ToyAgumon | 6/102 | 3 | 2 |  | 1 |  |  |
| EX7-044 | Gigadramon | 6/102 | 3 | 3 |  |  |  |  |
| P-170 | AvengeKidmon | 6/102 | 6 | 6 |  |  |  |  |
| BT7-056 | Dorumon | 4/102 | 2 | 1 | 1 |  |  |  |
| EX7-013 | MagnaKidmon | 4/102 | 3 | 3 |  |  |  |  |
| BT17-070 | Gulfmon | 2/102 | 3 | 3 |  |  |  |  |
| BT25-064 | ToyAgumon | 2/102 | 3 | 2 | 1 |  |  |  |
| BT25-100 | Iron Slash | 2/102 | 8 | 8 |  |  |  |  |
| BT26-078 | *(no card data)* | 2/102 | 1 |  |  |  |  | 1 |
| EX1-066 | Analog Youth | 2/102 | 3 | 3 |  |  |  |  |
| EX7-011 | Megadramon | 2/102 | 3 | 3 |  |  |  |  |
| P-198 | DemiDevimon | 2/102 | 4 | 4 |  |  |  |  |
| P-212 | Asuna Shiroki | 2/102 | 4 | 4 |  |  |  |  |
| BT16-077 | Dinobeemon | 1/102 | 5 | 5 |  |  |  |  |
| BT19-075 | MoonMillenniummon | 1/102 | 4 | 4 |  |  |  |  |
| BT20-020 | Imperialdramon: Fighter Mode | 1/102 | 5 | 5 |  |  |  |  |
| BT21-054 | Shotmon | 1/102 | 4 | 4 |  |  |  |  |
| BT24-011 | Cyclonemon | 1/102 | 4 | 4 |  |  |  |  |
| BT24-030 | Neptunemon | 1/102 | 5 | 5 |  |  |  |  |
| BT25-026 | Crescemon | 1/102 | 4 | 4 |  |  |  |  |
| BT25-091 | Monica Simmons | 1/102 | 4 | 4 |  |  |  |  |
| BT25-093 | Ignition Flare | 1/102 | 6 | 6 |  |  |  |  |
| BT26-083 | *(no card data)* | 1/102 | 1 |  |  |  |  | 1 |
| BT3-096 | Mimi Tachikawa | 1/102 | 2 | 2 |  |  |  |  |
| BT3-105 | Breath of the Gods | 1/102 | 2 | 2 |  |  |  |  |
| BT6-060 | Deputymon | 1/102 | 2 | 1 | 1 |  |  |  |
| BT6-105 | Gewalt Schwärmer | 1/102 | 3 | 3 |  |  |  |  |
| BT7-071 | Loweemon | 1/102 | 1 | 1 |  |  |  |  |
| BT8-071 | Psychemon | 1/102 | 1 | 1 |  |  |  |  |
| BT8-094 | Digimon Emperor | 1/102 | 3 | 3 |  |  |  |  |
| EX7-010 | Deputymon | 1/102 | 4 | 4 |  |  |  |  |
| P-108 | Wisdom Training | 1/102 | 3 | 3 |  |  |  |  |
| ST13-08 | Chikurimon | 1/102 | 1 |  |  |  | 1 |  |
| **POOL TOTAL** | 64 cards | — | **224** | **216** | **4** | **1** | **1** | **2** |
| **of which CORE** | 15 cards | ≥72/102 | **54** | **53** | **1** | **0** | **0** | **0** |

`EX1-066` and `P-130` were confirmed by the earlier `toho-braves-exam` campaign and
are counted here but were neither claimed nor re-run by this job (their exam-log
lines belong to that job).

## Regression state at close-out

### Full behavioural suite

```
RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml \
  --test cards_behavioral -- --test-threads=8
  → test result: ok. 8227 passed; 0 failed; 37 ignored; 0 measured; 0 filtered out;
    finished in 272.77s        # 2026-09-21, at HEAD, close-out
```

### Full sim-only corpus — and the five scenarios it caught

```
dcgo-harness --root <ROOT> exam --scenario <each> --sim-only \
    --cards-json data/cards.json --decks <that scenario's own book>
  → 369 scenarios / 369 lowered / 369 run / 0 failed
```

The corpus is run one scenario at a time against *its own* deck book, because four
deck names (`quiet-opponent`, `tm-quiet-red`, `tm-ts-line`, `tm-deputymon`) are
defined differently in more than one pool file. A single merged book — which
`docs/DCGO_EXAM.md` §"Quick start" suggests for a whole-tree run — would silently hand
some scenarios the wrong deck and still report green.

**This did not start out green.** The first pass failed **5 Three Musketeers
scenarios**, every one of them a `confirmed` clause whose line no longer replayed
after this job's own engine fixes:

| Scenario | What broke | Cause |
|---|---|---|
| `BT19/BT19-075-effect2.yaml` | `assert:` mismatch — `p1.trash` expected `[ST1-10, P-180]`, ours had `[P-180, ST1-04, ST1-10]` | The assert was written while the deletion observer was still broken. `4c609b5e2` fixed it, so the clause now trashes p1's top security as printed. |
| `BT25/BT25-085-effect2.yaml`, `-effect3.yaml` | never lowered — unanswered `Replacement` gate, "You may activate BT25-085's triggered effect" | The sibling trigger DCGO always offered is now offered by us too (`b77d1b288`). The row was authored `dcgo_only` with the note "our engine never queued it". |
| `EX7/EX7-073-effect0.yaml` | never lowered — unanswered `SourceMulti { min: 0, max: 2 }` cost prompt | `b77d1b288` + `G-TOOLING-EXAM-TRAILING-PASS-EATS-NEXT-PROMPT`: the unpayable cost pick is now PARKED and visible instead of silently resolved. |
| `EX7/EX7-073-effect1.yaml` | never lowered — a `sim_only` TriggerOrder row landed on the placement prompt | `b77d1b288` removed the TriggerOrder our engine used to park between a used Option's `[Main]` and a pending sibling. |

All five were repaired **against the oracle, not against our own output**: each was
re-diffed against its own preserved sidecar (zero Unity time), and all five came back
**CLEAN**, so every one keeps its `confirmed` verdict, now re-earned on the current
engine:

| Clause | Preserved sidecar | Re-diff |
|---|---|---|
| BT19-075#effect#2 | `20260921T041928Z_40f7887a…` | CLEAN 22 of 22 |
| BT25-085#effect#2 | `20260918T072533Z_2a42c78d…` | CLEAN 18 of 21 ours / 20 dcgo (3 sim-only + 2 DCGO-intermediate rows) |
| BT25-085#effect#3 | `20260918T072624Z_1cc179a1…` | CLEAN 25 of 30 ours / 28 dcgo (5 sim-only + 3 DCGO-intermediate rows) |
| EX7-073#effect#0 | `20260918T103726Z_f2a2241d…` | CLEAN 19 of 24 ours / 21 dcgo (5 sim-only + 2 DCGO-intermediate rows) |
| EX7-073#effect#1 | `20260918T105217Z_0a6e7c8a…` | CLEAN 16 of 21 ours / 18 dcgo (5 sim-only + 2 DCGO-intermediate rows) |

The BT19-075 repair took its asserted value straight out of DCGO's sidecar (final row:
`p1.trash ['P-180','ST1-04','ST1-10']`, `p1.security 4 → 3`), so the scenario now
encodes the oracle's state rather than ours.

**`--sim-only` is still not confirmation.** A green corpus proves the lines are legal
*here* after this campaign's engine changes. It observes no DCGO. Only an oracle diff
moves a clause to `confirmed`.

## What this close-out job changed

### Engine findings CLOSED (15)

| Gap | Closed by | Driver clause(s) |
|---|---|---|
| `G-ENGINE-OPTION-TRASH-VS-ON-USE-TRIGGER-ORDER` | `2dc48e616` (re-measured CLEAN against `5108e58ee`; regression test added) | BT25-091#effect#2, BT24-030#effect#4 |
| `G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER` | `5aef07fa8` (ENGINE FIX) | BT3-096, BT25-091 |
| `G-ENGINE-MAIN-ON-FIELD-ACTIVATION-COST-UNPAID` | `b96ebd4df` (ENGINE FIX) | BT25-089, EX11-071 (no exam clause) |
| `G-ENGINE-PARTITION-SLOT-ENFORCEMENT-DEFERRED` | `e01f319ec` (ENGINE FIX) | BT16-077#effect#2, #inherited#0 |
| `F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST` | `f24b986d0` (ENGINE FIX) | BT25-100, BT25-093, ST22-08 |
| Condition-false scheduled `<Delay>` trashed its carrier | `5f091f5b2` (ENGINE FIX) | LM-032#effect#0/#1/#2 |
| `G-ENGINE-OPTION-LINK-FROM-HAND` | `7e30343d4` (ENGINE FIX) | BT25-093#effect#4, BT25-100#effect#5 |
| `G-ENGINE-USED-OPTION-BODY-VS-PENDING-SIBLING-TRIGGER` | `b77d1b288` (ENGINE FIX) | EX7-073#effect#2 |
| `G-ENGINE-OPTION-HAND-LINK-COST-TIMING` | `9af21698c` (ENGINE FIX) | BT25-093#effect#4, BT25-100#effect#5, BT24-091#effect#4 |
| `G-ENGINE-REPLACEMENT-COST-DELETION-NO-OBSERVER` | `4c609b5e2` (ENGINE FIX) | BT19-075#effect#2 |
| `G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK` | `8cb317bb4` | EX7-073#effect#2 |
| `G-TOOLING-EXAM-NO-DNA-VERB` | `dec94c05c` | BT8-084#effect#0 |
| `G-TOOLING-EXAM-OPTION-HAND-LINK` | `7e30343d4` | BT25-093#effect#4, BT25-100#effect#5 |
| `G-TOOLING-EXAM-PAIRING-INDEXED-BY-LOWERED-ENTRY` | `4df70706f` | corpus-wide |
| `G-TOOLING-EXAM-TRAILING-PASS-EATS-NEXT-PROMPT` | `b77d1b288` | EX7-073#effect#2, BT25-028#effect#3 |

One finding was **retracted** rather than fixed:
`F-ENGINE-BT25-028-OPP-SOURCE-PICK-NEVER-OFFERED` (`46e8cdff1`) — the clause's trash
pick never reached a row because the harness's trailing PASS had already declined it;
with the tooling fix in, the same preserved sidecar diffs CLEAN and the engine needed
no change. `BT25-028#effect#3` is `confirmed`.

One finding was **half-closed at this close-out** (`docs/RUST_ENGINE_GAPS.md` updated
in the same commit as this report): `F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE`.
Its ordering half is superseded by `b77d1b288` — *measured*, not inferred: the standing
witness for it was the `sim_only` TriggerOrder row in `EX7-073-effect1.yaml`, which no
longer lowers because our engine asks nothing there; removing it leaves the line CLEAN
against its preserved sidecar. Its second half (`play_from_hand_free` silently
accepting an Option/Dual hand card and playing it to the battle area as a permanent)
is **untouched and still OPEN**.

### Exam verbs and wires added

- **`dna:` scenario verb** (`dec94c05c`) — DNA digivolution on both wires; needed
  `scenario.rs STEP_VERBS`, a `lower::matches_intent` arm, and DCGO-side
  `InputDriver` support. Unblocked BT8-084#effect#0.
- **A Plug-In Option's from-hand link is its own main-phase action** (`7e30343d4`) —
  previously our plug-in rode the PLAY bit behind a mode-select while DCGO used a
  separate `ActivateCardAction`. Unblocked BT25-093#effect#4 and BT25-100#effect#5.
- **`SourceMulti` cost prompts answerable by card identity** (`8cb317bb4`) —
  unblocked EX7-073#effect#2.

These three required a DCGO rebuild; the oracle used for this job's runs is
**`D:/dcgo-build/scripted-v16`** (base-repo DCGO `fc67f9ae6`). The submodule gitlink is
deliberately not bumped.

### Clauses moved OFF `unreachable` — 8 of job 1's 9

| Clause | Was blocked on | Now |
|---|---|---|
| BT25-085#effect#4 / #5 / #6 **(core)** | card-data gap: `cards.json` carried BT25-085 with no `dual` block | `confirmed` — scenarios authored `0b1c164dd`, oracle-run `d02a2128a` |
| EX7-073#effect#2 **(core)** | `G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK` | `confirmed` — via `8cb317bb4`, which then exposed a real engine bug, fixed in `b77d1b288` |
| BT25-093#effect#4 | `G-TOOLING-EXAM-OPTION-HAND-LINK` | `confirmed` — via `7e30343d4` + engine fix `9af21698c` |
| BT25-100#effect#5 | same | `confirmed` — same route |
| BT8-084#effect#0 | `G-TOOLING-EXAM-NO-DNA-VERB` | `confirmed` — via `dec94c05c` |
| EX7-040#effect#0 | no separating line exists | **still `unreachable`** — re-verified, see below |

### Clauses moved off `unmeasured` — 15 of job 1's 17

BT25-028 ×5, BT25-026 ×2, LM-056 ×2, P-108 ×2, BT25-058 ×1, BT19-075#effect#2,
BT24-091#effect#4 and BT16-077#effect#0 were authored and/or run on the oracle in this
job and are now `confirmed`. Only the BT26 pair remains.

## Engine fixes — FLAG FOR HUMAN REVIEW

**Nine rules-behaviour engine fixes landed in this close-out job**, each under the
campaign's three-part gate: a citation, a test that fails before and passes after, and
`cards_behavioral` green. Two further engine-tree commits are exam tooling
(`8cb317bb4`, and the `runners/selection_resolve.rs` half of `b77d1b288`) and one is
the harness `dna:` verb (`dec94c05c`, which also touched `game_actions/digivolve.rs`
and `selection.rs`). They change the behaviour of the shared engine and deserve a
human read before they are taken for granted.

| Commit | Fix | Citation |
|---|---|---|
| `5aef07fa8` | The 9-1-5 Option-trash ORDER is surfaced to the Option's user as a two-entry `TriggerOrder` instead of a hard-coded trash-first | `general_rule.pdf` 9-1-5 (p.19) + 18-1-1/18-1-2 (p.40) + 15-4-3-2 (p.22) + **15-4-3-5-1** (p.23); DCGO `CardController.cs` `UseOptionClass.UseOption` is *one* legal order, not the only one |
| `b96ebd4df` | A field `[Main]`'s `activation_cost` is paid on the action path, and the mask gates the `[Main]` bit when it is unpayable | `general_rule.pdf` §15-7-1/§15-7-2; DCGO `BT25_089.cs:36` → `CanSuspend.cs:17-25` |
| `e01f319ec` | `<Partition>` reads the card's printed parenthetical and plays exactly one of each specified card, masking picks that strand a slot | `general_rule.pdf` §16-28-1/-5/-6 (pp.37-38); DCGO `Partition.cs:66-119,145-159` + `:89,117` |
| `f24b986d0` | `option_legal_play_modes` drops the Link play mode when there is no legal host | `general_rule.pdf` §10-1-3-1 (+ §6-5-1-4); DCGO `Link.cs:24,53` |
| `5f091f5b2` | A condition-false scheduled `<Delay>` leaves its carrier in the battle area and reschedules, instead of trashing it for free | `general_rule.pdf` §16-16-1 (p.35); DCGO `LM_032.cs:137-143,146-158` |
| `7e30343d4` | A Plug-In Option's from-hand link is its own main-phase action | `general_rule.pdf` §10-1-3; DCGO `ActivateCardAction` path |
| `b77d1b288` | A used Option's `[Main]` runs to completion before any pending sibling trigger activates | `general_rule.pdf` **15-8-3-2** (p.25) — "Trigger-type effects can't activate during the processing for a rule or effect" — + 9-1-5, 15-4-3-4 / 15-4-4-2 (p.23); DCGO `UseOptionClass.UseOption` runs the OptionSkill INLINE |
| `9af21698c` | A from-hand Plug-In Option link pays its cost **after** the §10-1-3-1 host pick, not at declaration | `general_rule.pdf` §10-1-3-1/-2 (p.19); DCGO `Link.cs:85,95` + `CardController.cs:3762` |
| `4c609b5e2` | A deletion trigger's dangling `PermanentHandle` no longer aliases the survivor that slid into the dead slot | `general_rule.pdf` §15-8-3-2 (+ §15-8-3-1/-5/-6, §15-8-5-4); fix at `code/digimon-engine/src/dsl_cards/predicate.rs:1890` |

Two of these deserve particular scrutiny, because each is broader than its driver:

- **`5aef07fa8`** introduces a *new player choice* the engine did not previously offer.
  An instrumented full run measured only 8 such sites across 8202 tests (BT3-096,
  BT25-091, one synthetic), and DCGO makes no such decision — so two exam lines carry
  `sim_only` rows and `ReplaySession::auto_answer_option_trash_order` answers it the
  oracle's way for DCGO corpus replay. If the 15-4-3-5-1 reading is wrong, this is the
  fix to revert.
- **`4c609b5e2`** is pool-wide, not replacement-specific. The notes predicted a
  replacement-cost mechanism; the actual defect was battle-area compaction aliasing a
  dangling handle, reproducible with no replacement at all.

## The 4 diverged, triaged — all four DCGO quirks, one family

`diverged` means both engines ran the line end to end and disagreed. It is a finding
to triage, not proof we are wrong: `general_rule.pdf` outranks DCGO.

All four surviving divergences are the **same** finding,
`G-EXAM-REVEAL-BUCKET-ADD-TIMING` — explicitly **not** an engine gap. One printed
"Add" with two targets is one process (`general_rule.pdf` §15-15-10-1 p.31, §15-1-2 /
§15-1-4 p.22), so our engine chooses both picks and then adds once; DCGO's shared
reveal helper (`RevealLibrary.cs:291-329`, `SelectCardEffect.cs:813-815`) runs a
per-condition `SelectCardEffect(Mode.AddHand)` loop that moves each bucket's pick to
hand as its own prompt closes. In every case the only diff is one intermediate
`p0.hand` row and the **end state is identical**.

| Clause | Core? | Measured evidence |
|---|---|---|
| EX7-008#effect#1 | **yes** (94/102) | Step 7, 2nd reveal pick row: `p0.hand` ours `[BT24-088 ×3, BT25-092 ×2]`, dcgo also holds `EX7-051`. Re-measured 2026-09-21 on a FRESH scripted-v16 oracle pass. |
| BT7-056#effect#0 | no | Re-triaged from scratch 2026-09-21 against sidecar `20260921T042630Z_14e62ea8`: step 7 `p0.hand` is the LEAD and the ONLY row, nothing downstream; `--sim-only` passes all 7 end-state checks. Not a slot artefact. `NOTES-BT7-056.md`. |
| BT6-060#effect#0 | no | Re-triaged from scratch 2026-09-21 against sidecar `20260921T042614Z_c72aa5cc`: one field at one step, end state identical. This was the divergence `NOTES-BT6-060.md` predicted before the oracle ran. |
| BT25-064#effect#1 | no | Re-triaged from scratch 2026-09-21 against sidecar `20260921T042338Z_438b4ce3`: step 3 `p0.hand` is the LEAD and the ONLY row. `NOTES-BT25-064.md`, commit `fc92c54e3`. |

**Untriaged diverged: 0.** Each carries a class, a citation and a re-measurement; none
is deferred to "look at it later".

## What remains unmeasured / unreachable / unavailable — with its CURRENT reason

### 1 unreachable

**EX7-040#effect#0** — "[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0".
**Re-verified 2026-09-21 against `data/cards.json`:** the only Lv.2 cards printing
"[Three Musketeers]" anywhere in their effect / inherited / security text are
Kapurimon (EX7-005) and Pagumon (BT25-005), and **both are Black** (`card_colors [5]`).
The printed Black Lv.2 cost-0 circle therefore opens at the same cost from the same
cards, so any line that "passes" would confirm the wrong clause. This is a
*separability* limit of the printed pool, not a tooling or engine gap — it cannot be
closed by writing a better scenario, only by a future card that separates the two
paths. `NOTES-EX7-040.md`.

### 1 unavailable

**ST13-08#effect#0** (Chikurimon) — DCGO has no script for the card, so there is no
oracle to cross-examine against. Measured **per card, not per set**:
`$BASE_DCGO/Assets/Scripts/CardEffect/ST13/{Black,Red}/` exist and hold
ST13_01..06/09/11/13..16, but `find -name 'ST13_08*'` returns nothing.
`NOTES-ST13-08.md`.

### 2 unmeasured

**BT26-078#security#0** and **BT26-083#security#0** — no card data.
**Re-verified 2026-09-21:** `data/cards.json` holds **0** BT26 cards (4294 ids, none
with a `BT26-` prefix), and while a `CardEffect/BT26/` directory now exists in the
base-repo DCGO it holds only `BT26_002/003/004/009/103` — **neither BT26-078 nor
BT26-083**. So there is nothing to author from and nothing to examine against. Both
clauses sit in the denominator as `image-required` and stay `unmeasured` by cause, not
by neglect. Closing them needs a BT26 ingest *and* upstream DCGO scripts; see the
standing note that BT26 blocks the EX12 meta top.

Nothing else in the pool is outstanding.

## One `confirmed` that is not currently backed by a clean re-diff

`BT25-091#effect#2` (Monica Simmons) is stored as **`confirmed`** and is counted as
such in the denominator above, but its line does **not** currently re-diff clean
against its preserved sidecar
(`20260918T130112Z_6a17348e…`). Re-running it gives one field on one row:

```
p0.field[0].suspended: ours=true  dcgo=false      (at the OptionalSkill accept)
```

Every later compared row agrees — both engines have Monica suspended by the next
prompt. It was **proved not to be caused by `f24b986d0`**: it reproduces on the
unmodified engine, and it appeared with `5aef07fa8`'s new `TriggerOrder` prompt, whose
insertion means the pre-`5aef07fa8` shape of the line cannot be re-run at all (it no
longer lowers). The stored verdict was deliberately left untouched rather than flipped
on a drive-by judgement.

**This is the one number in this report that is softer than it looks.** The next
dispatch should decide whether the snapshot boundary is a harness property — in which
case the exam differ should tolerate it for activation-cost rows — or a real ordering
divergence, and re-record the verdict either way. Full detail in
`qa/dcgo-exams/BT25/NOTES-BT25-091.md`. If it resolves against us, the pool reads
215 confirmed / 5 diverged and the core reads 54/54 with 2 diverged.

## Engine findings still OPEN

| Finding | Where | Status |
|---|---|---|
| `G-ENGINE-PARTITION-PLAYS-NOT-SIMULTANEOUS` | `docs/RUST_ENGINE_GAPS.md` | NEW, logged by this job while fixing `G-ENGINE-PARTITION-SLOT-ENFORCEMENT-DEFERRED`. `<Partition>`'s specified cards are still played sequentially, so a later card's would-play window can see an earlier one already on the field; §16-28-6 and judge-quiz Q30 call the plays simultaneous and DCGO batches them through one `PlayCardClass`. |
| `fire_effect_security_removal` re-enters the drain mid-body | inside the `G-ENGINE-USED-OPTION-BODY-VS-PENDING-SIBLING-TRIGGER` entry | Adjacent exposure left open deliberately. It calls `drain_effect_queue` (not `maybe_drain_effect_queue`), so ANY effect body that removes a security card re-enters the drain mid-body — which 15-8-3-2 also forbids. Only the Option-`[Main]` window was closed; the general case needs its own driver clause, citation and blast-radius measurement first. |
| `F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE` (second half only) | `docs/RUST_ENGINE_GAPS.md` | `play_from_hand_free` silently accepts an Option/Dual hand card and plays it to the battle area as a permanent. A pool sweep for `play_from_hand_free` fed by a `kind: option` pick is warranted. |
| `BT25-091#effect#2` re-diff is not clean | `qa/dcgo-exams/BT25/NOTES-BT25-091.md` | See the section above. |
| `G-EXAM-REVEAL-BUCKET-ADD-TIMING` | `docs/RUST_ENGINE_GAPS.md` | DCGO quirk, explicitly **not** an engine gap. Four measured drivers. |
| Job-1 findings not taken up here | `docs/RUST_ENGINE_GAPS.md` | `F-DATA-LINK-OPTION-PRINTED-LINK-BOX-NOT-AUTHORED`, `F-DATA-DUAL-CARD-KIND-MISSING-IN-CARDS-JSON`, `F-ENGINE-TRIGGER-GATE-IS-TRIGGER-TIME-NOT-ACTIVATION-TIME`, `F-ENGINE-RETALIATION-LOST-BEHIND-TRIGGER-ORDER`, `F-CARD-BT24-088-DRAW-NOT-GATED-ON-COST` / `F-CARD-BT25-078-YAML-COLOUR`, `F-CARD-BT16-077-WD-WHOLE-CLAUSE-DNA-GATED`, `F-ENGINE-FREE-OPTION-USE-IS-AFFORDABILITY-GATED`. |

## Reproducing

```bash
# recompute the denominator from disk
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "Three Musketeers" --json

# CI-safe, no Unity: re-check one scenario's assertions (each scenario needs its own book)
dcgo-harness --root <harness-root> exam --scenario qa/dcgo-exams/BT25/BT25-078-effect0.yaml \
  --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/BT25/tm_bt25_078_pool.json

# re-diff a clause against its PRESERVED sidecar -- a real oracle measurement,
# zero Unity time. The sidecar for a clause is named in
# <harness-root>/done/exam-<clause>.result.json -> recording_path (.state.jsonl).
dcgo-harness --root <harness-root> exam --scenario qa/dcgo-exams/BT19/BT19-075-effect2.yaml \
  --sidecar <recording>.state.jsonl --cards-json data/cards.json \
  --decks qa/dcgo-exams/BT19/tm_bt19_075_pool.json --all-diffs

# a fresh oracle pass needs the DCGO player build scripted-v16 -- see docs/DCGO_EXAM.md
```

## What the next dispatch should take first

1. **Triage `BT25-091#effect#2`** — the only clause in the pool whose stored
   `confirmed` is not currently backed by a clean re-diff. Decide whether the
   activation-cost snapshot boundary is a harness property or a real divergence.
2. **`G-ENGINE-PARTITION-PLAYS-NOT-SIMULTANEOUS`** — §16-28-6 and judge-quiz Q30 both
   say the plays are simultaneous; ours are sequential.
3. **The `fire_effect_security_removal` nested drain** — 15-8-3-2 forbids it in
   general, and only the Option-`[Main]` window is closed. Needs a driver clause and a
   blast-radius measurement before the call site is touched.
4. **BT26 ingest** — the last 2 unmeasured clauses in this pool, and (per the standing
   note) the blocker on the EX12 meta top generally.
5. **EX7-040#effect#0 stays unreachable** until a card exists that separates the
   "[Three Musketeers] in text" Lv.2 branch from the printed Black Lv.2 circle. Do not
   spend a dispatch trying to author around it.
