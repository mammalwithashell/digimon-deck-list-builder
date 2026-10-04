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

The DCGO build must be at or after `fc67f9ae6` (`scripted-v16`), which adds the `dna:`
and `link:` support. None of these lines use either verb. A DCGO card-script check
(`$BASE/DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<ID>.cs`) is still owed for each
card before reading anything as `unavailable`.

**DCGO C# could not be consulted for this authoring pass.** The cloud container has no
DCGO checkout, neither the local placeholder nor the base repo. Every expectation about
DCGO's prompt shape below comes from sibling exam files, the DCGO notes in the card YAML,
and `docs/DCGO_EXAM.md`. Treat each one as a guess for the oracle's abort message to
confirm or refute.

## Scenario ↔ clause table

"Partial" means the line witnesses only part of the clause. The unwitnessed part is named.

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| **BT26-033 Jupitermon (DUAL)** |||
| `#effect#0` | [Digivolve] Lv.5 w/[TS]: Cost 4 | `BT26-033-effect0.yaml` | onto P-213, memory 3→−1 |
| `#effect#1` | `<Raid>` | `BT26-033-effect1.yaml` | redirect to Biyomon; P1 security stays 5 |
| `#effect#2` | `<Alliance>` | `BT26-033-effect2.yaml` | one attack checks 2 (5→3) |
| `#effect#3` | `<Engage>` | `BT26-033-effect3.yaml` | **asserts NO end-of-turn window: engine bug, expected oracle abort** |
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
| `#effect#5` | [WD][Counter][OPT] trash top security, Recovery +2 | `BT26-103-effect5.yaml` | partial: [When Digivolving] arm only; the [Counter] arm cannot fire (DSL bug) |
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
| `#effect#3` | [Opp Turn] [Iliad] gain `<Reboot>`, `<Blocker>` | `BT24-041-effect3.yaml` | **the Reboot half asserts our engine's wrong result (expected divergence)** |
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
| `#effect#1` | `<Raid>` | `P/P-213-effect1.yaml` | carries a phantom-Barrier `sim_only` row (engine bug) |
| `#effect#2` | `<Decode>` | `P/P-213-effect2.yaml` | |
| `#effect#3` | [WD] ≤3 security → Rush + 3000, may attack | `P/P-213-effect3.yaml` | partial: the DP and attack are observed; Rush on its own is not |
| `#effect#4` | Rule: Trait [Dragonkin] | `P/P-213-effect4.yaml` | BT21-007 recovers it from the trash only through the grant (variant deck) |
| `#inherited#0` | inherited `<Decode>` | `P/P-213-inherited0.yaml` | |

## Expected aborts and divergences (read before calling anything a bug)

`diverged` is not proof we are wrong. `general_rule.pdf` outranks DCGO.

**Deliberate. Each pins an engine bug we already logged.**
- `BT26-033-effect3`: our engine never opens `<Engage>` on a DUAL card
  (`G-ENGINE-DUAL-END-OF-TURN-WINDOW`). DCGO's end-of-turn OptionalSkill is left
  unanswered, so the oracle aborts there. That abort is the finding. Separately,
  `BT26-033-effect0/1/2/4/5/9` each carry `dcgo_only` decline rows for DCGO's Engage
  gate. If DCGO does not ask that gate when the turn ended on negative memory, those
  lines abort on that row.
- `BT24-041-effect3`: aura-granted `<Reboot>` does not unsuspend in our engine
  (`G-ENGINE-AURA-REBOOT-NOT-APPLIED`). The step-16 assert says `suspended: true`; DCGO
  should say `false`.
- `BT26-103-effect3`: DCGO will likely run Wrath Mode's [Counter] arm and then `#effect#6`'s
  pick before the block prompt. Ours runs neither (`G-DSL-COUNTER-TIMING-NO-COUNTER-FLAG`).
- `BT26-081-effect4`: our engine skips Mervamon's [On Play] pick after an Assembly play
  (`G-ENGINE-ASSEMBLY-PLAY-SKIPS-ON-PLAY-PICK`). DCGO will ask it.
- `P-213-effect1`, `BT24-041-effect3`: a `sim_only` row answers our phantom `<Barrier>`
  prompt to player 1 (`G-ENGINE-BARRIER-CANDIDATE-NOT-CARRIER-GATED`). DCGO asks nothing.
- `BT26-103-effect4`: three extra TriggerOrder rows from `#effect#6` being re-queued
  (`G-ENGINE-OPT-NOOP-REQUEUE`).
- `BT24-102-effect1`: a `sim_only` "on_play vs when_digivolving" choice for one combined
  effect (`G-ENGINE-REFIRE-SPLITS-COMBINED-TIMING`).
- `BT26-090-effect0`: a `sim_only` "Play as Digimon / Use as Option" choice that the card
  does not grant (`G-DSL-USE-OPTION-ONLY`).

**Translation shapes. Guessed without DCGO C#; fix the row, not the engine.**
- **Cost-route menus:** our `EffectChoice` against DCGO's `SelectCountEffect`, answered by
  `value:` (BT26-029, BT26-033, BT26-103, BT24-101-effect1, BT25-025-inherited0). DCGO may
  list a different number of routes, or auto-take the cheapest one.
- **BT26-073 cost menu:** a `sim_only` `choice:` plus a `dcgo_only` `generic_int`. The
  values (1 delete / 2 return / 3 don't pay) are copied from BT25-083's menu. DCGO may skip
  the menu when there is no Lv.≤5 target (073-effect0/3/inherited0).
- **OptionalSkill gates:** `optional_gate_fold` and `dcgo_only` yes/no rows assume DCGO
  gates these optional picks before the pick: Asuna, Hiroko, Junomon and Minervamon
  [On Play]; Raid; Decode; Ascension (`generic_bool`); BT24-034-effect2; BT26-092-effect1;
  BT24-102-effect1. A DCGO that gives no gate, or picks without one, aborts on the row.
- **Empty optional picks:** our engine skips a pick with no candidates (Minervamon
  [On Play] in BT26-103/BT24-041 lines; Asuna [Start of Your Turn]). DCGO may open an
  empty prompt (EX12-047 class, `docs/DCGO_EXAM.md` Known gaps).
- **Prompts we ask with no candidates:** Aegiomon BT25-033's optional [On Play] / [When
  Digivolving] with no opponent Digimon (BT26-073-effect0, BT25-022-inherited0,
  BT24-031-inherited0). DCGO may skip it.
- **RevealBucket** (BT24-031-effect1, BT25-022-effect1, BT24-083-effect1, and the
  reveal-carrying lines): a one-field `p0.hand` diff on the second pick row is the
  documented DCGO per-bucket add, a DCGO quirk. Also expect a possible leading
  `generic_bool`, and the OrderedPermutation N=1 leftover row is `sim_only`.
- **Mervamon's free-play pick (BT26-081):** a `sim_only` + `dcgo_only SelectHandEffect`
  pair (`G-TOOLING-EXAM-PLAYCOSTBUDGET-ZONE`). DCGO may loop a hand/trash/stop zone menu.
- **BT24-031-inherited0:** "you may add your top security card" is our Security pick.
  The harness refuses an OptionalSkill fold on that kind, so expect a prompt-class
  mismatch (`G-TOOLING-EXAM-SECURITY-PICK-NO-FOLD`). Aegiomon over the yellow [TS] Elecmon
  also has two digivolve routes at the same cost: we ask nothing, DCGO may ask.
- **Single-candidate mandatory picks** (BT26-073 delete, BT26-081 DP pick, BT26-083
  `#effect#5` 2-of-2 delete): we always park these; DCGO may auto-resolve with no row.
- **Trigger ordering:** BT26-029-effect1 resolves Aegiomon's [On Play] then `<Ascension>`
  with no ordering prompt (DCGO may ask `MultipleSkills`: a decision missing on our side).
  BT26-029-inherited0 answers a Junomon-vs-inherited `MultipleSkills` by identity (DCGO may
  drop Junomon's trigger). The 9-1-5 Option-trash order is a `sim_only` row in
  BT26-029-effect4 and inherited0, and in BT25-025-inherited0.
- **BT26-029-effect3:** our `<De-Digivolve>` still offers the protected Digimon as a
  target. If DCGO filters it out, the prompts will differ.
- **BT26-083 `<Execute>` gate:** it opens whenever BT26-083 ends a turn unsuspended,
  including on negative memory (effect0/5/7).
- **Field slots:** DCGO lists Tamers after Digimon. Lines that `attack: field.0` were
  built so the target is the only permanent, or the first Digimon.

## Bugs found while authoring (logged, not fixed)

All of these are in `docs/RUST_ENGINE_GAPS.md` § "TS Jupitermon exam-authoring findings
(2026-10-04)" and `qa/dsl-vocab-gaps.md` (same heading):

- **Engine:** `G-ENGINE-DUAL-END-OF-TURN-WINDOW` (confirmed in code),
  `G-ENGINE-COUNTER-NO-WINDOW-ON-PLAYER-ATTACK` (confirmed in code, impact unverified),
  `G-ENGINE-BARRIER-CANDIDATE-NOT-CARRIER-GATED`, `G-ENGINE-AURA-REBOOT-NOT-APPLIED`,
  `G-ENGINE-ALL-TURNS-TRIGGER-FROM-TRASH` (BT24-101, BT25-044),
  `G-ENGINE-ASSEMBLY-PLAY-SKIPS-ON-PLAY-PICK`, `G-ENGINE-DECODE-ON-PLAY-BEFORE-CARRIER-LEAVES`,
  `G-ENGINE-OPT-NOOP-REQUEUE`, `G-ENGINE-REFIRE-SPLITS-COMBINED-TIMING`.
- **Data:** `G-DATA-BT25-044-COLOR-ATTRIBUTE`. The YAML says yellow/white and Vaccine; the
  official card is Yellow/Purple and Virus, with a Purple Lv.5 circle.
- **DSL:** `G-DSL-COUNTER-TIMING-NO-COUNTER-FLAG`, `G-DSL-USE-OPTION-ONLY`.
- **Exam tooling:** `G-TOOLING-EXAM-PLAYCOSTBUDGET-ZONE`, `G-TOOLING-EXAM-ACTOR-UNCHECKED`,
  `G-TOOLING-EXAM-SECURITY-PICK-NO-FOLD`.

## Authoring notes

- A card costing more than 10 cannot be hard-played from 0 memory (−10 is the floor), so
  Lv.6/7 lines play on T3 or later.
- `actor:` is not checked on `pass` steps (`G-TOOLING-EXAM-ACTOR-UNCHECKED`). The agents
  checked `turn`/`phase` with asserts at key points to make sure no step lands on the
  wrong seat.
- A security card's own `[All Turns]` trigger fires as it leaves security
  (`G-ENGINE-ALL-TURNS-TRIGGER-FROM-TRASH`), so lines that remove security keep BT24-101
  and BT25-044 off the top of the stack.
