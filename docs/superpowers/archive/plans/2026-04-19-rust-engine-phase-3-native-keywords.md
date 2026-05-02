# Rust Engine Phase 3 — Native Keyword Parsing

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Parse printed keywords from a card's `effect_text` / `inherited_text` / `security_text` at registry-build time into a `CardData::keywords: Vec<Keyword>` field. Add a unified `Game::has_keyword(handle, Keyword)` query that checks BOTH modifier-granted AND native printed keywords. Update every keyword check site to use it. Closes parity §2.1b (Rush) and §2.5f (Jamming).

**Architecture:** Keywords in card text appear as `＜Keyword＞` (full-width angle brackets). A new `parse_printed_keywords` function scans the three text fields and returns a deduplicated `Vec<Keyword>`. The parser handles parametric variants (`Security A. +N`, `De-Digivolve N`). The new `Game::has_keyword` method is the single canonical lookup — callers no longer access `game.modifiers.has_keyword` directly (that becomes a private implementation detail).

**Tech Stack:** Rust 2021, `digimon-engine` crate.

**Roadmap context:** Phase 3 in [.claude/plans/recursive-coalescing-candle.md](../../../.claude/plans/recursive-coalescing-candle.md). Unblocks ~100 cards across all 5 archetypes.

---

## Current State

From the Phase 3 context inventory:
- `Keyword` enum at `enums.rs:197` — 25 variants, including already-parametric `SecurityAttackPlus(i8)`, `SecurityAttackMinus(i8)`, `DeDigivolve(u8)`, `DrawX(u8)`.
- `CardData` struct at `card_data.rs:44` — no `keywords` field yet.
- 13 keyword-check sites use `game.modifiers.has_keyword(handle, Keyword::X)` — enumerated in the plan's §3 inventory.
- `CardData::load_from_str` at `card_data.rs:161` — the insertion point for the parser.
- Parity §2.1b and §2.5f block on this phase.

---

## Task 1: Add `CardData::keywords` field + keyword text parser

**Files:**
- Modify: `digimon-engine/src/card_data.rs` — add `keywords` field + `parse_printed_keywords` function + wire into `load_from_str`
- Create: `digimon-engine/tests/keyword_parsing.rs` — unit tests for the parser

**Steps:**

- [ ] **1.1 Add `keywords: Vec<Keyword>` field to `CardData` struct**

In `digimon-engine/src/card_data.rs`, in the `pub struct CardData { ... }` block, add after the `security_text` field:

```rust
/// Keywords printed on the card's face (Rush, Jamming, Blocker, etc.),
/// parsed from effect_text / inherited_text / security_text at load
/// time. Distinct from modifier-granted keywords managed by
/// `ModifierRegistry`. The unified `Game::has_keyword` query considers
/// both.
#[serde(default)]
pub keywords: Vec<Keyword>,
```

(Import `Keyword` from `crate::enums` at the top of the file if not already present.)

- [ ] **1.2 Write `parse_printed_keywords` function**

Add at the bottom of `card_data.rs`:

```rust
/// Extract printed keywords from a card's text fields. Keywords appear
/// in card text as `＜Keyword＞` (full-width angle brackets) optionally
/// followed by a parenthetical English description. Parametric
/// keywords (`Security A. +N`, `De-Digivolve N`) are parsed into their
/// typed variants.
///
/// Returns a deduplicated Vec in document order.
pub fn parse_printed_keywords(
    effect_text: &str,
    inherited_text: &str,
    security_text: &str,
) -> Vec<crate::enums::Keyword> {
    use crate::enums::Keyword;

    let mut found: Vec<Keyword> = Vec::new();
    let mut push_unique = |k: Keyword, found: &mut Vec<Keyword>| {
        if !found.contains(&k) {
            found.push(k);
        }
    };

    for text in [effect_text, inherited_text, security_text] {
        // Iterate every `＜...＞` substring.
        let mut remaining = text;
        while let Some(start) = remaining.find('＜') {
            let after_open = &remaining[start + '＜'.len_utf8()..];
            let Some(end) = after_open.find('＞') else { break };
            let inside = &after_open[..end];
            remaining = &after_open[end + '＞'.len_utf8()..];

            // Parse the content. Match prefixes (case-insensitive
            // starts-with) for each known keyword. Some are parametric.
            let trimmed = inside.trim();

            // Non-parametric keywords:
            for (prefix, kw) in [
                ("Rush", Keyword::Rush),
                ("Jamming", Keyword::Jamming),
                ("Blocker", Keyword::Blocker),
                ("Piercing", Keyword::Piercing),
                ("Reboot", Keyword::Reboot),
                ("Blitz", Keyword::Blitz),
                ("Armor", Keyword::Armor),
                ("Raid", Keyword::Raid),
                ("Alliance", Keyword::Alliance),
                ("Blast Digivolve", Keyword::Blast),
                ("Save", Keyword::Save),
                ("Fortitude", Keyword::Fortitude),
                ("Overclock", Keyword::Overclock),
                ("Barrier", Keyword::Barrier),
                ("Decoy", Keyword::Decoy),
                ("Material", Keyword::Material),
                ("Partition", Keyword::Partition),
                ("Vortex", Keyword::Vortex),
                ("Collision", Keyword::Collision),
            ] {
                if trimmed.starts_with(prefix) {
                    push_unique(kw, &mut found);
                    break;
                }
            }

            // Parametric: Security A. +N / -N
            if let Some(rest) = trimmed.strip_prefix("Security A.") {
                let rest = rest.trim();
                let (sign, digits) = if let Some(d) = rest.strip_prefix('+') {
                    (1i8, d)
                } else if let Some(d) = rest.strip_prefix('-') {
                    (-1i8, d)
                } else {
                    (1i8, rest)
                };
                if let Ok(n) = digits.trim().parse::<u8>() {
                    if sign < 0 {
                        push_unique(Keyword::SecurityAttackMinus(n as i8), &mut found);
                    } else {
                        push_unique(Keyword::SecurityAttackPlus(n as i8), &mut found);
                    }
                }
            }

            // Parametric: De-Digivolve N
            if let Some(rest) = trimmed.strip_prefix("De-Digivolve") {
                let digits = rest.trim();
                // Handle "De-Digivolve 2" and "De-Digivolve2" variants
                let n_str = digits.split_whitespace().next().unwrap_or(digits);
                if let Ok(n) = n_str.parse::<u8>() {
                    push_unique(Keyword::DeDigivolve(n), &mut found);
                }
            }

            // Parametric: Draw N
            if let Some(rest) = trimmed.strip_prefix("Draw") {
                let digits = rest.trim().split_whitespace().next().unwrap_or("");
                if let Ok(n) = digits.parse::<u8>() {
                    push_unique(Keyword::DrawX(n), &mut found);
                }
            }
        }
    }

    found
}
```

- [ ] **1.3 Wire the parser into `CardData::load_from_str`**

Find the `CardData { ... }` constructor inside `load_from_str` (around line 173). After `security_text: raw_card.security_effect_description_eng,` add:

```rust
keywords: parse_printed_keywords(
    &raw_card.effect_description_eng,
    &raw_card.inherited_effect_description_eng,
    &raw_card.security_effect_description_eng,
),
```

(The field names `effect_description_eng` etc. are on the `RawCard` intermediate struct used for JSON deserialization.)

- [ ] **1.4 Create `digimon-engine/tests/keyword_parsing.rs`**

```rust
//! Phase 3 native-keyword parser tests.

use digimon_engine::card_data::parse_printed_keywords;
use digimon_engine::enums::Keyword;

#[test]
fn parses_rush() {
    let kw = parse_printed_keywords(
        "＜Rush＞ (This Digimon can attack the turn it comes into play.)",
        "",
        "",
    );
    assert_eq!(kw, vec![Keyword::Rush]);
}

#[test]
fn parses_jamming_in_inherited() {
    let kw = parse_printed_keywords("", "＜Jamming＞ (...)", "");
    assert_eq!(kw, vec![Keyword::Jamming]);
}

#[test]
fn parses_multiple_keywords_in_same_field() {
    let kw = parse_printed_keywords(
        "＜Raid＞ (When this Digimon attacks, you may...)\r\n＜Piercing＞ (...)",
        "",
        "",
    );
    assert!(kw.contains(&Keyword::Raid));
    assert!(kw.contains(&Keyword::Piercing));
}

#[test]
fn dedupes_same_keyword_in_multiple_fields() {
    let kw = parse_printed_keywords(
        "＜Rush＞ (...)",
        "＜Rush＞ (...)",
        "",
    );
    assert_eq!(kw, vec![Keyword::Rush]);
}

#[test]
fn parses_security_attack_plus() {
    let kw = parse_printed_keywords("＜Security A. +1＞ (...)", "", "");
    assert_eq!(kw, vec![Keyword::SecurityAttackPlus(1)]);
}

#[test]
fn parses_security_attack_minus() {
    let kw = parse_printed_keywords("＜Security A. -2＞ (...)", "", "");
    assert_eq!(kw, vec![Keyword::SecurityAttackMinus(2)]);
}

#[test]
fn parses_de_digivolve_with_arg() {
    let kw = parse_printed_keywords("＜De-Digivolve 2＞ (...)", "", "");
    assert_eq!(kw, vec![Keyword::DeDigivolve(2)]);
}

#[test]
fn ignores_unrecognized_keywords() {
    let kw = parse_printed_keywords("＜MadeUpKeyword＞ (...)", "", "");
    assert!(kw.is_empty());
}

#[test]
fn handles_empty_input() {
    assert!(parse_printed_keywords("", "", "").is_empty());
}

#[test]
fn parses_blocker_and_security_attack_together() {
    let kw = parse_printed_keywords(
        "＜Blocker＞ (...)\r\n＜Security A. +1＞ (...)",
        "",
        "",
    );
    assert!(kw.contains(&Keyword::Blocker));
    assert!(kw.contains(&Keyword::SecurityAttackPlus(1)));
}
```

- [ ] **1.5 Run tests; verify all pass**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_parsing 2>&1 | tail -20
cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | tail -5
```

- [ ] **1.6 Commit**

```bash
git add digimon-engine/src/card_data.rs digimon-engine/tests/keyword_parsing.rs
git commit -m "feat(engine): Phase 3 — CardData.keywords field + parse_printed_keywords

Adds native-keyword parsing at card load time. Parses ＜Keyword＞
patterns from effect_text / inherited_text / security_text into
a Vec<Keyword>. Handles parametric variants (Security A. ±N,
De-Digivolve N, Draw N).

Part of Phase 3 (Cluster C)."
```

---

## Task 2: Add `Game::has_keyword` unified query

**Files:**
- Modify: `digimon-engine/src/game.rs` — add `has_keyword(&self, handle, Keyword) -> bool`
- Modify: `digimon-engine/tests/keyword_parsing.rs` — integration test

**Steps:**

- [ ] **2.1 Failing integration test**

Append to `tests/keyword_parsing.rs`:

```rust
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind, Expiry};
use digimon_engine::permanent::PermanentHandle;

fn digimon_with_text(card_id: &str, effect_text: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: effect_text.to_string(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        keywords: digimon_engine::card_data::parse_printed_keywords(
            effect_text, "", ""
        ),
    }
}

#[test]
fn game_has_keyword_sees_native_printed() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text("NATIVE_RUSH", "＜Rush＞ (...)"))
        .hand(0, &["NATIVE_RUSH"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };

    assert!(
        r.game_mut().has_keyword(handle, digimon_engine::enums::Keyword::Rush),
        "Game::has_keyword should see native printed Rush"
    );
}

#[test]
fn game_has_keyword_sees_modifier_granted() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text("NO_NATIVE", ""))
        .hand(0, &["NO_NATIVE"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    r.game_mut().modifiers.grant_keyword(
        handle,
        digimon_engine::enums::Keyword::Rush,
        Expiry::EndOfTurn,
        0,
    );

    assert!(r.game_mut().has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}

#[test]
fn game_has_keyword_false_when_neither() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text("NEITHER", ""))
        .hand(0, &["NEITHER"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    assert!(!r.game_mut().has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}
```

- [ ] **2.2 Verify compile fail**

- [ ] **2.3 Add `Game::has_keyword` in `game.rs`**

Find a suitable section (near the existing `suspend`/`unsuspend` methods from Phase 1, around line 569+). Add:

```rust
/// Unified keyword query — returns true if the permanent's top card
/// has `keyword` printed natively on its face (from CardData.keywords)
/// OR has it granted by an active modifier.
///
/// This is the canonical lookup. Call sites should NOT use
/// `self.modifiers.has_keyword(...)` directly — that only sees granted
/// keywords and will miss native printed keywords.
pub fn has_keyword(&self, handle: PermanentHandle, keyword: crate::enums::Keyword) -> bool {
    // Modifier-granted (end-of-turn, granted by effect, etc.)
    if self.modifiers.has_keyword(handle, keyword) {
        return true;
    }
    // Native printed on the top card's face.
    let Some(player) = self.players.get(handle.player as usize) else {
        return false;
    };
    let Some(perm) = player.battle_area.get(handle.index as usize) else {
        return false;
    };
    let top = perm.top_card();
    let Some(data) = self.card_data.iter().find(|d| d.card_id == top.card_id(&self.card_data)) else {
        return false;
    };
    data.keywords.contains(&keyword)
}
```

**IMPORTANT:** the `card_data` field may be a `Vec<CardData>` or a `HashMap<String, CardData>` — check the actual shape. If `HashMap`, use `.get(top.card_id(...))` instead of `.iter().find(...)`.

Also verify `top.card_id(&self.card_data)` returns the card_id string — the accessor might be different.

- [ ] **2.4 Run tests; verify PASS**

- [ ] **2.5 Commit**

```bash
git add digimon-engine/src/game.rs digimon-engine/tests/keyword_parsing.rs
git commit -m "feat(engine): Phase 3 — Game::has_keyword unified query

Canonical keyword lookup: checks modifier-granted AND native printed
keywords. Call sites migrate to this in the next task."
```

---

## Task 3: Migrate 13 keyword check sites to `Game::has_keyword`

**Files:**
- Modify: `digimon-engine/src/combat.rs` (5 sites)
- Modify: `digimon-engine/src/action/mask.rs` (6 sites)
- Modify: `digimon-engine/src/game_phases.rs` (3 sites)
- Modify: `digimon-engine/tests/keyword_parsing.rs` — regression tests for printed Rush / Jamming

**Steps:**

- [ ] **3.1 Write behavioral regression tests** asserting that cards with PRINTED Rush / Jamming / Blocker / Raid exhibit correct behavior:

```rust
#[test]
fn native_printed_rush_allows_same_turn_attack() {
    // Without Rush a freshly-played Digimon can't attack. With native
    // printed Rush it can.
    let mut attacker = digimon_with_text("R", "＜Rush＞ (...)");
    attacker.level = Some(5);
    attacker.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .add_card(attacker)
        .add_card(digimon_with_text("F", ""))
        .hand(0, &["R"])
        .deck(0, &["F"; 10]).deck(1, &["F"; 10])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    // Verify can_attack returns true because of native Rush.
    assert!(r.game_mut().can_attack(handle, false),
        "native printed Rush should allow fresh attack");
}

#[test]
fn native_printed_jamming_survives_losing_security_battle() {
    // Attacker has Jamming printed natively; loses DP comparison
    // against a Digimon in security; Jamming protects it from deletion.
    let mut atk = digimon_with_text("J", "＜Jamming＞ (...)");
    atk.level = Some(5); atk.dp = Some(2000);  // weak attacker

    let mut def = digimon_with_text("SEC", "");
    def.level = Some(5); def.dp = Some(9000);  // strong security

    let mut r = DebugRunner::builder()
        .add_card(atk.clone())
        .add_card(def.clone())
        .add_card(digimon_with_text("F", ""))
        .hand(0, &["J"])
        .deck(0, &["F"; 10]).deck(1, &["F"; 10])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    let result = r.attack_player(handle, 1, true);
    // With Jamming, the attacker survives the losing security battle.
    let atk_still_alive = r.battle_area_size(0) > 0;
    assert!(atk_still_alive, "Jamming should protect attacker from losing security battle");
    let _ = result;
}
```

- [ ] **3.2 Verify the tests fail** (because check sites still use `modifiers.has_keyword` only).

- [ ] **3.3 Migrate combat.rs sites (5)**

Replace every `self.modifiers.has_keyword(handle, Keyword::X)` with `self.has_keyword(handle, Keyword::X)` at the 5 sites in combat.rs:
- Line ~99 (`can_attack` — Rush)
- Line ~366 (`try_enter_alliance` — Alliance)
- Line ~636 (`resolve_pending_battle` — Collision)
- Line ~661 (blocker scan — Blocker)
- Line ~938 (security DP — Jamming)

For each site, replace the method call and leave a `// Phase 3: unified native + modifier lookup` comment.

- [ ] **3.4 Migrate action/mask.rs sites (6)**

Same migration at lines ~108, ~127, ~425, ~439, ~583, ~696. The mask.rs accesses are likely through `game` — use `game.has_keyword(handle, Keyword::X)`.

- [ ] **3.5 Migrate game_phases.rs sites (3)**

Same migration at lines ~200, ~206, ~279.

- [ ] **3.6 Run the new tests + full suite**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_parsing 2>&1 | tail -15
cargo test --manifest-path digimon-engine/Cargo.toml 2>&1 | tail -5
```

Expected: Rush and Jamming regression tests pass. All existing tests still pass.

- [ ] **3.7 Commit**

```bash
git add -u
git commit -m "feat(engine): Phase 3 — migrate 14 keyword check sites to Game::has_keyword

Every callsite in combat.rs, action/mask.rs, and game_phases.rs that
previously read modifier-granted keywords only now reads the unified
query. Cards with Rush/Jamming/Blocker/etc. printed on their face now
exhibit correct behavior without a granting modifier.

Closes parity §2.1b (native Rush) and §2.5f (native Jamming).

Part of Phase 3 (Cluster C)."
```

---

## Task 4: Docs + parity + gap log

**Files:**
- Modify: `docs/RUST_ENGINE_API.md` — append §Phase 3 section
- Modify: `docs/RUST_PYTHON_PARITY.md` — flip §2.1b + §2.5f 🟡 → 🟢
- Modify: `docs/RUST_ENGINE_GAPS.md` — annotate "native keyword parsing" entries closed

**Steps:**

- [ ] **4.1 Append §Phase 3 to `docs/RUST_ENGINE_API.md`**

```markdown
## Phase 3 — Native Keyword Parsing

Added in Phase 3 to honor keywords printed on a card's face (not just
modifier-granted keywords). Closes parity §2.1b (native Rush) and §2.5f
(native Jamming).

### CardData surface

`CardData::keywords: Vec<Keyword>` — populated at load time by
`parse_printed_keywords(effect_text, inherited_text, security_text)`.
Parametric keywords (`Security A. ±N`, `De-Digivolve N`, `Draw N`) are
parsed into their typed variants.

### Unified query

`Game::has_keyword(handle, Keyword) -> bool` — the canonical keyword
lookup. Returns true if the permanent has the keyword either printed
natively on its top card OR granted by an active modifier.

**Call-site policy:** engine code never accesses
`game.modifiers.has_keyword(...)` directly — that only sees granted
keywords and would miss native printed keywords. Always use
`game.has_keyword(...)`.

### Keyword extraction patterns

Keywords appear in card text as `＜Keyword＞` (full-width angle brackets).
The parser recognizes the 19 non-parametric keywords in the `Keyword`
enum plus three parametric patterns:

- `＜Security A. +N＞` / `＜Security A. -N＞` → `SecurityAttackPlus(N)` / `SecurityAttackMinus(N)`
- `＜De-Digivolve N＞` → `DeDigivolve(N)`
- `＜Draw N＞` → `DrawX(N)`

Unrecognized keywords are ignored silently — scripts that need custom
keyword behavior must use the modifier-based API.
```

- [ ] **4.2 Flip parity doc §2.1b + §2.5f**

In `docs/RUST_PYTHON_PARITY.md`, update §2.1b:
- Change the 🟡 to 🟢
- Replace the body with a status-closed note: "Native keyword parsing landed in Phase 3 — see `CardData::keywords` (digimon-engine/src/card_data.rs) and the unified `Game::has_keyword` query (digimon-engine/src/game.rs). All 14 keyword check sites updated to use the unified lookup."

Same for §2.5f.

- [ ] **4.3 Annotate `docs/RUST_ENGINE_GAPS.md`**

```bash
rg -n "native keyword\|printed keyword\|card-text.*Rush\|native.*Jamming" docs/RUST_ENGINE_GAPS.md
```

Annotate each match with:

```markdown
**Closed by Phase 3 (2026-04-19):** Native-keyword parsing landed — see
`docs/RUST_ENGINE_API.md` §Phase 3 and `Game::has_keyword` unified query.
```

- [ ] **4.4 Commit**

```bash
git add docs/RUST_ENGINE_API.md docs/RUST_PYTHON_PARITY.md docs/RUST_ENGINE_GAPS.md
git commit -m "docs(engine): Phase 3 native-keyword API + close parity §2.1b/§2.5f + gap entries"
```

---

## Self-Review

- [x] **Spec coverage:** Parser (Task 1), unified query (Task 2), check-site migration (Task 3), docs (Task 4).
- [x] **No placeholders:** Every snippet has concrete code; open questions (card_data shape, top.card_id() accessor) tagged for implementer to verify.
- [x] **Tests cover both regression (old behavior still works) and new (printed keywords now honored).**

## Verification

After all tasks complete:
- `cargo test --manifest-path digimon-engine/Cargo.toml --test keyword_parsing` — all parser + integration tests pass.
- Full suite has no regressions.
- `docs/RUST_PYTHON_PARITY.md` §2.1b and §2.5f show 🟢.
