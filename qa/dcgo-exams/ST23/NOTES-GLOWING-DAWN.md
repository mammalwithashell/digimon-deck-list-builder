# Glowing Dawn: DCGO exam notes (2026-10-04)

Glowing Dawn is the #2 BT26 deck (8.1% of the field per DigiLab's BT26 retrospective) and,
before this pass, the largest deck with **0%** of its clauses carrying a DCGO oracle verdict
(`qa/qa-reports/meta-coverage/latest.md`). Every printed clause of its 29-card BT26 pool now
has a scenario that passes `--sim-only`. **None of them has been run through the oracle**, so
every clause is still `unmeasured`.

## Post-#708 update (2026-10-04)

PR #708 (engine/tooling) merged alongside this pass and changed behaviour nine scenarios had
pinned. All nine were re-authored; none needed an engine fix. DCGO C# was read for each change
(the "could not be consulted" note below predates this update).

| Scenario | What #708 changed | Change |
|---|---|---|
| `BT25-041-inherited0`, `ST23-04-inherited0` | Spent-OPT triggers are no longer offered | ST23-09's spent WD/WA dropped from the T5 attack, so its MultipleSkills row is removed (ST23_09.cs: shared hash, maxCountPerTurn 1) |
| `BT25-057-effect2`, `ST23-02-inherited0` | `<Barrier>` is carrier-scoped | Phantom p1 row removed; tied Agumon now deleted (Barrier.cs CanUse is carrier-scoped) |
| `BT25-088-effect3`, `BT26-089-effect2` | Security card's own effects no longer enqueued from the revealed card | The self-trigger is offered once, not twice; declined `sim_only` |
| `BT26-070-inherited0` | Spent-OPT triggers are no longer offered | WA ordering row is now `dcgo_only` (DCGO's `RemoveUse()` refunds the unpaid OPT) |
| `BT26-075-effect1` | DUAL cards open end-of-turn windows | Witnesses `<Execute>` (attack, delete, `<Ascension>`) |
| `BT26-075-effect6` | Same | Declines the new `<Execute>` window |

## State

Pool: the 9 Glowing Dawn lists in the BT26 window (2026-09-04 → 2026-09-12, dcg-nexus) in
`data/deck_library.json`. Core = cards in ≥70% of lists (`tools.clause_coverage.archetype.core`:
≥7 of 9): 16 cards. The other 13 cards that appear in any of the 9 lists are the tail.

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids <29 ids below>`):
**113 clauses over 29 cards** (96 effect, 17 inherited; all from the official Bandai bundle,
0 image-required).

| | Core (16 cards) | Tail (13 cards) | Total |
|---|---|---|---|
| Clauses | **65** | **48** | **113** |
| Scenario authored, passes `--sim-only` | 65 | 48 | 113 |
| `unreachable` (documented, no scenario) | 0 | 0 | 0 |
| `confirmed` / `diverged` | 0 / 0 | 0 / 0 | 0 / 0 |
| **`unmeasured`** (verdict sense) | **65** | **48** | **113** |

- Core: ST23-12, BT25-043, ST23-09, ST23-13, ST23-15, ST23-01, ST23-06, BT25-049, BT25-041,
  BT25-057, ST23-03, BT25-032, BT25-035, ST23-04, BT25-090, BT25-046.
- Tail: P-236, ST23-08, ST23-14, BT26-089, BT26-025, BT26-070, ST23-02, BT25-088, ST23-11,
  BT26-075, ST23-07, ST23-10, BT16-082. (BT26-026, BT26-031, BT26-093, ST23-05, BT9-103, BT4-104
  and LM-047 each appear in 1-2 lists and are not part of this pass.)

A scenario that lowers is **not** a measurement. Every `assert:` block is OUR engine's result,
written at authoring time, and is provisional until the oracle pass backfills it. Several
scenarios deliberately pin behaviour we believe is wrong so the oracle reports it as a
divergence; their headers say `ENGINE FINDING`. No verdict files were written.
`exam_binding`: 0 orphan scenarios, every clause id bound to exactly one file.

## Deck book

`qa/dcgo-exams/ST23/glowing_dawn_pool.json` resolves every scenario in this pass.

| Deck | What it is |
|---|---|
| `gd-bt26-1st` | 1st place, 2026-09-08, dcg-nexus `8df91c2a-cde1-41af-b383-f38c4caac935`. Eggs 4× ST23-01. |
| `gd-bt26-4th` | 4th place, 2026-09-08, `13f8ed64-51a7-440f-a022-3583fb022e60` (BT25-046, BT26-070/075). |
| `gd-bt26-4th-frimon-eggs` | `gd-bt26-4th` main deck with 4× BT25-003 Frimon eggs (played in other GD lists). Used by `ST23-06-effect0` / `BT25-046-effect0`, so a green Lv.3 can reach the yellow egg only through its [Glowing Dawn] special condition. |
| `gd-bt26-7th` | 7th place, 2026-09-08, `5ae885b6-c89e-491b-af0f-d44dbfbfddee` (BT25-088, P-236, ST23-02/07/08/14). |
| `gd-bt26-8th` | 8th place, 2026-09-05, `5c6adac4-e562-4b37-83c6-9f6716ea5cb2` (ST23-10/11). |
| `tm-quiet-red` | The shared quiet ST1 red opponent (same deck as the BT24/BT26 books). |

Every deck is 50 main / 4 eggs / ≤4 copies (`DeckData.IsValidDeckData()`).

**Printed-data fix in this PR.** `cards.json` filed ST23-06 Gekkomon at play cost 0 / no DP
(so it was played free and died at 0 DP in every real game) and BT25-088 at cost 0. Corrected
to the official DB (3 / 1000 DP and 4) in `cards.json` + `card_overrides.json`.

## Running the oracle pass (local: Unity, base-repo DCGO)

The scenarios span five set folders that also hold other decks' scenarios on other books, so
copy this pass's files into one directory first.

```bash
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"   # see docs/DCGO_HARNESS.md
BOOK=qa/dcgo-exams/ST23/glowing_dawn_pool.json
CARDS="ST23-12 BT25-043 ST23-09 ST23-13 ST23-15 ST23-01 ST23-06 BT25-049 BT25-041 BT25-057
       ST23-03 BT25-032 BT25-035 ST23-04 BT25-090 BT25-046
       P-236 ST23-08 ST23-14 BT26-089 BT26-025 BT26-070 ST23-02 BT25-088 ST23-11 BT26-075
       ST23-07 ST23-10 BT16-082"
GD=$(mktemp -d)
for c in $CARDS; do cp qa/dcgo-exams/${c%%-*}/$c-*.yaml "$GD"/; done    # 113 files

# 0. Clause text for the verdict fingerprints.
PYTHONPATH=code python -m tools.clause_coverage.extract --quiet --out "$GD/clauses.json" --card-ids $CARDS

# 1. Re-check sim-only and emit one scripted DCGO job per scenario (113 jobs).
cargo run -p dcgo-harness -- --root "$ROOT" exam --scenario "$GD" --sim-only \
    --cards-json data/cards.json --decks "$BOOK" --emit-job "$ROOT/jobs"
#   expect: scenarios seen 113 / lowered 113 / run 113 / diffed 0 / failed 0

# 2. Drain. Stop Play before queueing (a running harness claims jobs instantly).
cargo run -p dcgo-harness -- --root "$ROOT" node status        # must report GO
cargo run -p dcgo-harness -- --root "$ROOT" enable
cargo run -p dcgo-harness -- --root "$ROOT" watch --build <player-dir>   # or: Play in Unity + `status`
cargo run -p dcgo-harness -- --root "$ROOT" disable

# 3. Diff each scenario against the sidecar its job wrote, and record verdicts.
#    --sidecar takes a directory holding one <scenario-stem>.state.jsonl per scenario.
cargo run -p dcgo-harness -- --root "$ROOT" exam --scenario "$GD" \
    --sidecar <dir-of-state.jsonl> --cards-json data/cards.json \
    --verdicts --clause-text-json "$GD/clauses.json" --all-diffs
```

Suggested order: run the 65 core scenarios first (the first 16 ids of `$CARDS`). Before reading
anything as `unavailable`, check the per-card script exists:
`ls "$BASE/DCGO/Assets/Scripts/CardEffect/"*/*/{ST23_06,BT25_043,...}.cs` (underscored ids).
A clean `confirmed` row backfills the scenario's `assert:` from DCGO; a `diverged` row is a
finding to triage (`general_rule.pdf` outranks DCGO), never an automatic engine fix.

**DCGO C# could not be consulted in this authoring pass**: the cloud container has no DCGO
checkout (neither the worktree placeholder nor a base repo copy). Every DCGO prompt-shape
expectation below comes from the "DCGO crosscheck" comments in each card's YAML
(`code/digimon-engine/cards/<set>/<ID>.yaml`, written when the C# was read at implementation
time), sibling exam files and `docs/DCGO_EXAM.md`. Treat each as a guess for the oracle's
abort message to confirm or refute; expect a first oracle pass to need a round of
prompt-shape re-authoring (`sim_only` / `dcgo_only` / `optional_gate_fold` rows) that is not a
rules finding.

## Engine / DSL findings (logged, not fixed)

Logged to `docs/RUST_ENGINE_GAPS.md` ("Found by the Glowing Dawn DCGO-exam authoring pass") and
`qa/archetype-qa/engine-gaps.md` ("Open gaps — Glowing Dawn DCGO-exam authoring"). Repros for
the ones without a committed exam line: `qa/archetype-qa/glowing-dawn-repros/`.

| Severity | Finding | Where it shows |
|---|---|---|
| ✅ | **Fixed by #708** (G-ENGINE-BARRIER-CANDIDATE-NOT-CARRIER-GATED). Was: `<Barrier>` replacement not scoped to its carrier: offered to the OPPONENT when an inherited-Barrier attacker deletes their Digimon; accepting spared their Digimon, and either answer dropped the attacker's `<Piercing>` check (G-ENGINE-BARRIER-CONDITION-NOT-CARRIER-SCOPED) | `ST23-02-inherited0`, `BT25-057-effect2`, repro (all now assert the correct result) |
| 🔴 | Face-down cards under a Tamer grant it inherited effects (Barrier, [End of Attack]) (G-ENGINE-FACE-DOWN-TAMER-SOURCES-INHERIT) | `BT25-090-effect1` variant (header) |
| ✅ | **Fixed by #708** (G-ENGINE-DUAL-END-OF-TURN-WINDOW). Was: `<Execute>` (and Vortex/Overclock/Engage) never opened on a DUAL Digimon: `is_digimon` excludes `CardKind::Dual` (G-ENGINE-END-OF-TURN-KEYWORDS-SKIP-DUAL) | `BT26-075-effect1` (now witnesses the clause), `BT26-075-effect6` |
| 🔴 | BT25-043 / ST23-05 / ST20-15 leave-prevention gated on `any_permanent { zone: [security] }` is always false; never offered (G-GD-SECURITY-ZONE-EXISTENTIAL) | `BT25-043-effect3` |
| 🔴 | BT25-049's Option-use −3 fires on a digivolve into a DUAL card (cost 3 → 0) (G-GD-BT25-049-DUAL-REDUCER) | repro |
| 🟠 | ST23-15 [Security] trashes e-Pulse instead of placing it (G-GD-ST23-15-SECURITY-PLACEMENT) | `ST23-15-effect3` |
| 🟠 | A Tamer played by its own [Security] triggers its own "security removed" clause (G-ENGINE-SECURITY-PLAYED-TAMER-SELF-TRIGGER). #708 removed the duplicate (it was offered twice); the one self-trigger is still open. DCGO never asks (OnLoseSecurity is collected before the [Security] play) | `BT26-089-effect2`, `BT25-088-effect3` |
| 🟠 | `<Retaliation>` no-ops after an effect-initiated battle (`pending_attack` unset) (G-ENGINE-RETALIATION-EFFECT-BATTLE) | repro |
| 🟠 | BT25-035's free-digivolve pick offers illegal Lv.6 targets; cost paid, no digivolve (G-GD-BT25-035-ILLEGAL-TARGETS) | Appendix, group D (F5) |
| 🟠 | BT25-057 Option face: in-effect "it may attack" deferred to an end-of-turn window; buff expires this turn vs printed "until your opponent's turn ends" (G-GD-BT25-057-OPTION-FACE) | `BT25-057-effect4` |
| 🟡 | ST23-13/ST23-14 "when effects trash cards from under this Tamer" resolves mid-effect, per card (15-8-3-2) | `ST23-12-effect1`, `ST23-13-effect1`, `ST23-01-inherited0`, `ST23-14-effect1`, `BT25-057-effect1/5`, `ST23-11-effect2`, repro |
| 🟡 | Inherited [End of Attack] (BT25-041/ST23-04/ST23-08) offered after the opponent's attacks and when unpayable | `ST23-09-effect3` and others (`sim_only` rows) |
| 🟡 | Unpayable optional "by trashing … under a Tamer" costs still park prompts (BT25-049/BT25-090/ST23-03 reducers; BT25-057 De-Digivolve in trigger order) | many `sim_only` rows |
| 🟡 | One printed [OP]/[WD] effect split into two triggers (ST23-08/ST23-04/BT25-057): extra TriggerOrder, optional half's yes/no skipped after an order pick (G-GD-SPLIT-OP-WD-ORDER) | `ST23-08-effect2`, `ST23-04-effect2` |
| 🟡 | Smaller: BT26-025 single-Tamer pick auto-resolved; P-236 Use Req. always met; BT25-090 "suspend only" branch folded; ST23-09 OPT re-offered (fixed by #708: spent-OPT triggers are no longer offered) | see per-group findings |
| tooling | Sim-only lowering did not check `actor:` against the deciding player (fixed by #708: G-TOOLING-EXAM-ACTOR-UNCHECKED); field rows cannot assert granted keywords (Jamming, Security A.) | `ST23-08-effect2`, `BT26-075-effect5`, `ST23-14-effect1` |

Pre-existing, not this PR: the `DCGO Exam (sim-only)` CI workflow runs the whole corpus
against one deck book (`EX12/toho_pool.json`), so on main 228 of 373 scenarios fail to lower
with "unknown deck". These 113 scenarios hit the same wall in CI; run them with this book as
above.

## Scenario ↔ clause table

"Witness / note" says what state proves the clause, and what part of it is not witnessed.
Asserts are provisional (our engine) everywhere.

### Core — BT25-043 Habakirimon, ST23-09 Atratusmon (DUAL Lv.5)

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| `BT25-043#effect#0` | `[Digivolve] Lv.5 w/[Glowing Dawn]: Cost 3` | `qa/dcgo-exams/BT25/BT25-043-effect0.yaml` | Digivolves onto yellow GD Lv.5 BT25-041. Both routes (Yellow 4 / GD 3) are open, so the cost-route pick `value: 3` (SelectCountEffect) is answered with 3, and memory goes 3 -> 0. The only non-yellow GD Lv.5 is ST23-08, rejected because it brings two [On Play] triggers and an unpayable optional. |
| `BT25-043#effect#1` | timing header `[When Digivolving][When Attacking][Once Per Turn]` | `qa/dcgo-exams/BT25/BT25-043-effect1.yaml` | [WA] arm plus OPT. On T5 the first attack fires the trigger (Recovery 6->7, own top card trashed 7->6, unsuspends). The second attack the same turn fires nothing: security stays 6 and the Digimon stays suspended. |
| `BT25-043#effect#2` | `<Recovery +1>. Then, by trashing top security of the player with the most, this Digimon unsuspends` | `qa/dcgo-exams/BT25/BT25-043-effect2.yaml` | [WD] arm onto a Murasamemon that is SUSPENDED (it attacked). After Recovery P1 has 4 > P0 3, so P1's top card is trashed (P1 4->3). Habakirimon reads unsuspended. All three sub-parts change state. |
| `BT25-043#effect#3` | `[All Turns][OPT] GD Digimon would leave -> trash top security, doesn't leave` | `qa/dcgo-exams/BT25/BT25-043-effect3.yaml` | **ENGINE FINDING (expected divergence).** P1's Gaia Force targets Habakirimon. Our engine offers NO replacement and Habakirimon is deleted. The DCGO OptionalSkill "yes" is a `dcgo_only` row. The asserts record our (wrong) behaviour. |
| `BT25-043#effect#4` | Option `<Use Req. ([Glowing Dawn])>` | `qa/dcgo-exams/BT25/BT25-043-effect4.yaml` | The yellow Option is used while P0's only permanent is the hatched GREEN GD Kekkomon in BREEDING, so the use is admitted by Use Req. alone. Memory 3 -> -3, Biyomon deleted (-8000), Option in trash. No Arts (Lv.2). |
| `BT25-043#effect#5` | Option `[Main]` -8000 to 1; trash top security -> all -5000 | `qa/dcgo-exams/BT25/BT25-043-effect5.yaml` | -8000 deletes Biyomon. P0 pays its top security (5->4, ST23-04 in trash), and Phoenixmon (not the first target) reads 7000 inside the same turn because P0 stays on positive memory (9 -> 3). |
| `BT25-043#effect#6` | `<Arts Digivolve>` | `qa/dcgo-exams/BT25/BT25-043-effect6.yaml` | Use for 6 (3 -> -3), then Arts onto BT25-041. Memory stays exactly -3, P0's trash is empty, Habakirimon sits on BT25-041, its WD Recovery fires (security 6), and there is one bonus draw. |
| `ST23-09#effect#0` | `[Digivolve] Lv.5 w/[Glowing Dawn]: Cost 3` | `qa/dcgo-exams/ST23/ST23-09-effect0.yaml` | Onto YELLOW GD Lv.5 BT25-041, which matches neither printed circle (Green 4 / Black 4), so this is the only route. Memory 3 -> 0 and there is no cost pick. P1's board is empty, so the WD delete is skipped. |
| `ST23-09#effect#1` | `<Security A. +1>` | `qa/dcgo-exams/ST23/ST23-09-effect1.yaml` | One attack takes P1's security 5 -> 3 (two vanilla checks trashed). The WA arm is already spent (OPT shared with WD). |
| `ST23-09#effect#2` | `<Reboot>` | `qa/dcgo-exams/ST23/ST23-09-effect2.yaml` | Suspended after its T3 attack (assert 11), then unsuspended at P1's T4 breeding step (assert 12). |
| `ST23-09#effect#3` | `<Blocker>` | `qa/dcgo-exams/ST23/ST23-09-effect3.yaml` | P1's Agumon attacks security and P0 blocks with Atratusmon (SelectPermanentEffect). Agumon is deleted, P0 security stays 5, Atratusmon is suspended. |
| `ST23-09#effect#4` | `[WD][WA][OPT]` immunity, then delete lowest-DP opp Digimon | `qa/dcgo-exams/ST23/ST23-09-effect4.yaml` | [WD] arm. P1 has Biyomon 3000 and Agumon 4000. Only Biyomon is a candidate and it is deleted; Agumon stays. The immunity half is NOT witnessed (P1's quiet list has no Digimon effects). |
| `ST23-09#effect#5` | Option `<Use Req. ([BEATBREAK])>` | `qa/dcgo-exams/ST23/ST23-09-effect5.yaml` | The green Option is used with only YELLOW BEATBREAK Liollmon ST23-02 in play (no hatch). Memory 3 -> -2, Biyomon bottom-decked (not in trash), Option trashed. No Arts (Lv.3). |
| `ST23-09#effect#6` | Option `[Main]` suspend 1; bottom-deck highest-DP suspended | `qa/dcgo-exams/ST23/ST23-09-effect6.yaml` | P1 has Agumon 4000 (already suspended from its attack) and Biyomon 3000 (unsuspended). Biyomon is suspended, and the highest-DP suspended Digimon, Agumon, is bottom-decked, so the bounced card is not the one just suspended. Biyomon's suspended flag can't be read afterwards (the turn hands over and P1 unsuspends). |
| `ST23-09#effect#7` | `<Arts Digivolve>` | `qa/dcgo-exams/ST23/ST23-09-effect7.yaml` | Use for 5 (3 -> -2): [Main] bounces Biyomon, then Arts onto BT25-041. Memory stays -2, trash is empty, Atratusmon sits on BT25-041. The arts-digivolved WD deletes Agumon, and there is one bonus draw. |

### Core — BT25-057 Monarchlizamon, BT25-041 / ST23-04 Murasamemon

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| BT25-057#effect#0 | [Digivolve] Lv.4 w/[Glowing Dawn]: Cost 3 | `qa/dcgo-exams/BT25/BT25-057-effect0.yaml` | Digivolves from YELLOW Cougarmon ST23-03. No printed Green/Black circle matches it, so this route is the only one open. Memory goes 0 -> -3. No cost-route prompt. |
| BT25-057#effect#1 | [WD][WA][OPT] trash bottom FD under Tamer -> De-Digivolve 1 | `BT25/BT25-057-effect1.yaml` | BT26-089's FD ST23-06 goes to trash. p1 Birdramon ST1-05 is stripped (ST1-05 to trash, Biyomon stays at 3000). Measured on the WD arm. |
| BT25-057#effect#2 | [WD] may battle 1 opp Digimon | `BT25/BT25-057-effect2.yaml` | The battle (8000 vs 5000) deletes p1's Digimon (ST1-02 + ST1-05). Monarchlizamon stays unsuspended, so it was a battle, not an attack. The phantom <Barrier> `sim_only` row is gone since #708 (see findings). |
| BT25-057#effect#3 | DUAL <Use Req. ([Glowing Dawn])> | `BT25/BT25-057-effect3.yaml` | The green Option is used for 4 (4 -> 0) while p0 has only yellow cards (BT26-089, Cougarmon). It ends in the trash. |
| BT25-057#effect#4 | DUAL [Main] Rush + SecA+1 + 5000, then may attack | `BT25/BT25-057-effect4.yaml` | Cougarmon goes 4000 -> 9000 and attacks. It checks 2 security cards (5 -> 3) and beats the BT4-014 8000 top card, which it only wins because of the +5000. Also asserts 4000 DP on p1's turn 6 (duration finding). |
| BT25-057#effect#5 | DUAL Rule <Arts Digivolve> | `BT25/BT25-057-effect5.yaml` | After use, Cougarmon digivolves into the used card for free (memory stays 0). The Option is not trashed, the draw lands, and the +5000 carries over (13000). |
| BT25-041#effect#0 | [Digivolve] Lv.4 w/[Glowing Dawn]: Cost 3 | `BT25/BT25-041-effect0.yaml` | Digivolves from GREEN Armalizamon BT25-049; the printed circle is Yellow Lv.4. Memory goes 2 -> -1. |
| BT25-041#effect#1 | <Alliance> | `BT25/BT25-041-effect1.yaml` | ST23-12 is suspended as the cost. 2 security checks (5 -> 3). 7000 Murasamemon beats the 8000 BT4-014 only with the borrowed 2000. |
| BT25-041#effect#2 | [WD][WA][OPT] pay (top security / FD) -> play/use GD card cost -3 | `BT25/BT25-041-effect2.yaml` | Top-security branch: security 5 -> 4 and BT25-043 to hand. ST23-12 (cost 3) is played free (memory stays -1). Tamer FD untouched. |
| BT25-041#inherited#0 | [End of Attack][OPT] trash FD -> this GD Digimon unsuspends | `BT25/BT25-041-inherited0.yaml` | Murasamemon is a source under Zephagamon ST23-09. Zephagamon attacks and ends unsuspended; BT26-089's bottom FD ST23-03 is trashed. |
| ST23-04#effect#0 | [Digivolve] Lv.4 w/[Glowing Dawn]: Cost 3 | `ST23/ST23-04-effect0.yaml` | Digivolves from GREEN Armalizamon. Memory goes 0 -> -3. |
| ST23-04#effect#1 | <Alliance> | `ST23/ST23-04-effect1.yaml` | Same witness as BT25-041#effect#1: ally suspended, 2 checks, beats the 8000 security card. |
| ST23-04#effect#2 | [OP][WD] -5000; then (your turn) trash FD -> play/use GD card cost -3 | `ST23/ST23-04-effect2.yaml` | Birdramon 5000 -> 0 and is deleted (p1 trash ST1-02, ST1-05). BT26-089's FD ST23-06 is trashed. ST23-12 is played free (memory stays -3). |
| ST23-04#inherited#0 | [End of Attack][OPT] trash FD -> unsuspend | `ST23/ST23-04-inherited0.yaml` | Same witness as BT25-041#inherited#0, with ST23-04 as the source. |

### Core — ST23-13, ST23-15 e-Pulse, BT25-090, ST23-01 Kekkomon, ST23-12 Chiropmon

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| ST23-13#effect#0 | [SOMP][On Play] may place deck top FD under this Tamer; then if opp has a Digimon, +1 memory | `qa/dcgo-exams/ST23/ST23-13-effect0.yaml` | [On Play] arm, both halves live. P1 plays Agumon on T2. T3 P0 hard-plays the Tamer (3 -> -1); places ST23-03 under it; +1 -> 0, so P0 keeps the turn (the next prompt is P0's main_phase). |
| ST23-13#effect#1 | [Your Turn] when effects trash from under this Tamer, suspend it -> 1 GD Digimon +3000 until opp's turn ends | `qa/dcgo-exams/ST23/ST23-13-effect1.yaml` | The trasher is Kekkomon's inherited (on attack). Tamer suspended; Armalizamon at 7000 during P1's T4 and back to 4000 on P0's T5 (shows the expiry). Trigger timing is an ENGINE FINDING (below). |
| ST23-13#security#0 | [Security] play this card free | `qa/dcgo-exams/ST23/ST23-13-effect2.yaml` | ST23-13 at P0 stack[9]; P1's Agumon attacks on T4. Tamer goes to P0's field, not the trash. Its [On Play] then runs on P1's turn: places ST23-03 and gives P0 +1 memory (P1 3 -> 2). |
| ST23-15#effect#0 | <Use Req. ([BEATBREAK])> ignore colour | `qa/dcgo-exams/ST23/ST23-15-effect0.yaml` | White e-Pulse used with only the green BEATBREAK Tamer BT25-090 in play (3 -> 0). It free-plays ST23-13. Negative control (probed, not committed): with no Tamer in play the use is not legal. |
| ST23-15#effect#1 | [Main] may play 1 BEATBREAK cost<=4 from hand/trash free; then place this in BA | `qa/dcgo-exams/ST23/ST23-15-effect1.yaml` | BT25-049 (cost 4) played free from hand; e-Pulse on the field. Only the hand branch is used: the trash is kept empty so DCGO's zone menu collapses. |
| ST23-15#effect#2 | [SOMP] place this from BA FD under a BEATBREAK Tamer -> Draw 1, +1 memory | `qa/dcgo-exams/ST23/ST23-15-effect2.yaml` | effect1 line carried on to T5. On a MultipleSkills with the Tamer's SOMP, e-Pulse resolves first: ST23-15 goes under the Tamer, ST23-06 is drawn, memory 3 -> 4, trash stays empty. |
| ST23-15#security#0 | [Security] activate this card's [Main] | `qa/dcgo-exams/ST23/ST23-15-effect3.yaml` | ST23-15 at stack[9]. P0 plays BT25-090 free on P1's turn. ENGINE FINDING: e-Pulse ends in the trash, but the text places it in the battle area. Our behaviour is asserted. |
| BT25-090#effect#0 | [Start of Your Turn] if memory <= 2, set to 3 | `qa/dcgo-exams/BT25/BT25-090-effect0.yaml` | P1 overspends on T2 (4 -> 1 -> -1). P0 opens T3 on 3, not 1. A ledger assert at P1's -1 is included. |
| BT25-090#effect#1 | [All Turns] when any Digimon suspends, suspend this -> may place top 2 FD under it | `qa/dcgo-exams/BT25/BT25-090-effect1.yaml` | The opponent's Agumon attacks on the opponent's turn. Tamer suspended with the two pinned deck-top cards under it. The placed cards are deliberately not ST23-03/04 (see the FD-inherited finding). |
| BT25-090#effect#2 | [Your Turn][OPT] when using a GD Option, trash bottom FD under a Tamer -> cost -1 | `qa/dcgo-exams/BT25/BT25-090-effect2.yaml` | The 2nd e-Pulse costs 2 (3 -> 1). BT25-041 (the bottom FD card, placed by T5's SOMP) is trashed. |
| BT25-090#security#0 | [Security] play this card free | `qa/dcgo-exams/BT25/BT25-090-effect3.yaml` | BT25-090 at stack[9]; P1 attacks. On P0's field with no memory paid. |
| ST23-01#inherited#0 | [When Attacking][OPT] trash bottom FD under a Tamer -> may digivolve into a GD hand card, cost -2 | `qa/dcgo-exams/ST23/ST23-01-inherited0.yaml` | Kekkomon -> Chiropmon (in breeding, T1), moved T3. Chiropmon attacks and digivolves into BT25-049 for 2 - 2 = 0 (memory stays 3). ST23-04 is trashed. |
| ST23-12#effect#0 | [Digivolve] Lv.2 w/[Glowing Dawn]: Cost 0 | `qa/dcgo-exams/ST23/ST23-12-effect0.yaml` | Green Kekkomon can't use the standard Purple circle, so the special route is the only one. Cost 0 at memory 0, P0 keeps the turn. T3 move shows the ST23-12-over-ST23-01 stack. |
| ST23-12#effect#1 | [On Play] trash bottom FD under a Tamer -> may return 1 GD Digimon from trash | `qa/dcgo-exams/ST23/ST23-12-effect1.yaml` | ST23-03 goes source -> trash -> hand (trash ends empty). Same trigger-timing ENGINE FINDING as ST23-13#effect#1. |
| ST23-12#inherited#0 | <Retaliation> | `qa/dcgo-exams/ST23/ST23-12-inherited0.yaml` | Armalizamon (4000, with Chiropmon under it) attacks and stays suspended. On T4, P1's 6000 Birdramon attacks it and Armalizamon is deleted. Retaliation then deletes Birdramon, leaving both fields empty. |

### Core — ST23-06 Gekkomon, BT25-049 Armalizamon, ST23-03 Cougarmon, BT25-032, BT25-035, BT25-046

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| ST23-06#effect#0 | [Digivolve] Lv.2 w/[Glowing Dawn]: Cost 0 | `qa/dcgo-exams/ST23/ST23-06-effect0.yaml` (variant `gd-D-frimon-eggs`) | Frimon (yellow Lv.2 GD) -> Gekkomon on T1. The green circle can't match a yellow egg, so the alt is the only route. Memory stays 0, it's still T1, and the digivolve draw lands in hand. |
| ST23-06#effect#1 | [When Moving] (timing fragment of the reveal-3 body) | `qa/dcgo-exams/ST23/ST23-06-effect1.yaml` | Body triggered by the T3 breeding->battle move with a GD Tamer in play. ST23-09 goes to hand, ST23-04 goes under the Tamer, memory stays 3. |
| ST23-06#effect#2 | [On Play] reveal 3, add 1 GD + place 1 under GD Tamer | `qa/dcgo-exams/ST23/ST23-06-effect2.yaml` (pre-existing, passes) | On Play arm, from the worked example. |
| ST23-06#inherited#0 | Inherited <Piercing> | `qa/dcgo-exams/ST23/ST23-06-inherited0.yaml` | Armalizamon over [ST23-06, Kekkomon] attacks a suspended Agumon and deletes it. P1 security goes 5 -> 4 and P1 trash is [ST1-02, ST1-03]. Armalizamon's own Piercing is a top-card inherited, so it doesn't count. |
| BT25-049#effect#0 | [Digivolve] Lv.3 w/[Glowing Dawn]: Cost 2 | `qa/dcgo-exams/BT25/BT25-049-effect0.yaml` | Built from yellow Liollmon, so the green circle can't match and only the alt applies. Memory 0 -> -2, and on T3 the field shows Armalizamon over [BT25-032, ST23-01]. |
| BT25-049#effect#1 | [OP][WD] may suspend 1 opp Digimon | `qa/dcgo-exams/BT25/BT25-049-effect1.yaml` | Measured through the WD arm on T3. Agumon is suspended. |
| BT25-049#effect#2 | [Your Turn][OPT] using a GD Option: trash bottom FD under a Tamer -> cost -3 | `qa/dcgo-exams/BT25/BT25-049-effect2.yaml` (gd-bt26-8th) | Uses P-236 (cost 3) for 0: memory stays 1 and it's still T3. BT25-043 (the bottom FD card) is trashed and the Tamer is left with 2 sources. |
| BT25-049#inherited#0 | Inherited <Piercing> | `qa/dcgo-exams/BT25/BT25-049-inherited0.yaml` | Murasamemon BT25-041 over [Armalizamon, Chiropmon, Kekkomon] attacks Monodramon on T5 (suspended because it attacked on T4). P1 security goes 5 -> 4. Chiropmon is the Lv.3 rather than Liollmon because of finding F1. |
| ST23-03#effect#0 | [Digivolve] Lv.3 w/[Glowing Dawn]: Cost 2 | `qa/dcgo-exams/ST23/ST23-03-effect0.yaml` | Built from green Gekkomon, so the yellow circle can't match and only the alt applies. Memory -2, then the field shows ST23-03 over [ST23-06, ST23-01]. |
| ST23-03#effect#1 | [OP][WD] top security to hand, then <Recovery +1> | `qa/dcgo-exams/ST23/ST23-03-effect1.yaml` | Measured through the On Play arm (hard-played T1). stack[9] (ST23-12) goes to hand and security is back to 5. |
| ST23-03#effect#2 | [Your Turn] when this digivolves into a GD Digimon: trash bottom FD under a Tamer -> cost -2 | `qa/dcgo-exams/ST23/ST23-03-effect2.yaml` | Cougarmon -> BT25-041 (cost 3) for 1: memory 1 -> 0 and it's still T3. BT25-043 is trashed and the Tamer is left with 2 sources. |
| ST23-03#inherited#0 | Inherited <Barrier> | `qa/dcgo-exams/ST23/ST23-03-inherited0.yaml` | Monarchlizamon over [ST23-03, ...] attacks security into Phoenixmon (12000). Barrier is accepted: P0 security goes 5 -> 4 with ST23-12 in trash, and the attacker survives suspended. Has two sim_only rows (findings F3, F4). |
| BT25-032#effect#0 | [Digivolve] Lv.2 w/[Glowing Dawn]: Cost 0 | `qa/dcgo-exams/BT25/BT25-032-effect0.yaml` | Built from green Kekkomon, so the yellow circle can't match and only the alt applies (no variant needed). Memory stays 0, then Liollmon over [ST23-01] on T3. |
| BT25-032#effect#1 | [On Play] reveal 3, add 1 GD + 1 yellow BEATBREAK | `qa/dcgo-exams/BT25/BT25-032-effect1.yaml` | ST23-09 (GD, not yellow) and ST23-03 (yellow BB) go to hand. BT16-082 goes to the bottom (sim_only row for the N=1 order). |
| BT25-032#inherited#0 | Inherited <Barrier> | `qa/dcgo-exams/BT25/BT25-032-inherited0.yaml` | Cougarmon BT25-035 over [Liollmon, Kekkomon] attacks security into Vermilimon (8000). Barrier is accepted: P0 security goes 5 -> 4 with ST23-12 in trash, and Cougarmon survives. No sim_only rows. |
| BT25-035#effect#0 | [Digivolve] Lv.3 w/[Glowing Dawn]: Cost 2 | `qa/dcgo-exams/BT25/BT25-035-effect0.yaml` | Built from green Gekkomon, so the yellow circle can't match and only the alt applies. Memory -2, then BT25-035 over [ST23-06, ST23-01]. |
| BT25-035#effect#1 | [OP][WD] -3000 to 1 opp Digimon; then trash 2 FD under Tamers -> may digivolve into a GD hand Digimon for free | `qa/dcgo-exams/BT25/BT25-035-effect1.yaml` | Agumon goes 4000 -> 1000. Cougarmon -> BT25-041 for free (memory stays 2). Trash is [BT25-043, ST23-04] and the Tamer is down to [ST23-03]. See F2 and F5. Most likely line in this group to need re-authoring after the oracle run. |
| BT25-035#inherited#0 | Inherited <Barrier> | `qa/dcgo-exams/BT25/BT25-035-inherited0.yaml` | Monarchlizamon over [BT25-035, Gekkomon, Kekkomon] attacks into Phoenixmon. Barrier is accepted: P0 security goes 5 -> 4, and the attacker survives. One sim_only TriggerOrder row (F4). |
| BT25-046#effect#0 | [Digivolve] Lv.2 w/[Glowing Dawn]: Cost 0 | `qa/dcgo-exams/BT25/BT25-046-effect0.yaml` (variant `gd-D-frimon-eggs`) | Same as ST23-06#effect#0: from a yellow Frimon only the alt applies. |
| BT25-046#effect#1 | [On Play] reveal 3, add 1 GD + 1 green BEATBREAK | `qa/dcgo-exams/BT25/BT25-046-effect1.yaml` (gd-bt26-4th) | BT25-043 (GD, yellow) and BT25-049 (green BB) go to hand. ST23-12 goes to the bottom. |
| BT25-046#inherited#0 | Inherited <Piercing> | `qa/dcgo-exams/BT25/BT25-046-inherited0.yaml` (gd-bt26-4th) | Armalizamon over [BT25-046, Kekkomon] attacks the suspended Agumon. P1 security goes 5 -> 4 and P1 trash is [ST1-02, ST1-03]. |

### Tail — P-236, BT26-089 Kyo Sawashiro, BT25-088 Kyo Sawashiro, ST23-14, BT16-082

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| P-236#effect#0 | <Use Req. ([Glowing Dawn] trait)> | `qa/dcgo-exams/P/P-236-effect0.yaml` | Green Option used with only a YELLOW [Glowing Dawn] Tamer (BT26-089) in play and nothing hatched: the use is legal, 3 memory paid (4 -> 1), P-236 placed in the battle area |
| P-236#effect#1 | [Main] reveal 3, add 1 [Glowing Dawn], rest to bottom, place self | `qa/dcgo-exams/P/P-236-effect1.yaml` | ST23-09 added to hand, 2 leftovers ordered to the bottom (one row), P-236 on the field rather than in the trash |
| P-236#effect#2 | [Main] <Delay> gain 2 memory | `qa/dcgo-exams/P/P-236-effect2.yaml` | Placed T3, activated T5 with `main: {on: field.1}`: memory 4 -> 6, P-236 in the trash |
| P-236#security#0 | [Security] place this card in the battle area | `qa/dcgo-exams/P/P-236-effect3.yaml` | Agumon flips P-236: P-236 lands on P0's field, trash empty, security 4 |
| BT26-089#effect#0 | [SOMP] put 1 [BEATBREAK] card from hand under -> Draw 1 + 1 memory | `qa/dcgo-exams/BT26/BT26-089-effect0.yaml` | Memory 3 -> 4, source [ST23-12], hand swaps ST23-12 for BT25-049 |
| BT26-089#effect#1 | [All Turns] on security removal: suspend, put deck top under; if by an effect, opp Digimon gets <Security A. -1> | `qa/dcgo-exams/BT26/BT26-089-effect1.yaml` | The whole clause, through the by-effects path: Liollmon BT26-025 [On Play] puts the top security card under BT26-089. The Tamer suspends, gains sources {ST23-03, ST23-15, BT25-043}, and Agumon gets SA-1. **Visible on T4:** Agumon attacks and checks 0 cards, so security stays at 5 |
| BT26-089#security#0 | [Security] play this card for free | `qa/dcgo-exams/BT26/BT26-089-effect2.yaml` | BT26-089 on P0's field, P1's memory untouched. ENGINE FINDING: the Tamer's own security-loss trigger fires (once since #708; declined `sim_only`, DCGO never asks) |
| BT25-088#effect#0 | [Start of Your Turn] memory <= 2 -> set to 3 | `qa/dcgo-exams/BT25/BT25-088-effect0.yaml` | P1 overshoots to give P0 1 memory (pinned at step 4: -2 before the overshoot). P0 opens T3 at 3 |
| BT25-088#effect#1 | [All Turns] on security removal: suspend, may put top 2 under | `qa/dcgo-exams/BT25/BT25-088-effect1.yaml` | On P1's turn: BT25-088 suspended with sources {ST23-15, ST23-02}, security 4 |
| BT25-088#effect#2 | [Your Turn][OPT] [Glowing Dawn] card would be played: trash bottom face-down card under a Tamer, cost -1 | `qa/dcgo-exams/BT25/BT25-088-effect2.yaml` | Liollmon ST23-02 played for 2 instead of 3 (memory -2, not -3). ST23-12 trashed from under BT26-089 |
| BT25-088#security#0 | [Security] play this card for free | `qa/dcgo-exams/BT25/BT25-088-effect3.yaml` | BT25-088 on P0's field. Same ENGINE FINDING (self-trigger, once since #708) |
| ST23-14#effect#0 | [SOMP][On Play] may put deck top under; then +1 memory if opp has a Digimon | `qa/dcgo-exams/ST23/ST23-14-effect0.yaml` | Both arms. [On Play] T1: 1 source, no memory (opponent has no Digimon). [SOMP] T3: 2 sources, memory 3 -> 4 (Agumon on board) |
| ST23-14#effect#1 | [Your Turn] effects trash cards from under: suspend -> [Glowing Dawn] Digimon gains <Jamming> | `qa/dcgo-exams/ST23/ST23-14-effect1.yaml` | Chiropmon ST23-12 [On Play] trashes the bottom card from under ST23-14. ST23-14 SUSPENDED and keeps 1 source, then the Jamming pick follows. <Jamming> itself can't be asserted (see the limit in the header). Prompt-ORDER finding in §3 |
| ST23-14#security#0 | [Security] play this card for free | `qa/dcgo-exams/ST23/ST23-14-effect2.yaml` | ST23-14 on the field. Its [On Play] fires off the effect play: the deck top goes under it, and +1 memory because P1 has Agumon (P1 3 -> 2) |
| BT16-082#effect#0 | [Your Turn][OPT] Digimon moves breeding -> battle: reveal 3, add a Digimon/Tamer, rest to bottom, may hatch | `qa/dcgo-exams/BT16/BT16-082-effect0.yaml` | Ukkomon is the mover (matches the card Q&A). BT25-090 added to hand. **Hatch witness:** T7 opens in Main with no breeding question because the area is occupied. A probe that declines the hatch reads Breeding instead |

### Tail — BT26-025 Liollmon, BT26-070 NightChiropmon, BT26-075 ScourgeChiropmon

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| BT26-025#effect#0 | [Digivolve] Lv.2 w/[Glowing Dawn]: Cost 0 | BT26-025-effect0.yaml | Green Kekkomon digivolves in breeding into yellow Liollmon. The printed yellow circle misses, so only the trait route applies. Memory stays 0; the field shows Liollmon over Kekkomon after the T3 move. |
| BT26-025#effect#1 | [When Moving] (header of the shared body) | BT26-025-effect1.yaml | Breeding-to-battle move with ST23-13 in play. The top security card ST23-09 goes face down under ST23-13, Recovery +1 follows, and security stays at 5. |
| BT26-025#effect#2 | [On Play] By placing top security under a GD Tamer, Recovery +1 | BT26-025-effect2.yaml | Liollmon played from hand (3 -> 0). ST23-09 ends up under ST23-13 and security stays at 5. |
| BT26-025#inherited#0 | [When Attacking][OPT] may add top security to hand; then if 0, Recovery +1 | BT26-025-inherited0.yaml | Armalizamon over Liollmon attacks. Answering "Add" puts ST23-09 in hand and p0 security drops to 4. The "if 0 security" tail is not witnessed (it needs a 1-security board). |
| BT26-070#effect#0 | [Digivolve] Lv.3 w/[Glowing Dawn]: Cost 2 | BT26-070-effect0.yaml | Yellow Liollmon becomes purple NightChiropmon by the trait route only (3 -> 1). |
| BT26-070#effect#1 | "[On Play][When Digivolving]" header (empty text; carries the dropped <Draw 1>) | BT26-070-effect1.yaml | T1 hard-play (0 -> -5) draws stack[10] ST23-09, and the mandatory discard trashes that same card. |
| BT26-070#effect#2 | "and trash 1 card in your hand." | BT26-070-effect2.yaml | Measured through [When Digivolving] on the field. p0.trash is [ST23-15], an opening-hand card. (In the breeding area nothing fires, which matches rule 6-4.) |
| BT26-070#effect#3 | [Main][OPT] trash 2 face-down from Tamers, then use a GD Option from trash at -2 | BT26-070-effect3.yaml | Kyo builds 2 face-down sources over T5/T7. The [Main] trashes both, then Atratusmon (ST23-09, use 5) is used from trash for 3 (4 -> 1). Kyo is used instead of ST23-13 because of the trigger finding below. |
| BT26-070#inherited#0 | <Retaliation> (inherited) | BT26-070-inherited0.yaml | Monarchlizamon over NightChiropmon is suspended by its own attack. On T4 p1's Phoenixmon (12000) attacks it, Monarchlizamon dies, and Retaliation deletes Phoenixmon. The effect-battle path is broken (finding below). |
| BT26-075#effect#0 | [Digivolve] Lv.4 w/[Glowing Dawn]: Cost 3 | BT26-075-effect0.yaml | Green Armalizamon becomes Scourge by the trait route only (1 -> -2), all in breeding (no prompts). The line ends on the T5 move so it never reaches the broken Execute window. |
| BT26-075#effect#1 | <Execute> (empty text) | BT26-075-effect1.yaml | Since #708 the end-of-turn window opens: Scourge attacks the player at the end of T5 (p1 security 5→4, no main-phase attack declared), is deleted at the end of that attack, and <Ascension> puts it on p0's security (5→6). (Before #708 the file pinned the never-opens bug.) |
| BT26-075#effect#2 | <Ascension> (reminder text) | BT26-075-effect2.yaml | Scourge loses a battle to the suspended Phoenixmon. Ascension is picked first (`trigger: Ascension`), so security goes 5 -> 6 and Scourge is not in trash. The [On Deletion] then fizzles, matching the official Q&A. |
| BT26-075#effect#3 | [Security][On Deletion] trash a Tamer face-down, play a GD card (cost <=5) from trash free | BT26-075-effect3.yaml | [Security] arm. On T4 Phoenixmon checks the Scourge that Ascension placed. ST23-15 is trashed from Kyo and Armalizamon is played from trash at no cost. |
| BT26-075#effect#4 | Use Req ([Glowing Dawn]) | BT26-075-effect4.yaml | The purple Option is used with only the green/yellow ST23-13 in play (4 -> 0) and deletes BT1-014. |
| BT26-075#effect#5 | [Main] Delete 1 opp Digimon with the lowest level | BT26-075-effect5.yaml | With p1 holding Lv.4 BT1-014 and Lv.3 ST1-02, the pick offers count 1 (ST1-02). It is deleted and BT1-014 stays. |
| BT26-075#effect#6 | <Arts Digivolve> | BT26-075-effect6.yaml | Option used (1 -> -3), then Armalizamon digivolves into Scourge for free. p0.trash is empty and the field shows Scourge over Armalizamon. |

### Tail — ST23-08, ST23-02, ST23-11, ST23-07, ST23-10

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| ST23-08#effect#0 | [Digivolve] Lv.4 w/[GD]: Cost 3 | `ST23-08-effect0.yaml` (8th) | Black Wolvermon → green Monarchlizamon in breeding (corner circle can't match); memory 3→0; T7 move shows stack [ST23-01, ST23-11, ST23-12] |
| ST23-08#effect#1 | `<Alliance>` | `ST23-08-effect1.yaml` (7th) | Monarchlizamon attacks, suspends Liollmon; SA+1 → p1.security 5→3, Liollmon suspended. Slot math for `attack: field.1` is in the header (same permanent on both wires) |
| ST23-08#effect#2 | [OP][WD] +3000 DP; then if your turn, trash Tamer FD → play/use GD card cost −3 | `ST23-08-effect2.yaml` (7th) | Hard-play (On Play arm), Tamer = BT26-089 (no trash trigger). DP 10000; BT26-070 trashed from under Tamer; Liollmon (3) played for 0, memory stays −3 |
| ST23-08#inherited#0 | [End of Attack][OPT] trash Tamer FD → this GD Digimon unsuspends | `ST23-08-inherited0.yaml` (7th) | ST23-09 on top of Monarchlizamon attacks (suspended at step 18); End of Attack trashes ST23-03 from BT26-089 → ST23-09 unsuspended (step 20) |
| ST23-02#effect#0 | [Digivolve] Lv.2 w/[GD]: Cost 0 | `ST23-02-effect0.yaml` (7th) | Yellow Liollmon onto green Kekkomon (alt only); memory 0→0; T3 move shows sources [ST23-01] |
| ST23-02#effect#1 | [Your Turn] digivolve into GD Digimon: cost −1 | `ST23-02-effect1.yaml` (7th) | Liollmon (battle area) → Armalizamon, printed 2 → paid 1 (memory 3→2). Probe: in breeding it pays 2 (breeding Digimon can't activate effects — correct) |
| ST23-02#inherited#0 | `<Barrier>` (inherited) | `ST23-02-inherited0.yaml` (7th) | ST23-03 over Liollmon, attacked by Agumon 4000 vs 4000; Barrier accepted → security 5→4 (ST23-07 trashed), ST23-03 survives, and the tied Agumon is deleted (fixed by #708; the file used to pin Agumon surviving) |
| ST23-11#effect#0 | [Digivolve] Lv.3 w/[GD]: Cost 2 | `ST23-11-effect0.yaml` (8th) | Purple ST23-12 → black Wolvermon in battle area (alt only); memory 3→1, sources [ST23-01, ST23-12] |
| ST23-11#effect#1 | `<Blocker>` | `ST23-11-effect1.yaml` (8th) | Wolvermon blocks Biyomon (4000 vs 3000): Biyomon deleted, security stays 5 |
| ST23-11#effect#2 | [Your Turn] digivolve into GD Digimon: trash Tamer FD → cost −2 | `ST23-11-effect2.yaml` (8th) | Wolvermon → Monarchlizamon printed 3 paid 1 (memory 3→2 at step 17); bottom FD BT25-032 trashed (ST23-13 placed it at the bottom per Q&A). Prompt-order risk noted below |
| ST23-11#inherited#0 | `<Blocker>` (inherited) | `ST23-11-inherited0.yaml` (8th) | Monarchlizamon (no printed Blocker) over Wolvermon blocks Biyomon; security stays 5 |
| ST23-07#effect#0 | [Digivolve] Lv.3 w/[GD]: Cost 2 | `ST23-07-effect0.yaml` (7th) | Purple ST23-12 → green Armalizamon (alt only); memory 3→1, sources [ST23-01, ST23-12] |
| ST23-07#effect#1 | [OP][WD] if ≤1 Tamers, may play GD Tamer free | `ST23-07-effect1.yaml` (7th) | WD arm, 0 Tamers: ST23-13 (cost 4) played free, memory stays 1; ST23-13's own [On Play] place accepted |
| ST23-07#inherited#0 | `<Piercing>` (inherited) | `ST23-07-inherited0.yaml` (7th) | Monarchlizamon over Armalizamon (climbed in breeding) deletes suspended Agumon; Piercing checks: p1.security 5→4, p1.trash [BT3-007, ST1-02] |
| ST23-10#effect#0 | [Digivolve] Lv.2 w/[GD]: Cost 0 | `ST23-10-effect0.yaml` (8th) | Black Pristimon onto green Kekkomon (alt only); memory 0; sources [ST23-01] |
| ST23-10#effect#1 | [On Play] place 1 hand card under GD Tamer → Draw 2 | `ST23-10-effect1.yaml` (8th) | BT9-103 placed under ST23-13 (sources gain it), hand 3→5 (ST23-06, ST23-09 drawn) |
| ST23-10#inherited#0 | `<Blocker>` (inherited) | `ST23-10-inherited0.yaml` (8th) | ST23-03 (no printed Blocker) over Pristimon blocks Biyomon; Biyomon deleted, security 5 |

## Expected DCGO quirks / prompt-shape guesses (per group)

Read these before triaging an oracle abort: most predicted aborts are prompt-shape translation rows, not rules findings.

### Core — BT25-043 Habakirimon, ST23-09 Atratusmon (DUAL Lv.5)

- **"Player with the most security" menu (BT25-043 #effect#1/#2, all WD/WA lines).** Ours always offers the 3-way `EffectChoice` (own / opponent / don't). The DCGO shape is taken from the BT26-031.yaml / ST23-05.yaml crosscheck of the same idiom: a `SetIntSelection` (generic_int) over only the *valid* players plus "Don't trash". Authored as a sim_only `choice:` plus a dcgo_only `value:`. **The values are a GUESS** (1 = own, 2 = opponent, 3 = don't), borrowed from BT25-041.yaml's documented 1/2/3 menu. If the oracle aborts on those rows, read the real values off the abort message. This affects effect0/1/2/3/6.
- **Habakiri "Then, by trashing your top security card"** (#effect#4/#5/#6). Per the BT25-043.yaml crosscheck, DCGO asks a Yes/No; I assumed generic_bool, following the EX7-048 convention (sim_only `choice:` plus dcgo_only `yes:` / `decline:`). It could be OptionalSkill instead.
- **BT25-041 [WD][WA] pay menu** (BT25-043-effect2, where Murasamemon attacks). Ours is a sim_only optional decline. DCGO uses generic_int `value: 3` = "Don't pay"; that value IS documented in BT25-041.yaml.
- **BT25-041 inherited [End of Attack]** (ST23-09-effect1/2/3, BT25-043-effect1). Authored as sim_only declines. DCGO is expected to ask nothing, because its sibling clauses gate CanActivate on "a Tamer with a face-down source" and P0 never has a Tamer. If DCGO does ask, these rows become shared declines.
- **Single-candidate picks** (ST23-09 IsMinDP delete, IsMaxDP bounce, Arts target, the -8000 pick). Authored as shared SelectPermanentEffect rows with `count: 1`. If DCGO auto-picks a lone candidate, the row aborts. That would be a prompt-shape asymmetry, not a rules finding.
- **Arts digivolve draws the digivolution bonus card on ours** (ST23-09-effect7, BT25-043-effect6 assert `p0.hand`). If the oracle disagrees, that is a finding.
- **Use Req. via a breeding permanent** (BT25-043-effect4). Relies on DCGO's UseRequirements counting breeding permanents, as documented in NOTES-BT25-085.md.

### Core — BT25-057 Monarchlizamon, BT25-041 / ST23-04 Murasamemon

- **BT25-057 WD pair** (YAML: one shared WD/WA ActivateClass, OPT, plus a separate skippable WD battle). Expect DCGO's MultipleSkills over [BT25-057, BT25-057]. Our scenarios answer it with `ordinal:` because neither branch is a keyword. A different DCGO order will show up as a divergence.
- **Optional trigger picked from a TriggerOrder.** Picking an optional trigger in our TriggerOrder commits to it: no separate yes/no, and the next pick can't be declined. In DCGO, MultipleSkills is followed by an OptionalSkill. Expected extra DCGO OptionalSkill row in BT25-057-effect2 (battle) and in BT25-041-effect1 / -inherited0 (the WA picks). If the oracle confirms it, add `dcgo_only` rows.
- **BT25-041 pay menu.** In DCGO this is one generic_int (1 = security, 2 = FD, 3 = don't pay). Ours is a Replacement yes plus an EffectChoice. Authored as `dcgo_only` value plus `sim_only` yes/choice rows. In ours, picking the WA from a TriggerOrder goes straight to the EffectChoice, so the TriggerOrder row is the accept.
- **ST23-04 OP/WD.** DCGO has one shared ActivateClass with optional:false. Ours has two clauses, so our TriggerOrder between them is `sim_only`. DCGO's "may" is the canNoSelect Tamer SelectPermanentEffect, so our optional-clause yes is `sim_only`.
- **<Alliance>.** Ours has its own combat window ("Declare an ally" OwnField, which becomes SelectPermanentEffect). DCGO's AllianceSelfEffect (OnAllyAttack) may batch into the same MultipleSkills as the other [When Attacking] triggers (BT25-041 WA, ST23-01 inherited). If so, DCGO's candidate list is longer, or DCGO asks a MultipleSkills we never ask (`dcgo_only`).
- **ST23-12 [On Play] decline.** Ours is a Replacement gate; DCGO's is a canNoSelect Tamer SelectPermanentEffect answered with no pick. These rows are left without `expect`.
- **Option use: BT25-057-effect3/4/5.** DCGO asks SelectAttackEffect inside the [Main] ("then may attack"), before the Arts/trash decision. Ours asks the Arts prompt first and has no in-effect attack (see findings). Expect an oracle abort at that step. The wire shape of the Arts Digivolve prompt itself is unverified.
- **De-Digivolve with no opponent Digimon (BT25-057-effect2).** Ours still asks the Tamer pick and trashes the FD. BT25_057.cs (read 2026-10-04) agrees: it asks the canNoSelect Tamer pick whenever a Tamer has a face-down card and trashes before checking for a De-Digivolve target.
- **Unpaid BT25-057 cost and its OPT (BT26-070-inherited0).** BT25_057.cs calls `RemoveUse()` when the Tamer cost was not paid, so DCGO re-offers the WA half after an unpaid WD activation; ours spends the OPT (15-14-1) and, since #708, does not offer it. The WA ordering row is `dcgo_only`.
- **Single-candidate opponent picks.** In ours these park (e.g. the BT25-057 De-Digivolve target with one candidate). That matches the usual DCGO "asked even with one candidate".

### Core — ST23-13, ST23-15 e-Pulse, BT25-090, ST23-01 Kekkomon, ST23-12 Chiropmon

These are guesses until the oracle runs. They cite the card YAMLs' DCGO crosscheck notes, because the C# is not in this container.

- **ST23-13 "You may place"** is an internal Yes/No (`SetIntSelection`, optional:false). It is answered with `yes: true`, as in ST23-06-effect2. The memory gain fires regardless of the answer.
- **ST23-12 [On Play] gate**: our clause-level `optional: true` parks a Replacement yes/no before the Tamer pick. The YAML says DCGO carries the "may" on the Tamer pick's `canNoSelect:true`, so our row is `sim_only`. If ST23_12.cs also registers the effect as optional, DCGO will abort asking OptionalSkill; drop `sim_only` then.
- **ST23-01 inherited**: authored with `optional_gate_fold` (expect OptionalSkill) on our hand pick. **Order risk:** ours picks the hand card BEFORE paying the Tamer trash cost. DCGO pays the cost first, then runs `DigivolveIntoHandOrTrashCard` -> SelectHandEffect. That would abort on prompt kind at the hand-pick row. This affects ST23-01-inherited0 and ST23-13-effect1.
- **BT25-090 [All Turns] suspend trigger**: the YAML says DCGO shows a **3-way menu** (place-2 / suspend-only / no). That is likely `generic_int` or `SelectCountEffect`, not OptionalSkill. If the oracle aborts on that row, re-author it as a `dcgo_only value:` row plus a `sim_only` yes.
- **BT25-090 cost reducer**: ours asks an EffectChoice accept and then the Tamer pick. DCGO asks one skippable Tamer pick, so our accept is `sim_only`.
- **ST23-15 [Main] / [Security]**: DCGO opens a Hand/Trash/Don't-play `SetIntSelection` zone menu. Every line keeps P0's trash empty (or free of candidates), so the menu should collapse to SelectHandEffect, as in the EX12-074-security0 precedent. Unverified for ST23_15.cs.
- **ST23-15 9-1-5 order prompt**: ours asks "Trash the used Option vs triggered effect" even though the Option is then PLACED, not trashed. It is authored as a `choice: ... sim_only` row (the BT3-096 idiom).
- **MultipleSkills** at T5 SOMP (ST23-13 + field ST23-15): answered by identity. DCGO enumerates its own list.
- **Single-candidate Tamer / target picks**: these park on our side (OwnField) and are kept as SelectPermanentEffect rows. We assume DCGO asks even with one candidate.

### Core — ST23-06 Gekkomon, BT25-049 Armalizamon, ST23-03 Cougarmon, BT25-032, BT25-035, BT25-046

- **Remainder ordering.** A 1-card remainder is our OrderedPermutation at N=1, which DCGO doesn't ask, so it's a sim_only row (ST23-06-effect1, BT25-032/046-effect1). A 2-card remainder (ST23-06-inherited0 [When Moving] without a Tamer, and P-236 in BT25-049-effect2) is two sim_only rows plus one `dcgo_only` SelectCardEffect row (EX7-044 convention).
- **Two-bucket reveals** (BT25-032/046 #effect#1, Gekkomon WM/OP). DCGO adds bucket 0's card to hand as soon as that prompt closes, so expect a one-field `p0.hand` diff on the 2nd pick row. That is the documented outcome-neutral quirk; no asserts sit on that row. I didn't author the "leading generic_bool gate" the doc mentions for >=2-bucket reveals, to match the passing ST23-06-effect2 worked example. If DCGO asks it, every reveal row here moves by one.
- **Optional cost reducers** (BT25-049#effect#2, ST23-03#effect#2). Ours parks an EffectChoice accept gate and then an OwnField Tamer pick. In DCGO the decline lives inside the Tamer SelectPermanentEffect (`canNoSelect`, per the crosschecks in BT25-049.yaml / ST23-03.yaml). So the gate is `sim_only` and the Tamer pick is the shared row (P-170-effect1 convention). BT25-049 also has `SetIsSkippable(true)`: if that makes DCGO ask an OptionalSkill, the sim_only gate row is the first place to look.
- **ST23-13's "may place" prompts.** The [On Play] and [Start of Your Main Phase] places are our EffectChoice yes/no. They're left without `expect:` (worked-example style), since their DCGO class isn't recorded in this group.
- **ST23-13's trash trigger** ("by suspending this Tamer, +3000") is a TriggerOrder with 1 declinable candidate on our side, written as `expect: OptionalSkill`. DCGO may place it after the triggering effect finishes instead of mid-cost (F2). In BT25-035-effect1 it may be batched with BT25-041's WD as MultipleSkills.
- **BT25-041's [WD]/[WA] optional** is our Replacement gate, written as OptionalSkill. Declining it doesn't spend [Once Per Turn], so it's asked again at [When Attacking] (BT25-049-inherited0).
- **Armalizamon's suspend** is SelectPermanentEffect Mode.Tap with canNoSelect: true, written 1:1. With an empty opponent board it parks nothing on our side (BT25-049-effect2 / ST23-03-effect2).
- **Barrier** is DCGO's OptionalSkill against our Replacement, 1:1. The sim-side expect is asserted.
- **Slot refs.** Lines with a Tamer use `field.0` = Tamer (played T1) and `field.1` = the moved Digimon (moved T3). This assumes DCGO's frame order matches our order of entry.
- **p1 hands** always include a playable Digimon, so DCGO's CanSelect() never skips p1's main phase (EX12-031 lesson).

### Tail — P-236, BT26-089 Kyo Sawashiro, BT25-088 Kyo Sawashiro, ST23-14, BT16-082

- **BT26-025 [On Play] (in BT26-089-effect1):** DCGO is optional:false with a canNoSelect:true SelectPermanentEffect Tamer pick, so it asks even with one candidate (cards/bt26/BT26-025.yaml). Ours parks a 1-pending, declinable TriggerOrder and then auto-resolves the single-candidate Tamer pick. Authored as a `sim_only` yes plus a `dcgo_only` `cards: [BT26-089]` row. Risk: the sim_only/dcgo_only pairing may be off by one if DCGO also opens a gate.
- **BT25-088#effect#1:** the YAML crosscheck records DCGO showing a 3-way menu (place 2 / suspend only / no). If that menu is a `generic_int` and not `OptionalSkill`, the single `yes` row is a prompt-shape mismatch, not a rules finding.
- **BT25-088#effect#2:** ours is EffectChoice ("Use Cost reduction...?") + OwnField Tamer pick. The `expect: OptionalSkill` is unasserted sim-side (the harness notes it). Expected DCGO shape: OptionalSkill + SelectPermanentEffect.
- **ST23-12 cost (in ST23-14-effect1):** ours adds a TriggerOrder gate before the Tamer pick (the clause has `optional: true`). DCGO's canNoSelect pick is that gate, so the gate row is `sim_only`.
- **ST23-14 / ST23-13 place prompt:** DCGO's "internal Yes/No". Answered with a bare `yes` and no `expect`, the way the passing ST23-06-effect2 does.
- **BT16-082 "you may hatch":** ours is a 2-label EffectChoice answered `yes`. DCGO is probably an OptionalSkill or generic_bool.
- **Skipped breeding phases (BT16-082 T3/T7):** with a Lv.2 already in breeding, ours skips the breeding phase and no row is authored. If DCGO parks a breeding_action there, the line misaligns.
- **2-leftover reveals (P-236 [Main], BT16-082):** one OrderedPermutation row carrying both ids, the EX12-009 idiom.
- **Slot addressing:** `main: {on: field.1}` (P-236-effect2) points at an Option placed beside a Tamer. The `own.field.N` picks in BT25-088-effect2 and ST23-14-effect1 go on boards holding a Tamer plus another permanent. The wire carries identities for picks, but `main:`/`attack:` slots use DCGO's compact frame order (NOTES-BT25-083.md). The ordering between Tamers and Option frames is unverified, so this is a risk on P-236-effect2.
- **OnLoseSecurity firing on a Tamer played from security** (BT26-089-effect2, BT25-088-effect3): DCGO never asks. `IReduceSecurity.ReduceSecurity` (CardController.cs) collects the OnLoseSecurity skill infos before the [Security] effect plays the Tamer, so its `IsExistOnBattleArea` CanUse is false. The decline row is `sim_only` (read 2026-10-04).

### Tail — BT26-025 Liollmon, BT26-070 NightChiropmon, BT26-075 ScourgeChiropmon

- **BT26-025 [When Moving]/[On Play]:** BT26-025.yaml says the C# uses a shared ActivateClass with optional:false, skippable. The Tamer pick is canNoSelect:true. So I expect one cancellable SelectPermanentEffect with no OptionalSkill, matching our optional OwnField pick 1:1.
- **BT26-025 inherited Yes/No:** ours is a 2-way EffectChoice with no sim-assertable mapping, so no `expect:` is pinned. In BT26-075-effect2/3 the "Don't add" answer is authored as a `choice:` sim_only row plus a `decline` dcgo_only row. In BT26-025-inherited0 a plain `yes: true` is used. Re-author it if DCGO turns out to want generic_bool rather than OptionalSkill.
- **BT26-070 [Main] Tamer pick:** DCGO uses one multi-pick (one Tamer holding 2 cards gives one SelectPermanentEffect row). Ours re-parks once per card, so the 2nd pick is `sim_only`.
- **BT26-070-inherited0 TriggerOrder:** the [When Digivolving] stack is two triggers on the same id, BT25-057, answered with `ordinal: 0`. That is an ours-only order. DCGO may not stage the unpayable De-Digivolve branch, or may stage and refund it (BT25-057.yaml: "RemoveUse if no cost paid"). This row may need a dcgo_only/sim_only rework after the oracle pass.
- **BT26-075-effect3 order:** we resolve Scourge's [Security] effect before Kyo's OnLoseSecurity trigger. If DCGO orders Kyo first, the line will diverge on prompt order at step 22.
- **Single-candidate TriggerOrder rows** (Kyo's security-loss trigger, ST23-13's trash trigger in the probe): our pick also acts as the accept. DCGO should show this as an OptionalSkill (table row `TriggerOrder` / 1 declinable candidate).
- **DUAL Option [Main] (BT26-075-effect4/5):** I expect a mandatory SelectPermanentEffect even with one candidate. Arts Digivolve should be a SelectPermanentEffect with canNoSelect (as in BT25-085-effect6).

### Tail — ST23-08, ST23-02, ST23-11, ST23-07, ST23-10

- **Monarchlizamon OP/WD is ONE ActivateClass in DCGO, TWO triggers in ours** (ST23-08 YAML crosscheck: `ActivateClassesForSharedEffects`, +3000 then canNoSelect Tamer pick → canNoSelect SelectHandEffect → play). Ours opens a TriggerOrder over `[ST23-08, ST23-08]` (authored `sim_only`, ordinal 0 = the mandatory +3000 half — verified: ordinal 1 jumps straight into the optional half's Tamer pick with no accept gate, ordinal 0 resolves +3000 silently and then asks "You may activate ST23-08's triggered effect") and then a Replacement accept gate that is DCGO's canNoSelect Tamer pick (authored `yes, sim_only`). Used in ST23-08-effect1/effect2/inherited0 and ST23-11-effect2.
- **Wolvermon reducer** (ST23-11#effect#2): DCGO = one canNoSelect Tamer `SelectPermanentEffect`; ours = EffectChoice "Use Cost reduction to reduce digivolution cost?" + OwnField Tamer pick → gate authored `sim_only` (BT24-017 precedent).
- **ST23-08#inherited#0** (End of Attack): authored as OptionalSkill yes + Tamer SelectPermanentEffect. GUESS: if ST23_08.cs carries the "may" on a canNoSelect Tamer pick instead (as it does for #effect#2), the yes row must become `sim_only`.
- **ST23-08#effect#1 Alliance**: authored as one SelectPermanentEffect (suspend pick). If DCGO's AllianceSelfEffect asks an OptionalSkill first, the row needs `optional_gate_fold`. GUESS (no C# in container).
- **ST23-09 cost choice** (ST23-08-inherited0 step 16): both circles open (green Lv.5/4, GD alt/3) → `value: 3`, expected SelectCountEffect (EX12-015 precedent); sim does not assert that kind.
- **Barrier** prompt: OptionalSkill on DCGO (BarrierSelfEffect); ours Replacement (maps to OptionalSkill, sim-asserted).
- **Blocker** rows: DCGO "select blocker" SelectPermanentEffect, ours "Declare a blocker" OwnField (BT25-058-effect2 precedent).
- **Slot safety**: every `field.N` main-phase action is either on a single-Digimon board or pinned by the README rules (Digimon before Tamers in DCGO's compact list; 2-Digimon centre-out) — ST23-08-effect1/inherited0 use `field.1` = Monarchlizamon on both wires (reasoning in header); ST23-11-effect2 moves the Digimon in BEFORE playing the Tamer so `field.0` = Wolvermon on both.

## Appendix: per-group engine findings (authoring evidence)

Raw evidence behind the findings table above, as recorded by each authoring pass. Code-location claims are from reading the source, not from a fix.

### Core — BT25-043 Habakirimon, ST23-09 Atratusmon (DUAL Lv.5)

1. **BT25-043 #effect#3 (and ST23-05, byte-identical clause): the leave-prevention replacement never fires.** In `BT25-043-effect3.yaml`, P1's Gaia Force deletes Habakirimon with no prompt and no security paid. The printed text says P0 may trash its top security card and Habakirimon stays.
   - **Root cause (code-read):** the clause's `active_when` uses `any_permanent { of: you, zone: [security] }`. `existential_any` (`code/digimon-engine/src/dsl_cards/predicate.rs` ~1148) scans battle areas only, and security cards are not Permanents, so the gate is always false. The fix is `security_count_gte: 1`.
   - **Why tests stayed green:** the per-card test `bt25_043_glowing_dawn_leave_prevention_installs` (`tests/cards_behavioral/bt25/bt25_043.rs`) has a catch-all `_ =>` arm that accepts "no prompt".
   - Tracker: `qa/archetype-qa/engine-gaps.md` (card faithfulness, BT25-043 + ST23-05), plus a test-quality note for that `_ =>` arm.
2. **BT25-041 / ST23-04 / ST23-08 inherited `[End of Attack]` has two problems.** The DSL timing is an observer fanned out to every battle area (`combat/mod.rs` end-of-attack cleanup), and the YAMLs carry no gate, so:
   - (a) It is offered at the end of the OPPONENT's attack, including when P0 blocks (`ST23-09-effect3.yaml`). `[End of Attack]` should mean this Digimon's own attack ending.
   - (b) It is offered even when the "by trashing a face-down card under a Tamer" cost is unpayable (no Tamer). It is outcome-neutral when declined, but it is a phantom decision exposed to RL.
   - Tracker: `qa/archetype-qa/engine-gaps.md`. Possibly `docs/RUST_ENGINE_GAPS.md` if an "attacker is source" timing filter is missing as a primitive.
3. **BT25-049 cost reducer is a phantom prompt.** "When you would use a GD Option, by trashing FD under a Tamer, reduce by 3" parks a ONE-branch EffectChoice "Cost reduction (-3)" with no Tamer at all, so the cost is unpayable and the choice changes nothing (memory still paid in full). DCGO gates on a Tamer with a face-down source. Found while drafting effect4; that line was re-authored to avoid BT25-049. Tracker: `qa/archetype-qa/engine-gaps.md`.
4. **ST23-09 #effect#4 immunity half is unwitnessed.** The quiet opponent has no Digimon effects. A line with an opposing Digimon effect targeting Atratusmon after its WD would need a variant opponent deck. Not authored; the clause file witnesses only the delete.

### Core — BT25-057 Monarchlizamon, BT25-041 / ST23-04 Murasamemon

1. **The effect queue drains in the middle of an effect after a parked cost pick.** (BT25-057#effect#1 and #5; seen also with ST23-04#effect#2 and ST23-13.) After the `trash_bottom_face_down_source_under_tamer` Tamer pick resumes, our engine resolves other pending triggers before the rest of the same effect body runs:
   - the other WD (the battle) is asked between the Tamer pick and the De-Digivolve target pick;
   - ST23-13's [Your Turn] trash reaction runs before Monarchlizamon's De-Digivolve target, and before ST23-04's hand pick.

   The rules say triggered effects wait until the current effect finishes; DCGO finishes the effect first. End state is the same, but the row order differs, so expect an oracle divergence. Shown in `BT25-057-effect1.yaml` (battle OptionalSkill sits between the two SPEs) and `BT25-057-effect5.yaml`. Suggested tracker: `docs/RUST_ENGINE_GAPS.md` (effect-queue / resume primitive).
2. **[FIXED by #708, G-ENGINE-BARRIER-CANDIDATE-NOT-CARRIER-GATED; the `sim_only` row is removed] Phantom <Barrier> prompt offered to the wrong player.** (BT25-057#effect#2.) In `keyword_effects.rs`, Barrier's `replacement_condition` checks only that the cause is Battle. It does not check subject == carrier; only `replacement_process` checks that. So when p1's Birdramon is deleted in battle by Monarchlizamon, Monarchlizamon's inherited <Barrier> (from Ukkomon/Cougarmon) becomes a candidate, and a Replacement prompt is parked for p1 (the subject's controller). Accepting does nothing. This is an extra decision in the RL action space and desyncs DCGO. Shown in `BT25-057-effect2.yaml` (the `sim_only` actor-1 decline). It did not appear on security-card battle deletions. Suggested tracker: `docs/RUST_ENGINE_GAPS.md` / `qa/archetype-qa/engine-gaps.md`.
3. **"Then, it may attack" is lowered as an end-of-turn MayAttack modifier.** (BT25-057#effect#4.) The YAML uses `may_attack_now ... windowed: true`, and `dsl_cards/step/combat.rs` turns that into `ModifierType::MayAttack` until end of turn. The printed text and DCGO (SelectAttackEffect) attack inside the effect, before the Arts/trash step. Ours offers the Arts prompt first, and the attack is a later main-phase action that could be taken after other actions. Shown in `BT25-057-effect4.yaml`. Suggested tracker: `qa/archetype-qa/engine-gaps.md` (card faithfulness) / `qa/dsl-vocab-gaps.md`.
4. **Option-face buff duration vs official printed text.** (BT25-057#effect#4.) The official Bandai DB (`data/card_bundles/BT25-057.md`) says "until your opponent's turn ends". The YAML uses `expiry: end_of_turn`; its header quotes the card image and DCGO's "UntilEachTurnEnd" as "for the turn". `BT25-057-effect4.yaml` asserts our 4000 DP on p1's turn 6; the printed text implies 9000. Needs a printed-data triage: errata, or a printing difference. Suggested tracker: `qa/archetype-qa/engine-gaps.md` (and `data/card_overrides.json` if the DB is right).
5. **[FIXED by #708, G-ENGINE-OPT-SPENT-TRIGGER-IN-TRIGGER-ORDER; the MultipleSkills rows are removed] Spent [Once Per Turn] trigger re-offered.** (ST23-09, seen in BT25-041-inherited0 and ST23-04-inherited0; uncertain.) On the T5 attack, our TriggerOrder offers ST23-09's [WD][WA][OPT] again even though it was used on WD that turn. With p1's board empty its resolution is invisible, so it is unclear whether it is only queued or actually re-resolves. DCGO's maxCountPerTurn should not offer it, so expect a MultipleSkills divergence. Suggested tracker: `qa/archetype-qa/engine-gaps.md` (ST23-09), to be confirmed with a targeted test.
6. **Committed optional trigger.** Picking an optional trigger in a 2+ TriggerOrder skips its accept/decline gate, and the following pick is not declinable (BT25-057 WD battle). There is no RL decline path once the trigger is chosen first. Suggested tracker: `docs/RUST_ENGINE_GAPS.md` (trigger-order surface) / exam tooling note.

### Core — ST23-13, ST23-15 e-Pulse, BT25-090, ST23-01 Kekkomon, ST23-12 Chiropmon

1. **Triggered effect resolves mid-effect (timing).** Cards: ST23-13#effect#1, as triggered by ST23-12#effect#1 and ST23-01#inherited#0.
   - What ours does: when a "by trashing the bottom face-down card" cost fires Tomoro & Kyo's "[Your Turn] When effects trash cards from under this Tamer", our engine parks and resolves that trigger at once (TriggerOrder -> suspend -> DP pick). This happens in the middle of the host effect: before Chiropmon's trash-return pick, and before Kekkomon's digivolve.
   - What the rules say: `general_rule.pdf` 15-8-3-2 says a trigger-type effect can't activate during effect processing; it waits pending until the host effect finishes. The outcome is neutral in these lines, but the prompt ORDER is expected to diverge from DCGO.
   - Scenarios: `ST23-12-effect1.yaml`, `ST23-13-effect1.yaml`, `ST23-01-inherited0.yaml` (declined there).
   - Tracker: `docs/RUST_ENGINE_GAPS.md` (trigger-pending primitive for `on_digivolution_card_trashed`) or `qa/archetype-qa/engine-gaps.md`.
2. **Face-down Tamer sources grant inherited effects (serious).** Card: BT25-090#effect#1 line, but this is a general engine issue.
   - What ours does: with ST23-03 and ST23-04 placed face down under BT25-090, P1's Agumon attack makes our engine offer two prompts:
     - `<Barrier>` ("May accept replacement: <Barrier>"), offered to **P1** when Agumon would be deleted in battle;
     - ST23-04's inherited `[End of Attack]` unsuspend, offered to P0.
   - Swapping the placed cards for BT25-043 / BT25-057 makes both prompts disappear. So the effects come from the face-down cards under a Tamer.
   - What the rules say: face-down cards have no effects, and a Tamer does not inherit (verify the exact § in `general_rule.pdf`; the PDF isn't in this container).
   - Second bug: `keyword_effects.rs` Barrier's `replacement_condition` has no subject == carrier guard (only `replacement_process` checks it). That is why it is offered for another player's Digimon as a phantom prompt that does nothing.
   - Repro: in `BT25-090-effect1.yaml`, set stack[11..12] to `ST23-03, ST23-04`.
   - Tracker: `qa/archetype-qa/engine-gaps.md` (+ `docs/RUST_ENGINE_GAPS.md` if the source-effect scan needs a face-down filter primitive).
3. **ST23-15 [Security] doesn't place the Option in the battle area.** Clause: ST23-15#security#0.
   - What ours does: the `on_security` body re-runs only the free play, so e-Pulse goes to the trash.
   - What the text says: "Activate this card's [Main] effects", and the [Main] ends "Then, place this card in the battle area". DCGO's AddActivateMainOptionSecurityEffect re-runs the whole [Main] (cf. EX12-071-security0, where DCGO placed the Option on the field).
   - Expected divergence: `p0.trash` / `p0.field`.
   - Scenario: `ST23-15-effect3.yaml` (asserts our behaviour). Tracker: `qa/archetype-qa/engine-gaps.md` (card YAML faithfulness).
4. **Kyo (BT26-089) security-removal trigger fires for a removal that happened before it was in play.**
   - What ours does: Kyo is free-played by ST23-15's [Security] (the check that removed ST23-15 had already happened). Our engine still parks Kyo's "[All Turns] When your security stack is removed from" TriggerOrder.
   - Why it's wrong: a trigger can't see an event from before its source entered play.
   - Repro: in `ST23-15-effect3.yaml`, replace BT25-090 with BT26-089 (hand slot 0 + the pick). Kept out of the committed line.
   - Tracker: `qa/archetype-qa/engine-gaps.md` (or the engine-gaps tracker if it is a generic deferred-event issue).
5. **BT25-090 cost reducer offered when unpayable.** Clause: BT25-090#effect#2 (seen in the ST23-15-effect0 line).
   - What ours does: the "Use Cost reduction to reduce Option-use cost?" EffectChoice is offered even though no Tamer holds a face-down card. Accepting it reduces nothing (probed: memory 0).
   - What the rules say: 15-7-3, an optional "by X" whose X can't be performed shouldn't be offered. It is a phantom decision in the RL action space.
   - Scenario: `ST23-15-effect0.yaml` (decline row is `sim_only`). Tracker: `qa/archetype-qa/engine-gaps.md`.
6. **Minor.** BT25-090#effect#1: DCGO's "suspend-only" branch (pay the suspend cost, decline the place) is collapsed into "no" in ours. That choice is legal under 15-7-5. It is rarely meaningful, but it is a missing decision. Tracker: `qa/archetype-qa/engine-gaps.md`.
7. **Doc nit.** `ST23-13.yaml` header quotes the clause as "[All Turns]". The official DB prints "[Your Turn]", which matches the implemented gate. Only the comment is stale.

### Core — ST23-06 Gekkomon, BT25-049 Armalizamon, ST23-03 Cougarmon, BT25-032, BT25-035, BT25-046

- **F1 (HIGH): an inherited <Barrier> on the attacker leaks to the defender and kills <Piercing>.** Card: BT25-032 inherited Barrier, in combination with BT25-049 inherited Piercing. Repro: BT25-049-inherited0.yaml with ST23-12 -> BT25-032 (the Lv.3 under Armalizamon). When Murasamemon (over [BT25-032, BT25-049, ST23-01]) attacks and deletes Monodramon, our engine parks `Replacement "May accept replacement: <Barrier>"` owned by PLAYER 1. Whichever way P1 answers, Monodramon is deleted, nobody's security is trashed, and Armalizamon's inherited Piercing check never happens (P1 security stays 5). The text and §16 say Barrier only protects "this Digimon" (the carrier), only on its controller's choice, and Piercing should check security. The same line with Chiropmon instead of Liollmon checks correctly. Repro file: `qa/archetype-qa/glowing-dawn-repros/repro-barrier-leak.yaml`. Tracker: `docs/RUST_ENGINE_GAPS.md` (keyword scope / replacement ownership) plus `qa/archetype-qa/engine-gaps.md`.
- **F2 (MED): ST23-13's "When effects trash cards from under this Tamer" fires once PER CARD, mid-effect.** Card: ST23-13, seen through BT25-035#effect#1. In BT25-035's trash-2 cost, the trigger fires after each card, and the first copy resolves between the two trashes, before the free digivolve. If the first copy is accepted (Tamer suspended), the second is still offered and does nothing (probed). One effect trashing two cards is one event, and a triggered effect waits until the effect finishes. The single-card cases (BT25-049-effect2, ST23-03-effect2) also resolve the trigger inside the cost payment. Scenario: BT25-035-effect1.yaml (2nd copy is a sim_only row). Tracker: `docs/RUST_ENGINE_GAPS.md` (trigger batching/timing).
- **F3 (LOW): ST23-03's optional digivolve cost-reducer is offered when the cost can't be paid.** Card: ST23-03#effect#2. "Use Cost reduction to reduce digivolution cost?" appears with no Tamer on board. Probed: accepting pays the full cost, so it's outcome-neutral but a phantom decision. `try_prompt_interactive_digivolve_cost_reducer` (game_actions/cost.rs) offers any `pay_cost_interactive` candidate without checking payability. DCGO gates the class on a Tamer with a face-down source. Scenario: ST23-03-inherited0.yaml (sim_only decline). Tracker: `qa/archetype-qa/engine-gaps.md`.
- **F4 (LOW): TriggerOrder over target-starved triggers.** Card: BT25-057 (not this group's card; it's the Barrier carrier). Its two [When Digivolving] effects are both unpayable/target-less on an empty board (no Tamer, no opponent Digimon), but they're still queued and ordered. DCGO's CanActivate gates very likely keep both off its stack (unverified, no C# here). Scenarios: ST23-03-inherited0 and BT25-035-inherited0 (sim_only). ST23-04 behaves the same way (it splits [OP][WD] into two queued clauses), so I avoided it as a carrier. Tracker: `qa/archetype-qa/engine-gaps.md`.
- **F5 (MED): BT25-035's free-digivolve pick offers cards that can't legally be digivolved into.** Card: BT25-035#effect#1. The hand pick and its "GD Digimon in hand" gate filter only on `kind: digimon + [Glowing Dawn]`, so Lv.6 ST23-09 / BT25-043 are offered over a Lv.4. Probed: picking ST23-09 trashes both face-down cards and then no digivolution happens, so the cost is paid for nothing. Expect DCGO's candidate list to differ. Scenario: BT25-035-effect1.yaml (it picks the legal BT25-041). Tracker: `qa/archetype-qa/engine-gaps.md`, plus `qa/dsl-vocab-gaps.md` if a `can_digivolve_onto: source` filter is missing.
- **F6 (HIGH): Armalizamon's Option-USE reducer also reduces a DIGIVOLVE into a DUAL card.** Card: BT25-049#effect#2. Digivolving Armalizamon into Monarchlizamon BT25-057 (a DUAL Digimon/Option) offers "Cost reduction (-3)". With a Tamer holding a face-down card, accepting it digivolves for 0 instead of 3 (memory stays 1; probed). Digivolving is not "using an Option card". The `when_any_ally_played { kind: option }` filter matches the DUAL card on the digivolve path. Repro file: `qa/archetype-qa/glowing-dawn-repros/repro-dual-option-reducer.yaml`. Tracker: `qa/archetype-qa/engine-gaps.md` (and `docs/RUST_ENGINE_GAPS.md` if the cost-target kind for DUAL cards needs a primitive).

### Tail — P-236, BT26-089 Kyo Sawashiro, BT25-088 Kyo Sawashiro, ST23-14, BT16-082

1. **A Tamer played by its own [Security] triggers its own "[All Turns] When your security stack is removed from" (offered TWICE before #708, once since).** Affects BT26-089 (#effect#1 vs #effect#2) and BT25-088 (#effect#1 vs #effect#3).
   - What ours does: after the [Security] play, ours parks the Tamer's OnLoseSecurity for the removal of that same card. BT26-089 gets two sequential 1-pending TriggerOrders. BT25-088 gets a 2-candidate `[BT25-088, BT25-088]` TriggerOrder followed by a 1-pending one. Accepting resolves the effect once (yes/yes leaves the Tamer suspended with sources), and the duplicate fizzles.
   - Likely cause (read, not fixed): `combat/mod.rs` `SecurityPhase::OnLoseSecurityDrain` enqueues OnLoseSecurity after the [Security] effect resolved, via `TriggerSource::SecurityRevealed{card}`. The revealed card and the new battle-area permanent both seem to get scanned.
   - What the rules say: 13-1-5 puts the checked card in no area, and 15-8-3-8 says removal triggers use the state at trigger time. So it arguably should not trigger at all, and certainly not twice.
   - #708's G-ENGINE-ALL-TURNS-TRIGGER-FROM-TRASH fix stopped the removed card's own effects being enqueued from the revealed card, so the duplicate is gone; the battle-area scan still sees the new Tamer.
   - Scenarios: `BT26-089-effect2.yaml` and `BT25-088-effect3.yaml`. Each now declines the one remaining prompt `sim_only` (DCGO never asks; see the prompt-shape note above). Before this update the old duplicate-answering row accepted the single remaining prompt, which is why the post-#708 run showed the Tamer suspended with two ST23-15 under it.
   - Tracker: `qa/archetype-qa/engine-gaps.md` (and `docs/RUST_ENGINE_GAPS.md` if the fix is in the trigger-source scan).
2. **ST23-14#effect#1: the trigger resolves in the middle of an effect.** When Chiropmon's [On Play] cost trashes from under ST23-14, ours asks ST23-14's OptionalSkill and Jamming pick BEFORE Chiropmon's own "return from trash" pick.
   - What the rules say: 15-8-3-2 — a trigger-type effect waits as pending until the current effect finishes. Expected order: Tamer pick -> trash-return pick -> OptionalSkill -> Jamming pick. The end state is the same either way.
   - Scenario: `ST23-14-effect1.yaml`.
   - Tracker: `docs/RUST_ENGINE_GAPS.md` (trigger fired from an activation cost isn't deferred). Expect a prompt-order divergence.
3. **Single-candidate optional permanent pick is auto-resolved after a trigger gate** (BT26-025 On Play, seen in BT26-089-effect1). Ours folds the optional `select_own_permanent` into a TriggerOrder gate and skips the pick when there's one candidate. DCGO asks the pick. This is a prompt-shape mismatch rather than a rules error (the outcome is the same). Tracker: exam tooling / `qa/archetype-qa/engine-gaps.md` (note).
4. **P-236 <Use Req.> is authored as unconditionally satisfied** (`use_requirement: {all_turns: true}`, per the YAML's reading of `UseRequirements(card, EqualsTraits("Glowing Dawn"))` as checking the card itself). The printed reminder suggests a [Glowing Dawn] card in play is required. My line keeps a [Glowing Dawn] Tamer on board, so it is legal under both readings. Deliberately not tested: using P-236 with an empty board, which ours would allow. Flag for triage: `qa/archetype-qa/engine-gaps.md`, also covering the same idiom on P-206.
5. **Exam vocabulary gap:** <Jamming> (and keyword grants in general) can't be asserted. Field rows only carry card_id/dp/suspended/sources. ST23-14#effect#1 witnesses the grant only through the target-pick row. Tracker: exam tooling.

### Tail — BT26-025 Liollmon, BT26-070 NightChiropmon, BT26-075 ScourgeChiropmon

1. **[FIXED by #708, G-ENGINE-DUAL-END-OF-TURN-WINDOW; BT26-075-effect1 now witnesses the clause, BT26-075-effect6 declines the window] <Execute> never fires on a DUAL Digimon (BT26-075#effect#1).**
   - Cause: `Game::has_end_of_turn_keywords` (`code/digimon-engine/src/game_phases.rs`) skips any permanent whose `perm.top_card().is_digimon(..)` is false. `CardSource::is_digimon` (`card_source.rs:194`) is `card_kind == Digimon`, so a `CardKind::Dual` top card fails it. (`Permanent::is_digimon` does include Dual.)
   - What happens: the Execute EndOfYourTurn body grants MayAttack, but the phase never parks in EndOfTurnAction, so the attack and the self-delete can't happen. The same skip would hit Vortex, Overclock and Engage on any DUAL Digimon.
   - Printed text / rule 16-37 says an optional end-of-turn attack is offered.
   - Probed three ways. Scenario: BT26-075-effect1.yaml (pins the bug).
   - Tracker: `qa/archetype-qa/engine-gaps.md` (or `docs/RUST_ENGINE_GAPS.md` as a generic DUAL kind-check bug).
2. **Retaliation does nothing after an effect-started battle (BT26-070#inherited#0).**
   - Line: BT25-057's "[When Digivolving] This Digimon may battle 1 of your opponent's Digimon".
   - What happens: Retaliation is offered (the Battle cause gate passes) but does nothing. `keyword_effects.rs` Retaliation finds its target through `ctx.battle_opponent_of`, which reads `Game.pending_attack`, and an effect battle has none.
   - Rule 16-12 says "deleted in battle", so any battle counts.
   - Probe: `qa/archetype-qa/glowing-dawn-repros/repro-retaliation-effect-battle.yaml`. The committed scenario uses an attack battle, where it works.
   - Tracker: `docs/RUST_ENGINE_GAPS.md`.
3. **A trigger resolves in the middle of a "by trashing 2" cost (BT26-070#effect#3 with ST23-13).**
   - What happens: `trash_bottom_face_down_sources_under_tamers count 2` re-parks per card. After the first card is trashed, ST23-13's "[Your Turn] When effects trash cards from under this Tamer" trigger is offered and resolves (suspend, +3000). Only then does the second Tamer pick come, then the trigger again (now unpayable, still offered), then the Option-use half.
   - The rules say triggers wait until the current effect finishes, and the Q&A treats the "by" cost as one action. DCGO trashes both cards, then triggers once.
   - The probe's step-14 state shows 1 of 2 cards trashed with the trigger already pending.
   - Tracker: `docs/RUST_ENGINE_GAPS.md` (trigger drain between resume frames).
4. **Armalizamon's (BT25-049) Option-use cost reducer is offered when it shouldn't be:**
   - (a) on a DIGIVOLVE into the DUAL Scourge, because its `kind: option` filter matches DUAL cards (BT26-075-effect2/3);
   - (b) with no Tamer in play, when its "by trashing" cost can't be paid (BT26-075-effect6).
   - DCGO gates it on option USE plus a Tamer holding a face-down card, so all three rows are authored `sim_only`.
   - Tracker: `qa/archetype-qa/engine-gaps.md` (BT25-049 card faithfulness). For (a), possibly `qa/dsl-vocab-gaps.md` (`kind: option` vs DUAL).
5. **Unpayable "by trashing" triggers still reach TriggerOrder** (BT25-057 De-Digivolve with no Tamer, BT26-070-inherited0). DCGO filters candidates by CanActivate. This may be outcome-neutral, but it is a phantom ordering decision. Tracker: `qa/archetype-qa/engine-gaps.md`.
6. **[FIXED by #708, G-TOOLING-EXAM-ACTOR-UNCHECKED] Exam tooling:** the sim-only lowering didn't check that a step's `actor` matches the seat that is due to decide. An `actor: 0 pass` was silently accepted in p1's breeding phase after an overspend ended P0's turn (caught and fixed in BT26-075-effect5). Tracker: exam tooling.
7. **Extractor quirks:** in BT26-070#effect#1 the <Draw 1> keyword text is swallowed (empty header). In BT26-075#effect#1, <Execute> has empty text. Both are bound as documented in each file's header.

### Tail — ST23-08, ST23-02, ST23-11, ST23-07, ST23-10

- **[FIXED by #708, G-ENGINE-BARRIER-CANDIDATE-NOT-CARRIER-GATED: the root cause was the non-carrier-scoped Barrier candidate, not the deletion batch; the asserts now hold p1.field [] / p1.trash [ST1-02, BT3-007]] ST23-02#inherited#0 — `<Barrier>` drops the opponent's battle deletion (likely engine bug).** Line `ST23-02-inherited0.yaml`: ST23-03 (4000, Liollmon inherited Barrier) vs Agumon BT3-007 (4000) — both should be deleted. Accepting Barrier keeps ST23-03 (correct) but **Agumon also stays on p1's field (4000, suspended)**. Declining Barrier on the same line deletes both (probed). Rules: 16-24 Barrier only prevents *this* Digimon's deletion; the tied/losing opponent Digimon is still deleted (battle deletion is simultaneous). Scenario asserts OUR behaviour (p1.field keeps BT3-007) and flags expected oracle divergence; text says p1.field [] / p1.trash [BT3-007, ST1-02]. Suspected site: parked optional-replacement path of the batched deletion (`code/digimon-engine/src/combat/deletion.rs`, `pending_deletion_batch_rest` — the "rest" of the kill list after a parked Replacement doesn't seem to be resumed on accept). Tracker: `docs/RUST_ENGINE_GAPS.md` (deletion-batch / replacement primitive) — probably affects every Barrier/Evade-style replacement in a mutual-destruction battle.
- **ST23-13 / ST23-14 "[Your Turn] When effects trash cards from under this Tamer" triggers resolve mid-processing (rules-suspect, prompt-order divergence risk).** In `ST23-11-effect2.yaml`: (a) Wolvermon's reducer trashes the FD card during digivolve cost calculation and ours opens the ST23-13 trigger (TriggerOrder of 1 → OptionalSkill, step 16) BEFORE the digivolution completes (memory still 3, Wolvermon still on top at step 16); (b) inside Monarchlizamon's optional half, ours opens the ST23-13 trigger between the Tamer trash and the hand pick (step 20 — i.e. in the middle of an effect). Also seen in a probe of ST23-08 #effect#2 with ST23-13 as the Tamer. Rules: triggered effects that trigger during an effect / a digivolution wait until that processing finishes, then activate (general_rule §15 timing). DCGO likely batches them after. Tracker: `docs/RUST_ENGINE_GAPS.md` (trigger-timing for triggers raised during cost/effect processing); verify against `general_rule.pdf` before filing. ST23-08-effect2/inherited0 deliberately use BT26-089 to stay clear of it.
- **Monarchlizamon OP/WD split into two triggers (prompt-shape, not outcome).** DCGO's single shared ActivateClass vs our two same-timing triggers creates a TriggerOrder the player is asked about (observed: with NO Tamer on board, a battle-area digivolve into Monarchlizamon still queued both halves as a 2-candidate TriggerOrder, whereas DCGO's half-B guard requires a Tamer with a face-down card — whether our optional gate then also parks was not probed). Outcome-neutral when the mandatory half goes first; but ordering the optional half first resolves the play BEFORE the +3000. Tracker: `qa/archetype-qa/engine-gaps.md` (card faithfulness: model as one sequenced effect, or gate the optional half on an eligible Tamer); same shape likely on ST23-04.
- Observed and consistent with rules (no action): breeding-area Digimon's static cost reducers (Liollmon −1) and [When Digivolving] don't apply in breeding; ST23-13 places at the BOTTOM of the sources (official Q&A) and "bottom face-down" trashing picks it.
- Tooling note: the sim does not check `actor:` against the turn player (a trailing `actor: 0` pass after a negative-memory turn-end lowered fine on p1's turn); fixed in ST23-08-effect2 by hand. Exam tooling could assert it.
