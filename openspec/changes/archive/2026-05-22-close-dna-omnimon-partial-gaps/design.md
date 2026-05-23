## Context

DNA Omnimon's Rust DSL coverage is already production-authored for the full 64-card pool, with no live `raw_rust` escapes. The remaining incompleteness is concentrated in two partial cards:

- `BT17-102 Greymon` omits its dynamic "[All Turns] has all names of level 3 and lower cards in its digivolution cards" clause. The current YAML uses a Koromon-name source predicate as a local proxy for one related DP clause.
- `BT23-096 Comet Hammer` omits its `[Your Turn]` Delay trigger that fires when one of the player's `[CS]` trait Digimon attacks.

DCGO is useful implementation evidence for both flows. `BT17_102.cs` models the stack-derived names through `ChangeCardNamesClass`; `BT23_096.cs` dispatches from `EffectTiming.OnAllyAttack`, gates on a `[CS]` attacker, declares Delay, then executes the shared de-digivolve body. Printed card text and the repo rules context remain authoritative where DCGO differs.

## Goals / Non-Goals

**Goals:**

- Add a reusable engine/DSL identity primitive for dynamic effective names derived from digivolution sources.
- Add reusable Delay event dispatch for attack events, including ally-attack gating with attacker predicates.
- Author the missing BT17-102 and BT23-096 clauses in production YAML.
- Re-enable the blocked behavioral tests for those clauses and keep the DNA Omnimon pool at zero live `raw_rust` escapes.
- Update DNA Omnimon verdict and gap trackers after verification.

**Non-Goals:**

- Do not expand `ACTION_SPACE_SIZE`, tensor contracts, or RL observation layouts.
- Do not add new raw Rust card functions for DNA Omnimon cards.
- Do not broaden this change into unrelated non-DNA Omnimon `raw_rust` cleanup.
- Do not treat DCGO as higher priority than printed card text or official rule context.

## Decisions

### Effective names are resolved through the engine identity layer

Implement stack-derived aliases as part of a permanent/card-source effective-name query, not as one-off BT17-102 predicate logic. Name predicates such as `name_is`, `name_contains`, `name_in`, aura filters, event-target name predicates, and rules helpers must be able to consult the same effective-name set so future cards do not need bespoke code.

Alternative considered: add a BT17-102-only YAML predicate that checks level-3-and-lower sources directly. This would close one test but would not faithfully model cards that care about the Digimon's names elsewhere.

### DSL exposes source-name overlays declaratively

Add a declarative DSL surface for "this Digimon has names from its sources matching a source filter", with BT17-102 using a `level_lte: 3` style filter or the closest naming pattern already present in the DSL. The compiled form should lower into a reusable permanent identity modifier rather than executable card-local Rust.

Alternative considered: require card authors to express each derived name through static aliases. That cannot work because the names depend on runtime digivolution cards.

### Delay attack triggers flow through existing event dispatch

Lower `on_attack`, `on_ally_attack`, and related Delay trigger spellings to event-backed delayed-option dispatch. Attack context should carry the attacking permanent so active conditions can evaluate predicates such as `attacker_trait_has: CS`.

Alternative considered: manually check delayed options inside combat attack code. That would make attack-triggered Delay a special combat branch and risks drifting from existing `DelayTrigger::OnEvent` semantics.

### Tests lead implementation and protect no-approximation behavior

Behavioral tests for BT17-102 and BT23-096 should first assert the currently omitted clauses, then the engine/DSL work should make those tests pass. The BT17-102 proxy predicate should be removed or replaced only after the effective-name primitive is available.

Alternative considered: update YAML first and rely on existing tests. The current ignored tests are the clearest guardrail against accidentally preserving the approximation.

## Risks / Trade-offs

- Effective-name queries may affect existing cards that use name predicates. Mitigation: add focused regression tests for existing static aliases and current name predicates alongside BT17-102.
- Attack-event Delay dispatch may accidentally fire for the wrong player or timing. Mitigation: test ally attack, opponent attack, wrong trait, and wrong turn cases for BT23-096.
- Dynamic aliases may create recursion or stale cache behavior if identity is cached too aggressively. Mitigation: compute from current stack state or invalidate any identity cache when sources change.
- Tracker updates can claim closure before behavior is verified. Mitigation: update verdicts and resolved-gap notes only after the targeted Rust tests pass.
