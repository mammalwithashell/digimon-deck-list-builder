# NOTES — Scopemon (BT21-071): clauses no scripted line can reach

Denominator (`clause_coverage.extract --card-ids BT21-071`): **5 clauses** —
`effect#0`, `effect#1`, `effect#2`, `inherited#0`, `inherited#1`.

Scenarios authored: `BT21-071-effect0.yaml` (alt-digivolve condition; needs
the book `tm_bt21_071_pool.json` for its black Digi-Egg) and
`BT21-071-effect1.yaml` ([On Play] tuck → gain 1 memory).

The remaining three clauses are the card's **DigiLink** half and are
`unreachable` for one measured reason: **the exam's scripted-step vocabulary
has no verb that declares a link, on either engine's side of the wire.**

| Clause id | Text | Verdict | Measured reason |
|---|---|---|---|
| `BT21-071#inherited#0` | `<Link>` [Appmon] trait: Cost 2 (Plug this card from the hand or battle area sideways into the specified Digimon…) | `unreachable` | No link verb (see below). |
| `BT21-071#inherited#1` | [When Linking] `<Draw 2>` and trash 2 cards in your hand. | `unreachable` | Fires only after a link is declared; no link verb. |
| `BT21-071#effect#2` | +3000 DP (the link-box DP grant) | `unreachable` | Observable only on a host while Scopemon is linked; no link verb. |

## Why "no link verb" is a measured limit, not a guess

1. **Scenario vocabulary.** `code/tools/dcgo-harness/src/exam/scenario.rs`
   line 208 fixes the `do:` verbs to exactly
   `hatch, pass, move, play, digivolve, attack, main, select`. There is no
   `link`.
2. **`play:` cannot stand in for a hand-link.** `lower.rs::matches_intent`
   accepts a `play:` step only for `ActionKind::Play`. Our engine exposes
   no hand-initiated *Digimon* link action at all: `action/mask.rs` emits
   link only on the per-permanent `FIELD_EFFECT` range at sub-slot
   `FIELD_EFFECT_SLOT_FOR_LINK = 3` (`space.rs:123`, `mask.rs:386-394`;
   `game_actions/link.rs::activate_field_link`). The only hand-side link
   path (`game_actions/mod.rs` ~1417, `OptionSubtype::Link`) is for
   *Option*-type Link cards, which Scopemon is not.
3. **`main:` cannot stand in for a field-link either.** `main: { on:
   field.N }` lowers to whatever `FIELD_EFFECT` bit the slot offers, but
   DCGO's harness refuses it: `Assets/Scripts/Script/Harness/InputDriver.cs`
   lines 380-386 map the `FIELD_EFFECT` range to
   `ActivatePermanentAction(slot, 0)` and reject any sub-slot other than
   `FIELD_EFFECT_SLOT_FOR_MAIN`. A lowered link id would abort the job on
   the DCGO side before any prompt is asked, so no oracle answer is
   obtainable through that route.

Until the scenario format gains a link verb (and DCGO's `InputDriver`
accepts the link sub-slot), these three clauses stay `unreachable` with this
reason — never a silent skip, never `confirmed`. This is a tooling gap in the
exam surface, not a finding about Scopemon's behaviour.

DCGO does script the link half (`BT21_071.cs`: `CardEffectFactory.LinkEffect`,
`AddSelfLinkConditionStaticEffect(HasAppmonTraits, linkCost: 2)`, and the
`WhenLinked` ActivateClass with `isOptional: false` → mandatory `DrawClass(2)`
then `SelectHandEffect(mode: Discard, maxCount: min(2, hand), canNoSelect:
false)`), so the card is **not** `unavailable`; the oracle exists, the wire
cannot reach it.
