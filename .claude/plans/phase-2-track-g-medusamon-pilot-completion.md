# Phase 2 Track G — Medusamon Pilot Completion

You are unblocking the Medusamon pilot archetype (30 stuck cards across base + 16 named batches as of 2026-05-17). Most of Medusamon's ignored tests are gated by Tracks C and D (G-OPT-TRIGGERED + G-INHERITED-DISPATCH cluster — 43 refs combined); this track addresses the *remaining* Medusamon-specific gaps that survive after C and D land.

Has a hard sequencing dependency on Tracks C and D for the bulk of the test-count payoff. The two Medusamon-specific DSL/substrate items below are independently shippable but won't deliver their full unblock until C and D are in.

## Why this matters

Medusamon was the **first archetype audited by `assess-archetype-rust`** (2026-04-17). Its 30 stuck cards are the densest concentration of triggered-clause + inherited-source observers in the test tree — most stop being stuck when C + D land. After that, the residual is:

| Tag | Refs | Type | Closure mode |
|---|---:|---|---|
| **G-PLACE-SELF-AS-OPTION-PERMANENT** | 6 BLOCKED | engine substrate | Option flow primitive |
| **G-EVENT-TARGET-OWNER** | 8 pending | DSL predicate | eval-arm bridge |
| **G-DSL-LINK-VERB** | 5 BLOCKED | DSL verb | Plug-In/Link DSL bridge |
| **G-DSL-LINKED-SCOPE** | 3 BLOCKED | DSL scope | sibling of LINK-VERB |
| **G-MAY-ATTACK-NOW** | 6 | (closed 2026-05-08) | un-ignore sweep |
| **G-OPP-SECURITY-COUNT-LTE** | 2 BLOCKED | DSL predicate | eval-arm bridge |
| **G-ADD-OPTION-SELF-TO-HAND** | 2 BLOCKED | DSL verb | bridge to existing helper |
| Long tail (1–2 ref each) | ~5 | mixed | ad-hoc |

Expected unblock after Tracks C+D+G: **~25 Medusamon cards advanced to IMPLEMENTED**.

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17, §18.
2. `qa/archetype-qa/medusamon.md` and `qa/archetype-qa/dsl/medusamon-final-report.md` (if exists) — archetype-specific QA. Otherwise `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md`.
3. `qa/qa-reports/validated_cards_dsl.json` — `"archetype": "Medusamon"` (plus Batch 1–16 variants).
4. `qa/dsl-vocab-gaps.md` — search each tag for the user-facing DSL syntax expected.
5. `code/digimon-engine/src/option_lifecycle.rs` — Option permanent state model. Where G-PLACE-SELF-AS-OPTION-PERMANENT and G-ADD-OPTION-SELF-TO-HAND land.
6. `code/digimon-engine/src/effect_context/mod.rs` — search `place_self_as_delay_option` and `add_pending_security_to_hand` for existing Option-self-disposition helpers.
7. `code/digimon-engine/src/dsl_cards/predicate.rs` — site for G-EVENT-TARGET-OWNER and G-OPP-SECURITY-COUNT-LTE eval arms.
8. `code/digimon-engine/src/dsl_cards/step/` — site for the new Link DSL verbs (and the existing Link substrate landed in Track I).
9. `code/digimon-engine/src/cards/link.rs` (if exists) or `option_lifecycle.rs` — `linked_cards: Vec<CardSource>` model and `relink_plug_in` / `orphan_linked_plug_in` substrate from Track I.

## Work to be done

### 1. Sequencing pre-check

Before starting, confirm Tracks C and D status:

- If both have landed: proceed full-scope.
- If only Track C has landed: you can still close G-EVENT-TARGET-OWNER, G-OPP-SECURITY-COUNT-LTE, G-ADD-OPTION-SELF-TO-HAND, G-DSL-LINK-VERB, G-MAY-ATTACK-NOW sweep, G-PLACE-SELF-AS-OPTION-PERMANENT. Tests gated on G-INHERITED-DISPATCH stay ignored.
- If neither has landed: the Medusamon-specific items below still close, but the test-count unblock will be smaller. Consider deferring this track or accepting the smaller delta.

### 2. `G-EVENT-TARGET-OWNER` (8 pending refs) — DSL predicate eval arm

Add `event_target_owner: PlayerRef` to `CompiledPredicate` if not present, with eval in `predicate.rs` reading `ctx.trigger_context.target_permanent.player` (or wherever the owner is exposed). The PlayerRef should support `Self`, `Opponent`, `Source`, `Controller`. Variant-coverage lint compliance.

### 3. `G-OPP-SECURITY-COUNT-LTE` (2 BLOCKED refs) — DSL predicate eval arm

Sibling of the existing security-count predicates. Add the OPP-side variant if missing, or fix the eval arm if the field exists but isn't read. Probably ~10 LOC in `predicate.rs`.

### 4. `G-PLACE-SELF-AS-OPTION-PERMANENT` (6 BLOCKED refs) — engine substrate

Per `docs/RUST_ENGINE_GAPS.md` "Option card play flow" residual: a `ctx.place_self_as_option_permanent()` helper that takes a normal Option card resolution and pivots to placing the card as a battle-area permanent (the OptionSecurity disposition + the BT8-097/Medusamon Main-flow shape). May already partially exist via `place_self_as_delay_option` — confirm and extend if needed.

DSL surface: `place_self_in_battle_area: {}` or similar verb.

### 5. `G-ADD-OPTION-SELF-TO-HAND` (2 BLOCKED refs) — DSL verb

Bridge to existing `EffectContext::add_pending_security_to_hand` (or sibling). DSL verb `add_self_to_hand: {}` (already partially exists as `add_this_option_to_hand: {}` per `qa/dsl-vocab-gaps.md` — confirm and ensure the Medusamon-specific shape is supported).

### 6. `G-DSL-LINK-VERB` + `G-DSL-LINKED-SCOPE` (5 + 3 BLOCKED refs) — DSL verbs

Track I shipped the engine substrate: `Game::orphan_linked_plug_in`, `Game::relink_plug_in`, `OptionFieldState::LinkedPlugIn` / `OrphanedPlugIn`. Missing: DSL verbs that consume them. Per `qa/dsl-vocab-gaps.md` and `docs/RUST_ENGINE_GAPS.md` "Plug-In re-link from battle area source zone":

```yaml
- link_plug_in:
    source: hand    # or: { battle_area: <permanent-binding> }
    target: <permanent-binding>
- on_link_target:
    body: [...]    # G-DSL-LINKED-SCOPE — scope a clause to "while linked"
```

Add `CompiledStep::LinkPlugIn { source: PlugInSource, target: PermanentRef }` and a `WhileLinked` aura scope. Variant-coverage compliance.

### 7. `G-MAY-ATTACK-NOW` sweep (6 refs)

Per `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` § 4, this primitive was resolved 2026-05-08. Sweep Medusamon tests for the `BLOCKED: G-MAY-ATTACK-NOW` annotations and un-ignore. If any test fails post-unignore, it's surfacing a different gap — leave ignored with the new tag.

### 8. Author Medusamon production YAML for the unblocked cards

Walk the per-card list. Expect strong batch effect — most Medusamon cards share the same triggered-clause + Option-permanent shape, so closing the substrate above should unblock cards in groups.

## Acceptance gates

- G-EVENT-TARGET-OWNER, G-OPP-SECURITY-COUNT-LTE eval arms wired.
- G-PLACE-SELF-AS-OPTION-PERMANENT substrate present (verify shape didn't already land in an adjacent Option-flow PR).
- G-ADD-OPTION-SELF-TO-HAND DSL verb wired.
- G-DSL-LINK-VERB + G-DSL-LINKED-SCOPE DSL verbs/scopes land with variant-coverage compliance.
- G-MAY-ATTACK-NOW sweep removes 6 ignore annotations.
- ≥ 12 Medusamon cards advance to IMPLEMENTED.
- All eval-arm-coverage and behavioral test suites pass.

## Constraints

- No-approximations: all Option-self-disposition choices and Plug-In link choices surface through pending_selection / action mask.
- Working Rule 1: no `ACTION_SPACE_SIZE` change — Group 5 contract note explicitly forbids this for Option flow.
- Source priority: printed text → Rules Manual → fandom wiki → DCGO.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

## Tracker discipline

- `qa/dsl-vocab-gaps.md` — close G-EVENT-TARGET-OWNER, G-OPP-SECURITY-COUNT-LTE, G-DSL-LINK-VERB, G-DSL-LINKED-SCOPE, G-ADD-OPTION-SELF-TO-HAND, G-PLACE-SELF-AS-OPTION-PERMANENT entries.
- `docs/RUST_ENGINE_GAPS.md` — sweep Option card play flow + Plug-In re-link entries.
- `qa/qa-reports/validated_cards_dsl.json` — advance Medusamon and Medusamon Batch N cards as YAML completes.

## Order of operations

1. Sequencing pre-check.
2. The 2 DSL eval-arm bridges (G-EVENT-TARGET-OWNER, G-OPP-SECURITY-COUNT-LTE) — batch in one commit.
3. G-MAY-ATTACK-NOW sweep (un-ignore).
4. G-ADD-OPTION-SELF-TO-HAND DSL verb (likely 1-line bridge).
5. G-PLACE-SELF-AS-OPTION-PERMANENT engine substrate (confirm size — may be small if helper exists).
6. Link DSL verbs (G-DSL-LINK-VERB + G-DSL-LINKED-SCOPE) — likely largest single item in this track.
7. Card authoring walk.
8. Tracker hygiene + PR(s).

## Out of scope

- Tracks C and D substrate (handled in their own tracks).
- BT24-016 / Lamiamon alt-digivolve subshapes (handled in Track F's alt-path direction work, or in a Dark Masters-shape track later).
- Counter Blast DNA (closed).
- Force-follow-up-attack (closed).

## Discovery rider

If G-PLACE-SELF-AS-OPTION-PERMANENT turns out to be more involved than a small helper (e.g., requires reshaping `Game::play_option_from_hand`), STOP and file as a substrate-level item rather than absorbing into this track. The intent is to close the Medusamon-specific authoring blockers, not to re-architect Option play flow.
