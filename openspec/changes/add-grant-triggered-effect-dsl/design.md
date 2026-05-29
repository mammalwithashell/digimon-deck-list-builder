## Context

`G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` is a hybrid engine+DSL gap documented at length in `qa/dsl-vocab-gaps.md` (and in the `EX1-068.yaml` header). The Python engine solved it with `permanent.grant_temp_effect(effect, expiry_turn)` + `clear_expired_effects()`; the Rust engine has the modifier-registry + expiry-tick substrate (`ModifierRegistry` carries per-permanent typed modifiers with `Expiry`) but NO typed granted-trigger slot and NO `CompiledStep::GrantTriggeredEffect`.

Judge-quiz consumers and what each needs beyond the bare grant:
- **Q2** (Ice Wall! EX1-068) — grant "[When Attacking] lose 2 memory" to all opponent Digimon, expiry end-of-opponents-turn. Then `<Progress>` (already implemented) makes Medusamon immune to the granted effect while attacking. Needs: grant + the granted effect being an "opponent's effect" that Progress can exclude.
- **Q16** (Lilithmon EX6-057) — grant "[End of Your Turn] Delete this Digimon" to opponent's Paildramon. Then `<Partition>` (cause-filter already present, keyword_effects.rs:839) must NOT fire because the delete is the grantee's OWN effect. Needs: grant + cause attribution = grantee.
- **Q17** (same grant onto Magnamon X BT16-102) — Magnamon X's `[When Digivolving]` immunity removes the granted effect. Needs: grant + the granted slot being subject to the immunity machinery (`permanent_is_unaffected_by_effect`, already present).

So the engine machinery the consumers lean on (Progress exclusion, `<Partition>` cause-filter, immunity controller-filter) is already correct; this change adds the GRANT itself plus the attribution that ties the granted effect to the grantee.

## Goals / Non-Goals

**Goals**
- A typed granted-trigger modifier slot with turn-scoped expiry, consulted when a timing fires on the carrier.
- A `grant_triggered_effect` DSL step (target selector + `when` + inline `process` + `expiry`) lowered against the granted permanent.
- Cause/controller attribution: a granted effect is the grantee's own effect (Q16 `<Partition>`, Q17 immunity, Q2 Progress-excludes-opponent-effect).
- Author Ice Wall! + Lilithmon; pin Q2/Q16/Q17.

**Non-Goals**
- One-shot or permanent grants; a general "grant any effect" framework. Scope = triggered effects with turn-scoped expiry.
- Authoring the ~20 other consumer cards.
- The `fix-judge-quiz-engine-gaps` deletion/routing/defer work.

## Decisions

### D1 — Inline body over named templates (dsl-vocab-gaps Option A)
The gap doc offered (A) an inline `process:` body compiled to a `CompiledTriggeredClause` and (B) a registry of named granted-effect templates. Choose **A**: a `grant_triggered_effect` step whose `process:` is lowered to a `CompiledTriggeredClause` and stored in the granted slot. Rationale: inline is self-describing per card, matches how the rest of the DSL authors effects, and avoids a parallel template registry to maintain. The ~20 sibling cards then express their grant inline too.

```yaml
- grant_triggered_effect:
    target: { of: opponent, zone: [battle_area], kind: digimon }
    when: when_attacking
    process:
      - lose_memory: 2        # affects the GRANTED permanent's controller
    expiry: end_of_opponents_turn
```

### D2 — Typed `ModifierType::GrantedTrigger` slot, reusing expiry-tick
Add a `ModifierPayload::GrantedTrigger { clause: CompiledTriggeredClause }` carried by a `ModifierEntry` with the existing `Expiry`. `clear_expired`/turn-tick infrastructure already drains expiring modifiers, so the turn-scoped expiry is free. Rationale: no new lifetime machinery; the registry already does per-permanent typed modifiers with expiry.

### D3 — Dispatch consults granted slots at trigger-fire
When the engine fires a timing on a permanent (the `enqueue_from_permanent` / trigger-fire path), it SHALL also enumerate that permanent's `GrantedTrigger` slots whose clause `when` matches and enqueue their bodies. Rationale: granted triggers must fire alongside the carrier's printed triggers. Open question: exact hook point so granted triggers participate in the same ordering/queue as printed ones (see Open Questions).

### D4 — Cause/controller attribution = grantee
A body resolved from a `GrantedTrigger` slot SHALL run with `effect_source_player = carrier.player` (the grantee), and any deletion it causes SHALL be attributed `ReplacementCause::OwnEffect` relative to the carrier. This is the load-bearing decision for Q16 (`<Partition>` skips OwnEffect → does not fire) and is consistent with Q17 (the immune carrier doesn't carry the slot) and Q2 (the granted effect is the granter's/opponent's effect from the ATTACKER's perspective — Progress excludes opponent-sourced effects on the attacker). Note the asymmetry: for Q2 the granted "lose memory" is evaluated relative to the attacking Medusamon as an opponent effect (Progress excludes it); for Q16 the granted "delete this" is the carrier's own effect (Partition doesn't fire). Both follow from "the granted effect belongs to the carrier, and Progress/Partition/immunity each evaluate controller relative to their own subject" — pin both directions with tests.

### D5 — Expiry semantics: end-of-opponents-turn
Ice Wall's "until the end of their next turn" maps to `Expiry::EndOfOpponentsTurn` (the granter's opponent's next turn end). Lilithmon's "[End of Your Turn]" granted clause fires at the grantee-controller's turn end (the clause's own `when`), and the SLOT itself persists until consumed/expired per the card. Confirm the two expiry flavors (slot-expiry vs the granted clause's firing timing) are modeled distinctly.

### D6 — TDD, card-by-card
Write the failing behavioral test first (per card), then the primitive, then author the YAML clause. Order: Ice Wall! (Q2 — simplest, `when_attacking` + lose_memory, leans on existing Progress) → Lilithmon (Q16/Q17 — `[EoT]` self-delete + cause attribution + immunity removal).

## Risks / Open Questions

- **Dispatch hook point (open).** Where exactly granted triggers are enumerated so they share ordering with printed triggers (and with `[Once Per Turn]` accounting if a granted clause is OPT). Spike the `enqueue_from_permanent` path.
- **Cause attribution direction (open).** Q2 needs the granted effect treated as the OPPONENT's effect from the attacker's perspective (Progress excludes it); Q16 needs it treated as the carrier's OWN effect (Partition doesn't fire). Confirm both fall out of "controller = carrier" + each keyword evaluating relative to its own subject. Pin with tests in both directions before authoring.
- **Snapshot semantics (DCGO parity).** DCGO's `UntilOpponentTurnEndEffects` foreach runs ONCE at resolution and snapshots the eligible opponent-Digimon set; a Digimon played LATER does not carry the grant. The `grant_triggered_effect` target selector must install on the snapshot set only — not re-evaluate for later-played Digimon. Avoid the over-fire approximation the gap doc warns about.
- **Interaction with `fix-judge-quiz-engine-gaps`.** Independent subsystems; no ordering dependency. If both land, run the combined judge-quiz suite as the gate.
