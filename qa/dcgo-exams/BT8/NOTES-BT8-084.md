# NOTES — Kimeramon (BT8-084)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT8-084`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `BT8/White/BT8_084.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 3 clauses — 2 scenarios authored (both lower sim-only), 1 `unreachable`.**
Book: `tm_bt8_084_pool.json` (decks `tm-kimeramon`, `quiet-opponent`).

| Clause | Text | Scenario | Status |
|---|---|---|---|
| `BT8-084#effect#0` | `[DNA Digivolve] 0 from Lv.4 + Lv.4 — Digivolve unsuspended with the 2 specified Digimon stacked on top of each other.` | — | **unreachable** — no DNA verb on either wire |
| `BT8-084#effect#1` | `[When Digivolving] You may place 1 level 5 or lower Digimon card from your trash under this Digimon as its bottom digivolution card. Then, up to 4 of your opponent's Digimon get -1000 DP for each of this Digimon's colors until the end of your opponent's next turn.` | `BT8-084-effect1.yaml` | lowers, asserts pass; **predicted `diverged`** on the second sentence (known gap) |
| `BT8-084#effect#2` | `[Your Turn] This Digimon is treated as also having the colors of its digivolution cards. While this Digimon has 4 or more colors, it gets +4000 DP.` | `BT8-084-effect2.yaml` (same line) | lowers, asserts pass; **predicted `diverged`** (clause gap-blocked in our engine) |

## `effect#0` — unreachable: no DNA verb (`G-TOOLING-EXAM-NO-DNA-VERB`)

Same measured limit as `../BT16/NOTES-BT16-077.md`, re-verified 2026-09-18
against the current harness:

- `code/tools/dcgo-harness/src/exam/scenario.rs` `STEP_VERBS` is
  `hatch, pass, move, play, digivolve, attack, main, link, select` — no DNA
  verb, and `lower::matches_intent` has no `DnaDigivolve` arm, so our engine's
  DNA action bits cannot be named by a scenario step.
- DCGO's `Assets/Scripts/Script/Harness/InputDriver.cs` (~277,
  `BuildMainPhaseAction`) refuses the DNA range outright, so even a
  hand-lowered id would abort the job.

The oracle exists (`BT8_084.cs` `AddJogressConditionClass`, two
`Levels_ForJogress(card).Contains(4)` elements, `JogressCondition(elements, 0)`);
the wire cannot reach it. Open gap: `docs/RUST_ENGINE_GAPS.md`
`G-TOOLING-EXAM-NO-DNA-VERB`.

## How `effect#1` / `effect#2` are reached WITHOUT the DNA route

The first pass of this audit had both down as DNA-only because the official-DB
bundle (`data/card_bundles/BT8-084.md`) lists "Standard digivolve cost circles —
none". **That is wrong: the card prints a circle.** The image
(`BT8-084.webp`) shows "Digivolve Cost — Lv.4 / 4" inside an ALL-COLOUR ring,
and DCGO's `CardBaseEntity/BT8/White/Digimon/BT8_084.asset` lists seven
`EvoCosts` rows (every colour, Lv.4, cost 4). So Kimeramon digivolves normally
from any Lv.4, the `[When Digivolving]` fires, and the placement gives it the
digivolution cards the `[Your Turn]` clause needs.

### DATA FINDING (not fixed here): the all-colour circle is lossy in our data

- `data/card_bundles/BT8-084.md` / `data/card_official.json`: no circle at all.
- `data/cards.json`: `evo_costs` keeps ONE row, `card_color: 0` (red) Lv.4 / 4
  — the API ingest dropped the other six colours. `data/card_overrides.json`
  has no BT8-084 entry, and `code/digimon-engine/cards/bt8/BT8-084.yaml`
  authors only the `dna_digivolve` alt path.
- Net effect, measured: our engine lets Kimeramon digivolve from a **red**
  Lv.4 only (it rides the `cards.json` row); DCGO allows any colour. The
  scenarios therefore use a red Lv.4 base (Flamedramon P-137, red/blue) —
  ground both engines share. The fix is a `card_overrides.json` entry with all
  seven colours (or the YAML equivalent); the same all-colour ring is worth
  checking on the other rainbow-circle Lv.5 DNA cards.

### The line (shared by both files)

T2 p1 Biyomon A; T4 Biyomon A attacks, flips p0's `stack[9]` = Hudiemon
BT23-101 (green/yellow Lv.4, 7000 DP, no `[Security]`, no inherited) and dies —
Hudiemon lands in p0's trash; p1 then plays Biyomon B + Phoenixmon (3 → 1 → -9);
T5 p0 on 9: Flamedramon (5), Kimeramon over it (4) → memory 0, still p0's turn.
Colours after the placement: white + red/blue + green/yellow = **5**.

### Adversarial pre-Unity review (from `BT8_084.cs`)

- `SetUpActivateClass(..., -1, false, ...)` → **no `OptionalSkill`**; the
  "you may" is the `SelectCardEffect`'s own `canNoSelect: true`.
- `SelectCardEffect` (root Trash, `maxCount` 1) is asked only when the trash
  holds a Lv.5-or-lower Digimon — it does (one candidate, still asked).
- Then, because p1 has Digimon: ONE `SelectPermanentEffect` (`maxCount`
  min(4, 2) = 2, `canNoSelect: false`, `canEndNotMax: true`). The row is
  `dcgo_only` with the single id `ST1-10`; `SelectPermanentEffect`'s harness
  intercept passes the listed picks straight through, so stopping short of
  `maxCount` is a legal "up to" answer.
- One trigger only (Flamedramon has no `[When Digivolving]`-timed text) → no
  `MultipleSkills`. P-137's `OnAttackTargetChanged` / `<Raid>` /
  `<Armor Purge>` never fire; Hudiemon BT23-101 has no `[Security]`.

### Predicted divergence — KNOWN gaps, not new findings

`BT8-084.yaml` is `PARTIAL` in `qa/qa-reports/validated_cards_dsl.json`: only
the placement is authored. Three legs are gap-blocked and deliberately not
approximated (each pinned by a tripwire test in
`tests/cards_behavioral/bt8/bt8_084.rs`, each logged in `qa/dsl-vocab-gaps.md`):

| Leg | Gap id | Expected oracle reading |
|---|---|---|
| DP-minus, scaled by colours | `G-DSL-SOURCE-STACK-UNION-COLOR-COUNT` | `p1.field` Phoenixmon: DCGO 7000, ours 12000 |
| additive colour treatment | `G-ENGINE-ADDITIVE-COLOR-TREATMENT` | (not projected; feeds the two DP readings) |
| 4+ colours → +4000 DP | `G-DSL-OWN-STACK-COLOR-COUNT-GTE` | `p0.field` Kimeramon: DCGO 12000, ours 8000 |

So both files are expected to come back `diverged` on the LAST row only (the
placement row pairs cleanly: three colours, 8000 on both sides). That is the
gaps showing up as data, which is what the verdict class is for; the `assert:`
blocks name only fields both engines agree on so sim-only stays green. When the
gaps close, replace the `dcgo_only` row with a shared `targets:` pick and
re-run.

## Oracle pass (2026-09-19, drain of 2026-09-18)

Both lines completed on DCGO and diverged on the LAST row exactly as predicted:
`effect1`: `p1.field[1].dp` (Phoenixmon) ours 12000 / DCGO 7000 (the DP-minus
sentence, `G-DSL-SOURCE-STACK-UNION-COLOR-COUNT`); `effect2`: `p0.field[0].dp`
(Kimeramon) ours 8000 / DCGO 12000 (`G-DSL-OWN-STACK-COLOR-COUNT-GTE` +
`G-ENGINE-ADDITIVE-COLOR-TREATMENT`). Both rows share one step, so each file
reports both fields. Known, logged gaps -- no new finding. `effect#0` stored as
`unreachable` (no DNA verb). **3 clauses: 0 confirmed, 2 diverged, 1 unreachable.**

## 2026-09-19 — BT8-084#effect#1 triage: gaps closed, CONFIRMED

The step-16 divergence (Phoenixmon ours 12000 / dcgo 7000; Kimeramon ours 8000
/ dcgo 12000) was the three logged gap legs, i.e. **our bug** (unimplemented
printed text), not a DCGO quirk: the printed card + official Q&A (additive
color treatment) and `BT8_084.cs` (ChangeCardColorClass skipping flipped
sources; `minusDP = 1000 * TopCard.CardColors.Count` after the placement;
ChangeSelfDPStaticEffect(4000) on `CardColors.Count >= 4`) agree.

- Engine: `Permanent::synth_identity` now reads `ModifierType::AddColor`
  additively (payload `None` = non-flipped digivolution-card colors).
- DSL: `per: source_rules_color_count` (carrier's synthesized colors); the
  permanent-subject `self_color_count_gte` no longer re-checks printed colors.
- Harness resolver: a battle-area "up to N" pick left open after the
  payload's picks is closed with PASS (the parked `CountCappedPermanentsStep`
  identifies it), so DCGO's one-row `canEndNotMax` stop lowers.
- Both scenarios' DP pick is now a shared `targets: [opp.field.1]` step
  (same DCGO wire, ST1-10). Oracle re-diff vs the preserved sidecar
  `20260918T131946Z_6b61efab...state.jsonl`: CLEAN 17/17.

## 2026-09-20 — `effect#0` is REACHABLE: the `dna:` verb landed

`G-TOOLING-EXAM-NO-DNA-VERB` is **closed** (close-out job
`three-musketeers-2`). Both halves of the block are gone:

- **Our wire.** `code/tools/dcgo-harness/src/exam/scenario.rs` gains a `dna:`
  step kind (`STEP_VERBS` now reads `hatch, pass, move, play, digivolve, dna,
  attack, main, link, select`) and `lower::matches_intent` a `DnaDigivolve`
  arm. `do: { dna: { card: <ID>, materials: [field.N, field.M] } }`.
- **DCGO.** `InputDriver.BuildMainPhaseAction` gains a `DNA_DIGIVOLVE` arm that
  resolves the two material identities to `fieldCardFrames` ids and returns a
  jogress `PlayCardAction` (base-repo commit `fc67f9ae6`; player
  `D:/dcgo-build/scripted-v16`, preflight GO, action-space digest unchanged at
  `711d23bf`). The pair rides a new `dna_materials` field — deliberately NOT
  `select_card_ids`, which would make the step read as a selection answer and
  abort at an action-id prompt.

The materials ride the VERB rather than a following `select:` step (the one
place this departs from the `link:` precedent) because the two wires disagree
about how many decisions the declaration contains: ours is three (the
`DNA_DIGIVOLVE` bit, then two `SelectionKind::Material` prompts over raw
own-battle-area indices), DCGO's is one `PlayCardAction`. So the step lowers to
ONE wire row carrying the action id plus both identities, and a `SimOnlySelect`
row that answers our two prompts. The engine half of that is a `targets:`
resolution for the two DNA material prompts
(`selection::is_dna_material_prompt` gating a raw-slot candidate in
`runners/selection_resolve.rs` — those ids are raw field indices, unlike every
other `Material` prompt).

**Scenario authored: `BT8-084-effect0.yaml`.** Both materials are Flamedramon
P-137 (red/blue Lv.4, cost 5) rather than the deck's other Lv.4, Hudiemon
BT23-101, whose `[On Play]` would add a prompt pair that measures nothing about
this clause. The line plays one Flamedramon per turn (T3, T5: 3 - 5 = -2 hands
the turn back each time) and declares the DNA on T7 for 0, so the turn stays
p0's and "digivolve UNSUSPENDED" is readable. The `[When Digivolving]` fires on
this route like any other and is deliberately SILENT: p0's trash is empty (no
trash candidate) and p1 has no Digimon (no DP pick), so the file reads the DNA
clause alone. Sim-only 2026-09-20: lowers 15 steps
(`DnaDeclaration { action_id: 63, material_ids: ["P-137", "P-137"] }` +
`SimOnlySelect`), 14 hand-authored assertions green.

`effect#0` therefore moves `unreachable` -> `unmeasured`: it needs an oracle
pass against `scripted-v16`, not a tooling change. The card's denominator is
now **3 clauses: 2 confirmed, 0 diverged, 0 unreachable, 1 unmeasured.**
