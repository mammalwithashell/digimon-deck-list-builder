# Chronomon + TS Mervamon: exam notes (2026-10-04)

Chronomon (5.7% of the BT26 field) and TS Mervamon (4.8%) are fully playable but had under 15% of their
clauses oracle-verified (`qa/qa-reports/meta-coverage/latest.md`). This pass authors one exam scenario per
printed clause of their non-shared cards, verified `--sim-only`, ready for the local DCGO oracle pass.

## Scope

- **Chronomon:** BT25-008 Coronamon, BT26-078 Cherubimon, BT26-016 Chronomon: Holy Mode, BT26-060 Chronomon:
  Destroy Mode, BT26-085 Giant Slayer, BT26-009 Hyokomon, BT26-011 Buraimon, BT26-015 Butenmon, BT26-001 Yokomon,
  BT26-087 Toya Kuga, BT26-096 Kosuke Misono, BT26-021 Gekomon, BT26-101 Cross Arts, BT26-032 Ceresmon (DUAL).
- **TS Mervamon:** BT26-067 Wizardmon, BT26-088 Hiroko Sagisaka, plus every other card in its BT26 lists that is
  neither in the shared TS Jupitermon core (authored in PR #707: BT26-033/-073/-081/-029/-103/-083/-090/-092,
  BT24-034/-101/-031/-102/-083/-041, BT25-022/-025, P-213) nor in the Chronomon lists: BT25-039 Sirenmon,
  BT25-020 Marsmon, BT25-034 Angemon, BT24-040 Venusmon, BT24-063 Locomon, BT24-051 Merukimon, BT25-001 Tokomon.
- **Skipped:** BT25-028 (scenarios already on main, `qa/dcgo-exams/BT25/BT25-028-*.yaml`); BT8-097 (in both decks'
  lists, so not Mervamon-only; out of this brief).

## State

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-008 BT26-078 BT26-016
BT26-060 BT26-085 BT26-009 BT26-011 BT26-015 BT26-001 BT26-087 BT26-096 BT26-021 BT26-101 BT26-032 BT26-067
BT26-088 BT25-039 BT25-020 BT25-034 BT24-040 BT24-063 BT25-001 BT24-051`): **96 clauses over 23 cards**
(81 effect, 15 inherited, all from the official Bandai bundle, 0 image-required).
`tools.clause_coverage.exam_binding.bind` over the 93 files: 0 orphan scenarios, 0 invalidated clause ids.

| | Clauses |
|---|---|
| Scenario authored, passes `--sim-only` (and emits a DCGO job) | **93** |
| `unreachable`: printed "ー", no inherited effect | **3** (`BT26-016#inherited#0`, `BT26-060#inherited#0`, `BT26-085#inherited#0`) |
| of the 93, witness only part of the clause (PARTIAL, named in the table) | 38 |
| of the 93, deliberately assert behaviour we believed wrong | **0** — the bugs behind `BT24-051-effect1`, `BT26-060-effect1/4`, `BT25-034-effect2`, `BT25-039-effect1`, `BT24-051-effect2` and `BT25-020-effect0` were fixed in this PR (2026-10-05) and those scenarios re-pinned to the corrected behaviour |
| of the 93, open rules question for the oracle | 1: `BT26-085-effect3` |
| `confirmed` | **0**: no oracle pass has been run |

**All 96 clauses are still `unmeasured` in the verdict sense.** A scenario that lowers is not a measurement:
every `assert:` block was written from OUR engine's result, not backfilled from DCGO, and is provisional until the
oracle pass. Every scenario header says so. No verdict files were written.

**No DCGO C# and no `general_rule.pdf` were available in this (cloud) container** — neither the worktree
placeholder nor a base-repo checkout exists. Expected DCGO prompt shapes come from the "DCGO crosscheck" notes in
each card's YAML header, sibling exam files and `docs/DCGO_EXAM.md`. Treat each as a guess for the oracle's abort
message to confirm or refute. A per-card `.cs` existence check
(`$BASE/DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<ID_underscored>.cs`) is still owed before anything is read
as `unavailable`.

## Deck book

`qa/dcgo-exams/BT26/chronomon_mervamon_pool.json` resolves every scenario in this set. Every deck is 50 main,
≤4 copies, 4 eggs (DCGO's `IsValidDeckData`).

| Deck | What it is |
|---|---|
| `bt26-chronomon` | Real list: 2nd place, dcg-nexus `d096f0ce-83b6-4b35-bb09-28024b8f5ede` (2026-09-09). Eggs 4× BT26-001. Carries BT26-032 and BT26-101. |
| `bt26-chronomon-b` | Real list: 13th place, dcg-nexus `7e9108cc-8ad5-49cd-a748-ed88a290b51c` (2026-09-05). Carries BT26-021 ×4, BT26-096 ×4, BT26-088 ×2. |
| `bt26-ts-mervamon` | Real list: 9th place, dcg-nexus `973eaa87-31a7-4a21-86c9-dfb9a85e15cc` (2026-09-07). Eggs 4× BT25-001. |
| `tm-quiet-red` | The shared quiet ST1 red opponent (copied from `bt26_dats_pool.json`). |
| `cm-chronomon-iso` | `bt26-chronomon` −BT9-103 +BT25-022, −BT25-092 +BT25-039, −BT26-033 +BT25-034; eggs → 4× BT24-003 (yellow Lv.2 [TS]) to isolate the "Lv.2 w/[TS]" conditions from the red circles. |
| `cm-chronomon-offcolor` | `bt26-chronomon` −BT26-090 +BT24-059 (black/blue Lv.5 [TS]), −BT25-100 +BT26-021 (blue/purple Lv.4 [TS]): off-colour bases for Butenmon / Holy Mode. |
| `cm-chronomon-ceres2` | `bt26-chronomon` −BT9-103 +BT25-059 (the only play-cost-12 [Ceresmon]) for BT26-032's special digivolve. |
| `cm-ts-mervamon-ceres` | `bt26-ts-mervamon` −BT24-051 +BT25-059 (Sirenmon's {Security} play target). |
| `cm-ts-mervamon-cresc` | `bt26-ts-mervamon` −BT24-051 +BT25-026 Crescemon (a quiet blue Lv.5 [TS] base for Marsmon). |
| `cm-opp-blocker` | `tm-quiet-red` −BT4-014 +BT14-011 Monochromon (vanilla `<Blocker>`) for Gekomon's attack-target lock. |

## Running the oracle pass (local: Unity, base-repo DCGO)

The scenarios live in three set directories next to other cards' scenarios that use other books, so collect the
file list first.

```bash
BASE="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"   # see docs/DCGO_HARNESS.md
BOOK=qa/dcgo-exams/BT26/chronomon_mervamon_pool.json
IDS="BT25-008 BT26-078 BT26-016 BT26-060 BT26-085 BT26-009 BT26-011 BT26-015 BT26-001 BT26-087 BT26-096 \
     BT26-021 BT26-101 BT26-032 BT26-067 BT26-088 BT25-039 BT25-020 BT25-034 BT24-040 BT24-063 BT25-001 BT24-051"
CM=$(mktemp -d)
for c in $IDS; do cp qa/dcgo-exams/${c%%-*}/$c-*.yaml "$CM"/ 2>/dev/null; done   # 93 files

# 0. Clause text for the verdict fingerprints.
PYTHONPATH=code python -m tools.clause_coverage.extract --quiet --out "$CM/clauses.json" --card-ids $IDS

# 1. Re-check sim-only and emit one scripted DCGO job per scenario (93 jobs).
cargo run -p dcgo-harness -- --root "$ROOT" exam --scenario "$CM" --sim-only \
    --cards-json data/cards.json --decks "$BOOK" --emit-job "$ROOT/jobs"
#   expect: scenarios seen 93 / lowered 93 / run 93 / diffed 0 / failed 0

# 2. Drain. Stop Play before queueing (a running harness claims jobs instantly).
cargo run -p dcgo-harness -- --root "$ROOT" node status        # must report GO
cargo run -p dcgo-harness -- --root "$ROOT" enable
cargo run -p dcgo-harness -- --root "$ROOT" watch --build <player-dir>   # or: Play in Unity + `status`
cargo run -p dcgo-harness -- --root "$ROOT" disable

# 3. Diff each scenario against the sidecar its job wrote, and record verdicts.
#    --sidecar takes a directory holding one <scenario-stem>.state.jsonl per scenario.
cargo run -p dcgo-harness -- --root "$ROOT" exam --scenario "$CM" \
    --sidecar <dir-of-state.jsonl> --cards-json data/cards.json \
    --verdicts --clause-text-json "$CM/clauses.json" --all-diffs
```

Then record the three "ー" clauses as `unreachable` (reason: no inherited effect printed), regenerate
`PYTHONPATH=code python -m tools.meta_coverage`, and backfill only CLEAN lines (backfill refuses diverged ones).
The DCGO build must be at or after `scripted-v16`; no line here uses `dna:` or `link:`.

## Scenario ↔ clause table

"PARTIAL" names the part of the clause the line does not witness. Paths are under `qa/dcgo-exams/`.

| Clause | Text (short) | Scenario | Witness / note |
|---|---|---|---|
| **BT25-008 Coronamon** |||
| `#effect#0` | [Digivolve] Lv.2 w/[TS]: Cost 0 | `BT25/BT25-008-effect0.yaml` | variant `cm-chronomon-iso`: yellow Tsunomon BT24-003 egg fails both red/blue circles, so legality isolates the condition; memory stays 0. PARTIAL: cost indistinguishable (circles also 0) |
| `#effect#1` | [When Moving] (trash up to 2 [Iliad]/[TS] → draw per card) | `BT25/BT25-008-effect1.yaml` | T3 move trashes 2 (Hyokomon, Aegiomon), draws 2; `dcgo_only` [A,B] pick + two `sim_only` single picks bridge the cardinality gap |
| `#effect#2` | [On Play] same body | `BT25/BT25-008-effect2.yaml` | T1 hard-play trashes 1, draws 1, declines the eligible 2nd pick ("up to") |
| `#inherited#0` | [Your Turn] +2000 DP | `BT25/BT25-008-inherited0.yaml` | Buraimon over Coronamon: 7000 on P0's turn, 5000 on P1's |
| **BT26-009 Hyokomon** |||
| `#effect#0` | [Digivolve] Lv.2 w/[TS]: Cost 0 | `BT26/BT26-009-effect0.yaml` | `cm-chronomon-iso` yellow egg fails the red circle; memory stays 0. PARTIAL: cost indistinguishable |
| `#effect#1` | [Start of Your Main Phase] trash Chronomon/Shaman → Draw 1 + 1 memory | `BT26/BT26-009-effect1.yaml` | memory 3→4, BT26-016 trashed, BT26-060 drawn |
| `#inherited#0` | [When Attacking][OPT] `<Draw 1>` | `BT26/BT26-009-inherited0.yaml` | hand 4→5, nothing returned (also the hand<6 negative of #1). PARTIAL: OPT not exercised |
| `#inherited#1` | Then, at hand ≥6 return 1 to deck bottom | `BT26/BT26-009-inherited1.yaml` | hand 7 after draw → BT8-097 leaves hand, not in trash; hand 6 |
| **BT26-011 Buraimon** |||
| `#effect#0` | [Digivolve] Lv.3 w/[TS]: Cost 2 | `BT26/BT26-011-effect0.yaml` | `cm-chronomon-iso`: onto blue Lunamon BT25-022 (fails red circle); 3→1. PARTIAL: cost indistinguishable |
| `#effect#1` | `<Raid>` | `BT26/BT26-011-effect1.yaml` | security attack redirected to Biyomon (3000, highest); deleted; P1 security stays 5 (Raid fold) |
| `#effect#2` | [On Play][When Digivolving] trash Chronomon/Shaman → `<Draw 2>` | `BT26/BT26-011-effect2.yaml` | both timings in one line. PARTIAL: Shaman-only filter branch and decline not exercised |
| `#inherited#0` | inherited `<Raid>` | `BT26/BT26-011-inherited0.yaml` | carrier Sirenmon BT25-039 (no own Raid) redirects to Biyomon (Raid fold) |
| **BT26-001 Yokomon** |||
| `#inherited#0` | [Your Turn][OPT] own effect adds to deck → may digivolve into [Chronomon]-text from hand, cost −1 | `BT26/BT26-001-inherited0.yaml` | Toya Kuga SOMP bottom-decks Hyokomon → Yokomon (under Coronamon) digivolves into Buraimon for 2−1=1 (4→3). PARTIAL: OPT, [Your Turn] gate, opponent-deck adds not exercised |
| **BT25-001 Tokomon** |||
| `#inherited#0` | [When Attacking][OPT] if [TS], `<Draw 1>` | `BT25/BT25-001-inherited0.yaml` | `bt26-ts-mervamon`; Coronamon over Tokomon attacks, hand +1. PARTIAL: non-[TS] negative and OPT not exercised |
| **BT26-015 Butenmon** |||
| `#effect#0` | [Digivolve] Lv.4 w/[TS]: Cost 3 | `BT26/BT26-015-effect0.yaml` | variant `cm-chronomon-offcolor`: onto blue/purple Gekomon BT26-021 (fails red/yellow circles); 3→0. Fully isolated |
| `#effect#1` | OP/WD −4000; then return 1 trash card → delete ≤5000 | `BT26/BT26-015-effect1.yaml` | Birdramon 5000→1000; BT24-034 returned from trash; Biyomon deleted. PARTIAL: decline-the-return branch and [On Play] arm not witnessed |
| `#effect#2` | [Your Turn][OPT] own effects add to decks → +3000 and attack | `BT26/BT26-015-effect2.yaml` | the WD bottom-deck triggers it; 10000 at the pick, attacks; P1 security 5→4 |
| `#inherited#0` | [All Turns][OPT] add to decks → this [Chronomon]-text Digimon may unsuspend | `BT26/BT26-015-inherited0.yaml` | Holy Mode over Hyokomon→Buraimon→Butenmon attacks; Hyokomon's inherited bottom-decks a card; Holy Mode unsuspends |
| **BT26-016 Chronomon: Holy Mode** |||
| `#effect#0` | [Digivolve] Lv.5 w/[TS]: Cost 3 | `BT26/BT26-016-effect0.yaml` | `cm-chronomon-offcolor`: onto black/blue BT24-059 (fails circles); 3→0. Fully isolated |
| `#effect#1` | `<Piercing>` | `BT26/BT26-016-effect1.yaml` | deletes suspended Dracomon, then checks: P1 security 5→4 |
| `#effect#2` | `<Engage>` | `BT26/BT26-016-effect2.yaml` | main passed; attacks in the end-of-turn window; P1 security 5→4 |
| `#effect#3` | OP/WD/WA [OPT] delete ≤ own DP; then return 3 trash cards → `<Recovery +1>` | `BT26/BT26-016-effect3.yaml` | WD deletes Dracomon, returns 3 own trash cards, security 4→5. PARTIAL: OP/WA arms, the cross-trash split (scriptable since the fixes, not yet added) and the partial return not witnessed |
| `#effect#4` | [All Turns][OPT] would leave → top security to deck bottom, doesn't leave | `BT26/BT26-016-effect4.yaml` | 12000 tie with Phoenixmon: Holy Mode stays, security 5→4; the next flip is the old 2nd card |
| `#inherited#0` | "ー" | **unreachable** | no inherited effect printed |
| **BT26-087 Toya Kuga** |||
| `#effect#0` | [Start of Your Main Phase] return a [TS] Digimon from trash to deck bottom → +1 memory; after, may return [Giant Slayer] to hand | `BT26/BT26-087-effect0.yaml` | 3→4; BT26-009 leaves trash; BT26-085 to hand. PARTIAL: "no return → no after" branch not witnessed |
| `#effect#1` | [On Play] trash 1 [TS] from hand → `<Draw 2>` | `BT26/BT26-087-effect1.yaml` | BT26-092 trashed, hand net +1 |
| `#effect#2` | [Security] play free | `BT26/BT26-087-effect2.yaml` | played from P0's security (5→4), memory unchanged; its [On Play] fires on P1's turn |
| **BT26-060 Chronomon: Destroy Mode** |||
| `#effect#0` | [Digivolve] Lv.6 w/[Chronomon] in text / [Giant Slayer]: Cost 5 | `BT26/BT26-060-effect0.yaml` | onto Giant Slayer (white, no level — no circle applies); 3→−2. PARTIAL: the Lv.6 [Chronomon]-text branch only appears in effect4 (over Holy Mode, where the red circle also applies) |
| `#effect#1` | `<Security A. +1>` | `BT26/BT26-060-effect1.yaml` | one attack checks 2 (5→3) — fixed in this PR (was 3, `<Security A. +1>` double count) |
| `#effect#2` | `<Reboot>` | `BT26/BT26-060-effect2.yaml` | suspended after attacking; unsuspended at P1's breeding step |
| `#effect#3` | `<Blocker>` | `BT26/BT26-060-effect3.yaml` | blocks Agumon on T6; Agumon trashed; P0 security stays 5 |
| `#effect#4` | `<Succession (Lv.6 w/[Chronomon] in name)>` | `BT26/BT26-060-effect4.yaml` | over Holy Mode: gained [When Digivolving] deletes Phoenixmon; gained `<Engage>` makes an end-of-turn attack. Security 5→2 (inherits the #1 double count) |
| `#effect#5` | [OP][WD] return top 5 stacked cards of 3 opp Digimon to deck top | `BT26/BT26-060-effect5.yaml` | Kokatorimon taken off Monodramon (single-card Phoenixmon untouched); P1's next draw is BT1-014 (deck top). PARTIAL: [On Play], "3 of" cap, 5-card cap, owner ordering of 2+ cards |
| `#effect#6` | [All Turns][OPT] own effects add to decks → may delete 1 opp Digimon | `BT26/BT26-060-effect6.yaml` | fires off its own WD return; deletes Phoenixmon. PARTIAL: decline and OPT not exercised |
| `#inherited#0` | "ー" | **unreachable** | no inherited effect printed |
| **BT26-085 Giant Slayer** |||
| `#effect#0` | `<Collision>` | `BT26/BT26-085-effect0.yaml` | Monodramon forced to block (no decline), deleted; P1 security stays 5 |
| `#effect#1` | `<Reboot>` | `BT26/BT26-085-effect1.yaml` | same line: suspended at 12, unsuspended at P1's breeding step |
| `#effect#2` | `<Blocker>` | `BT26/BT26-085-effect2.yaml` | blocks Monodramon; P0 security stays 5 |
| `#effect#3` | [On Play] opp effects can't reduce DP / trash stacked cards until opp turn ends | `BT26/BT26-085-effect3.yaml` | P1 (`bt26-chronomon` mirror) Butenmon −4000: GS still 14000 on T4. **RULES QUESTION**: GS reads 10000 on T5 (the −4000 reappears when the immunity lapses; consistent with the BT19-089 Q&A, the oracle decides). PARTIAL: stack-trash half not witnessed |
| `#effect#4` | [All Turns] would leave → digivolve into Destroy Mode from hand/trash free, doesn't leave | `BT26/BT26-085-effect4.yaml` | Gaia Force targets GS; Destroy Mode from hand, GS becomes its source, memory unchanged. PARTIAL: from-trash branch |
| `#effect#5` | Assembly −5: 5 different-level [Chronomon]-text/[Shaman] cards | `BT26/BT26-085-effect5.yaml` | 5 trash cards Lv.3–7 declared; 3→−4 (7 paid); all 5 stacked under GS |
| `#inherited#0` | "ー" | **unreachable** | no inherited effect printed |
| **BT26-078 Cherubimon** |||
| `#effect#0` | [Digivolve] Lv.5 w/[TS]: Cost 5 | `BT26/BT26-078-effect0.yaml` | onto red/yellow Butenmon BT26-015 (fails purple/green circles); 3→−2 |
| `#effect#1` | `{Trash}` zone marker for #2 | `BT26/BT26-078-effect1.yaml` | contrast line: same play with Cherubimon in HAND → no trigger, no `<Execute>` window, P1 security stays 5 |
| `#effect#2` | [Trash][Your Turn] own [Chronomon]/[Titan] Digimon played + opp ≥5 memory → bottom-deck self → `<Rush>` + `<Execute>` | `BT26/BT26-078-effect2.yaml` | Cherubimon trashed by Hyokomon's SOMP; Buraimon played 0→−5; Cherubimon leaves trash; Buraimon makes an end-of-turn `<Execute>` attack (5→4) and is deleted |
| `#effect#3` | [OP][WD] delete self → play cost ≤12 [Chronomon]/[Titan] from trash free | `BT26/BT26-078-effect3.yaml` | WD arm: Cherubimon deleted, Butenmon (qualifies via its inherited [Chronomon] text) replayed free. PARTIAL: [On Play] arm not exercised |
| **BT26-096 Kosuke Misono** |||
| `#effect#0` | [Start of Your Turn] memory ≤2 → set to 3 | `BT26/BT26-096-effect0.yaml` | P0 would start T3 at 2; reads 3 at the breeding prompt |
| `#effect#1` | [Main] bottom-deck self → play [Chronomon] Digimon / [TS] Tamer from hand/trash at −2 | `BT26/BT26-096-effect1.yaml` | Kosuke leaves; Hyokomon played from hand for 1 (3→2). PARTIAL: trash zone and [TS] Tamer branch not exercised |
| `#effect#2` | [Security] play free | `BT26/BT26-096-effect2.yaml` | Agumon's attack flips Kosuke; played to P0's field (5→4) |
| **BT26-021 Gekomon** |||
| `#effect#0` | [Digivolve] Lv.3 w/[TS]: Cost 2 | `BT26/BT26-021-effect0.yaml` | onto red Hyokomon (fails blue/purple circles); 3→1 |
| `#effect#1` | [OP][WD] 1 own [TS] Digimon's attack target can't change | `BT26/BT26-021-effect1.yaml` | variant `cm-opp-blocker`: locked Gekomon attacks past an unsuspended Monochromon `<Blocker>`; no block prompt; 5→4 (unlocked, our engine parks P1's block pick). PARTIAL: [On Play] arm, Raid and effect redirects not probed |
| `#effect#2` | [Main][OPT] play [TS] Tamer from trash at −2 | `BT26/BT26-021-effect2.yaml` | Kosuke played from trash for 2 (2→0). PARTIAL: OPT not asserted |
| `#inherited#0` | [All Turns][OPT] a Digimon attacks → trash 1 hand card → trash bottom 2 sources of 1 opp Digimon | `BT26/BT26-021-inherited0.yaml` | Butenmon over Gekomon attacks; Coronamon trashed; Vermilimon's sources [BT1-009, BT1-014] → []. PARTIAL: ally/opponent attacks not walked |
| **BT26-101 Cross Arts** |||
| `#effect#0` | `<Use Req. ([TS])>` (ignore colour) | `BT26/BT26-101-effect0.yaml` | white Option used with only a red [TS] Digimon; same line covers #1's no-Dan/Kanan case and the unsuspend branch |
| `#effect#1` | [Main] Dan/Kanan Tamer → [TS] `<Blocker>` +3000; then delete-by-DP or unsuspend | `BT26/BT26-101-effect1.yaml` | Kanan in play: Hyokomon 2000→5000 deletes Dracomon (4000); on P1's turn still 5000 and blocks. PARTIAL: [Dan Yuki] name and later-played re-evaluation not walked |
| `#effect#2` | [Security] may play cost ≤4 [TS] card from hand/trash free | `BT26/BT26-101-effect2.yaml` | flipped; Hyokomon played from hand (5→4). PARTIAL: trash branch, cost-4 boundary not walked |
| **BT26-032 Ceresmon (DUAL)** |||
| `#effect#0` | [Digivolve] Play cost 12 [Ceresmon]: Cost 2 | `BT26/BT26-032-effect0.yaml` | variant `cm-chronomon-ceres2`: onto Lv.6 BT25-059 (Lv.5 circles can't apply); 3→1 |
| `#effect#1` | `<Alliance>` | `BT26/BT26-032-effect1.yaml` | suspends Hyokomon; one attack checks 2 (5→3). PARTIAL: +DP not observable after the attack |
| `#effect#2` | `<Succession ([Ceresmon])>` | `BT26/BT26-032-effect2.yaml` | BT25-059's [When Digivolving] offered via Ceresmon; its [All Turns] fires on attack (Biyomon −3000, deleted) |
| `#effect#3` | [WD] opp suspended −5000; by suspending 1, play/use [Vegetation]/[TS] at −5 | `BT26/BT26-032-effect3.yaml` | suspended Monodramon → 0 DP, deleted; Shota played for 0. PARTIAL: already-suspended target, not-your-turn branch, (Rule) [Vegetation] not witnessed. Targets `opp.field.1` to dodge the AnyField resolver bug (findings) |
| `#effect#4` | DUAL `<Use Req. ([TS])>` | `BT26/BT26-032-effect4.yaml` | only a black [TS] Tamer on board; green Option used, 9→4. PARTIAL: negative not scripted |
| `#effect#5` | DUAL [Main] suspend 2; 3 can't unsuspend | `BT26/BT26-032-effect5.yaml` | both P1 Digimon stay suspended through P1's unsuspend phase. PARTIAL: cap of 3 clamped to 2 |
| `#effect#6` | DUAL Rule `<Arts Digivolve>` | `BT26/BT26-032-effect6.yaml` | the Option digivolves onto BT26-015 for free (memory 0, trash empty) |
| **BT26-067 Wizardmon** |||
| `#effect#0` | [Digivolve] Lv.3 w/[TS]: Cost 2 | `BT26/BT26-067-effect0.yaml` | onto yellow Elecmon BT24-031 in breeding (fails purple/red circles); 3→1 |
| `#effect#1` | [On Play][When Digivolving] (timing marker; body = Draw 1 + trash 1) | `BT26/BT26-067-effect1.yaml` | [On Play] arm: draws, trashes BT26-088. PARTIAL: [When Digivolving] arm not witnessed (effect0 digivolves in breeding, where WD doesn't activate) |
| `#effect#2` | "and trash 1" + [End of Your Turn] bottom-deck self → play red/blue [Iliad] from trash at −4 | `BT26/BT26-067-effect2.yaml` | extractor merged both; measures the [End of Your Turn]: Wizardmon #1 to deck bottom, #2 played from trash for 0 |
| `#inherited#0` | `<Retaliation>` | `BT26/BT26-067-inherited0.yaml` | BT25-039 over Wizardmon dies in battle to ST1-10 → ST1-10 deleted (`trigger: Retaliation`) |
| **BT26-088 Hiroko Sagisaka** |||
| `#effect#0` | [Start of Your Main Phase] opp has a Digimon → +1 memory | `BT26/BT26-088-effect0.yaml` | 3→4 (negative seen in effect1: stays 3) |
| `#effect#1` | [Your Turn] [Boss]/[TS] play −1, −2 with no Digimon | `BT26/BT26-088-effect1.yaml` | Wizardmon played for 2 with no Digimon (3→1), Hiroko suspended. PARTIAL: −1 branch not witnessed |
| `#effect#2` | [Security] play free | `BT26/BT26-088-effect2.yaml` | Biyomon attacks; Hiroko flips and is played; security 5→4 |
| **BT25-039 Sirenmon** |||
| `#effect#0` | [Digivolve] Lv.4 w/[TS]: Cost 3 | `BT25/BT25-039-effect0.yaml` | onto purple/red Wizardmon BT26-067 (fails yellow/green circles); 3→0 (cost 3, not the circle's 4) |
| `#effect#1` | {Security}[End of Your Turn] play [Ceresmon] at −7; may place self as its bottom source | `BT25/BT25-039-effect1.yaml` | variant `cm-ts-mervamon-ceres`. Face-up Sirenmon (via its own #3) → Ceresmon BT25-059 played for 5 (−3→−8), Sirenmon placed under it, security 6→5. Ceresmon's [On Play] is asked and declined on both sides (was eaten sim-side by a lowering trailing-PASS bug, fixed in this PR) |
| `#effect#2` | [All Turns] other [Shaman]/[Iliad] would leave (not by your effects) → delete self, they don't leave | `BT25/BT25-039-effect2.yaml` | P1's security Gaia Force ST1-16 targets Coronamon; Sirenmon deleted instead, Coronamon stays |
| `#effect#3` | [On Deletion] may place self face up as bottom security | `BT25/BT25-039-effect3.yaml` | same line accepting it: trash → security 5→6. Face-up/bottom not projectable (effect1 relies on face-up) |
| `#inherited#0` | [Opp Turn][OPT] opp attacks → may redirect to 1 of your suspended Digimon | `BT25/BT25-039-inherited0.yaml` | Marsmon over Sirenmon (suspended); Birdramon's security attack redirected; P0 security stays 5, Birdramon deleted |
| **BT25-020 Marsmon** |||
| `#effect#0` | [Digivolve] Lv.5 w/[TS]: Cost 3 | `BT25/BT25-020-effect0.yaml` | variant `cm-ts-mervamon-cresc`: onto blue Crescemon BT25-026 (fails circles); 3→0; also shows [When Digivolving] +3000 |
| `#effect#1` | play cost −5 if a 13000+ DP Digimon exists | `BT25/BT25-020-effect1.yaml` | own 15000 Mervamon on board; Marsmon costs 7: 3→−4 (not −9) |
| `#effect#2` | [OP][WD][WA] +3000; then 1 of your Digimon may battle 1 opp Digimon | `BT25/BT25-020-effect2.yaml` | [On Play]: 17000 Marsmon battles and deletes Biyomon, no security check. PARTIAL: [When Attacking] arm not witnessed ([When Digivolving] in effect0) |
| `#effect#3` | [All Turns][OPT] your [TS] Digimon wins a battle → trash opp top security | `BT25/BT25-020-effect3.yaml` | winner is Mervamon (board-wide reading); P1 security 5→4, BT1-009 to trash. PARTIAL: OPT not exercised |
| **BT25-034 Angemon** |||
| `#effect#0` | [Digivolve] Lv.3 w/[TS]: Cost 2 | `BT25/BT25-034-effect0.yaml` | onto red Coronamon (fails the yellow circle); 3→1 |
| `#effect#1` | effects trash this from security → may play Lv.≤4 [Angel]/[Iliad] from hand free | `BT25/BT25-034-effect1.yaml` | Aegiomon's `<Barrier>` trashes Angemon from security; Coronamon played free |
| `#effect#2` | `<Ascension>` | `BT25/BT25-034-effect2.yaml` | deleted by security Phoenixmon; accepted → security 5→6. One `<Ascension>` (the duplicate trigger was fixed in this PR) |
| `#inherited#0` | inherited `<Barrier>` | `BT25/BT25-034-inherited0.yaml` | Sirenmon over Angemon loses to Phoenixmon; top security trashed; Sirenmon survives |
| **BT24-040 Venusmon** |||
| `#effect#0` | [Digivolve] Lv.5 w/[TS]: Cost 3 | `BT24/BT24-040-effect0.yaml` | onto purple/red BT26-073 (fails yellow/blue circles); 3→0 (a circle costs 4) |
| `#effect#1` | play cost −5 at ≤3 security | `BT24/BT24-040-effect1.yaml` | P1 attacks twice → security 3; hard-play 3→−4 (7, not 12) |
| `#effect#2` | OP/WD: trash all sources of 1; 2 can't suspend / activate [When Digivolving] | `BT24/BT24-040-effect2.yaml` | Birdramon's source trashed; next turn Garudamon onto the locked Birdramon: its WD +3000 does not fire. PARTIAL: "can't suspend" seen only indirectly (BT24-063-inherited0) |
| `#effect#3` | [All Turns][OPT] [TS] would leave → place another sourceless Digimon as bottom security | `BT24/BT24-040-effect3.yaml` | Coronamon #1 loses to security Phoenixmon; Coronamon #2 to bottom security (5→6), #1 survives. PARTIAL: OPT, "other than by your effects", plural not walked |
| **BT24-063 Locomon** |||
| `#effect#0` | [Digivolve] Lv.4 w/[TS]: Cost 3 | `BT24/BT24-063-effect0.yaml` | onto purple/red Wizardmon (outside the black circle); 3→0. PARTIAL: circle also costs 3 (colour-isolated only) |
| `#effect#1` | `<Collision>` | `BT24/BT24-063-effect1.yaml` | Agumon forced to block (non-declinable P1 prompt), deleted; P1 security stays 5. PARTIAL: one opposing Digimon only |
| `#effect#2` | OP/WD: reveal 3, play cost ≤5 Machine/Cyborg/TS free, rest top or bottom | `BT24/BT24-063-effect2.yaml` | Coronamon played free; rest to top (T3 draw = revealed BT24-041). PARTIAL: bottom branch only in no-pick lines |
| `#inherited#0` | inherited `<Collision>` | `BT24/BT24-063-inherited0.yaml` | Venusmon over Locomon attacks; only unlocked Agumon must block (candidates `[ST1-03]`) |
| **BT24-051 Merukimon** |||
| `#effect#0` | [Digivolve] Lv.5 w/[Beastkin]/[TS]: Cost 3 | `BT24/BT24-051-effect0.yaml` | onto purple/red BT26-073; 3→0. PARTIAL: [TS] arm only (no [Beastkin] Lv.5 in deck) |
| `#effect#1` | play cost −5 if 3+ Digimon | `BT24/BT24-051-effect1.yaml` | 1 own + 2 opponent Digimon: costs 7 (7→0), turn continues — fixed in this PR (was 12, filter lacked `owner: any`) |
| `#effect#2` | OP/WD: suspend 2 opp; 1 own may +5000 and attack an opp Digimon | `BT24/BT24-051-effect2.yaml` | WD arm: Agumon + Monodramon suspended; Merukimon 17000 attacks and deletes Agumon; `<Piercing>` 5→3. PARTIAL: Tamers not walked |
| `#effect#3` | WD/WA [OPT] 1 own Digimon may unsuspend | `BT24/BT24-051-effect3.yaml` | WD unsuspends BT26-073; a second attack gets no prompt (OPT shared across timings). PARTIAL: WA arm offered/declined only |
| `#effect#4` | [Your Turn] [Iliad] Digimon gain `<Rush>` + `<Piercing>` | `BT24/BT24-051-effect4.yaml` | Coronamon attacks the turn it is played (5→4); Merukimon deletes suspended Monodramon then checks (4→2) |

## Expected aborts and divergences — triage these first

Read the printed card and `general_rule.pdf` before calling any of these an engine bug; DCGO is below the PDF.

**Fixed in this PR (2026-10-05).** The four engine/card bugs these lines used to pin (Security A. double count,
Merukimon's owner filter, duplicate `<Ascension>`, a declined folded [Once Per Turn] spending the use) and the two
exam-tooling bugs (AnyField side resolution, trailing PASS eating a fresh pick) are fixed, and the affected scenarios
now assert the corrected behaviour. Two of those corrections are judgement calls the oracle should confirm:

| Scenario | Ours now | What to look for | Tracker |
|---|---|---|---|
| `BT24/BT24-051-effect2.yaml` | declining the [When Attacking] unsuspend does not spend the [Once Per Turn]; the queued [When Digivolving] arm is then offered | DCGO's `isOptional` OptionalSkill "no" should leave `maxCountPerTurn` unspent | §G-ENGINE-OPT-DECLINE-CONSUMES-SIBLING-TIMING (RESOLVED) |
| `BT26/BT26-085-effect3.yaml` | Giant Slayer reads 10000 on T5 | Rules question: our live "can't be reduced" reading matches the BT19-089 Q&A; if DCGO reads 14000, the install-time-refusal reading wins | "Closed during re-verification" (G-ENGINE-IMMUNE-DP-MINUS-LAZY) |

**Prompt-shape guesses most likely to abort** (fix the row from DCGO's abort message, don't guess again):

- **Multi-pick cardinality.** Ours asks N single picks where DCGO asks one prompt with `maxCount`: Coronamon's
  trash-up-to-2 (`BT25-008-effect1/2`, bridged with `dcgo_only` + `sim_only` rows; ours draws between the two
  trashes, so DCGO's 2nd candidate set can differ), Venusmon's two locks and Merukimon's "suspend 2"
  (`BT24-040-effect1/2`, `BT24-063-inherited0`, `BT24-051-effect1/2/4`), Ceresmon DUAL lock (2-of-2 — DCGO may
  auto-select: `BT26-032-effect5/6`).
- **Optional gates before a pick.** DCGO may ask `OptionalSkill` before the pick (→ `optional_gate_fold`):
  Hyokomon [Start of Your Main Phase] (pilot `BT26-009-effect1` and every line that declines it), Sirenmon
  {Security} pick and inherited redirect, Angemon security-trash play, Gekomon inherited (ours is a Replacement
  gate answered `sim_only`; DCGO notes say skippable-not-optional), Yokomon inherited digivolve (DCGO may also ask a
  `SelectCountEffect` for the cost route).
- **Zone menus.** DCGO's hand/trash/"Don't" `generic_int` menu may appear where only the hand qualifies:
  Kosuke `BT26-096-effect1`, Cross Arts `BT26-101-effect2`, Giant Slayer replacement `BT26-085-effect4`, Hyokomon
  inherited return `BT26-009-inherited1` (ours a `UnionZone`, not sim-asserted).
- **Choice values that are guesses:** Cross Arts branch `generic_int` (0 = delete, 1 = unsuspend) in
  `BT26-101-effect0/1`; Destroy Mode cost route `value: 5` in `BT26-060-effect4`; BT26-073's cost menu
  `generic_int value: 3` (sibling idiom) in the BT24-040/051 lines; Sirenmon placement expected as `generic_bool`;
  Locomon's top/bottom remainder (EX7-044 idiom: `generic_bool` true = top, first-clicked = top) in
  `BT24-063-effect2` — if T3 draws Mervamon rather than BT24-041 the order convention is reversed.
- **Holy Mode return (`BT26-016-effect3`):** DCGO asks an area choice (own / enemy trash / Cancel) before the
  multi-pick — expect to add a `dcgo_only` row before step 16.
- **Trigger attribution.** Succession-gained triggers are named by their source card on our side (BT25-059 under
  Ceresmon, BT26-016 under Destroy Mode); DCGO may name them by the top card (→ `ordinal:`). Retaliation in
  `BT26-067-inherited0` is named by BT26-067, not the top card BT25-039.
- **Forced blocks.** `<Collision>` (`BT24-063-effect1`, `-inherited0`, `BT26-085-effect0`): we park a
  non-declinable P1 `SelectPermanentEffect`; DCGO may force a lone blocker silently. Gekomon's lock
  (`BT26-021-effect1`): if DCGO still opens the blocker timing despite `CanNotSwitchAttackTargetEffect`, that's a
  real divergence.
- **Raid** lines use the documented fold (`expect: OptionalSkill` over our pick, BT20-020 precedent); our engine
  merges an inherited `<Raid>` into the attack-target list where DCGO asks it separately after the declaration —
  lines are chosen so Raid has no target except where it is the clause.
- **Face-down {Security}.** DCGO's Sirenmon uses `IsExistInSecurity(card, false)` (face-down also fires); ours
  requires face-up (rule 15-14-5, G-ENGINE-SECURITY-ICON-REQUIRES-FACE-UP). The line uses a face-up Sirenmon, so
  both should agree.
- **Two-Digimon slot addressing.** DCGO seats permanents centre-out, so `field.N` can name a different Digimon on
  a two-Digimon board (`../BT25/NOTES-BT25-083.md`). Lines attack / digivolve only from an unambiguous slot, except
  `BT26-085-effect5` (two identical Coronamon).
- **Wizardmon #2 at end of turn** (`BT26-067-effect2`): enters during the end-of-turn window and ours does not
  offer its own [End of Your Turn]; DCGO might ask a second `OptionalSkill` (triage against §6-6).

## Tooling findings (exam harness)

- **AnyField answers resolved to the wrong side** — RESOLVED in this PR (`18eb1e7e`): AnyField picks resolve the
  absolute id only, and an OwnField/OppField reference naming the other side is an error.
  §G-TOOLING-EXAM-ANYFIELD-SIDE-IMPLICIT-FIRST.
- **Trailing PASS ate a fresh optional battle-area "up to N" pick** that the row's own answer caused (Ceresmon's
  [On Play] after Sirenmon's placement) — RESOLVED in this PR (`18eb1e7e`).
  §G-TOOLING-EXAM-TRAILING-PASS-EATS-FRESH-FIELD-MULTIPICK.
- Fixed on main since authoring: the trailing PASS on a `CountCappedMultiSelect` (`e2a69509`; Holy Mode's
  cross-trash split is now scriptable) and the unchecked `actor:` (`9f408863`).
- **clause_coverage split on BT26-067** (`#effect#1` empty marker, `#effect#2` = "and trash 1" + the whole
  [End of Your Turn]) — logged in `qa/dsl-vocab-gaps.md`.
- **CI note:** `.github/workflows/dcgo-exam-sim.yml` runs the whole `qa/dcgo-exams/` tree against only
  `EX12/toho_pool.json`, so scenarios that use any other book (these included) do not lower there; that check
  already exits non-zero on main (228 never-lowered scenarios at `56728ca1`). Run this set with its own book as above.
