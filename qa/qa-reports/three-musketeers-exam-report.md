# Three Musketeers — DCGO card-clause exam report

**Date:** 2026-09-19 — **Campaign:** `three-musketeers-1` (node `james-desktop`) —
**Oracle:** DCGO player build `D:/dcgo-build/scripted-v15` (`dcgo_commit c98bab2a4`,
action-space digest `711d23bf`) — **Store:** `qa/qa-reports/exam-verdicts/<CARD-ID>.json`
— **Attempt log:** `qa/qa-reports/exam-log.jsonl` — **Scenarios:** 203 under
`qa/dcgo-exams/` for the 60 claimed cards — **Meta source:** `data/deck_library.json`,
102 tournament lists (EX12 format).

## The denominator, always first

**224 clauses across the 64-card Three Musketeers pool: 194 confirmed — 4 diverged
— 8 unreachable — 1 unavailable — 17 unmeasured.**
(Updated 2026-09-20 by the close-out job `three-musketeers-2`: `BT25-091#effect#2` and
`BT24-030#effect#4` re-diffed CLEAN against their preserved sidecars and moved
`diverged → confirmed` — see "Re-measured at close-out" below.)

**The 15-card competitive core (in ≥72 of 102 lists) is at 54 of 54 clauses
adjudicated — 49 confirmed, 1 diverged, 4 unreachable, 0 unmeasured** — every core
clause either ran clean against the oracle or carries a named, *measured* reason.
There are **zero untriaged `diverged`** in the pool: all four carry a triage class
and a citation (below).

Recomputed from disk at close, not from stage reports:

```
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "Three Musketeers" --json
  → denominator {total_cards: 64, total_clauses: 224,
                 confirmed 194, diverged 4, unreachable 8, unavailable 1, unmeasured 17}
```

The five classes sum to the denominator per card and in total; `exam_binding.bind()`
appends exactly one class per clause, and the index refuses to render a row whose
counts do not sum.

Never read this as "Three Musketeers passed". Read it per clause: a `confirmed` is a
scripted line that ran identically in our engine and in DCGO under a full per-step
state diff; everything else is exactly as unproven as it says.

### `--sim-only` is not confirmation

The 203-scenario corpus lowers 203/203 in our engine alone with every backfilled
`assert:` green (re-run at close, 2026-09-19). That proves the lines are still legal
*here* after this campaign's 15 engine/DSL changes; it says nothing about DCGO's
prompt sequence. Only an oracle diff moves a clause to `confirmed`.

Two scenarios had to be repaired at close to stay lowerable, and the reason is
itself a result: `LM-056-effect2.yaml` and `P-108-effect1.yaml` addressed their
`<Delay>` target as `own.field.1` because *our* engine used to park the pick while
the Option still sat in slot 0. Engine fix `70694334d` (a `<Delay>` pays its
trash-this-card cost **before** the body, `general_rule.pdf` 16-16-1) moved us onto
DCGO's ordering, so the Digimon is now `own.field.0` on both wires. Both clauses were
already `unmeasured`; no confirmed clause regressed.

### Full-suite regression at close

```
RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml \
  --test cards_behavioral -- --test-threads=8
  → test result: ok. 8198 passed; 0 failed; 37 ignored (281s)
```

## Per-card denominator

| Card | Name | In lists | Clauses | confirmed | diverged | unreachable | unavailable | unmeasured |
|---|---|---|---|---|---|---|---|---|
| BT21-071 **(core)** | Scopemon | 102/102 | 5 | 5 |  |  |  |  |
| EX7-051 **(core)** | Sparrowmon | 102/102 | 3 | 3 |  |  |  |  |
| EX7-071 **(core)** | Hurricane Screw Shot | 102/102 | 3 | 3 |  |  |  |  |
| P-180 **(core)** | Bind Red Trigger | 102/102 | 3 | 3 |  |  |  |  |
| BT21-074 **(core)** | Satellamon | 100/102 | 5 | 5 |  |  |  |  |
| EX7-073 **(core)** | BeelStarmon (X Antibody) | 100/102 | 3 | 2 |  | 1 |  |  |
| BT24-088 **(core)** | Asuna Shiroki | 99/102 | 3 | 3 |  |  |  |  |
| EX7-070 **(core)** | Der Blitz | 97/102 | 3 | 3 |  |  |  |  |
| BT25-005 **(core)** | Pagumon | 96/102 | 1 | 1 |  |  |  |  |
| BT25-078 **(core)** | Gazimon | 96/102 | 4 | 4 |  |  |  |  |
| BT25-082 **(core)** | BlackGatomon | 96/102 | 4 | 4 |  |  |  |  |
| BT25-083 **(core)** | LadyDevimon | 96/102 | 4 | 4 |  |  |  |  |
| BT25-085 **(core)** | BeelStarmon | 96/102 | 7 | 4 |  | 3 |  |  |
| BT25-092 **(core)** | Asuna Shiroki | 95/102 | 3 | 3 |  |  |  |  |
| EX7-008 **(core)** | ToyAgumon | 94/102 | 3 | 2 | 1 |  |  |  |
| BT25-058 | Callismon | 50/102 | 6 | 5 |  |  |  | 1 |
| LM-056 | Image Training | 38/102 | 4 | 2 |  |  |  | 2 |
| BT24-081 | Titamon + SkullBaluchimon | 37/102 | 6 | 6 |  |  |  |  |
| EX4-074 | ShineGreymon: Ruin Mode | 36/102 | 3 | 3 |  |  |  |  |
| BT25-028 | Dianamon | 24/102 | 5 |  |  |  |  | 5 |
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
| BT8-084 | Kimeramon | 6/102 | 3 | 2 |  | 1 |  |  |
| EX7-040 | ToyAgumon | 6/102 | 3 | 2 |  | 1 |  |  |
| EX7-044 | Gigadramon | 6/102 | 3 | 3 |  |  |  |  |
| P-170 | AvengeKidmon | 6/102 | 6 | 6 |  |  |  |  |
| BT7-056 | Dorumon | 4/102 | 2 | 1 | 1 |  |  |  |
| EX7-013 | MagnaKidmon | 4/102 | 3 | 3 |  |  |  |  |
| BT17-070 | Gulfmon | 2/102 | 3 | 3 |  |  |  |  |
| BT25-064 | ToyAgumon | 2/102 | 3 | 2 | 1 |  |  |  |
| BT25-100 | Iron Slash | 2/102 | 8 | 7 |  | 1 |  |  |
| BT26-078 | *(no card data)* | 2/102 | 1 |  |  |  |  | 1 |
| EX1-066 | Analog Youth | 2/102 | 3 | 3 |  |  |  |  |
| EX7-011 | Megadramon | 2/102 | 3 | 3 |  |  |  |  |
| P-198 | DemiDevimon | 2/102 | 4 | 4 |  |  |  |  |
| P-212 | Asuna Shiroki | 2/102 | 4 | 4 |  |  |  |  |
| BT16-077 | Dinobeemon | 1/102 | 5 | 4 |  |  |  | 1 |
| BT19-075 | MoonMillenniummon | 1/102 | 4 | 4 |  |  |  |  |
| BT20-020 | Imperialdramon: Fighter Mode | 1/102 | 5 | 5 |  |  |  |  |
| BT21-054 | Shotmon | 1/102 | 4 | 4 |  |  |  |  |
| BT24-011 | Cyclonemon | 1/102 | 4 | 4 |  |  |  |  |
| BT24-030 | Neptunemon | 1/102 | 5 | 4 | 1 |  |  |  |
| BT25-026 | Crescemon | 1/102 | 4 | 2 |  |  |  | 2 |
| BT25-091 | Monica Simmons | 1/102 | 4 | 3 | 1 |  |  |  |
| BT25-093 | Ignition Flare | 1/102 | 6 | 5 |  | 1 |  |  |
| BT26-083 | *(no card data)* | 1/102 | 1 |  |  |  |  | 1 |
| BT3-096 | Mimi Tachikawa | 1/102 | 2 | 2 |  |  |  |  |
| BT3-105 | Breath of the Gods | 1/102 | 2 | 2 |  |  |  |  |
| BT6-060 | Deputymon | 1/102 | 2 | 1 | 1 |  |  |  |
| BT6-105 | Gewalt Schwärmer | 1/102 | 3 | 3 |  |  |  |  |
| BT7-071 | Loweemon | 1/102 | 1 | 1 |  |  |  |  |
| BT8-071 | Psychemon | 1/102 | 1 | 1 |  |  |  |  |
| BT8-094 | Digimon Emperor | 1/102 | 3 | 3 |  |  |  |  |
| EX7-010 | Deputymon | 1/102 | 4 | 4 |  |  |  |  |
| P-108 | Wisdom Training | 1/102 | 3 | 1 |  |  |  | 2 |
| ST13-08 | Chikurimon | 1/102 | 1 |  |  |  | 1 |  |

`EX1-066` and `P-130` were confirmed by the earlier `toho-braves-exam` campaign and
are counted here but were neither claimed nor re-run by this job (their exam-log
lines belong to that job).

## Cards: implemented, blocked, unimplementable

**31 cards reached `IMPLEMENTED` under this campaign** — 29 new YAML specs plus two
lifted from `PARTIAL`:

| Stage | Commit | Cards |
|---|---|---|
| slice `ex7-line` | `91f16817f` | EX7-008, EX7-010, EX7-011, EX7-040, EX7-043, EX7-044, EX7-051 |
| slice `splash-digimon` | `f308bd317` | BT6-060, BT7-071, BT7-073, BT8-071, BT17-070, BT24-081 |
| slice `ex7-tops` | `9aa21ed0f` | BT21-054, BT21-074, BT25-085, EX7-013, EX7-059, EX7-066, P-170 |
| slice `tamers-options` | `c7581932c` | BT3-096, BT3-105, BT6-105, BT18-093, P-108, P-212 |
| gap closure (BT25-092) | `ceebcb6e3` | BT25-092 (PARTIAL → IMPLEMENTED) |
| engine stage (OnUseOption) | `556166895` | BT25-091 (PARTIAL → IMPLEMENTED) |
| OnAddDigivolutionCards drivers | `61fc7c7d9` | BT25-005, BT7-056, EX7-005 |

**Blocked: none.** The three cards blocked mid-campaign on a missing engine trigger
(`EX7-005` Kapurimon, `BT7-056` Dorumon, `BT25-005` Pagumon) were unblocked by the
`OnAddDigivolutionCards` timing (`d74328811`) and implemented in the same campaign.

**Unimplementable — reported, not hidden (2 cards, 2 clauses, both `unmeasured`):**

| Card | Reason |
|---|---|
| BT26-078 | No card data — `data/cards.json` carries no BT26 set, and DCGO has no script for it. |
| BT26-083 | Same: no BT26 card data, no DCGO script. |

Both appear in 1–2 of the 102 lists. Until a BT26 ingest lands, nothing can be
authored or examined for them; their single `#security#0` slot is `image-required`
in the denominator and stays `unmeasured` by cause, not by neglect.

## Substrate widenings landed — FLAG FOR HUMAN REVIEW

Rule 28's flywheel: every card the DSL could not express widened the substrate
rather than being routed around. **Engine fixes land flagged for human review.**

### Engine fixes (15) — each has a rule/DCGO citation and a failing-then-passing test

| Commit | Fix | Citation | Driver clause |
|---|---|---|---|
| `c7581932c` | `OnUseOption` observers fire **after** the Option's `[Main]` body | DCGO stacking + official BT3-096 Q&A | BT3-096 |
| `d74328811` | `OnAddDigivolutionCards` trigger timing + per-host batch window (DSL `when: on_add_digivolution_cards`) | `general_rule.pdf` 15-5-2 / 15-5-3; `Permanent.cs:1078-1240` | BT25-005 / EX7-005 / BT7-056 |
| `cf8e2519f` | From-hand Digimon link initiation (`G-ENGINE-DIGIMON-LINK-FROM-HAND`) | DCGO `InputDriver` link path | BT21-071 / BT21-074 |
| `35958972b` | `OnUseOption` observers see **who used what**; security-flipped Plug-In Options can link (`G-ENGINE-ON-USE-OPTION-EVENT-CARD`, `G-ENGINE-SECURITY-OPTION-LINK-TO-OWN-DIGIMON`) | DCGO `BT25_091.cs`, `CardController.cs` | BT25-091 |
| `a46c74066` | `<Retaliation>` resolves **once** and survives a parked trigger-order prompt; inherited-text keywords no longer attach to the card's own face; battle opponent captured at trigger time (`G-ONDELETION-PARK-CLEARS-BATTLE-STATE`) | `general_rule.pdf` §16-12-1/-3/-4; `BT25_078.cs:113-115` | BT25-078#inherited#0, EX7-051#inherited#0 |
| `24f8e6551` | A free Option **USE** is `use_option_from_hand`, and the outer optional first step is declinable (`G-OUTER-OPTIONAL-USE-OPTION-FIRST-STEP`) | `EX7_073.cs:81-130` | EX7-073#effect#1 |
| `baba1a0ea` | New DSL step `delete_permanents` — a multi-target delete is **one simultaneous** `delete_permanents_batch` (`G-DSL-DELETE-PERMANENTS-BATCH`) | `general_rule.pdf` §15-4-3-2; `EX7_071.cs:170-252` | EX7-071#effect#1 / #effect#2 |
| `a263c9aff` | Attack declaration suspends via `suspend_state_only` and fires the suspend trigger alongside `[When Attacking]` | `general_rule.pdf` 11-2-1 / 11-1-4; `AttackProcess.cs:166` | BT24-030#effect#3 |
| `52579f0a8` | A security-played **Tamer** honours `CannotPlayTamerByEffect` | `CardEffectFactory.cs:165-169` | BT20-020#effect#3 |
| `edbb740c0` | `<Blast Digivolve>` draws 1 card like every digivolution | `general_rule.pdf` 8-1-3-3 / 16-25-4; `CardController.cs:1762` | EX7-059#effect#1 |
| `5108e58ee` | A used Option is trashed **before** the triggers its `[Main]` caused | `general_rule.pdf` 9-1-5 / 18-1-2 / 15-4-3-5 | P-170#effect#5 |
| `701fad56a` | `<Overflow>` is **owner**-relative, not turn-player-relative | `general_rule.pdf` 4-17-1 / 4-1-4 / 4-17-5; `CardController.cs:6151` | EX7-059#effect#4 |
| `fb5517067` | Additive colour treatment + colour-count DP-minus (`synth_identity`, `source_rules_color_count`, `self_color_count_gte`) | `BT8_084.cs` `ChangeCardColorClass` + official Q&A | BT8-084#effect#1 / #effect#2 |
| `c652676d1` | A queued effect's **body-raised triggers wait** until the body resolves (was Option-`[Main]`-only) | `general_rule.pdf` 15-8-3-2 | EX4-074#effect#1 / #effect#2 |
| `70694334d` | `<Delay>` pays its trash-this-card cost **before** the body | `general_rule.pdf` §16-16-1; `LM_032.cs:146-158` | LM-032#effect#1 |

Each landed only under the campaign's three-part gate: a citation, a test that fails
before and passes after, and `cards_behavioral` green.

### DSL vocabulary widenings (10)

- `grant_traits` on an aura; `return_union_bound_to_deck` (`91f16817f`).
- Draw-formula `count`; `distinct_names_count`; `link_card_count`; trash-an-Option-from-sources
  **as a cost** (`9aa21ed0f`).
- `can_digivolve_from_source` now enumerates routes via `Game::all_digivolve_routes_for_card`
  (printed circles + DSL alt-paths), so a hand filter offers exactly what the commit
  accepts (`61fc7c7d9`).
- `can_digivolve_onto` (card-subject) and `has_digivolve_candidate` (permanent-subject)
  predicate leaves, closing `G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST` (`ceebcb6e3`).
- `delete_permanents` batch step (`baba1a0ea`, listed above — it is both).

### Exam/harness tooling widenings (3, plus DCGO-side codegen)

- `link:` scenario verb — a DigiLink declaration from field *or* hand, on both wires
  (`be2aed9ce`); needs DCGO-side `InputDriver` + regenerated `ActionSpace.cs`
  (`403f164ab`, `c98bab2a4` in the **base-repo** DCGO; gitlink deliberately not bumped),
  built into oracle `scripted-v15`.
- `select: { choice: "<label>", sim_only: true }` — answer an `EffectChoice` branch by
  label (`d5f23792f`).
- `select: { cards: [...], sim_only: true }` / `dcgo_only` row alignment used throughout
  to keep prompt-count asymmetries honest rather than papered over.

## The 6 diverged, triaged — 4 DCGO quirks, 1 rules-ambiguous, 1 ours

`diverged` means both engines ran the line end to end and disagreed. It is a finding
to triage, not proof we are wrong: `general_rule.pdf` outranks DCGO.

| Clause | Class | Citation | Disposition |
|---|---|---|---|
| EX7-008#effect#1 | DCGO quirk | `general_rule.pdf` §15-15-10-1; `RevealLibrary.cs:229`, `SelectCardEffect.cs:726` | One printed "Add" with two targets is one action; DCGO adds each bucket's pick as its prompt closes, ours adds after the last pick. Only diff is `p0.hand` at the middle step; end state agrees. Logged `G-EXAM-REVEAL-BUCKET-ADD-TIMING`. |
| BT7-056#effect#0 | DCGO quirk | `general_rule.pdf` §15-15-10-1 / §15-1-2; `RevealLibrary.cs:291-336` | Same family (Dorumon's two-bucket reveal). `NOTES-BT7-056.md`. |
| BT6-060#effect#0 | DCGO quirk | `general_rule.pdf` 15-1-2 / 15-1-6; `RevealLibrary.cs:291-328` | Same family; `--all-diffs` shows one field at one step, end state identical. |
| BT25-064#effect#1 | DCGO quirk | `general_rule.pdf` §15-15-10-1 (p.31) / §15-1-2 / §15-1-4 (p.22) / §15-15-3-2 (p.29); `BT25_064.cs:60-80`, `RevealLibrary.cs:291-329`, `SelectCardEffect.cs:751,813-815` | Same family ("Add 1 Option card and 1 [TS] trait card"). **Re-triaged from scratch at close-out (2026-09-21)** in the full order of evidence and re-measured `--all-diffs` against the preserved sidecar `20260921T042338Z_438b4ce3`: step 3 `p0.hand ours=[BT2-052,BT2-056,BT3-059,BT8-061] dcgo=[..., EX7-070]` is the LEAD and the ONLY row; not a slot artefact (no `field.N`; P0 holds one permanent). Triage written into the verdict `reason` + `NOTES-BT25-064.md`; recorded as a **measured** driver of `G-EXAM-REVEAL-BUCKET-ADD-TIMING`. No engine/DSL/card change. |

**Untriaged diverged: 0.**

### Re-measured at close-out (2026-09-20, `three-musketeers-2`)

`BT25-091#effect#2` and `BT24-030#effect#4` were both logged under
`G-ENGINE-OPTION-TRASH-VS-ON-USE-TRIGGER-ORDER` from oracle runs taken *before*
engine fix `5108e58ee` ("a used Option is trashed before the triggers its `[Main]`
caused", 9-1-5 / 18-1-2 / 15-4-3-5) landed, and were never re-diffed against it.
Re-run against their PRESERVED sidecars (zero Unity time) they are **CLEAN** —
12/12 and 18/18 steps compared — and both verdicts are now `confirmed`. The gap is
RESOLVED in `docs/RUST_ENGINE_GAPS.md`; a regression test
(`bt24_030_used_option_is_trashed_before_the_non_turn_players_on_suspend_prompt`)
now pins the non-turn-player half, which had no coverage. The residual — the Option's user may
*order* the trash among their **own** pending triggers — was taken up at close-out
and is no longer ambiguous: `general_rule.pdf` 9-1-5 + 18-1-2 + 15-4-3-2 +
**15-4-3-5-1** grant that pick, so the engine now surfaces it
(`G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER`, RESOLVED 2026-09-20, `5aef07fa8`). DCGO's
trash-first remains one of the two legal answers and is what every scripted line
picks; the BT25-091#effect#2 scenario carries a `sim_only` row for the
engine-only prompt.

```
RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml   --test cards_behavioral -- --test-threads=8
  → test result: ok. 8199 passed; 0 failed; 37 ignored (420s)   # 2026-09-20, close-out
```

## The 8 unreachable + 1 unavailable — each measured

| Clause(s) | Measured cause |
|---|---|
| BT25-085#effect#4 / #5 / #6 (core) | Recorded against the card-data gap "cards.json carries BT25-085 as `card_kind 0` with no `dual` block". **That cause was fixed mid-campaign** by `20df249e6` (official DB `data/card_bundles/BT25-085.md` + `BT25_085.cs:60`), so the DUAL Option face is authorable now and simply was not re-authored before close. Staleness recorded in `qa/dcgo-exams/BT25/NOTES-BT25-085.md`; **next dispatch's first pick.** |
| EX7-073#effect#2 (core) | The cost prompt is a `SelectionKind::SourceMulti` the exam resolver cannot answer by identity (two sources, so the single-accept `yes:` path does not apply) — `G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK`. Tooling gap, not a card gap. |
| BT25-093#effect#4, BT25-100#effect#5 | "Plug this Option in from hand" has no shared wire action: our plug-in rides the PLAY bit behind a mode-select, DCGO's is a separate `ActivateCardAction` the harness only emits from a `HAND_EFFECT` bit — `G-TOOLING-EXAM-OPTION-HAND-LINK`. |
| BT8-084#effect#0 | `[DNA Digivolve]` has no verb on either wire: `scenario.rs STEP_VERBS` has no DNA verb, `lower::matches_intent` no `DnaDigivolve` arm, and DCGO's `InputDriver.BuildMainPhaseAction` refuses the DNA action range — `G-TOOLING-EXAM-NO-DNA-VERB`. The oracle exists; the wire does not. |
| EX7-040#effect#0 | `[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0` has **no separating line**: the only Lv.2 cards printing that text (EX7-005, BT25-005) are both Black, so the printed Black Lv.2 cost-0 circle opens at the same cost and the line would confirm the wrong clause. |
| ST13-08#effect#0 | `unavailable`: DCGO has no script for Chikurimon. Checked per card, not per set — `ST13/{Black,Red}/` exist and hold ST13_01..06/09/11/13..16; `find -name 'ST13_08*'` returns nothing. `NOTES-ST13-08.md`. Recorded into the verdict store at close (it had been asserted in a commit message but never landed in the ledger). |

## The 17 unmeasured — and why

| Clauses | Why |
|---|---|
| BT25-028 ×5 (Dianamon) | 3 scenarios authored and lowering, 2 unauthored; the card's oracle pass never ran before the session limit. 24/102 lists — the largest single block of unproven clauses in the pool. |
| BT25-026#effect#2, #inherited#0 | Scenarios authored and lowering; no oracle run. |
| LM-056#effect#2, #effect#3 | Scenarios authored and lowering (effect#2 repaired at close, see above); no oracle run. |
| P-108#effect#0, #effect#1 | Same (effect#1 repaired at close). |
| BT25-058#effect#3 | Scenario authored and lowering; no oracle run. |
| ~~BT19-075#effect#2~~ | **CLOSED in three-musketeers-2.** The line was re-authored slot-safe, run on the oracle (sidecar `20260921T041928Z_40f7887a`) and measured `diverged` on a single LEAD row (`p1.security ours=4 dcgo=3`): the card's own `[All Turns][Once Per Turn]` deletion observer missed the deletion its leave-replacement paid as a cost. That was a real engine finding, but NOT the replacement-specific one the notes predicted — `PermanentHandle` is a battle-area INDEX and the area compacts on removal, so a deletion trigger's `event_permanent` aliased onto the survivor that slid into the dead slot, and `event_permanent_is_source: false` rejected the carrier's own trigger. Pool-wide, and reproducible with no replacement at all. Fixed by `4c609b5e2` (`G-ENGINE-REPLACEMENT-COST-DELETION-NO-OBSERVER`, now RESOLVED). Re-diffed against the same preserved sidecar: **CLEAN (22 of 22)**, verdict `confirmed`. **BT19-075 is 4/4 confirmed.** |
| BT16-077#effect#0 (Special Digivolution Condition) | No scenario authored. |
| ~~BT24-091#effect#4 (Link Condition)~~ | **CLOSED in three-musketeers-2.** Scenario `qa/dcgo-exams/BT24/BT24-091-effect4.yaml` authored 2026-09-21, run on the oracle, and measured `diverged` on a single LEAD row (the host pick: `memory ours=0 dcgo=3`, the Option already out of our hand). That was a real engine finding — the from-hand Plug-In **Option** link ran §10-1-3-2 (pay) before §10-1-3-1 (choose the host) — and is fixed by `9af21698c` (`G-ENGINE-OPTION-HAND-LINK-COST-TIMING`, now RESOLVED). Re-diffed against the same preserved sidecar: **CLEAN**, verdict `confirmed`. **BT24-091 is 6/6 confirmed.** |
| BT26-078#security#0, BT26-083#security#0 | Unimplementable — no BT26 card data (above). |

Nothing in this list is blocked on a technical impossibility except the BT26 pair;
the rest is unauthored or unrun tail work on 1-of and 2-of support cards, which the
campaign gate deliberately does not hold the line for (core first).

## Engine findings still OPEN (not fixed — logged)

| Gap | Where |
|---|---|
| ~~`G-ENGINE-MAIN-ON-FIELD-ACTIVATION-COST-UNPAID`~~ **RESOLVED 2026-09-20** (three-musketeers-2) | `activate_field_main` now pays `activation_cost_fn`, and `field_main_match` gates the `[Main]` bit on a new data-twin `ActivationCostKind::is_payable` probe. general_rule.pdf §15-7-1/2; DCGO `BT25_089.cs:36` → `CanSuspend.cs:17-25`. 4 tests (BT25-089 / EX11-071); entry in `qa/resolved-gaps.md`. |
| ~~`G-ENGINE-PARTITION-SLOT-ENFORCEMENT-DEFERRED`~~ **RESOLVED 2026-09-20** (three-musketeers-2) | `<Partition>` now reads the card's printed parenthetical (`CardEffect::partition_slots`), gates the trigger on a complete slot assignment and plays exactly 1 of each specified card. general_rule.pdf §16-28-1/-5/-6; DCGO `Partition.cs:66-119,145-159` + `:89,117`. 3 tests (BT16-077); BT16-077#effect#2 and #inherited#0 re-diffed CLEAN against their preserved sidecars. Follow-up `G-ENGINE-PARTITION-PLAYS-NOT-SIMULTANEOUS` logged. |
| ~~`F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST`~~ **RESOLVED 2026-09-20** (three-musketeers-2) | `option_legal_play_modes` now drops the Link play mode when `link_host_candidates` is empty, so a dual-mode Plug-In Option no longer offers (or executes) a hostless "Plug in" branch. general_rule.pdf §10-1-3-1 / §6-5-1-4; DCGO `Link.cs:24,53`. 3 tests (BT25-100) + ST22-08 / BT25-093 cases retargeted; entry in `qa/resolved-gaps.md`. Commit `f24b986d0`. The Option **hand** plug-in half (BT25-100#effect#5 / BT25-093#effect#4 `unreachable`) is unchanged. |
| **NEW** `BT25-091#effect#2` re-diff is not clean (pre-existing, not from the fix above) | `qa/dcgo-exams/BT25/NOTES-BT25-091.md` — one field, one row: `p0.field[0].suspended: ours=true dcgo=false` at the OptionalSkill accept. Reproduced on the unmodified engine; appeared with `5aef07fa8`'s TriggerOrder prompt. Stored verdict is still `confirmed` and was deliberately left untouched pending triage. |
| ~~Condition-false scheduled-`<Delay>` carrier trash~~ **RESOLVED 2026-09-20** (three-musketeers-2) | `resolve_delayed_options_matching` now probes the carrier's `DelayEffect` conditions before enqueueing and, when the printed gate is false, leaves the carrier in the battle area and reschedules its window instead of trashing it. general_rule.pdf §16-16-1/-2; DCGO `LM_032.cs:137-143,146-158`. 2 tests (LM-032); all three LM-032 exam clauses re-diffed CLEAN against their preserved sidecars. Commit `5f091f5b2`; entry in `qa/resolved-gaps.md`. |
| `G-EXAM-REVEAL-BUCKET-ADD-TIMING` | DCGO quirk, explicitly **not** an engine gap |

## Reproducing

```bash
# recompute the denominator from disk
PYTHONPATH=code python -m tools.clause_coverage.campaign --archetype "Three Musketeers" --json

# CI-safe, no Unity: re-check one scenario's assertions (each scenario needs its own book)
dcgo-harness --root <harness-root> exam --scenario qa/dcgo-exams/BT25/BT25-078-effect0.yaml \
  --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/BT25/tm_bt25_078_pool.json

# oracle pass (needs the DCGO player build scripted-v15) -- see docs/DCGO_EXAM.md
```

## What the next dispatch should take first

1. **BT25-085's three DUAL clauses** — core, and their blocking cause is already fixed.
2. **BT25-028 (Dianamon)** — 5 unmeasured clauses, 24/102 lists, 3 lines already lowering.
3. ~~The Option-trash ordering gap~~ — **closed 2026-09-20**, both halves: the two
   `G-ENGINE-OPTION-TRASH-VS-ON-USE-TRIGGER-ORDER` drivers re-measured CLEAN against
   `5108e58ee` (verdicts `confirmed`, regression test added), and the residual
   `G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER` was fixed in the engine — 15-4-3-5-1
   gives the Option's user the order, so the pick is now a two-entry `TriggerOrder`
   instead of a hard-coded trash-first (8 sites in the whole behavioural suite;
   BT3-096 / BT25-091).
