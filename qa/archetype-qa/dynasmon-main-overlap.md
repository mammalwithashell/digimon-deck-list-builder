# Current-main compatibility review

Read-only review after [PR #757](https://github.com/mammalwithashell/digimon-deck-list-builder/pull/757) merged.

- Validated task source base: `d0d0520262c6d8a4df0216ca6b5743052fe24789`.
- Fetched main: `77f4702f9c786f95aabb4745593e358c6face855`.
- Overlapping changed paths: 17.
- No rebase, merge, integration edit or publication was performed.
- Current-main build/behavior compatibility is **not validated** by the task's base-qualified runs.

## Integration requirements

Main adds a belief knowledge ledger and a separate closed-belief sampler. Temporary
public security reveals must feed that sampler's knowledge acquisition, allowed
identities and fixed-slot pins, with departure/shuffle invalidation. Open
Infoset/materializer pins alone do not cover the new closed sampler.

Retain both main's `remember_belief_security_*` bookkeeping and the task's
physical movement receipts/provenance. In the refactored security-trash path,
knowledge changes belong to the actual pinned movement commit, never an old
pre-replacement top/bottom index. Keep both belief-ledger and public-reveal
shuffle invalidation. Retain main's `pub(super)` materializer visibility.
Regenerate lowering coverage after eventual integration. Requalify continuation
and pending receipt state against main's new continuation audits.

## Overlapping paths

- `code/digimon-engine/src/combat/mod.rs`
- `code/digimon-engine/src/determinize/materialize.rs`
- `code/digimon-engine/src/effect_context/action/play.rs`
- `code/digimon-engine/src/effect_context/action/security.rs`
- `code/digimon-engine/src/effect_context/action/trash.rs`
- `code/digimon-engine/src/effect_queue.rs`
- `code/digimon-engine/src/game/mod.rs`
- `code/digimon-engine/src/game/setup.rs`
- `code/digimon-engine/src/game_actions/misc.rs`
- `code/digimon-engine/src/game_actions/mod.rs`
- `code/digimon-engine/src/game_actions/options.rs`
- `code/digimon-engine/src/game_actions/zones.rs`
- `code/digimon-engine/src/replacement.rs`
- `code/digimon-engine/src/search/determinized.rs`
- `code/digimon-engine/src/trigger_context.rs`
- `docs/RUST_ENGINE_GAPS.md`
- `qa/impact/lowering_coverage.json`
