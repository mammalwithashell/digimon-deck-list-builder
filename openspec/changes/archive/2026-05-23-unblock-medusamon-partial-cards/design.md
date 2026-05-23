## Context

The Medusamon DSL run left 7 cards `PARTIAL`, each blocked on a distinct engine/DSL substrate gap (see `proposal.md` and the gap trackers). This change implements the 5 low-risk gaps and spikes the 2 that touch the RL action-space contract.

Current state, verified against the worktree source:

- **Security resolution** — `combat.rs::drive_security_resolution` is a phase machine (`SecurityPhase` in `selection.rs`). The `SecuritySkillDrain` arm (`combat.rs:2497`) enqueues `EffectTiming::SecuritySkill`, drains, and on a parked `pending_selection` `return None` **without advancing phase or recording the drain fired**. The sibling `Dispose` phase already solved the identical re-entry hazard — its doc comment (`selection.rs:627`) states it "parks with phase advanced to `DisposeFinalize` so the resume path doesn't re-enqueue the observer on re-entry". `SecuritySkillDrain` was never given the equivalent guard.
- **Trash→deck moves** — `EffectContext::return_all_trash_to_deck_bottom` / `return_trash_cards_to_deck_bottom` (`effect_context/mod.rs:4606`, `:4628`) both hard-code `deck.insert(0, …)`. The DSL exposes only `ReturnAllTrashToDeckBottom` / `ReturnTrashListToDeckBottom`.
- **Security-card trashing** — `trash_top_security` (`effect_context/mod.rs:1973`) and `trash_bottom_security` (`:2027`) exist; there is no arbitrary-index variant and no DSL verb consuming a `select_security` binding.
- **`activation_cost`** — the `ActivationCost` step struct (`digimon-dsl/src/step.rs:874`) has exactly two `bool` fields; the compile arm (`compile.rs:2457`) is a 2-tuple `match`.
- **Alt-path `from:` filters** — support `level_eq` / `trait_has` / `name_contains`; no predicate inspects the source card's printed keyword text.

Constraint that drives the Tier split: working rules 1 & 4 require `ACTION_SPACE_SIZE` / `TENSOR_SIZE` changes to be made in lockstep with the spec docs, masks, and decoder. The 5 committed gaps need none of that; the 2 spiked gaps need a new action ID.

## Goals / Non-Goals

**Goals:**
- Close the 5 substrate gaps (G-SECURITY-SKILL-RESUME-REFIRE, G-ZONE-SELECTED-TRASH-TO-DECK-TOP, G-TRASH-SELECTED-SECURITY, G-ACTIVATION-COST-TRASH-SELF, G-ALT-PATH-SAVE-IN-TEXT) with TDD, no approximations.
- Keep all existing YAML compiling unchanged — every new DSL field is additive and defaults to today's behavior.
- Leave `ACTION_SPACE_SIZE` and `TENSOR_SIZE` untouched.
- Produce a written action-space decision for the 2 Tier-3 gaps so a follow-up change can implement them.

**Non-Goals:**
- Implementing G-ACTIVATED-DIGIVOLVE-EXECUTION or G-LINK-OPTION-DUAL-PLAY-MODE (spike output only).
- Re-authoring the 7 card YAMLs — that is a downstream `/batch-implement-cards-rust-dsl` re-run, not part of this change.
- Any RL retraining or tensor-profile work.

## Decisions

### D1 — Security resume: `security_skill_drained` flag, not a phase advance
Add a `security_skill_drained: bool` to `SecurityResolutionState` (init `false` in the `resolve_security_card` constructor at `combat.rs:2451`). The `SecuritySkillDrain` arm: on first entry (`!security_skill_drained`) enqueue `SecuritySkill`, set the flag, drain; on every entry drain the queue (to flush a resumed continuation) and, if `pending_selection.is_some()`, park; otherwise advance to `BattleResolved`.
*Alternative considered:* advance the phase directly to `BattleResolved` on park (as `Dispose`→`DisposeFinalize` does). Rejected — a player who *accepts* the optional `[Security]` effect parks mid-process (e.g. picking which card to play); resume must re-enter `SecuritySkillDrain` to finish draining that process, not skip to battle. A flag keeps the phase stable while suppressing only the re-enqueue.

### D2 — Trash→deck-top: mirror method + additive `destination` param
Add `EffectContext::return_trash_cards_to_deck_top` (mirror `return_trash_cards_to_deck_bottom`, `deck.push` instead of `deck.insert(0, …)`). Extend the existing `ReturnTrashListToDeckBottom` DSL step with a `destination: top | bottom` field defaulting to `bottom`; rename the compiled step to a neutral `ReturnTrashListToDeck { destination }` (or keep the name and add the field — naming finalized at implementation).
*Alternative considered:* a brand-new `return_selected_trash_to_deck_top` verb. Rejected — duplicates the binding-consumption logic; a `destination` param is one enum field and keeps the two positions discoverable together.

### D3 — Trash-selected-security: new engine method + DSL verb consuming a binding
Add `EffectContext::trash_security_at_index(player, index)` (the `security[len-1]` / `security[0]` indexing in `trash_top`/`trash_bottom_security` is the template). Add a `trash_selected_security` DSL verb that consumes a `select_security` binding and lowers to the new method. The selection step (`select_security`) already exists and produces the index binding.

### D4 — `trash_self` activation cost: third mutually-exclusive variant
Add `trash_self: bool` to the `ActivationCost` step struct and a third arm to the `compile.rs:2457` tuple `match` (now 3-tuple), enforcing exactly-one-of-three. It lowers onto `EffectBuilder::activation_cost(...)` like the existing two — the declinable accept/decline gating already exists for `suspend_self` / `return_self_to_deck_bottom` and is reused unchanged. This makes `<Delay>` "by trashing this card" a genuine declinable cost per Comprehensive Rules 16-16-2 instead of a mandatory first body step.

### D5 — Alt-path text predicate: generalized `keyword_in_text`
Implement a generalized `keyword_in_text: <keyword>` predicate leaf for alt-path `from:` filters rather than a narrow `save_in_text: bool`. The generalized form is marginally more code and forecloses the next near-identical gap (any "w/<Keyword> in text" alt-path requirement). `save_in_text: true` semantics = `keyword_in_text: Save`.

### D6 — Tier 3 is a spike
G-ACTIVATED-DIGIVOLVE-EXECUTION and G-LINK-OPTION-DUAL-PLAY-MODE both need a new action ID. The spike answers one question per gap: **can the new play mode reuse existing action IDs** (digivolve-from-hand IDs for activated digivolve; `PLAY_HAND_START` with a mode discriminant for link-from-hand), keeping `ACTION_SPACE_SIZE` fixed — **or** must the space grow (precedent: `G-BREEDING-PERMANENT-SELECTION` raised it 2168→2192)? Deliverable: a follow-up proposal with the chosen approach. No engine code in this change.

## Risks / Trade-offs

- **[D1] A new `SecurityResolutionState` field could regress the accept path** → the regression suite must keep an accept-path test (play the `[Security]` effect to completion) alongside the new decline-path test; both share the flag.
- **[D2] `destination` default** → defaulting to `bottom` means zero churn for existing YAML; only the LM-027-family opts into `top`. A non-defaulted field would break every shipping `ReturnTrashListToDeckBottom` user.
- **[D3] Security index staleness** → a bound `select_security` index is only valid until the next security-stack mutation. The `trash_selected_security` verb must consume the binding with no intervening security move; document this and cover it with a test where the chosen card is not the top card.
- **[D4] Trashing the source as a cost may itself trigger observers** → trashing the `<Delay>` card is a zone move; confirm whether `OnDeletion`/trash-fan-out should fire (see Open Questions) so the declinable cost does not silently skip a trigger.
- **[D6] Deferring Tier 3 leaves BT24-016 and ST22-08 `PARTIAL`** → accepted; they were already `PARTIAL` and the spike de-risks a larger follow-up.

## Migration Plan

Internal engine/DSL change — no data migration. Each gap lands TDD-first (failing test → fix). Rollback = revert the commit. After merge, re-run `/batch-implement-cards-rust-dsl Medusamon --cards <the 5 unblocked cards>` to promote them to `IMPLEMENTED`, and move the 5 closed gap entries to `qa/resolved-gaps.md`.

## Open Questions

- **Q1** — `keyword_in_text` data source: does the alt-path `from:` evaluator have access to a parsed keyword set on the source card, or must it substring-scan raw effect text? Resolve with a 10-minute read of the alt-path filter evaluator before committing the predicate shape (D5).
- **Q2** — Should `trash_self` activation cost fire `OnDeletion` / trash observers for the trashed `<Delay>` card? Check printed rules + DCGO before finalizing D4.
- **Q3** — Tier 3 only: the two action-space questions in D6 — answered by the spike, not now.
