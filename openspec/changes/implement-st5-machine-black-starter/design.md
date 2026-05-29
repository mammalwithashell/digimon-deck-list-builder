## Context

The Rust engine already carries ST5 JSON card records under `code/digimon-engine/cards/st5/`, but the DSL registry has no ST5 YAML specs and `load_implemented_card_ids()` does not report any `ST5-*` IDs. The exact starter deck from ST-5: Starter Deck Machine Black contains 4 Digi-Eggs plus 50 main-deck cards across 16 unique card IDs.

Most ST5 effects map to existing DSL capabilities: vanilla Digimon, static keywords, inherited keywords, DP modifiers, keyword grants until end of opponent's next turn, Digi-Burst source costs, De-Digivolve, delete-by-play-cost, and option security effects that activate the main effect. The primary missing reusable vocabulary is the inherited end-of-opponent-turn condition on ST5-04 and ST5-06: draw 1 when the opponent did not attack with a Digimon this turn.

ST5-14 Tai Kamiya also needs careful validation. It reacts when the controller uses `<Blocker>` to suspend one of their Digimon, then optionally suspends the Tamer to unsuspend one of their Digimon. Existing attack-target-change context appears close, but implementation must prove the trigger is tied to an actual blocker redirect rather than any attack redirection.

## Goals / Non-Goals

**Goals:**

- Implement every ST5 card faithfully in Rust DSL/YAML, including inherited and security effects that are currently stored in legacy JSON fields.
- Add behavioral tests that prove each non-vanilla card's printed text and negative cases.
- Add a reusable DSL condition for "referenced player attacked with any Digimon this turn" so ST5-04 and ST5-06 are not approximated.
- Add the exact ST-5 Machine Black decklist to validation/training fixture surfaces only after all card IDs are executable.
- Keep all gameplay choices visible through existing action masks and pending selections.

**Non-Goals:**

- No action-space expansion, tensor-layout change, observation-profile change, or model metadata migration.
- No new legacy Python card scripts.
- No partial starter-deck readiness claim while any ST5 card is missing, stubbed, or test-ignored for an unresolved gap.
- No broad refactor of combat, blocker, or end-of-turn processing beyond the reusable predicate/trigger support required for this deck.

## Decisions

### Add a reusable attack-history predicate instead of card-specific Rust

ST5-04 and ST5-06 use the same inherited condition and similar conditions are likely to appear on other cards. The DSL should expose a general condition, such as `player_digimon_attacked_this_turn` or a negated equivalent, parameterized by `you` / `opponent`. The engine can evaluate it from authoritative turn attack history rather than relying on observation metadata or UI state.

Alternative considered: implement ST5-04 and ST5-06 with raw Rust card effects. That would unblock two cards quickly but would fail the capability-centric gap policy and would not help later cards with the same timing pattern.

### Prefer existing attack-target-change context for Tai, but test it as blocker semantics

Tai's trigger should be authored with the existing blocker/attack-target-change event context if it can precisely distinguish blocker redirects. A behavioral test must cover a real blocker redirect and at least one non-blocker attack redirect case. If the existing context cannot make that distinction faithfully, add the smallest DSL timing/context extension needed for `on_block` semantics.

Alternative considered: add a new `on_block` timing immediately. That may be cleaner, but it should be driven by failing tests rather than assumed necessary.

### Author YAML from printed text, not the legacy JSON field shape

Several older ST5 JSON records store inherited or security text in fields that do not match the modern `effect_text` / `inherited_text` / `security_text` split. The implementation should normalize against printed card text from the local data and the starter-deck source, then author YAML in the engine's current effect sections.

Alternative considered: copy field names mechanically from the JSON. That risks placing inherited/security effects in the wrong YAML section and silently creating wrong runtime behavior.

### Gate decklist promotion on complete card implementation

The exact starter deck should enter deck validation/training fixture surfaces only when all 16 IDs are in the Rust registry and their tests pass. This avoids presenting a starter deck that can be selected but cannot run faithfully.

Alternative considered: add the decklist first and let validation fail for missing IDs. That creates noisy intermediate state and weakens the implemented-card gate.

## Risks / Trade-offs

- **Attack history ambiguity** -> Define and test turn-boundary semantics explicitly: attack history resets at turn start/end as appropriate, and only Digimon attacks by the referenced player satisfy the predicate.
- **Tai blocker trigger overfires** -> Add negative tests for non-blocker redirects and only add a new timing if existing context cannot encode the distinction.
- **Old JSON text placement causes authoring mistakes** -> Review each ST5 card against printed card text and place inherited/security clauses in YAML by effect role, not source JSON field.
- **Deck surfaces diverge** -> Add a smoke test that resolves the exact starter deck, confirms every card ID is implemented, and can initialize a Rust headless game with it.
- **Broad contract drift** -> Keep this change inside existing action/pending-selection surfaces; stop and open a separate action/tensor proposal if any required choice cannot be represented.

## Migration Plan

1. Add failing tests for the attack-history predicate and the ST5 cards that require it.
2. Implement the reusable predicate and lowering/evaluation support.
3. Author and test ST5 YAML cards in small batches, starting with simple/static cards and ending with ST5-04, ST5-06, ST5-13, ST5-14, ST5-15, and ST5-16.
4. Add the exact starter decklist and implemented-card/tested-card metadata once the registry exposes all 16 IDs.
5. Reconcile gap trackers and readiness ledgers.

Rollback is straightforward: remove the ST5 YAML/test/decklist additions and the predicate if it has no other users. If another card adopts the predicate during implementation, keep the predicate and roll back only the ST5 deck content.

## Open Questions

- Should the new attack-history condition be named positively (`player_digimon_attacked_this_turn`) with normal DSL negation, or named for the exact common authoring shape (`no_player_digimon_attacked_this_turn`)?
- Does the existing attack-target-change reason context fully identify `<Blocker>` usage for Tai, including edge cases such as forced blocking effects, or is a dedicated `on_block` timing needed?
