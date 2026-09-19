# NOTES — Neptunemon (BT24-030)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT24-030`):
**5 clauses** — `effect#0` (`[Digivolve] Lv.5 w/[Aqua]/[Sea Animal] in any
trait or w/[TS] trait: Cost 3`), `effect#1` (play-cost −5 vs 2+ opposing
Digimon), `effect#2` ([On Play] [When Digivolving] fewest-digivolution-cards
deck-bottom bounce), `effect#3` ([All Turns] [Once Per Turn] "When this Digimon
suspends, it may unsuspend"), `effect#4` ([All Turns] would-leave protection
"by suspending this Digimon").
DCGO script: `BT24/Blue/BT24_030.cs` exists — the card is **not** `unavailable`.
**Status: 5 clauses, 5 scenarios, 0 unreachable.** Book:
`qa/dcgo-exams/BT24/tm_bt24_030_pool.json` (decks `tm-neptune`, `tm-quiet-red`).

| Clause | Scenario | Sim-only | Expected at the oracle |
|---|---|---|---|
| `effect#0` | `BT24-030-effect0.yaml` | lowers, asserts pass | clean |
| `effect#1` | `BT24-030-effect1.yaml` | lowers, asserts pass | clean |
| `effect#2` | `BT24-030-effect2.yaml` (RE-AUTHORED) | lowers, asserts pass | clean |
| `effect#3` | `BT24-030-effect3.yaml` | lowers, asserts pass | **predicted divergence — OUR ENGINE** (below) |
| `effect#4` | `BT24-030-effect4.yaml` (REPAIRED) | lowers, asserts pass | clean |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

All five were on disk, uncommitted, and referenced this NOTES file, which did
not exist. Three needed work.

### `effect#4` — the "duplicate" second prompt was a DIFFERENT clause (it would have aborted the oracle line)

The draft answered our second yes/no `sim_only`, reading it as "two decisions
for one printed *by suspending*". It is not: paying the protection cost
SUSPENDS Neptunemon, which triggers its own `effect#3` ("When this Digimon
suspends, it may unsuspend"). DCGO does the same — `SuspendPermanentsClass.Tap()`
stacks `EffectTiming.OnTappedAnyone` (`CardController.cs` ~5952) and the
"All Turns - OPT" ActivateClass is `SetUpActivateClass(…, 1, TRUE, …)`, i.e. a
second `OptionalSkill` for P0. `cards_behavioral`
`bt24_030_protects_matching_digimon_from_opponent_effects_by_suspending` pins
the same order on our side. The row is now SHARED and DECLINED (so the paid
cost stays visible: Neptunemon suspended). The draft also asserted at step 18,
after P0's unsuspend phase — the witness had already been erased and the file
FAILED its own assert on the current engine; it now reads step 16 (P1's
trailing pass, still turn 6).

### `effect#2` — re-authored so "fewest" discriminates

The draft fielded two hard-played Digimon, both with zero digivolution cards,
so "all opposing Digimon" and "the fewest group" were the same set. The new
line fields Birdramon-over-Biyomon (one digivolution card) beside a hard-played
Fugamon (zero): only Fugamon goes to the bottom of the deck and Birdramon stays
with its source. Mandatory, no pick, and no ordering prompt for a one-card
group (`DeckBouncePeremanentAndProcessAccordingToResult`). P1 digivolves while
it has exactly ONE Digimon, so `field.0` is slot-safe.

### Stale deck notes

`effect1` / `effect3` / `effect4` described the book as carrying Vademon
BT24-061; it carries Aegiochusmon BT24-014 (the `effect0` header explains why:
`data/cards.json` ingests BT24-061 with no DP, a card-data gap). Comments fixed.

## ENGINE FINDING — attack-declaration suspension fires no `OnSuspend` (`effect#3`)

Logged as `G-ENGINE-ATTACK-SUSPENSION-NO-ONSUSPEND` in
`docs/RUST_ENGINE_GAPS.md`. Verified by reading the code, and measured at
lowering (`BT24-030-effect3.yaml` step 15: "select answered no live prompt"):

- `Game::commit_attack_declaration` → `suspend_and_count_attack`
  (`code/digimon-engine/src/combat/mod.rs` ~3792) writes `perm.is_suspended =
  true` directly. The only site that enqueues `EffectTiming::OnSuspend` is
  `Game::suspend` / `suspend_with_cause` (`game/suspend.rs` ~96). So a Digimon
  that suspends BY ATTACKING never triggers "when this Digimon suspends".
- DCGO suspends the attacker through `SuspendPermanentsClass(…).Tap()`
  (`AttackProcess.cs:166`), which stacks `OnTappedAnyone`. Declaring an attack
  IS suspending the attacker; the rules carve nothing out.
- Impact is pool-wide, not Neptunemon-only: every `when: on_suspend` card that
  should see its own (or an ally's) attack declaration. For Neptunemon it is
  the card's main line of play (attack, unsuspend, attack again).

The `effect#3` line is the honest one (attack, accept the unsuspend) and is
PREDICTED to read `diverged` at the oracle on `p0.field[0].suspended`. The
effect-suspension path of the same clause is crossed — and expected clean — in
`effect#4`'s line (declined there). Not fixed here (exam stage: triage and
report); the fix gate needs a failing-then-passing test that attacks with an
`on_suspend` carrier.

## Card-YAML observations (reported for the card-fix gate, not exam findings)

- `code/digimon-engine/cards/bt24/BT24-030.yaml` authors `color: [blue,
  purple]` and only the `Blue Lv.5 / 4` circle. The official DB bundle and
  `data/card_overrides.json` print **Blue / Black**, circles `Blue Lv.5 / 4`
  and `Black Lv.5 / 4`. `effect#0` isolates the special condition on a
  RED/YELLOW base, so neither printed circle is in play on that line.
- `return_to_deck … include_sources: true` on the [On Play] bounce: DCGO's deck
  bounce sends a returned Digimon's digivolution cards to the TRASH. The
  re-authored `effect#2` bounces a stack with no sources (Fugamon), so this is
  un-probed; a line bouncing a sourced stack would measure it.

## Prompt shapes (re-derived from the C#)

- `effect#0`: `AddSelfDigivolutionRequirementStaticEffect(IsLevel5 &&
  (HasAquaTraits || HasTSTraits), 3, ignoreDigivolutionRequirement: false)` —
  one route on a red/yellow [TS] base, no `SelectCountEffect`.
- `effect#1`: `MandatorySelfPlayCostReduction(5, card, Condition)` — no prompt.
- `effect#2`: `SetUpActivateClass(…, -1, FALSE, …)` — mandatory, no pick.
- `effect#3`: `OnTappedAnyone`, `SetUpActivateClass(…, 1, TRUE, …)` →
  `OptionalSkill`.
- `effect#4`: `WhenRemoveField`, `SetUpActivateClass(…, -1, TRUE, …)` gated on
  `CanActivateSuspendCostEffect` → `OptionalSkill`; then the suspend, which
  stacks `effect#3` (above).

## RESOLVED 2026-09-19 — `effect#3` attack-suspension OnSuspend (ENGINE FIX)

Triage class: **our bug** (general_rule.pdf 11-2-1 "suspend their Digimon ...
and make an attack declaration"; 11-1-4; DCGO `AttackProcess.cs:166`
`SuspendPermanentsClass.Tap()` → `OnTappedAnyone`, the timing `BT24_030.cs:127`
keys the unsuspend on). Fixed in `combat/mod.rs` + `game/suspend.rs` (see
`G-ENGINE-ATTACK-SUSPENSION-NO-ONSUSPEND`, RESOLVED). Oracle re-diff against the
preserved sidecar `20260918T123758Z_b45375dd` is CLEAN 18/18 → **confirmed**.

**Separate finding (not fixed, printed-data):** `cards/bt24/BT24-030.yaml`
authors `color: [blue, purple]` and only a Blue Lv.5 digivolve circle; the
official bundle (`data/card_bundles/BT24-030.md`) prints **Blue/Black** with
Blue Lv.5 AND Black Lv.5 cost-4 circles. Needs its own fix + guard test.
