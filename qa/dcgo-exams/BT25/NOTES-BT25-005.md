# NOTES — Pagumon (BT25-005)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-005`):
**1 clause** — `inherited#0`.
DCGO script: `BT25/Black/BT25_005.cs` exists — the card is **not** `unavailable`.
The clause has a scenario; nothing is `unreachable`.

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT25-005#inherited#0` | `[Your Turn] [Once Per Turn] When [Three Musketeers] trait cards are placed in this Digimon's digivolution cards, it may digivolve into a Digimon card with [Three Musketeers] in its text or the [TS] trait in the hand with the cost reduced by 2.` | `BT25-005-inherited0.yaml` (book `tm_bt25_005_pool.json`, deck `tm-pagumon-bt25-005-egg` = the shared 50 with BT25-005 ×4 as the egg deck) | lowers, 7/7 asserts pass |

## The placing effect, and why the base is RED

The timing fires only when an **effect** adds a digivolution card (DCGO
passes `cardEffect: null` for normal / DNA digivolution and the engine
deliberately does not fire the batch window for those — see the gap-stage
result in the campaign ledger). The placer is LadyDevimon BT25-083's
`[On Play]` cost (P-180 Bind Red Trigger, a `[Three Musketeers]` **trait**
card, from hand under any own Digimon): its prompt rows are exactly those of
the committed `BT25-083-effect1.yaml` (`generic_int` zone menu `dcgo_only`,
shared `SelectHandEffect`, shared `SelectPermanentEffect`), and
`AddDigivolutionCardsBottom(..., activateClass)` carries the effect into
the hashtable that `CanTriggerOnAddDigivolutionCard` reads.

The host is a **ToyAgumon EX7-008** stack (RED Lv.3) over Pagumon, reached
through ToyAgumon's alt-circle "Lv.2 w/[Three Musketeers] in text: Cost 0"
(the oracle-authored `EX7-008-effect0` shape). Red is the point: the
clause's digivolve target, BlackGatomon BT25-082, then has **exactly one**
open route — its alt-circle "Lv.3 w/[Three Musketeers] in text or w/[TS]
trait: Cost 2" (ToyAgumon prints "[Three Musketeers]" in its effect text;
`BT25_082.cs` checks `TopCard.HasText("Three Musketeers")`, our alt path
`effect_text_contains`) — reduced to 0. A purple base (Sparrowmon) would
open the purple 3-cost circle beside the alt 2 and put a two-cost
`SelectCountEffect` / `EffectChoice` inside the effect-initiated digivolve,
a shape no line in this campaign has oracle-confirmed. With one route,
`PlayCardClass.PlayCard` asks no count row and the only decisions on the
clause are the two the printed text names.

## Adversarial pre-Unity review — prompt sequence re-derived from `BT25_005.cs`

1. **Trigger shape.** `SetUpActivateClass(CanActivate, ActivateCoroutine,
   1 /* OPT */, TRUE /* isOptional */, ...)`. The lone stacked
   OnAddDigivolutionCards effect is asked as an `OptionalSkill`
   (`ICardEffect.cs:1064`, `IsOptional` → `OptionalSkill.SelectOptional`;
   at bundle length 1 no `MultipleSkills`). Ours: `optional: true` +
   `outer_prompt: true` → a `Replacement`-kind yes/no — the 1:1 row of the
   surface table. Authored as a SHARED `select: { yes: true }` /
   `expect: OptionalSkill`; the lowered wire shows one plain row
   (`bool_answer: Some(true)`, no fold).
2. **Trigger gate.** `IsExistOnBattleAreaDigimon(card)` resolves a
   digivolution card through `PermanentOfThisCard()` (`CardSource.cs:335`,
   `cardSources.Contains(this)`), so the buried Pagumon qualifies;
   `cardEffectCondition: null` (any effect); `cardCondition:
   HasThreeMusketeersTraits` (`CardTraits.Contains("Three Musketeers")`)
   — P-180 qualifies; `IsOwnerTurn`. Ours mirrors all three
   (`event_host_permanent_is_source`, `event_added_card_any {trait_has}`,
   `active_when: your_turn`) plus a `count_gte` "legal target in hand"
   gate that DCGO carries INSIDE the body (`HasMatchConditionOwnersHand`).
   The line keeps BlackGatomon in hand so both engines reach the same
   prompt; a line with **no** legal target would be a prompt-existence
   finding (DCGO asks the yes/no and then does nothing; we do not fire) —
   not exercised here, recorded so the next author does not trip on it.
3. **Body.** `DigivolveIntoHandOrTrashCard(payCost: true,
   reduceCostTuple: (2, null), isHand: true, isOptional: default true)`:
   registers a `ChangeDigivolutionCostPlayerEffect(-2)` until the fixed
   cost is calculated, then ONE `SelectHandEffect` (`maxCount 1`,
   `canNoSelect: true`) over Digimon cards that pass `HasText("Three
   Musketeers") || HasTSTraits` **and** `CanPlayCardTargetFrame` (level
   +1, an open route, affordable under `MaxMemoryCost` = |10 − memory|,
   which is 14 at −4 — cost 0 is always payable). Candidates in the
   authored hand: BlackGatomon only (Gazimon ×4 are Lv.3; BT25-058 is
   Lv.6; BT24-081 is a Lv.7 DNA card). Ours: `select_hand` filtered by
   `can_digivolve_from_source` + the same text/trait disjunction — the
   behavioral test `bt25_005_may_digivolve_into_ts_card_in_hand_with_cost_reduced_by_2`
   pins the candidate set. SHARED `select: { cards: [BT25-082] }`.
4. **After the digivolve.** BlackGatomon's `[When Digivolving]` is gated
   in DCGO by `AdditionalActivateCondition` (a TM-text **Tamer in hand**
   && ≤1 Tamers on the field). The stack keeps every Asuna (BT24-088 /
   BT25-092) out of the hand, so DCGO activates nothing. Our
   `BT25-082.yaml` carries only the "≤1 Tamers" half of that gate, yet the
   line lowered with **no** sim-side row after the digivolve: with zero
   `select_hand` candidates the optional trigger voids itself before
   parking. So no `sim_only` compensation row was needed — but if
   `BT25-082.yaml` ever gains a candidate-independent outer prompt, this
   line stops lowering and needs one (the `NOTES-BT25-083.md` family).
5. **Order.** DCGO stacks the trigger (`StackSkillInfos`) and resolves it
   after LadyDevimon's `<Draw 1>`; our batch window flushes at the
   outermost `drain_effect_queue`, i.e. also after the `[On Play]`
   completes. Both pre-decision snapshots at the `OptionalSkill` row
   therefore already hold the drawn card.
6. **Memory.** LadyDevimon 3 → −4; the clause's digivolve costs 0; the
   turn ends at −4 → the trailing step is P1's breeding prompt (empty
   breeding area). ToyAgumon's inherited "[Your Turn] +2000 DP" is off on
   P1's turn, so the asserted BlackGatomon DP is the printed 6000.

## Not measured by this line

- The **cost-reduction arithmetic** is only observable as "cost 2 became
  0"; a target whose reduced cost is still > 0 (e.g. a 3-cost route → 1)
  would make the −2 visible in the memory delta. Deliberately not chosen:
  every such target in this pool opens a second route and the count
  prompt (above).
- **Declining** the outer yes/no and the rule 15-14-1-5 OPT refund
  (`bt25_005_declining_the_outer_prompt_keeps_the_opt_use`) — a second
  placement in the same turn would need a second placer; not authored.
- An **opponent's** effect placing the card on your turn (DCGO
  `cardEffectCondition: null`) — the quiet-opponent pool has no such
  effect.
