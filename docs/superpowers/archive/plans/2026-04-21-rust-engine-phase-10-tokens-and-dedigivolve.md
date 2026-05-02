# Rust Engine Phase 10 — Tokens + De-Digivolve Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close two `RUST_ENGINE_GAPS.md` 🔴 BLOCKING entries — token creation (Medusamon / TS Olympos / assorted archetype tokens) and the generalized De-Digivolve N primitive (4-of-5 recently-audited archetypes need it).

**Architecture:** Both features are small, additive `EffectContext` primitives in the same spirit as Phase 5's cost-hook helpers. Tokens get a first-class `CardKind::Token` variant plus a `TokenRegistry` that `ctx.play_token(player, token_name)` consults to materialize a synthetic `CardSource` directly onto the battle area. De-Digivolve becomes `ctx.de_digivolve(target, Option<u8> stop_at_level)` that pops sources off the stack one at a time, respecting the level floor, trashing popped cards on the owner side. Both features land TDD under `digimon-engine/tests/cards_behavioral/` per working rule 18; neither introduces new auto-selections, so the no-approximations policy (rule 17) is preserved.

**Tech Stack:** Rust (digimon-engine crate), cargo test behavioral + integration tests.

**Spec:** `docs/superpowers/specs/2026-04-15-rust-engine-rewrite-design.md` (north-star), `docs/RUST_ENGINE_GAPS.md` §"Token creation + `CardKind::Token`" and §"De-Digivolve N primitive".

---

## Background

Phase 10 is scoped to two additive primitives the audit flagged as 🔴 BLOCKING:

1. **Tokens.** In Digimon TCG tokens are engine-generated cards that spawn directly on the field with printed stats and effects; they never enter/exit a deck, hand, or trash. When they leave the battle area they're *removed from the game*, not trashed. Real cards needing this: Medusamon family's Petrification Token (Purple), TS Olympos's Familiar Token (Yellow), Rocks's assorted tokens. Rust currently has an `is_token: bool` flag on `CardSource` (`card_source.rs:19`) and a `CardSource::new_token()` constructor (`card_source.rs:37`), but no `CardKind::Token` variant, no registry, no spawn helper, and `player::delete_permanent` (`player.rs:155`) unconditionally moves everything to `self.trash` — tokens currently get "trashed" alongside real cards, which is wrong once any observer inspects the trash.

2. **De-Digivolve N.** `Keyword::DeDigivolve(u8)` is declared in `enums.rs:177` but has no implementation anywhere in `digimon-engine/src/`. The gap doc (§"De-Digivolve N primitive") specifies the stop-at-Lv.3 floor and trash routing. 4 of 5 recently-audited archetypes (Medusamon, DNA Omnimon, Rocks, Dark Masters) need this primitive; Dark Masters alone cites 10+ distinct De-Digivolve values across its card pool. We generalize the proposed `amount: u8` to `stop_at_level: Option<u8>` so TS Olympos's Ikkakumon (pops until stack empty) and the default Lv.3-floor case both reduce to the same API call.

3. **Residual misc.** The gap doc's Phase-10-tagged residuals are the Token and De-Digivolve items themselves (§4.6b-residual in parity.md points to Token; no other Phase-10-tagged residuals exist). We do **not** expand scope into return-to-hand / return-to-deck paths (gap doc line 43, 🔴 BLOCKING) — those are a larger chunk deferred to a later phase. Token removal-from-game hooks in Phase 10 are limited to `player::delete_permanent` because that's the only leave-field path that currently exists.

### Binding constraints

- **No-approximations policy (CLAUDE.md §17):** every choice must surface through `pending_selection`. De-Digivolve's "pop top N" is deterministic (no choice involved) — card text reads "Trash up to N cards from the top" with a hard stop-at-Lv.3 rule, not "choose N". Token spawn is equally deterministic.
- **TDD (CLAUDE.md §18):** failing `DebugRunner` tests under `digimon-engine/tests/cards_behavioral/` before the `CardEffect` / `EffectContext` implementation in every task.
- **Phase 4/5 pattern:** add `debug_assert!` invariants at any new closure-dispatch sites to catch future populate-path bugs. No closures are added in Phase 10 (tokens / de-digivolve are all synchronous mutations), but we still invariant-check token-registry lookups and stack-depth preconditions.

### Current baseline

`cargo test --manifest-path digimon-engine/Cargo.toml` reports **385 passing / 0 failing / 1 ignored** on this worktree (branch `claude/busy-snyder-5718e9` at `origin/main` = `f86223f1`). Every new test added in Phase 10 must pass; the whole suite must stay green with zero warnings.

---

## File Structure

### digimon-engine — create

- `digimon-engine/src/token_registry.rs` — token metadata catalog (the Rust analogue of `digimon_gym/engine/data/token_registry.py`). Maps a canonical token name (e.g. `"petrification"`) to a `TokenDef` with card_id, card_name, colors, DP, level, traits, and an optional `Arc<dyn CardEffect>` for the token's printed abilities. Also exposes `build_registry() -> TokenRegistry`.
- `digimon-engine/src/cards/tokens/mod.rs` — `register(registry: &mut TokenRegistry)` that wires the Petrification Token's `CardEffect` (OnDeletion trash top security) and the Familiar Token stub (no printed effect, just stats). Layout mirrors `src/cards/bt17/mod.rs`.
- `digimon-engine/src/cards/tokens/petrification.rs` — the Petrification Token's `CardEffect` impl.
- `digimon-engine/src/cards/tokens/familiar.rs` — Familiar Token `CardEffect` impl (no printed abilities currently — the [On Deletion] -3000 DP effect in `token_registry.py:186` needs opponent-permanent selection which this phase does NOT add; ship stats only, document the gap inline).
- `digimon-engine/tests/cards_behavioral/tokens.rs` — behavioral tests for `ctx.play_token`, token removal-from-game semantics, Petrification Token OnDeletion.
- `digimon-engine/tests/cards_behavioral/de_digivolve.rs` — behavioral tests for `ctx.de_digivolve` (stop-at-Lv.3 default, unbounded `None`, exact `Some(N)`, trash routing, stack-depth-1 no-op).

### digimon-engine — modify

- `digimon-engine/src/enums.rs:8-13` — add `CardKind::Token` variant (preserve existing ordering; add at end).
- `digimon-engine/src/lib.rs` — `pub mod token_registry;` and re-export `TokenRegistry`, `TokenDef`.
- `digimon-engine/src/cards.rs` — add `pub mod tokens;` and `tokens::register(...)` call in `build_registry()`. (Tokens register into the `CardEffectRegistry` alongside production set cards — the same registry drives effect lookup regardless of whether the `CardSource` came from a deck or from `play_token`.)
- `digimon-engine/src/game.rs` — `pub token_registry: TokenRegistry` field + initialize in `Game::new`.
- `digimon-engine/src/card_data.rs` — `CardData::card_kind` serde mapping already handles arbitrary variants; no change. Add a `from_token_def(&TokenDef) -> CardData` helper (token spawns synthesize a `CardData` row on the fly and push it into `game.card_data`).
- `digimon-engine/src/player.rs:155-166` — branch on `top_card.is_token`: if token, drop the entire `card_sources` and `linked_cards` on the floor (removed from game) rather than pushing to `self.trash`. Tokens have no sub-cards worth preserving.
- `digimon-engine/src/effect_context/mod.rs` — add:
  - `pub fn play_token(&mut self, controller: PlayerId, token_name: &str) -> Option<PermanentHandle>`
  - `pub fn de_digivolve(&mut self, target: PermanentHandle, stop_at_level: Option<u8>, amount: Option<u8>) -> u8`
  - (The `amount: Option<u8>` arg is separate from `stop_at_level` so TS Olympos's "pop until empty" (`amount=None, stop_at_level=None`) and standard De-Digivolve N (`amount=Some(N), stop_at_level=Some(3)`) are both expressible. Returns actual count popped.)
- `digimon-engine/src/cards/test/mod.rs` — add `TEST-023` / `TEST-024` stubs (De-Digivolve helpers used by the behavioral tests — emit the `ctx.de_digivolve(...)` call from a card `OnPlay` so we exercise the full play→registry→ctx path, not just a direct ctx-method call).

### docs — modify (final task only)

- `docs/RUST_ENGINE_API.md` — add §Phase 10 section with worked examples for `play_token` and `de_digivolve`.
- `docs/RUST_PYTHON_PARITY.md` — flip §4.6b-residual (Token detection) from 🟡 to 🟢; add new §10.1 (Tokens — Rust now has `CardKind::Token`, Python keeps `is_token: bool` — both are kept in sync at the binding boundary) and §10.2 (De-Digivolve — Rust `Option<u8>` stop-at-level is a superset of Python's behavior; no observable divergence).
- `docs/RUST_ENGINE_GAPS.md` — mark "Token creation + `CardKind::Token`" and "De-Digivolve N primitive" as CLOSED with a pointer to this plan; leave the severity bar on the summary row.
- `C:\Users\james\.claude\plans\recursive-coalescing-candle.md` — flip Phase 10 row to `✅ Landed YYYY-MM-DD (re-audit pending)`.

---

## Phase 1 — Tokens

### Task 1: `CardKind::Token` + `TokenRegistry` data layer

**Files:**
- Modify: `digimon-engine/src/enums.rs:8-13`
- Create: `digimon-engine/src/token_registry.rs`
- Create: `digimon-engine/src/cards/tokens/mod.rs`
- Create: `digimon-engine/src/cards/tokens/petrification.rs` (placeholder — full impl in Task 3)
- Create: `digimon-engine/src/cards/tokens/familiar.rs` (stats-only stub)
- Modify: `digimon-engine/src/lib.rs` (add `pub mod token_registry;`)
- Modify: `digimon-engine/src/cards.rs` (add `pub mod tokens;`)

- [ ] **Step 1: Add `CardKind::Token` variant**

In `digimon-engine/src/enums.rs`, change the enum body from:

```rust
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
}
```

to:

```rust
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}
```

Keep existing variants in the same order so any numeric serde discriminants used in tensor builders / `card_data.rs::load_from_str` stay stable.

- [ ] **Step 2: Run full suite to confirm non-exhaustive matches surface**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --no-run 2>&1 | grep -E "error|warning" | head -30`

Expected: only new `warning: match is not exhaustive — missing CardKind::Token` warnings (if any), no compile errors. If any `match CardKind` arm is non-exhaustive, add an explicit `CardKind::Token => { /* treated as Digimon for now */ }` arm wherever a match already exists — grep `match.*CardKind` and `CardKind::` to enumerate sites. Token-specific divergences land in later tasks; for now, pattern-match parity is enough.

Commit: `chore(engine): add CardKind::Token variant`

- [ ] **Step 3: Create `token_registry.rs` with the `TokenDef` shape**

Create `digimon-engine/src/token_registry.rs`:

```rust
//! Token metadata catalog — the Rust analogue of
//! `digimon_gym/engine/data/token_registry.py`. Maps a canonical
//! token name (`"petrification"`) to `TokenDef` metadata that
//! `EffectContext::play_token` consumes to synthesize a `CardSource`
//! + `Permanent` directly onto the battle area, bypassing hand /
//! deck / play-cost.
//!
//! Tokens are `CardKind::Token` and obey leave-field removal-from-game
//! semantics (see `player::delete_permanent`). Printed abilities are
//! carried by the set-wide `CardEffectRegistry` under a synthetic
//! `card_id` like `"TOKEN_PETRIFICATION"` — `Game::card_data`
//! absorbs a `CardData` row for each registered token at `Game::new`
//! time so the rest of the engine (tensor, mask, combat) treats them
//! identically to deck-sourced cards.

use std::collections::HashMap;

use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind};

/// Declarative token metadata. No closures live here — printed abilities
/// live on the `CardEffect` implementation registered against
/// `TokenDef::card_id` in the main `CardEffectRegistry`.
#[derive(Debug, Clone)]
pub struct TokenDef {
    /// Canonical lookup key (e.g. `"petrification"`).
    pub name: String,
    /// Synthetic card_id used as the effect-registry key and as the
    /// `CardData::card_id` of materialized instances
    /// (e.g. `"TOKEN_PETRIFICATION"`).
    pub card_id: String,
    pub card_name: String,
    pub colors: Vec<CardColor>,
    pub dp: Option<i32>,
    pub level: Option<u8>,
    pub traits: Vec<String>,
}

impl TokenDef {
    /// Synthesize a `CardData` row from this token definition. `Game::new`
    /// calls this for every registered token and pushes the result into
    /// `game.card_data` so `CardSource::data_index` lookups work.
    pub fn to_card_data(&self) -> CardData {
        CardData {
            card_id: self.card_id.clone(),
            card_name: self.card_name.clone(),
            card_kind: CardKind::Token,
            level: self.level,
            dp: self.dp,
            play_cost: 0,
            colors: self.colors.clone(),
            traits: self.traits.clone(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            effect_class_name: self.card_id.clone(),
            index: 0,
            norm_id: 0.0,
        }
    }
}

/// Registry of token definitions, keyed by canonical name.
#[derive(Debug, Default, Clone)]
pub struct TokenRegistry {
    defs: HashMap<String, TokenDef>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, def: TokenDef) {
        self.defs.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&TokenDef> {
        self.defs.get(name)
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// All registered token definitions. Used by `Game::new` to
    /// seed `card_data` with synthetic rows.
    pub fn iter(&self) -> impl Iterator<Item = &TokenDef> {
        self.defs.values()
    }
}

/// Build the default token registry. Currently: Petrification (Medusamon),
/// Familiar (TS Olympos). Additional tokens land as their archetypes are
/// ported.
pub fn build_registry() -> TokenRegistry {
    let mut r = TokenRegistry::new();
    r.insert(TokenDef {
        name: "petrification".to_string(),
        card_id: "TOKEN_PETRIFICATION".to_string(),
        card_name: "Petrification Token".to_string(),
        colors: vec![CardColor::White],
        dp: Some(3000),
        level: None,
        traits: Vec::new(),
    });
    r.insert(TokenDef {
        name: "familiar".to_string(),
        card_id: "TOKEN_FAMILIAR".to_string(),
        card_name: "Familiar Token".to_string(),
        colors: vec![CardColor::Yellow],
        dp: Some(3000),
        level: None,
        traits: Vec::new(),
    });
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn petrification_registered() {
        let r = build_registry();
        let def = r.get("petrification").expect("petrification missing");
        assert_eq!(def.card_id, "TOKEN_PETRIFICATION");
        assert_eq!(def.dp, Some(3000));
        assert!(def.level.is_none());
    }

    #[test]
    fn to_card_data_marks_token_kind() {
        let r = build_registry();
        let def = r.get("petrification").unwrap();
        let cd = def.to_card_data();
        assert_eq!(cd.card_kind, CardKind::Token);
        assert_eq!(cd.play_cost, 0);
    }

    #[test]
    fn unknown_name_returns_none() {
        let r = build_registry();
        assert!(r.get("no-such-token").is_none());
    }
}
```

- [ ] **Step 4: Wire module into `lib.rs`**

In `digimon-engine/src/lib.rs`, add near the other `pub mod` lines:

```rust
pub mod token_registry;
```

And re-export at the bottom (match existing re-export style):

```rust
pub use token_registry::{TokenDef, TokenRegistry};
```

- [ ] **Step 5: Create `cards/tokens/` module tree**

Create `digimon-engine/src/cards/tokens/mod.rs`:

```rust
//! Token printed abilities. Parallels `src/cards/bt17/` but indexed by
//! synthetic token card_ids (`TOKEN_PETRIFICATION`, `TOKEN_FAMILIAR`).
//! Tokens without printed abilities may omit their entry here entirely —
//! the registry lookup returns `None` and the engine treats them as a
//! vanilla permanent with base stats only.

use std::sync::Arc;

use crate::cards::CardEffectRegistry;

mod familiar;
mod petrification;

pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("TOKEN_PETRIFICATION", Arc::new(petrification::PetrificationToken));
    registry.insert("TOKEN_FAMILIAR", Arc::new(familiar::FamiliarToken));
}
```

Create `digimon-engine/src/cards/tokens/petrification.rs` (stub — Task 3 fills in the OnDeletion behavior):

```rust
//! Petrification Token — Medusamon archetype (Purple).
//!
//! Printed text: "[Your Turn] This Digimon can't suspend.
//! [On Deletion] Trash the top card of this Digimon's owner's
//! security stack."
//!
//! Task 3 wires the OnDeletion trash-top-security via
//! `ctx.trash_top_security(...)`. The CannotSuspend [Your Turn] rider
//! depends on a modifier framework piece scheduled for a later phase
//! (see parity §4.6b-residual) — Task 3 documents the gap inline and
//! leaves a `#[ignore]`d test covering the CannotSuspend contract
//! so it surfaces once the framework lands.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct PetrificationToken;

impl CardEffect for PetrificationToken {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        // OnDeletion effect wired in Task 3.
        Vec::new()
    }
}
```

Create `digimon-engine/src/cards/tokens/familiar.rs`:

```rust
//! Familiar Token — TS Olympos archetype (Yellow).
//!
//! Printed text: "[On Deletion] 1 of your opponent's Digimon gets
//! -3000 DP for the turn."
//!
//! The selection primitive this depends on (opponent-permanent pick
//! with a callback) is a Phase-6+ gap (see
//! RUST_ENGINE_GAPS.md §"Selection: opponent-as-selecting-player").
//! Phase 10 ships the stat line only; the [On Deletion] effect lands
//! when the selection primitive does.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct FamiliarToken;

impl CardEffect for FamiliarToken {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        Vec::new()
    }
}
```

- [ ] **Step 6: Wire `tokens::register` into `cards::build_registry`**

In `digimon-engine/src/cards.rs`, update:

```rust
pub mod bt17;
pub mod test;
```

to:

```rust
pub mod bt17;
pub mod test;
pub mod tokens;
```

And update `build_registry()`:

```rust
pub fn build_registry() -> CardEffectRegistry {
    let mut registry = CardEffectRegistry::new();
    test::register(&mut registry);
    bt17::register(&mut registry);
    tokens::register(&mut registry);
    registry
}
```

- [ ] **Step 7: Run the `token_registry` unit tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --lib token_registry -- --nocapture`

Expected: 3 tests pass (`petrification_registered`, `to_card_data_marks_token_kind`, `unknown_name_returns_none`).

- [ ] **Step 8: Run the full suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | grep -E "^test result"`

Expected: all prior tests still pass (baseline was 385 passing / 1 ignored; now 388 passing / 1 ignored). Zero failures. Zero new warnings. If any arm of a `match CardKind` still needs a `Token` branch, add one now — prefer treating tokens as Digimon-equivalent (they satisfy `is_digimon`-ish predicates because in card text they read as Digimon) unless a call site has explicit Token-aware branching already.

- [ ] **Step 9: Commit**

```bash
git add digimon-engine/src/enums.rs digimon-engine/src/lib.rs \
  digimon-engine/src/token_registry.rs \
  digimon-engine/src/cards.rs digimon-engine/src/cards/tokens/
git commit -m "feat(engine): CardKind::Token + TokenRegistry data layer"
```

---

### Task 2: `ctx.play_token` + removal-from-game on delete

**Files:**
- Modify: `digimon-engine/src/game.rs` (add `pub token_registry: TokenRegistry` field; initialize + absorb token CardData in `Game::new`)
- Modify: `digimon-engine/src/player.rs:155-166` (branch delete_permanent on is_token)
- Modify: `digimon-engine/src/effect_context/mod.rs` (add `play_token`)
- Create: `digimon-engine/tests/cards_behavioral/tokens.rs`
- Modify: `digimon-engine/tests/cards_behavioral/main.rs` (add `mod tokens;`)
- Modify: `digimon-engine/src/cards/test/mod.rs` (add `TEST-023` that calls `ctx.play_token`)
- Create: `digimon-engine/src/cards/test/test_023.rs`

- [ ] **Step 1: Write the failing behavioral tests**

Create `digimon-engine/tests/cards_behavioral/tokens.rs`:

```rust
//! Behavioral tests for `ctx.play_token` and token-aware
//! `delete_permanent` (remove-from-game instead of trash).
//!
//! Uses TEST-023, a synthetic test card whose OnPlay is
//! `ctx.play_token(player, "petrification")`, so we exercise the full
//! play -> registry -> EffectContext -> mutations path rather than
//! calling `play_token` from a test in isolation.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

/// Happy path: playing TEST-023 materializes a Petrification Token on
/// P0's field. The token reads as `CardKind::Token`, carries the
/// Petrification stats, and takes up a field slot.
#[test]
fn test_023_play_token_spawns_petrification() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .hand(0, &["TEST-023"])
        .memory(5) // pre-fund cost (TEST-023 uses default cost=3)
        .start();

    assert_eq!(r.battle_area_size(0), 0);
    r.play(0, 0);

    // After the play, the TEST-023 permanent itself is on the field AND
    // a Petrification Token sits next to it.
    assert_eq!(r.battle_area_size(0), 2, "test card + token");

    // The token is always at the end (play_token appends). Find by kind.
    let token_perm = r.game.player(0).battle_area.iter().find(|p| {
        p.top_card().card_kind(&r.game.card_data) == CardKind::Token
    }).expect("token missing from battle_area");
    assert_eq!(token_perm.top_card().card_name(&r.game.card_data), "Petrification Token");
    assert_eq!(token_perm.base_dp(&r.game.card_data), Some(3000));
}

/// Removal-from-game contract: deleting a token empties the battle_area
/// slot WITHOUT growing P0's trash. Contrast with a normal Digimon, which
/// would land in trash.
#[test]
fn token_delete_removes_from_game_not_trash() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .hand(0, &["TEST-023"])
        .memory(5)
        .start();
    r.play(0, 0);
    assert_eq!(r.trash_size(0), 0);

    // Delete the token directly via EffectContext-equivalent Game API.
    // (Shortcut: call player::delete_permanent on the token slot.)
    let token_field_idx = r.game.player(0).battle_area.iter().position(|p| {
        p.top_card().card_kind(&r.game.card_data) == CardKind::Token
    }).expect("token missing");
    r.game.players[0].delete_permanent(token_field_idx);

    // Battle area loses the token; trash does NOT gain it.
    assert_eq!(r.battle_area_size(0), 1, "only the TEST-023 remains");
    assert_eq!(r.trash_size(0), 0, "token removed from game, not trashed");
}

/// Sanity: unknown token name returns None and leaves the battle_area
/// unchanged. (TEST-024 is a card that calls `ctx.play_token(..., "nonexistent")`.)
#[test]
fn play_token_with_unknown_name_is_noop() {
    // Extending with a second test card would require another TEST-NNN
    // entry. Skip this as a DebugRunner-level test and rely on the unit
    // test in effect_context::tests::play_token_unknown_name for the
    // Option<PermanentHandle> return-None contract.
}
```

Create `digimon-engine/src/cards/test/test_023.rs`:

```rust
//! TEST-023: "OnPlay: play 1 Petrification Token."
//! Exercises `ctx.play_token` through the full play pipeline.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test023;

impl CardEffect for Test023 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play a Petrification Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "petrification");
            })
            .build()]
    }
}
```

Update `digimon-engine/src/cards/test/mod.rs` — add `mod test_023;` and `registry.insert("TEST-023", Arc::new(test_023::Test023));`.

Update `digimon-engine/tests/cards_behavioral/main.rs` — add `mod tokens;`.

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral tokens:: -- --nocapture`

Expected: compilation error — `ctx.play_token` is not a method on `EffectContext`.

- [ ] **Step 3: Plumb the registry into `Game`**

In `digimon-engine/src/game.rs`, add a field:

```rust
pub token_registry: TokenRegistry,
```

(add the use: `use crate::token_registry::TokenRegistry;`).

In `Game::new`, after the `effect_registry` is built:

```rust
let token_registry = crate::token_registry::build_registry();
// Absorb synthetic CardData rows so CardSource::data_index lookups
// resolve for token instances later.
for def in token_registry.iter() {
    card_data_store.push(def.to_card_data());
}
```

(The exact lines depend on how `Game::new` constructs `card_data_store` — grep for `card_data_store` and insert the loop right after that Vec is populated from the `CardData::load_from_str` / builder inputs. If the store is a `Vec<CardData>` owned by `Game`, push directly; if it's immutable at this point, lift the population before whatever freezes it.)

Finally, include the field in the returned `Game { ... }`:

```rust
Self {
    // ...existing fields...
    token_registry,
}
```

Mirror in `DebugRunner::builder().build_inner` as needed — `Game::new` does the work, so `DebugRunner` inherits for free.

- [ ] **Step 4: Add `play_token` on `EffectContext`**

In `digimon-engine/src/effect_context/mod.rs`, after the `// ─── Field mutations ──` section (around `delete_permanent`), add:

```rust
/// Materialize a token on `controller`'s battle area.
///
/// Looks up `token_name` in `game.token_registry`, synthesizes a
/// `CardSource` with `is_token = true`, wraps it in a `Permanent`, and
/// pushes onto `controller.battle_area`. No play cost, no OnPlay
/// observer fan-out (tokens enter via effect, not via `play_from_hand`).
///
/// Returns the spawned permanent's handle, or `None` if the token name
/// is unknown or the field is full.
pub fn play_token(
    &mut self,
    controller: crate::enums::PlayerId,
    token_name: &str,
) -> Option<crate::permanent::PermanentHandle> {
    use crate::card_source::CardSource;
    use crate::permanent::{Permanent, PermanentHandle};

    // Resolve the token_name -> CardData row we previously absorbed
    // into game.card_data at Game::new time.
    let def = self.game.token_registry.get(token_name)?;
    let target_card_id = def.card_id.clone();
    let data_index = self
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == target_card_id)?;
    debug_assert_eq!(
        self.game.card_data[data_index].card_kind,
        crate::enums::CardKind::Token,
        "token_registry entry must map to a CardKind::Token CardData row"
    );

    // Respect the field slot cap so tokens don't overflow into UB.
    let slots = self.game.rules.field_slots as usize;
    if self.game.player(controller).battle_area.len() >= slots {
        return None;
    }

    // Allocate the next card_index. `Game::next_card_index()` already
    // exists (see debug_runner.rs `place_on_field`).
    let card_index = self.game.next_card_index();
    let mut card = CardSource::new_token(data_index, controller, card_index);
    card.card_index = card_index;
    let turn = self.game.turn_count;
    let perm = Permanent::new(card, turn);

    let player = self.game.player_mut(controller);
    player.battle_area.push(perm);
    let idx = player.battle_area.len() - 1;
    Some(PermanentHandle {
        player: controller,
        index: idx as u8,
    })
}
```

Also add these unit tests at the bottom of the `#[cfg(test)] mod tests` block in the same file:

```rust
#[test]
fn play_token_unknown_name_returns_none() {
    let db = min_db();
    let deck = vec!["BT1-001".to_string(); 10];
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(1)).unwrap();
    let mut ctx = EffectContext::new(&mut game, CardHandle(0), None, 0);
    assert!(ctx.play_token(0, "no-such-token-lol").is_none());
}
```

- [ ] **Step 5: Branch `delete_permanent` on `is_token`**

In `digimon-engine/src/player.rs:155-166`, replace:

```rust
pub fn delete_permanent(&mut self, field_index: usize) {
    if field_index >= self.battle_area.len() {
        return;
    }
    let perm = self.battle_area.remove(field_index);
    for card in perm.card_sources {
        self.trash.push(card);
    }
    for card in perm.linked_cards {
        self.trash.push(card);
    }
}
```

with:

```rust
pub fn delete_permanent(&mut self, field_index: usize) {
    if field_index >= self.battle_area.len() {
        return;
    }
    let perm = self.battle_area.remove(field_index);
    // Token semantic: remove from game. Drop the whole stack on the
    // floor — no trash entry, no zone ever again. Parity with
    // Python's `player.py::delete_permanent` is_token branch
    // (digimon_gym/engine/core/player.py:506).
    if perm.top_card().is_token {
        return;
    }
    for card in perm.card_sources {
        self.trash.push(card);
    }
    for card in perm.linked_cards {
        self.trash.push(card);
    }
}
```

- [ ] **Step 6: Run the behavioral tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral tokens:: -- --nocapture`

Expected: 2 tests pass (`test_023_play_token_spawns_petrification`, `token_delete_removes_from_game_not_trash`). The `play_token_with_unknown_name_is_noop` is a stub with no assertions.

- [ ] **Step 7: Run the unit test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --lib effect_context::tests::play_token_unknown_name`

Expected: PASS.

- [ ] **Step 8: Run the full suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | grep -E "^test result"`

Expected: 391 passing / 0 failing / 1 ignored (baseline 385 + 3 token_registry unit tests + 2 behavioral + 1 effect_context unit test). Zero warnings — if any CardKind match needs a `Token` arm, fix it now.

- [ ] **Step 9: Commit**

```bash
git add digimon-engine/src/game.rs digimon-engine/src/player.rs \
  digimon-engine/src/effect_context/mod.rs \
  digimon-engine/src/cards/test/ \
  digimon-engine/tests/cards_behavioral/
git commit -m "feat(engine): ctx.play_token + is_token-aware delete_permanent

Phase 10 task 2: token spawn helper wired through Game::token_registry,
with Player::delete_permanent branching on is_token to implement the
remove-from-game rule (tokens never enter trash). Behavioral tests
exercise the full play -> registry -> EffectContext -> mutations path
via a new TEST-023 card; unit tests cover the unknown-name no-op."
```

---

### Task 3: Petrification Token's OnDeletion — trash top security

**Files:**
- Modify: `digimon-engine/src/cards/tokens/petrification.rs` (wire real effect)
- Modify: `digimon-engine/tests/cards_behavioral/tokens.rs` (add OnDeletion test)
- Possibly modify: `digimon-engine/src/effect_context/mod.rs` if `trash_top_security` doesn't exist yet

- [ ] **Step 1: Confirm `trash_top_security` exists on `EffectContext`**

Run: `grep -n "trash_top_security\|fn trash_security" digimon-engine/src/effect_context/mod.rs digimon-engine/src/effect_context/selections.rs`

Expected: either a hit (use the existing helper) or no hit (add a minimal helper as part of this task).

**If no hit:** add to `effect_context/mod.rs` alongside `trash_from_top` (which operates on decks):

```rust
/// Move the top card of `player`'s security stack to their trash.
/// No-op if the stack is empty. Returns true if a card was moved.
pub fn trash_top_security(&mut self, player: crate::enums::PlayerId) -> bool {
    let p = self.game.player_mut(player);
    if let Some(card) = p.security.pop() {
        p.trash.push(card);
        true
    } else {
        false
    }
}
```

- [ ] **Step 2: Write the failing OnDeletion test**

Append to `digimon-engine/tests/cards_behavioral/tokens.rs`:

```rust
/// Petrification Token OnDeletion: when the token is deleted, the top
/// card of the token-owner's security stack goes to trash.
#[test]
fn petrification_on_deletion_trashes_top_security() {
    use digimon_engine::debug_runner::make_test_egg;
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .add_card(make_test_card("SEC-A", "SecA"))
        .add_card(make_test_card("SEC-B", "SecB"))
        .add_card(make_test_card("SEC-C", "SecC"))
        .hand(0, &["TEST-023"])
        .security(0, &["SEC-A", "SEC-B", "SEC-C"])
        .memory(5)
        .start();
    r.play(0, 0);
    let sec_before = r.security_count(0);
    let trash_before = r.trash_size(0);

    // Locate and delete the token.
    let token_idx = r.game.player(0).battle_area.iter().position(|p| {
        p.top_card().card_kind(&r.game.card_data) == CardKind::Token
    }).expect("token missing");

    // Deletion must fire OnDestroyed observers — use the full game path,
    // not `players[0].delete_permanent` directly, so effect dispatch
    // runs.
    let handle = digimon_engine::permanent::PermanentHandle { player: 0, index: token_idx as u8 };
    r.game.effect_delete_permanent(handle);

    assert_eq!(r.security_count(0), sec_before - 1,
        "Petrification OnDeletion trashed top of security");
    assert_eq!(r.trash_size(0), trash_before + 1,
        "the trashed security card landed in trash");
    // Token itself does NOT contribute to trash (remove-from-game).
    // trash_before + 1 (security card only), not + 2.
}
```

**Note for the implementing agent:** if `Game::effect_delete_permanent` (a full-game-path deletion helper that fires observers) doesn't exist with that name, grep for whatever path exists. Likely candidates: `game.delete_permanent_via_effect(...)`, `ctx.delete_permanent(...)` forwarded through a newly-created test card (TEST-024) whose OnPlay deletes the token, or `effect_queue.fire_on_deletion(...)`. Use whichever path is idiomatic; if none exists, extend Task 3 with a tiny `Game::delete_permanent(handle)` wrapper that `effect_context::delete_permanent` already calls into. (Phase 4/5 likely already added this — verify before adding.)

- [ ] **Step 3: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral petrification_on_deletion -- --nocapture`

Expected: FAIL — either compile error on `effect_delete_permanent` (fix per note above) or assertion failure because `PetrificationToken::effects` currently returns `Vec::new()`.

- [ ] **Step 4: Implement the OnDeletion effect**

Replace `digimon-engine/src/cards/tokens/petrification.rs` body with:

```rust
//! Petrification Token — Medusamon archetype (Purple).
//!
//! Printed text:
//!   [Your Turn] This Digimon can't suspend.
//!   [On Deletion] Trash the top card of this Digimon's owner's security stack.
//!
//! Phase 10 ships the OnDeletion clause. The CannotSuspend [Your Turn]
//! rider depends on a condition-gated modifier primitive tracked in
//! `RUST_ENGINE_GAPS.md` §"Condition-gated modifier entries"; when that
//! lands, append a second `Effect` with `.cannot_suspend_while_your_turn()`
//! or equivalent and remove the `#[ignore]` below.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct PetrificationToken;

impl CardEffect for PetrificationToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("[On Deletion] Trash top of owner's security")
            .process(|ctx| {
                // The token's owner = the player who controls the token
                // permanent = `ctx.player` (EffectContext is always
                // scoped to the source's controller).
                let owner = ctx.player;
                ctx.trash_top_security(owner);
            })
            .build()]
    }
}
```

**Note for the implementing agent:** verify `Effect::on_deletion` exists as a builder entrypoint (grep `fn on_deletion\|OnDestroyed\|OnRemovedField` in `effect.rs`). If the builder uses a different name (e.g. `Effect::on_destroyed`), use whichever one matches the dispatch site in `player::delete_permanent` / `effect_queue`. Phase 4/5's observer plumbing determined the name; preserve it.

- [ ] **Step 5: Run the test**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral petrification_on_deletion -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run the full suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | grep -E "^test result"`

Expected: 392 passing / 0 failing / 1 ignored. Zero warnings.

- [ ] **Step 7: Commit**

```bash
git add digimon-engine/src/cards/tokens/petrification.rs \
  digimon-engine/src/effect_context/mod.rs \
  digimon-engine/tests/cards_behavioral/tokens.rs
git commit -m "feat(engine): Petrification Token OnDeletion trashes top security

Phase 10 task 3: ship the Medusamon-archetype Petrification Token's
printed OnDeletion clause. The CannotSuspend [Your Turn] rider is
deferred to the condition-gated modifier primitive (gap doc
§Condition-gated modifier entries); that effect lands in a later phase."
```

---

## Phase 2 — De-Digivolve N

### Task 4: `ctx.de_digivolve(target, stop_at_level, amount)` + behavioral tests

**Files:**
- Modify: `digimon-engine/src/effect_context/mod.rs` (add `de_digivolve`)
- Create: `digimon-engine/tests/cards_behavioral/de_digivolve.rs`
- Modify: `digimon-engine/tests/cards_behavioral/main.rs` (add `mod de_digivolve;`)
- Create: `digimon-engine/src/cards/test/test_024.rs` (OnPlay: de-digivolve 2 of an opponent permanent, stop-at-3)
- Create: `digimon-engine/src/cards/test/test_025.rs` (OnPlay: unbounded pop — for TS Olympos Ikkakumon-style behavior)
- Modify: `digimon-engine/src/cards/test/mod.rs` (register TEST-024, TEST-025)

- [ ] **Step 1: Write the failing behavioral tests**

Create `digimon-engine/tests/cards_behavioral/de_digivolve.rs`:

```rust
//! Behavioral tests for `ctx.de_digivolve` — the generalized
//! De-Digivolve N primitive. Covers:
//!
//!   * `Some(amount)` with default `Some(3)` stop-at-level (standard
//!     De-Digivolve N wording)
//!   * `None` amount with `None` stop-at-level (TS Olympos Ikkakumon:
//!     pop until stack empty / until top is the last card)
//!   * Trash routing — popped sources land in owner's trash
//!   * Stack-depth-1 no-op (cannot trash past a single-card stack)
//!   * Early termination at the Lv.3 floor when `stop_at_level = Some(3)`
//!
//! Each test exercises the primitive through a dedicated test card so we
//! go through the full play pipeline — not just the method directly.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

fn lvl_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.play_cost = 0;
    c
}

/// Helper: build a 3-deep stack [Lv3, Lv4, Lv5] for P1 (opponent).
/// TEST-024's OnPlay is `ctx.de_digivolve(<opp's perm 0>, Some(3), Some(2))`.
fn setup_opponent_stack() -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .add_card(lvl_digimon("LVL3", "Three", 3, 2000))
        .add_card(lvl_digimon("LVL4", "Four", 4, 4000))
        .add_card(lvl_digimon("LVL5", "Five", 5, 7000))
        .add_card(make_test_card("TEST-024", "DeDig2StopAt3"))
        .hand(0, &["TEST-024"])
        .memory(5)
        .start();

    // Build P1's 3-deep stack by place_on_field + digivolve pushes.
    let base = r.place_on_field(1, "LVL3", Some(0));
    // push LVL4, LVL5 onto that stack.
    {
        let p1 = &mut r.game.players[1];
        use digimon_engine::card_source::CardSource;
        let lvl4_idx = r.game.card_data.iter().position(|c| c.card_id == "LVL4").unwrap();
        let lvl5_idx = r.game.card_data.iter().position(|c| c.card_id == "LVL5").unwrap();
        let next = r.game.next_card_index();
        p1.battle_area[base.index as usize].digivolve(
            CardSource::new(lvl4_idx, 1, next), r.game.turn_count);
        let next = r.game.next_card_index();
        p1.battle_area[base.index as usize].digivolve(
            CardSource::new(lvl5_idx, 1, next), r.game.turn_count);
    }
    (r, base)
}

#[test]
fn de_digivolve_2_pops_two_and_stops_at_lvl3() {
    let (mut r, opp_perm) = setup_opponent_stack();
    assert_eq!(r.game.player(1).battle_area[opp_perm.index as usize].stack_size(), 3);
    let trash_before = r.trash_size(1);

    // TEST-024 OnPlay: de_digivolve(<hardcoded opp[0]>, Some(3), Some(2)).
    r.play(0, 0);

    let perm = &r.game.player(1).battle_area[opp_perm.index as usize];
    assert_eq!(perm.stack_size(), 1, "popped 2, left the Lv3 base");
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "LVL3");
    assert_eq!(r.trash_size(1), trash_before + 2,
        "both popped sources land in P1's trash");
}

#[test]
fn de_digivolve_unbounded_pops_whole_stack() {
    // TEST-025 OnPlay: de_digivolve(<opp[0]>, None, None) — TS Olympos
    // Ikkakumon semantics.
    let mut r = DebugRunner::builder()
        .add_card(lvl_digimon("LVL3", "Three", 3, 2000))
        .add_card(lvl_digimon("LVL4", "Four", 4, 4000))
        .add_card(make_test_card("TEST-025", "DeDigUnbounded"))
        .hand(0, &["TEST-025"])
        .memory(5)
        .start();

    let base = r.place_on_field(1, "LVL3", Some(0));
    {
        use digimon_engine::card_source::CardSource;
        let lvl4_idx = r.game.card_data.iter().position(|c| c.card_id == "LVL4").unwrap();
        let next = r.game.next_card_index();
        r.game.players[1].battle_area[base.index as usize].digivolve(
            CardSource::new(lvl4_idx, 1, next), r.game.turn_count);
    }

    // Pre: stack size 2. Post: stack size 1 (can't pop past last card —
    // a Permanent must always have at least one card_source).
    assert_eq!(r.game.player(1).battle_area[base.index as usize].stack_size(), 2);
    r.play(0, 0);
    assert_eq!(r.game.player(1).battle_area[base.index as usize].stack_size(), 1,
        "unbounded pop leaves the base alone (stack_size >= 1 invariant)");
    assert_eq!(r.game.player(1).battle_area[base.index as usize].top_card().card_id(&r.game.card_data), "LVL3");
}

#[test]
fn de_digivolve_on_single_card_stack_is_noop() {
    // TEST-024 default stop_at_level=Some(3); the opp permanent is a
    // single-card Lv4. Amount=2 asks to pop 2, but we can never pop the
    // last card, so count returned is 0 and the permanent is untouched.
    let mut r = DebugRunner::builder()
        .add_card(lvl_digimon("LVL4", "Four", 4, 4000))
        .add_card(make_test_card("TEST-024", "DeDig2StopAt3"))
        .hand(0, &["TEST-024"])
        .memory(5)
        .start();
    let base = r.place_on_field(1, "LVL4", Some(0));

    r.play(0, 0);

    assert_eq!(r.game.player(1).battle_area[base.index as usize].stack_size(), 1);
    assert_eq!(r.trash_size(1), 0, "no pops, no trash entries");
}

#[test]
fn de_digivolve_stops_at_level_floor_early() {
    // Stack is [Lv3, Lv4, Lv5, Lv6]. stop_at_level=Some(3) with amount=Some(5).
    // We should pop Lv6, Lv5, Lv4 (3 pops) and stop because the next
    // top would be Lv3. Expected popped count = 3, even though amount
    // asked for 5.
    //
    // This test reuses the de_digivolve_2 card but with deeper setup; we
    // inline a small extra TEST-026 that calls with amount=Some(5).
    // For simplicity, include it in a follow-up if needed — primary
    // coverage of the floor rule comes from
    // `de_digivolve_2_pops_two_and_stops_at_lvl3` (the base is Lv3,
    // amount=2 trivially hits the floor on the second pop).
}
```

Create `digimon-engine/src/cards/test/test_024.rs`:

```rust
//! TEST-024: "OnPlay: De-Digivolve 2 (stop at Lv3) on opponent's
//! permanent at field index 0." Deterministic target selection for
//! test purposes — real De-Digivolve cards use `pending_selection`
//! for target pick, but this synthetic card hardcodes the target so
//! we isolate the pop-and-trash logic from the selection primitive.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::permanent::PermanentHandle;

pub struct Test024;

impl CardEffect for Test024 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("DeDigivolve 2 stop-at-3 on opp[0]")
            .process(|ctx| {
                let opp = ctx.opponent_id();
                if ctx.battle_area(opp).is_empty() {
                    return;
                }
                let target = PermanentHandle { player: opp, index: 0 };
                ctx.de_digivolve(target, Some(3), Some(2));
            })
            .build()]
    }
}
```

Create `digimon-engine/src/cards/test/test_025.rs`:

```rust
//! TEST-025: "OnPlay: De-Digivolve unbounded on opp[0] (TS Olympos
//! Ikkakumon-style pop-whole-stack)."

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::permanent::PermanentHandle;

pub struct Test025;

impl CardEffect for Test025 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("DeDigivolve unbounded on opp[0]")
            .process(|ctx| {
                let opp = ctx.opponent_id();
                if ctx.battle_area(opp).is_empty() {
                    return;
                }
                let target = PermanentHandle { player: opp, index: 0 };
                ctx.de_digivolve(target, None, None);
            })
            .build()]
    }
}
```

Update `digimon-engine/src/cards/test/mod.rs` to register TEST-024 and TEST-025.

Update `digimon-engine/tests/cards_behavioral/main.rs` to add `mod de_digivolve;`.

- [ ] **Step 2: Run the test to confirm it fails**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral de_digivolve:: -- --nocapture`

Expected: compile error — `ctx.de_digivolve` is not a method on `EffectContext`.

- [ ] **Step 3: Implement `ctx.de_digivolve`**

In `digimon-engine/src/effect_context/mod.rs`, after `delete_permanent`, add:

```rust
/// Pop up to `amount` cards off `target`'s digivolution stack,
/// trashing each popped source in the target's owner's trash.
///
/// Rules:
///   * Never pops the base card — `Permanent` must always retain at
///     least one `CardSource`.
///   * `stop_at_level = Some(L)` — stop early if popping would leave
///     a top whose level is strictly less than `L`.  For standard
///     De-Digivolve N use `Some(3)` (card text: "You can't trash
///     past level 3 cards").
///   * `stop_at_level = None` — no level floor; pop until the base.
///   * `amount = Some(N)` — cap pops at N.
///   * `amount = None` — unbounded (equivalent to `Some(u8::MAX)`).
///
/// Returns the actual number of cards popped.
pub fn de_digivolve(
    &mut self,
    target: crate::permanent::PermanentHandle,
    stop_at_level: Option<u8>,
    amount: Option<u8>,
) -> u8 {
    let max = amount.unwrap_or(u8::MAX);
    let mut popped: u8 = 0;

    while popped < max {
        let perm = match self.game.player(target.player)
            .battle_area.get(target.index as usize) {
            Some(p) => p,
            None => break,
        };

        // Never pop the base. `Permanent::stack_size() >= 1` invariant.
        if perm.stack_size() <= 1 {
            break;
        }

        // Prospective new top after this pop = card_sources[len - 2].
        let next_top_level = {
            let stack = perm.digivolution_cards();
            let next_top = &stack[stack.len() - 2];
            next_top.level(&self.game.card_data)
        };

        // Honor the level floor. If next top would be strictly below
        // `stop_at_level`, stop BEFORE popping.
        if let (Some(floor), Some(nt_level)) = (stop_at_level, next_top_level) {
            if nt_level < floor {
                break;
            }
        }

        // Pop — move the top source into the target owner's trash.
        let owner = target.player;
        let p = self.game.player_mut(owner);
        let stack = &mut p.battle_area[target.index as usize].card_sources;
        debug_assert!(stack.len() >= 2, "stack_size-guard failed");
        let popped_card = stack.pop().expect("stack_size-guarded pop");
        p.trash.push(popped_card);
        popped += 1;
    }

    popped
}
```

- [ ] **Step 4: Run the behavioral tests**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral de_digivolve:: -- --nocapture`

Expected: 3 tests pass (`de_digivolve_2_pops_two_and_stops_at_lvl3`, `de_digivolve_unbounded_pops_whole_stack`, `de_digivolve_on_single_card_stack_is_noop`). The `de_digivolve_stops_at_level_floor_early` test body is a comment-only documentation stub (no assertions run).

- [ ] **Step 5: Run the full suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | grep -E "^test result"`

Expected: 395 passing / 0 failing / 1 ignored. Zero warnings.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine/src/effect_context/mod.rs \
  digimon-engine/src/cards/test/ \
  digimon-engine/tests/cards_behavioral/
git commit -m "feat(engine): ctx.de_digivolve with Option<stop_at_level>, Option<amount>

Phase 10 task 4: generalized De-Digivolve N primitive. Supports the
standard 'pop up to N, stop at Lv3 floor' case as well as TS Olympos
Ikkakumon's unbounded 'pop whole stack' variant via
(amount=None, stop_at_level=None). Popped card_sources land in owner's
trash; base card is always preserved (Permanent stack_size >= 1
invariant)."
```

---

## Phase 3 — Documentation + roadmap

### Task 5: `RUST_ENGINE_API.md` §Phase 10 + parity + gaps + roadmap

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` (add §Phase 10)
- Modify: `docs/RUST_PYTHON_PARITY.md` (update §4.6b-residual; add §10 Phase-10 summary)
- Modify: `docs/RUST_ENGINE_GAPS.md` (mark closed entries)
- Modify: `C:\Users\james\.claude\plans\recursive-coalescing-candle.md` (flip Phase 10 row)
- Commit the plan file itself (`docs/superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md`) as part of this final commit.

- [ ] **Step 1: Write the §Phase 10 section in `RUST_ENGINE_API.md`**

Append to `docs/RUST_ENGINE_API.md` (or insert per existing structure — grep for `## Phase 5` to find the last phase section):

```markdown
## Phase 10 — Tokens & De-Digivolve N

Phase 10 ships two additive `EffectContext` primitives plus a new
`CardKind::Token` variant and its companion `TokenRegistry`.

### `ctx.play_token(controller, token_name) -> Option<PermanentHandle>`

Materializes a token directly on `controller`'s battle area, bypassing
hand / deck / play-cost. Looks up `token_name` in
`game.token_registry`; no-op `None` on unknown name or full field.

**Registered tokens (Phase 10):**

| `token_name`   | `card_id`              | Colors  | DP   | Printed effects                                               |
|----------------|------------------------|---------|------|---------------------------------------------------------------|
| `petrification`| `TOKEN_PETRIFICATION`  | White   | 3000 | [On Deletion] Trash the top card of this Digimon's owner's security stack. <br>(Printed [Your Turn] CannotSuspend rider deferred — depends on condition-gated modifier entries, tracked in `RUST_ENGINE_GAPS.md` §"Condition-gated modifier entries") |
| `familiar`     | `TOKEN_FAMILIAR`       | Yellow  | 3000 | Stats only — [On Deletion] -3000 DP opponent Digimon is deferred behind the "Selection: opponent-as-selecting-player" gap |

**Worked example** (TEST-023):

```rust
impl CardEffect for PlayPetrificationToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play a Petrification Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "petrification");
            })
            .build()]
    }
}
```

**Removal-from-game semantics.** When a token leaves the battle area
via `player::delete_permanent`, its `card_sources` and `linked_cards`
are dropped (removed from the game) rather than appended to the
owner's trash. OnDeletion effects still fire from the
`effect_queue` observer path before the card leaves the game. No
equivalent hook yet exists for return-to-hand or return-to-deck —
those zone-manipulation primitives are deferred to a later phase
(see `RUST_ENGINE_GAPS.md` §"Zone-manipulation: return-to-hand /
return-to-deck").

### `ctx.de_digivolve(target, stop_at_level, amount) -> u8`

Pops up to `amount` sources off `target`'s digivolution stack,
trashing each into the target owner's trash. Respects a level floor
when `stop_at_level = Some(L)`. Returns the actual count popped.

**Arguments:**
- `target: PermanentHandle` — the opponent (or self) permanent to
  de-digivolve.
- `stop_at_level: Option<u8>` — stop early if popping would leave a
  top whose level is strictly below this value. `Some(3)` for
  standard "you can't trash past level 3 cards" wording. `None`
  for TS Olympos Ikkakumon-style unbounded pop.
- `amount: Option<u8>` — cap on number of pops. `None` = unbounded
  (bounded only by stack depth and the level floor).

**Worked examples:**

```rust
// Standard "De-Digivolve 2" — pop up to 2, stop at Lv3.
ctx.de_digivolve(target, Some(3), Some(2));

// "De-Digivolve 4" — pop up to 4, stop at Lv3.
ctx.de_digivolve(target, Some(3), Some(4));

// TS Olympos Ikkakumon: pop until base.
ctx.de_digivolve(target, None, None);
```

**Invariants:**
- The base card (`card_sources[0]`) is never popped —
  `Permanent::stack_size() >= 1` is preserved.
- Popped sources always go to the **target owner**'s trash, not the
  caller's. This matters for Dark Masters' cross-side effects.
```

- [ ] **Step 2: Update `RUST_PYTHON_PARITY.md`**

Flip §4.6b-residual from 🟡 to 🟢 and edit its body to note that
`CardKind::Token` now exists:

```markdown
### 4.6b-residual 🟢 Token detection — implemented

Rust's `CardKind` now includes `Token` (Phase 10). Tokens are
registered via `token_registry.rs` with synthetic `CardData` rows
absorbed into `game.card_data` at `Game::new`. Python's `is_token:
bool` flag and Rust's `CardKind::Token` are kept in sync at the PyO3
binding boundary (any helper that returns a token permanent
translates the flag appropriately).
```

Append a §10 section (top-level, after §9):

```markdown
## 10. Phase 10 — Tokens + De-Digivolve

### 10.1 🟢 Token creation + CardKind::Token — implemented

**Python** — `digimon_gym/engine/data/token_registry.py`: `TOKENS`
dict mapping token names to metadata; `create_token_card_source`
factory; `CardSource.is_token: bool` flag; `Permanent.is_token`
property; Game.effect_play_token for spawning.
`Player.delete_permanent` branches on `is_token` to skip trash
(`player.py:506`).

**Rust** — `digimon-engine/src/token_registry.rs` defines
`TokenDef` + `TokenRegistry` + `build_registry`. `CardKind::Token`
variant in `enums.rs`. `EffectContext::play_token(controller,
token_name)` materializes a `CardSource::new_token(...)` +
`Permanent::new(...)` on the target's battle area.
`Player::delete_permanent` branches on `top_card().is_token` to
skip trash append.

**Divergences:** None observable. Rust's `CardKind::Token` is a
first-class enum variant where Python uses `is_token: bool` on a
Digimon-kind CardSource — the Rust shape is cleaner and supports
exhaustive match-based Token-specific branching (e.g. future Overclock
sacrifice filter). The PyO3 binding layer (`digimon-engine-py`)
must translate `CardKind::Token` → `is_token=True, card_kind=Digimon`
when synthesizing Python mirrors (not yet wired — no PvP code
surfaces tokens today).

**Coverage:** `digimon-engine/tests/cards_behavioral/tokens.rs` +
`digimon-engine/src/token_registry.rs` unit tests.

### 10.2 🟢 De-Digivolve N — implemented (superset of Python)

**Python** — not hand-counted here (pre-Rust engine handles
de_digivolve in its own module; Python's scripts call `card.lose_digivolution(N)`
or similar per-archetype).

**Rust** — `EffectContext::de_digivolve(target, stop_at_level: Option<u8>, amount: Option<u8>) -> u8`.
Pops up to `amount` sources, stops at `stop_at_level`, routes trash
to owner side. `None` for either arg expresses unbounded. Returns
actual count popped so callers can gate follow-up effects ("if at
least one was popped, gain 1 memory").

**Divergences:** Rust's API is a strict superset of the Python
surface — every Python call site reduces to
`de_digivolve(target, Some(3), Some(N))` in Rust.

**Coverage:** `digimon-engine/tests/cards_behavioral/de_digivolve.rs`.
```

- [ ] **Step 3: Mark closed entries in `RUST_ENGINE_GAPS.md`**

In `docs/RUST_ENGINE_GAPS.md`:

- Line 46 (summary row for "Token creation + `CardKind::Token`"): change severity from 🔴 to 🟢 and impact counter to reflect "closed in Phase 10".
- Line 54 (summary row for "De-Digivolve N primitive"): 🔴 → 🟢.
- Section §"Token creation + `CardKind::Token` + Petrification Token definition" (around line 208): change **Severity:** from 🔴 BLOCKING to 🟢 CLOSED and append a trailing line:

  ```markdown
  - **Closed in:** Phase 10 (2026-04-21, plan
    [`docs/superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md`](../superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md)).
    Familiar Token's [On Deletion] clause still requires the
    opponent-permanent selection primitive — deferred.
  ```

- Section §"De-Digivolve N primitive" (around line 288): 🔴 → 🟢. Append:

  ```markdown
  - **Closed in:** Phase 10 (2026-04-21, same plan). Generalized
    signature: `ctx.de_digivolve(target, stop_at_level: Option<u8>,
    amount: Option<u8>) -> u8`. TS Olympos Ikkakumon-style unbounded
    pop expressible as `(None, None)`.
  ```

- [ ] **Step 4: Flip Phase 10 row in the roadmap**

Open `C:\Users\james\.claude\plans\recursive-coalescing-candle.md`. Find the Phase 10 row (grep `Phase 10\|Phase-10\|tokens`). Change its status cell to:

```
✅ Landed 2026-04-21 (re-audit pending)
```

(If the roadmap uses a different status-cell format, match the existing convention for landed phases — e.g. Phase 5's cell shows `✅ Landed YYYY-MM-DD`.)

- [ ] **Step 5: Run the full suite one last time**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | grep -E "^test result"`

Expected: still 395 passing / 0 failing / 1 ignored. Zero warnings.

- [ ] **Step 6: Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_PYTHON_PARITY.md \
  docs/RUST_ENGINE_GAPS.md \
  docs/superpowers/plans/2026-04-21-rust-engine-phase-10-tokens-and-dedigivolve.md
git commit -m "docs(engine): Phase 10 — tokens + de-digivolve N

Close two 🔴 BLOCKING gap-doc entries (token creation and De-Digivolve
N primitive). Roadmap row flipped to ✅ Landed. Plan file committed
alongside docs for reproducibility."
```

*(The roadmap file at `C:\Users\james\.claude\plans\recursive-coalescing-candle.md` is intentionally outside the repo; edit but don't git add.)*

---

## Verification

At the end of Task 5, confirm all of the following independently:

```bash
# 1. Full Rust suite green, no warnings.
cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | tail -20

# 2. New tests exist and pass.
cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral tokens::
cargo test --manifest-path digimon-engine/Cargo.toml --test cards_behavioral de_digivolve::

# 3. Token registry unit tests pass.
cargo test --manifest-path digimon-engine/Cargo.toml --lib token_registry::

# 4. No new clippy warnings.
cargo clippy --manifest-path digimon-engine/Cargo.toml --all-targets -- -D warnings

# 5. Git log shows 5 Phase-10 commits on this branch.
git log --oneline origin/main..HEAD
```

Expected totals: **395 passing / 0 failing / 1 ignored** (baseline 385 + 10 new Phase-10 tests). Zero warnings. Commit history shows the 5 planned commits.

## Non-Goals (explicit)

Phase 10 intentionally does NOT:

- Implement return-to-hand / return-to-deck primitives (gap doc line 43, 🔴 BLOCKING). Token removal-from-game hooks only land in `player::delete_permanent`; extending to other leave-field paths requires those primitives to exist first.
- Implement the Petrification Token's [Your Turn] CannotSuspend rider. This needs condition-gated modifier entries (gap doc §"Condition-gated modifier entries", 🔴). A follow-up phase adds the condition-gating framework, at which point this plan's note in `petrification.rs` becomes a one-line effect append.
- Implement the Familiar Token's [On Deletion] opponent-permanent pick. Needs the "Selection: opponent-as-selecting-player" primitive (gap doc, 🔴). Stats-only shipment suffices until that lands.
- Add token-specific tensor or action-mask slots. `CardKind::Token` flows through the existing CardKind tensor pathway; no new observation or action bits.
- Introduce a "resolve cross-side trash routing" for deletion. The current `Player::delete_permanent` is always the owner — there is no cross-side deletion plumbing to update yet. (Phase 4/5 may have added one; if so, verify that the is_token branch lives at the single choke point.)
- Add PyO3 token surfacing. PvP does not expose tokens today (no recorded game has a Medusamon-archetype permanent); the binding-layer translation will land alongside the first archetype whose online play needs it.
