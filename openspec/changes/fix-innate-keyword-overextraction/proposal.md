## Why

`parse_printed_keywords` (`card_data.rs`) infers a card's **innate** keywords by scanning its *entire* effect description for `＜…＞` tokens. But that prose also contains keywords that are not innate:

- **granted** — "[Your Turn] …it gains ＜Security A. +1＞" (WarGreymon ST1-11, BT1-085 Tai Kamiya)
- **conditional** — "[Your Turn] While you have 3 or more memory, this Digimon gains ＜Security A. +1＞" (BT1-018 Flarerizamon)
- **formula units** — "For every 2 digivolution cards … it gains ＜Security A. +1＞" (the unit of WarGreymon's base-inclusive `security_attack_fn`)
- **target filters** — "[On Play] Delete 1 of your opponent's Digimon with ＜Blocker＞" (SkullGreymon BT1-023 — `＜Blocker＞` describes the *target*, not this card)

The parser cannot tell innate from granted, so it treats all of them as innate printed keywords. For **boolean** keywords this is a latent, idempotent semantic error (a conditionally-gained `＜Blocker＞` is reported as always-innate). For **parametric** keywords (`Security A. +N`, `Draw N`, `De-Digivolve N`) it actively misbehaves: the bonus **double-counts** and applies **unconditionally**.

Found live (desktop debug bridge): WarGreymon over 4 sources checked **4** security instead of **3**, because its `＜Security A. +1＞` formula-reminder token was parsed as a flat `SecurityAttackPlus(1)` *on top of* the formula — and it leaked +1 even on the opponent's turn. The DSL already models the grants correctly as effects; only the innate-keyword set is wrong.

A prior interim patch fixed only WarGreymon at the combat read-site. This change fixes the root cause durably for **all** keywords.

## What Changes

- **Grammar-based innate-keyword extraction.** Replace the whole-text `＜…＞` scan in `parse_printed_keywords` with a **leading keyword-line tokenizer**: a card's innate keywords are the `＜kw＞ (optional reminder)` units at the *start* of each text field (effect / inherited / security); parsing stops at the first `[Timing]`/header label or prose sentence. Everything after is effect prose (grants, conditions, filters) — already modeled by DSL effects — and is NOT treated as innate. This relies on the consistent grammar of Digimon card text (keyword line precedes timed effects), not English NLP. The `＜Decoy ([Bagra Army] trait)＞` edge is handled because the inner `[…]` lives inside the keyword unit's own `(reminder)`, which the tokenizer consumes.
- **Pool-wide keyword diff + audit.** A harness computes old-parse vs new-parse innate keywords for every card in `cards.json`, emitting the delta (which cards lose which keywords), partitioned **implemented (has a DSL spec)** vs **unimplemented**. This is the worklist and the safety net.
- **Model the gaps (implemented cards).** For each implemented card the diff shows regressing — its keyword was granted/conditional, not innate — confirm a DSL effect already grants it; where missing, add the proper *conditional* grant so net behavior is **correct-or-better** (conditional instead of the old unconditional phantom). Unimplemented regressors are logged to `docs/RUST_ENGINE_GAPS.md`, not fixed here.
- **Supersede the interim WarGreymon patch.** Remove `top_card_has_security_attack_formula` / `top_face_security_attack_keyword_bonus` and the `raw_security_strike` subtraction (`game/queries.rs`, `combat/mod.rs`) — the parser fix makes them redundant. **Keep** the WarGreymon real-card-data regression tests, which now pass via the durable path.

Non-goals (explicit): NOT migrating to `spec.keywords`-authoritative sourcing (the larger re-architecture where the DSL spec, not prose, declares innate keywords — a possible follow-up); NOT fixing unimplemented cards' grants (logged only); NOT adding a `security_attack` assertion kind to the scenario evaluator (separate follow-up).

## Capabilities

### New Capabilities
- `innate-keyword-extraction`: grammar-based separation of a card's innate (printed-attribute) keywords from effect-granted / conditional / formula / filter keyword tokens, and the pool-wide regression-audit contract that implemented regressors are modeled as effects.

### Modified Capabilities
<!-- None as a separate spec. The printed-keyword semantics live in card_data and were previously undocumented; this change defines them under the new capability. The combat security-attack contract is unchanged (the interim patch is removed, not a published capability). -->

## Impact

- **Engine**: `code/digimon-engine/src/card_data.rs` — `parse_printed_keywords` rewritten as a leading tokenizer (+ unit tests). `code/digimon-engine/src/game/mod.rs` — `face_keywords`/`inherited_keywords` inherit the fix (call sites unchanged). `code/digimon-engine/src/combat/mod.rs` + `game/queries.rs` — interim WarGreymon patch removed.
- **Audit harness**: `code/tools/keyword-parse-diff/` (or a `cargo test` that writes the diff artifact).
- **Card YAML**: conditional-grant effects added for regressing implemented cards under `code/digimon-engine/cards/**` (count gated by the diff).
- **Docs**: `docs/RUST_ENGINE_GAPS.md` — mark the WarGreymon entry resolved-by-root-cause; log unimplemented regressors.
- **Tests**: `card_data` tokenizer unit tests; `cards_behavioral/st1/wargreymon_security_attack.rs` real-data tests retained; full `cards_behavioral` suite is the regression gate.
- **No API/contract changes**: no new public engine API; `face_keywords` signature unchanged. No frontend/desktop changes (the display reads `effective_security_strike`, which simply returns the corrected value).
