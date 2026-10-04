# TS Jupitermon: exam notes (2026-10-04)

TS Jupitermon is the #1 BT26 deck (10.5% of the field per DigiLab). Its 17 core cards
lead the `exam_next` queue (`PYTHONPATH=code python -m tools.meta_coverage`, BT26
window). Every printed clause of those cards now has a scenario or a recorded reason
why it has none.

## State

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT26-033
BT26-073 BT26-081 BT26-029 BT26-103 BT26-083 BT26-090 BT26-092 BT24-034 BT24-101 BT24-031
BT24-102 BT24-083 BT24-041 BT25-022 BT25-025 P-213`): **86 clauses over 17 cards**
(76 effect, 10 inherited, all from the official Bandai bundle, 0 image-required).

| | Clauses |
|---|---|
| Scenario authored, passes `--sim-only` | **83** |
| `unreachable`: printed "ー", there is no inherited effect | **3** (`BT26-081#inherited#0`, `BT26-103#inherited#0`, `BT26-083#inherited#0`) |
| `confirmed` | **0**: no oracle pass has been run |

**All 86 clauses are still `unmeasured` in the verdict sense.** A scenario that lowers is
not a measurement. Every `assert:` block was written from OUR engine's result, not
backfilled from DCGO, and is provisional until the oracle pass. Every scenario header
says so. No verdict files were written.

Several scenarios deliberately assert behaviour we believe is wrong, so the oracle shows
it as a divergence. See "Expected aborts and divergences" below.

## Deck book

`qa/dcgo-exams/BT26/ts_jupitermon_pool.json`. This one file resolves every scenario in
this set and every other scenario in `qa/dcgo-exams/BT26/`, including the Data Squad ones.

| Deck | What it is |
|---|---|
| `ts-jupitermon` | A real BT26 list: 10th place, dcg-nexus `6da588b7-4c01-4ff3-9b9e-63caf1762be6` (2026-09-07, themilkmann, `data/deck_library.json` → `TS Jupitermon`). It covers 15 of the 17 cards. **Two 1-copy swaps** add the other two: −1 BT24-084 Inori +1 BT26-092 Shota, and −1 BT24-088 Asuna +1 BT26-090 Kanan. Eggs: 4× BT24-003. |
| `tm-quiet-red` | The shared quiet ST1 red opponent, the same deck as in the BT24/BT26 books. |
| `bt26-dats` | The existing Data Squad deck, copied in so the BT26 directory runs against one book. |
| `tsj-p-213-dragonkin` | `ts-jupitermon` −1 BT26-103 +1 BT21-007. Used only by `P-213-effect4`, because nothing in the list reads [Dragonkin]. |
| `tsj-bt24-031-pagumon` | `ts-jupitermon` main deck, eggs 4× BT25-005 (black). Used only by `BT24-031-effect0`, because a yellow Tsunomon egg also satisfies Elecmon's yellow circle at the same cost. |
| `tsj-bt26-083-neptune` | `ts-jupitermon` −1 BT4-104 +1 BT24-030. Used only by `BT26-083-effect0`, because every Lv.6 in the list is yellow and would also meet a colour circle. |

Every variant is 50 main cards, at most 4 copies of any card, and 4 eggs.

## Running the oracle pass (local: Unity, base-repo DCGO)

The scenarios live in four set directories, so build the file list first. Scenarios
already in those directories that belong to other cards use other books.

```bash
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"   # see docs/DCGO_HARNESS.md
BOOK=qa/dcgo-exams/BT26/ts_jupitermon_pool.json
TSJ=$(mktemp -d)
for c in BT26-033 BT26-073 BT26-081 BT26-029 BT26-103 BT26-083 BT26-090 BT26-092 \
         BT24-034 BT24-101 BT24-031 BT24-102 BT24-083 BT24-041 BT25-022 BT25-025 P-213; do
  cp qa/dcgo-exams/${c%%-*}/$c-*.yaml "$TSJ"/
done        # 83 files

# 0. Clause text for the verdict fingerprints.
PYTHONPATH=code python -m tools.clause_coverage.extract --quiet --out "$TSJ/clauses.json" \
  --card-ids BT26-033 BT26-073 BT26-081 BT26-029 BT26-103 BT26-083 BT26-090 BT26-092 \
             BT24-034 BT24-101 BT24-031 BT24-102 BT24-083 BT24-041 BT25-022 BT25-025 P-213

# 1. Re-check sim-only and emit one scripted DCGO job per scenario (83 jobs).
cargo run -p dcgo-harness -- --root "$ROOT" exam --scenario "$TSJ" --sim-only \
    --cards-json data/cards.json --decks "$BOOK" --emit-job "$ROOT/jobs"
#   expect: scenarios seen 83 / lowered 83 / run 83 / diffed 0 / failed 0

# 2. Drain. Stop Play before queueing (a running harness claims jobs instantly).
cargo run -p dcgo-harness -- --root "$ROOT" node status        # must report GO
cargo run -p dcgo-harness -- --root "$ROOT" enable
cargo run -p dcgo-harness -- --root "$ROOT" watch --build <player-dir>   # or: Play in Unity + `status`
cargo run -p dcgo-harness -- --root "$ROOT" disable

# 3. Diff each scenario against the sidecar its job wrote, and record verdicts.
#    --sidecar takes a directory holding one <scenario-stem>.state.jsonl per scenario.
cargo run -p dcgo-harness -- --root "$ROOT" exam --scenario "$TSJ" \
    --sidecar <dir-of-state.jsonl> --cards-json data/cards.json \
    --verdicts --clause-text-json "$TSJ/clauses.json" --all-diffs
```

The DCGO build must be at or after `fc67f9ae6` (`scripted-v16`); the pinned submodule
commit `b9a0638cd` contains it. None of these lines use the `dna:` or `link:` verbs.

**Checked against DCGO C# (2026-10-04, pinned commit `b9a0638cd`).** All 17 cards (plus
BT25-044 Junomon) have a DCGO script, so none is `unavailable`. None register
`SetIsBackgroundProcess(true)`, so none is structurally unmeasurable. Every scenario's
DCGO-side rows (`expect:` prompts, `dcgo_only` rows, OptionalSkill folds, `value:`
answers) were derived from the card scripts and the shared selection classes. Each
scenario header carries its C# citations. The `assert:` blocks are still our engine's
result: only the oracle pass can confirm them.

## Scenario ↔ clause table

"Partial" means the line witnesses only part of the clause. The unwitnessed part is named.

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| **BT26-033 Jupitermon (DUAL)** |||
| `#effect#0` | [Digivolve] Lv.5 w/[TS]: Cost 4 | `BT26-033-effect0.yaml` | onto P-213, memory 3→−1 |
| `#effect#1` | `<Raid>` | `BT26-033-effect1.yaml` | redirect to Biyomon; P1 security stays 5 |
| `#effect#2` | `<Alliance>` | `BT26-033-effect2.yaml` | one attack checks 2 (5→3) |
| `#effect#3` | `<Engage>` | `BT26-033-effect3.yaml` | end-of-turn attack on security (re-authored after the DUAL end-of-turn fix, 2026-10-04) |
| `#effect#4` | [WD] top security → hand, play [Iliad]/[TS] at −5 | `BT26-033-effect4.yaml` | security 5→4; Inori played for 0 |
| `#effect#5` | [All Turns] protect [TS] by placing top card as bottom security | `BT26-033-effect5.yaml` | Elecmon survives Gaia Force; security 4→5 |
| `#effect#6` | DUAL: +1 use cost per security card | `BT26-033-effect6.yaml` | memory 0→−7 (2+5) |
| `#effect#7` | DUAL: `<Use Req. ([TS])>` | `BT26-033-effect7.yaml` | only a white Tamer on board; the use still resolves |
| `#effect#8` | DUAL [Main]: delete all lowest-DP Digimon, Recovery +1 | `BT26-033-effect8.yaml` | both 3000s deleted, 4000 stays, security 5→6 |
| `#effect#9` | DUAL Rule: `<Arts Digivolve>` | `BT26-033-effect9.yaml` | free digivolve onto P-213 |
| **BT26-073 Aegiochusmon: Dark** |||
| `#effect#0` | [Digivolve] [Aegiomon]: Cost 3 | `BT26-073-effect0.yaml` | memory 3→0 |
| `#effect#1` | OP/WD: delete self or return [Shaman]/[TS] → delete opp Lv.≤5 | `BT26-073-effect1.yaml` | "delete this" branch |
| `#effect#2` | On Deletion: play [TS] cost ≤5 free (+ Rule [Wizard]) | `BT26-073-effect2.yaml` | partial: the [Wizard] trait grant is not observed |
| `#effect#3` | Assembly −2 | `BT26-073-effect3.yaml` | play costs 6; Elecmon under Dark |
| `#inherited#0` | `<Security A. +1>` | `BT26-073-inherited0.yaml` | one attack 5→3 |
| **BT26-081 Mervamon** |||
| `#effect#0` | [Digivolve] [Minervamon]: Cost 2 | `BT26-081-effect0.yaml` | 3→1 |
| `#effect#1` | [Digivolve] Lv.5 w/[TS]: Cost 4 | `BT26-081-effect1.yaml` | onto red P-213 |
| `#effect#2` | OP/WD: play ≤8 cost of [Iliad] free, −4000 per [Iliad]/[TS] | `BT26-081-effect2.yaml` | 2 Elecmon played, −12000 deletes |
| `#effect#3` | [Iliad] gain Alliance/Reboot/Blocker/+2000 | `BT26-081-effect3.yaml` | partial: Blocker and +2000 observed; Alliance and Reboot not |
| `#effect#4` | Assembly −5: [Minervamon] | `BT26-081-effect4.yaml` | **pins a skipped [On Play] pick (engine bug)** |
| `#inherited#0` | "ー" | **unreachable** | no inherited effect printed |
| **BT26-029 Aegiochusmon: Holy** |||
| `#effect#0` | [Digivolve] [Aegiomon]: Cost 3 | `BT26-029-effect0.yaml` | cost choice `value: 3`, 3→0 |
| `#effect#1` | `<Decode ([Aegiomon])>` | `BT26-029-effect1.yaml` | Aegiomon played from sources |
| `#effect#2` | `<Ascension>` | `BT26-029-effect2.yaml` | security 5→6 |
| `#effect#3` | OP/WD: trash top security → protect 1 Digimon | `BT26-029-effect3.yaml` | partial: only the strip protection is witnessed (vs `<De-Digivolve>`); the DP and bounce halves are not |
| `#effect#4` | [All Turns][OPT] security removed → 3 opp −5000 | `BT26-029-effect4.yaml` | two deleted, one survives at 3000 |
| `#inherited#0` | security removed → `<De-Digivolve 1>` | `BT26-029-inherited0.yaml` | Birdramon stripped to Biyomon |
| **BT26-103 Jupitermon: Wrath Mode** |||
| `#effect#0` | [Digivolve] Lv.6 w/[Olympos XII]: Cost 5 | `BT26-103-effect0.yaml` | onto Minervamon, 3→−2 |
| `#effect#1` | `<Piercing>` | `BT26-103-effect1.yaml` | Biyomon deleted + 1 check |
| `#effect#2` | `<Reboot>` | `BT26-103-effect2.yaml` | unsuspends at P1's turn |
| `#effect#3` | `<Blocker>` | `BT26-103-effect3.yaml` | blocks Biyomon; see the [Counter] note |
| `#effect#4` | `<Succession ([Jupitermon])>` | `BT26-103-effect4.yaml` | copied WD and [All Turns] both observed |
| `#effect#5` | [WD][Counter][OPT] trash top security, Recovery +2 | `BT26-103-effect5.yaml` | partial: [When Digivolving] arm only; the [Counter] arm now fires in combat (DSL bug fixed 2026-10-04) but has no scenario yet |
| `#effect#6` | [All Turns][OPT] security removed → −15000 | `BT26-103-effect6.yaml` | the chosen Phoenixmon is deleted |
| `#inherited#0` | "ー" | **unreachable** | no inherited effect; Lv.7, so nothing can digivolve onto it either |
| **BT26-083 Junomon: Hysteric Mode** |||
| `#effect#0` | [Digivolve] Lv.6 w/[TS]: Cost 4 | `BT26-083-effect0.yaml` | onto blue/black Neptunemon (variant deck) |
| `#effect#1` | `<Rush>` | `BT26-083-effect1.yaml` | attacks the turn it was played |
| `#effect#2` | `<Piercing>` | `BT26-083-effect2.yaml` | deletes BT4-014 + 1 check |
| `#effect#3` | `<Execute>` | `BT26-083-effect3.yaml` | end-of-turn attack, then self-delete |
| `#effect#4` | `<Decode>` | `BT26-083-effect4.yaml` | Junomon played from sources |
| `#effect#5` | OP/WD: trash all security, delete 1 per card, Recovery +3 | `BT26-083-effect5.yaml` | 5 trashed, 2 deleted, security 3 |
| `#effect#6` | [On Deletion] opp `<Security A. −1>` | `BT26-083-effect6.yaml` | the attack checks 0 cards |
| `#effect#7` | Assembly −4: [Junomon] | `BT26-083-effect7.yaml` | 14−4 = 10 paid |
| `#inherited#0` | "ー" | **unreachable** | no inherited effect printed |
| **BT26-090 Kanan Yuki** |||
| `#effect#0` | [Start of Main] +1 memory at ≤4, plus [End of Your Turn] use a [TS] Option at reduced cost | `BT26-090-effect0.yaml` | both timing blocks witnessed (the extractor merged them into one clause) |
| `#effect#1` | [Security] play free | `BT26-090-effect1.yaml` | |
| **BT26-092 Shota Kuroi** |||
| `#effect#0` | [Start of Main] trash a [TS] card → Draw 1 + 1 memory | `BT26-092-effect0.yaml` | |
| `#effect#1` | [Opp Turn] bottom-deck a [TS] Tamer → redirect the attack | `BT26-092-effect1.yaml` | |
| `#effect#2` | [Security] play free | `BT26-092-effect2.yaml` | |
| **BT24-034 Aegiomon** |||
| `#effect#0` | [Digivolve] [Elecmon]/Lv.3 w/[TS]: Cost 2 | `BT24-034-effect0.yaml` | onto blue Lunamon |
| `#effect#1` | `<Barrier>` | `BT24-034-effect1.yaml` | survives; security 5→4 |
| `#effect#2` | [On Play] top security → hand, play a [TS] Tamer free | `BT24-034-effect2.yaml` | Inori played |
| `#inherited#0` | inherited `<Barrier>` | `BT24-034-inherited0.yaml` | carrier: BT25-025 |
| **BT24-101 Jupitermon** |||
| `#effect#0` | [Digivolve] Lv.5 w/[TS]: Cost 5 | `BT24-101-effect0.yaml` | |
| `#effect#1` | [Digivolve] Lv.5 [Aegiochusmon]: cost = security count | `BT24-101-effect1.yaml` | pays 4 at 4 security |
| `#effect#2` | OP/WD: trash top security, −13000; Recovery +2 at ≤1 | `BT24-101-effect2.yaml` | partial: the Recovery branch is not reached |
| `#effect#3` | [All Turns][OPT] security removed → trash opp top security | `BT24-101-effect3.yaml` | fires on two separate turns |
| `#effect#4` | [All Turns][OPT] [TS] would leave → trash top security instead | `BT24-101-effect4.yaml` | |
| **BT24-031 Elecmon** |||
| `#effect#0` | [Digivolve] Lv.2 w/[TS]: Cost 0 | `BT24-031-effect0.yaml` | black Pagumon egg (variant deck) |
| `#effect#1` | [On Play] reveal 3, add [Iliad] + [TS] | `BT24-031-effect1.yaml` | |
| `#inherited#0` | [When Attacking][OPT] may add top security; Recovery +1 at 0 | `BT24-031-inherited0.yaml` | partial: the Recovery-at-0 branch is not reached |
| **BT24-102 Homeros** |||
| `#effect#0` | [Start of Main] +1 memory; at ≥5 suspend + Draw 1 | `BT24-102-effect0.yaml` | |
| `#effect#1` | [All Turns] [TS] +1000, plus [End of Your Turn] refire an OP/WD | `BT24-102-effect1.yaml` | both timing blocks witnessed |
| `#effect#2` | [Security] play free | `BT24-102-effect2.yaml` | |
| **BT24-083 Hiroko Sagisaka** |||
| `#effect#0` | [Start of Turn] return self → play [Hiroko]/[TS] ≤5000 DP free | `BT24-083-effect0.yaml` | |
| `#effect#1` | [On Play] reveal 3, add 1 [TS] | `BT24-083-effect1.yaml` | |
| `#effect#2` | [Security] play free | `BT24-083-effect2.yaml` | |
| **BT24-041 Minervamon** |||
| `#effect#0` | [Digivolve] Lv.5 [Beastkin]/[Dark Dragon]/[TS]: Cost 3 | `BT24-041-effect0.yaml` | onto red P-213 |
| `#effect#1` | play cost −5 with an [Iliad] Digimon/Tamer | `BT24-041-effect1.yaml` | 3→−4 |
| `#effect#2` | OP/WD/OnDel: play an [Iliad] ≤5 free; `<De-Digivolve 1>` per own Digimon | `BT24-041-effect2.yaml` | strips 2 |
| `#effect#3` | [Opp Turn] [Iliad] gain `<Reboot>`, `<Blocker>` | `BT24-041-effect3.yaml` | both halves (Reboot fixed 2026-10-04) |
| **BT25-022 Lunamon** |||
| `#effect#0` | [Digivolve] Lv.2 w/[TS]: Cost 0 | `BT25-022-effect0.yaml` | |
| `#effect#1` | [On Play] reveal 3, add [Iliad] + [TS] | `BT25-022-effect1.yaml` | |
| `#inherited#0` | `<Jamming>` | `BT25-022-inherited0.yaml` | survives an 8000 DP security Digimon |
| **BT25-025 Aegiochusmon: Blue** |||
| `#effect#0` | [Digivolve] [Aegiomon]: Cost 3 | `BT25-025-effect0.yaml` | |
| `#effect#1` | `<Blocker>` | `BT25-025-effect1.yaml` | |
| `#effect#2` | `<Decode ([Aegiomon])>` | `BT25-025-effect2.yaml` | |
| `#effect#3` | [On Play][When Digivolving] (timing) | `BT25-025-effect3.yaml` | [On Play] arm |
| `#effect#4` | `<De-Digivolve 1>`, then unsuspend 1 at ≤3 security (+ Rule [Cyborg]) | `BT25-025-effect4.yaml` | [When Digivolving] arm; partial: the [Cyborg] trait grant is not observed |
| `#inherited#0` | [All Turns][OPT] security removed → a [Shaman] may unsuspend | `BT25-025-inherited0.yaml` | |
| **P-213 Aegiochusmon** |||
| `#effect#0` | [Digivolve] [Aegiomon]: Cost 3 | `P/P-213-effect0.yaml` | |
| `#effect#1` | `<Raid>` | `P/P-213-effect1.yaml` | the phantom-Barrier row was removed after the fix |
| `#effect#2` | `<Decode>` | `P/P-213-effect2.yaml` | |
| `#effect#3` | [WD] ≤3 security → Rush + 3000, may attack | `P/P-213-effect3.yaml` | partial: the DP and attack are observed; Rush on its own is not |
| `#effect#4` | Rule: Trait [Dragonkin] | `P/P-213-effect4.yaml` | BT21-007 recovers it from the trash only through the grant (variant deck) |
| `#inherited#0` | inherited `<Decode>` | `P/P-213-inherited0.yaml` | |

## Expected aborts and divergences (read before calling anything a bug)

`diverged` is not proof we are wrong. `general_rule.pdf` outranks DCGO.

### Predicted divergences (from the C# read)

- **`BT24-102-effect1`: our engine is right.** DCGO lists BT24-101's combined
  [On Play]/[When Digivolving] effect twice (two `ActivateClass` objects; `BT24_102.cs`
  "Select 1 effect to activate."), which the scenario answers with a `dcgo_only` row.
  Homeros then reads the choice through an index callback, and the harness passes
  `Indicies = null` (`SelectCardEffect.cs` ~706-718, ~881), so DCGO re-activates nothing.
  At the assert step, expect DCGO `p0.security` one higher than ours.
- **`BT26-103-effect4`: prompt shape only, end state equal.** DCGO labels Succession-copied
  triggers with the host card's id (`CopiedEffects.cs`). So the rows stay split
  `sim_only` / `dcgo_only`, even though both engines now ask the same two ordering prompts.
- **`P-213-effect4`: DCGO data risk.** If DCGO's card data lacks P-213's rule-granted
  [Dragonkin] trait, BT21-007's trash pick will not offer it. An abort there is a DCGO
  data gap, not an engine finding.

### Translation facts, verified

- **Gates:**
  - **OptionalSkill gates DCGO really asks:** Asuna BT24-088 [On Play] and
    [Start of Your Turn]; Hiroko [Start of Your Turn]; Aegiomon BT24-034 (whenever
    security > 0); Aegiomon BT25-033 (whenever security > 0, even with no opponent
    Digimon); Homeros [End of Your Turn] (whenever Homeros can suspend); `<Raid>`;
    `<Decode>`; `<Execute>` and `<Engage>` (including when the turn ended on negative
    memory, `AutoProcessing.cs` `EndTurnCheck`); BT21-007 [On Play]; Kanan [End of Your
    Turn]; Shota's redirect; and Wrath Mode's [Counter] (`CounterClass` forces optional).
  - **Not gated** (DCGO opens the pick directly, which you can decline):
    - Minervamon BT24-041 [On Play]/[When Digivolving] and Junomon BT25-044
      [On Play]/[When Digivolving].
    - `<Alliance>`, Mervamon's free-play pick, BT26-033's [When Digivolving], and
      BT26-073's cost menu (always asked whenever either cost is payable).
  - **`<Ascension>`** is a mandatory `generic_bool`, with no OptionalSkill before it.
- **Empty picks:** a pick with no candidates opens no prompt in DCGO
  (`SelectHandEffect.active` / `SelectCardEffect` maxCount ≤ 0).
- **Single-candidate picks:** DCGO asks them under the harness. The auto-pick exists only
  for a pick you can't decline with exactly as many candidates as required, in the
  human-UI branch.
- **Menus:**
  - **Cost routes:** `SelectCountEffect` over the distinct costs. Equal costs are
    de-duplicated, which gives no prompt (`BT24-101-effect0`).
  - **BT26-073 menu values:** Delete=1 / Return=2 / Don't use=3 (`BT26_073.cs:116-118`),
    asked even with no Lv.≤5 target.
  - **Zone menus:** skipped silently when only one zone qualifies
    (Mervamon, Dark's [On Deletion]).
  - **Mervamon's loop:** re-asks only if something still fits the remaining budget.
- **RevealBucket on the simplified reveal path:**
  - No leading `generic_bool`.
  - One `SelectCardEffect` per bucket. Each pick goes to the hand as its prompt closes,
    which gives the known one-field `p0.hand` diff on the second pick row. That is a DCGO
    quirk.
  - A single leftover card is bottomed with no prompt; a 2-card leftover is one
    multi-pick.
- **BT24-031 inherited:** a mandatory `generic_bool` (`SetBoolSelection`) with no card
  widget, so the line carries a `sim_only` pick plus a `dcgo_only` `generic_bool` row.
- **Counter:** DCGO enters Counter timing on every attack, including attacks on the
  player (`AttackProcess.cs` ~242).
- **Trigger order:**
  - **Inactive triggers:** `MultipleSkills` drops triggers that can't activate and asks
    no ordering prompt for a lone one.
  - **Decode:** the played Digimon's [On Play] waits on the main queue until the carrier
    has left. It is then ordered against the carrier's [On Deletion]/`<Ascension>`.
  - **9-1-5 Option-trash order:** DCGO asks nothing, so the row is `sim_only`.
- **BT26-029-effect3:** DCGO also offers the protected Digimon to the `<De-Digivolve>`
  pick, and the immunity then cancels the strip (`IDegeneration`).
- **Field slots:** DCGO lists Tamers after Digimon. Lines that `attack: field.0` were
  built so the target is the only permanent, or the first Digimon.

## Defects found and fixed (stacked PR #708)

Each fix is marked RESOLVED in `docs/RUST_ENGINE_GAPS.md` / `qa/dsl-vocab-gaps.md`.
Every engine fix was cross-checked against the DCGO C# and agrees with it.

- **Engine:**
  - `G-ENGINE-DUAL-END-OF-TURN-WINDOW`
  - `G-ENGINE-AURA-REBOOT-NOT-APPLIED`
  - `G-ENGINE-COUNTER-NO-WINDOW-ON-PLAYER-ATTACK`
  - `G-ENGINE-BARRIER-CANDIDATE-NOT-CARRIER-GATED` (with Fragment and Decoy)
  - `G-ENGINE-DECOY-CAUSE-GATE`
  - `G-ENGINE-ALL-TURNS-TRIGGER-FROM-TRASH`
  - `G-ENGINE-OPT-NOOP-REQUEUE`
  - `G-ENGINE-DECODE-ON-PLAY-BEFORE-CARRIER-LEAVES`
  - `G-ENGINE-REFIRE-SPLITS-COMBINED-TIMING`
  - `G-ENGINE-TRIGGER-ORDER-DERIVED-FIRST`
  - `G-ENGINE-TRIGGER-ORDER-PICK-MAPPING`
  - `G-ENGINE-TRIGGER-ORDER-UNACTIVATABLE`
- **DSL / cards / data:**
  - `G-DSL-COUNTER-TIMING-NO-COUNTER-FLAG`
  - `G-DSL-USE-OPTION-ONLY`
  - `G-DSL-PLACE-ON-SECURITY-OWNER-AND-SUCCESS`
  - `G-DATA-BT25-044-COLOR-ATTRIBUTE`
  - BT25-044, BT24-034 and BT24-102 now match DCGO and §15-7-5.
- **Exam tooling:**
  - `G-TOOLING-EXAM-PLAYCOSTBUDGET-ZONE`
  - `G-TOOLING-EXAM-ACTOR-UNCHECKED`
  - The trailing-PASS resolver: this was the real cause of the reported
    "Assembly skips [On Play]".
  - `G-TOOLING-EXAM-SECURITY-PICK-NO-FOLD` is superseded: the fold was retired.

## Authoring notes

- A card costing more than 10 cannot be hard-played from 0 memory (−10 is the floor), so
  Lv.6/7 lines play on T3 or later.
- `actor:` is now checked on every action step (`G-TOOLING-EXAM-ACTOR-UNCHECKED`,
  resolved 2026-10-04): a wrong-seat step refuses to lower. No TS Jupitermon scenario
  had one.
- Before `G-ENGINE-ALL-TURNS-TRIGGER-FROM-TRASH` was fixed, lines that remove security
  kept BT24-101 and BT25-044 off the top of the stack. They still do, which is harmless.
