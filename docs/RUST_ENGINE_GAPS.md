# Rust Engine Gaps

Capability gaps in the Rust engine's scripting surface (`digimon-engine/`), discovered during archetype audits by `/assess-archetype-rust`. Distinct from `RUST_PYTHON_PARITY.md`, which tracks Rust↔Python divergences in shared subsystems — this file tracks *Rust-only* missing primitives: timings, `EffectContext` helpers, `ModifierType` variants, `Keyword` variants, selection kinds, and trigger fan-outs that cards in the pool need but the curated API does not yet expose.

Format and conventions mirror `qa/archetype-qa/engine-gaps.md` (Python-scoped). Every entry is **capability-centric** — card IDs are listed as evidence, not as the gap's identity.

## Severity legend

- **🔴 BLOCKING** — no faithful workaround exists; card cannot be authored without this primitive.
- **🟡 PARTIAL** — the primitive or a workaround exists but has a specific fidelity cost (degraded UX, hidden RL choice, scope over-reach). Sub-kinds marked inline:
  - *"ergonomics / sugar"* — fully expressible today but awkward; scripts currently need to reach around `EffectContext` or duplicate state.
  - *"primitive-with-fidelity-cost"* — a modifier / keyword exists but its scope is too coarse for the card text's restriction.
- Pure verification / test-coverage items are **not** filed as gaps — see the "Deferred" section at the bottom of this file.

## Open gaps — tally

As of 2026-04-17: **68 entries** — **62 🔴 BLOCKING + 6 🟡 PARTIAL**. Of the 6 partials, 3 are "ergonomics / sugar" (OPT recording helper, dual-timing builder, aggregate filter helpers) and 3 are "primitive-with-fidelity-cost" (native printed-keyword parsing, attack-without-suspending for MayAttack, if-effect-didn't-resolve else-branch).

## Open gaps

### Play card from hand without paying the cost
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095 Miraculous Mega Knight, BT22-084 Nokia Shiramine, BT22-089 Mirei Mikagura, BT17-081 Tai Kamiya & Matt Ishida, BT5-092 Nokia Shiramine, BT17-102 Greymon, BT21-102 Tai Kamiya, BT17-093 Tai Kamiya & Kari Kamiya, BT22-026 MetalGarurumon (via `[Hand][Main]`), EX4-061 Matt Ishida & Tai Kamiya, ST20-15 Island of Adventure, LM-034 Wisteria Memory Boost!, BT22-099 Kuremi Detective Agency, BT23-018 Garurumon (cost-reduction variant), BT22-094 Yuugo Kamishiro, BT13-012 GeoGreymon (from security)
- **Effect text:** "you may play 1 [Agumon] or [Gabumon] from your hand without paying the cost." (and ~15 other phrasings)
- **What's missing:** `EffectContext` only has `play_from_security()`. `Game::play_from_hand` unconditionally calls `pay_memory(base_cost)`; there is no effect-initiated free-play or cost-override variant. Python has `Game.effect_play_from_zone` covering hand / trash / security uniformly.
- **Suggested API shape:** `ctx.play_from_hand_free(player, hand_index) -> Option<PermanentHandle>`, `ctx.play_from_hand_for_cost(player, hand_index, cost: u16)`, analogous `ctx.play_from_trash_free(player, trash_index)`. All fire `OnPlay` / `WhenPlayedFromHand`, skip `pay_memory` when free.
- **Workaround:** None — BLOCKED. Reaching into `players[p].hand.remove` + `battle_area.push` misses OnPlay firing, `turn_played` bookkeeping, and field-slot limits.
- **Related:** RUST_PYTHON_PARITY.md §1.1; `ctx.play_from_security` is the shape to clone.

### Return permanent to bottom of deck
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-078 Omnimon, BT22-015 Omnimon, BT22-026 MetalGarurumon (opponent's lowest-level), BT22-089 Mirei Mikagura (self-return cost), BT21-102 Tai Kamiya (self), BT22-094 Yuugo Kamishiro (self), BT17-093 Tai Kamiya & Kari Kamiya (self), AD1-025 Omnimon, BT20-102 Omnimon (X Antibody), EX4-060 Omnimon Alter-S, EX1-021 MetalGarurumon
- **Effect text:** "return all of your opponent's Digimon with the same level as it to the bottom of the deck" / "By returning this Tamer to the bottom of the deck, …"
- **What's missing:** No `EffectContext` helper moves a permanent (top card + its digivolution stack) to `player.deck[0]`. `delete_permanent` trashes; `return_to_hand` is also absent (see sibling gap). Must handle leave-field firing, modifier cleanup, source-card disposition per DCGO (top card to deck-bottom, materials to trash).
- **Suggested API shape:** `ctx.return_permanent_to_deck(target: PermanentHandle, position: DeckPosition::Bottom | Top, keep_sources: bool)`. Fires `OnLeaveField` with `cause = Effect`, clears modifier entries for the handle.
- **Workaround:** None — BLOCKED. `delete_permanent` trashes (wrong destination).
- **Related:** "Return permanent to hand"; `OnLeaveField` replacement gap below.

### Return permanent to hand
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-026 MetalGarurumon, AD1-012 CresGarurumon
- **Effect text:** "Return 1 of your opponent's Digimon with the lowest level to the hand." / "You may return 1 of your opponent's lowest level Digimon to the hand."
- **What's missing:** No `ctx.return_permanent_to_hand(target)`. Bounce is one of the most common removal primitives; `ModifierType::CannotReturnToHand` exists but the action verb does not.
- **Suggested API shape:** `ctx.return_permanent_to_hand(target)` — top card to owner's hand, remaining sources to trash (DCGO rule), fires `OnLeaveField`.
- **Workaround:** None — BLOCKED.
- **Related:** "Return permanent to bottom of deck".

### Move card from trash to hand
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-008 Agumon, EX9-066 Tai Kamiya & Matt Ishida, BT17-007 Agumon, ST21-13 Matt Ishida & T.K. Takaishi (indirect)
- **Effect text:** "You may return 1 Digimon card with [Greymon], [Garurumon] or [Omnimon] in its name from your trash to the hand."
- **What's missing:** `ctx.select_trash` exposes the choice, but there is no `ctx.move_trash_to_hand(player, trash_index)` mutator. Reaching into `players[p].trash/hand` directly skips `OnAddToHand` event plumbing.
- **Suggested API shape:** `ctx.move_trash_to_hand(player, trash_index)` removing the entry and firing `OnAddToHand`.
- **Workaround:** None — BLOCKED.
- **Related:** `EffectTiming::OnAddToHand` declared but never fired.

### Reveal top N cards of deck
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-017 Gabumon, EX4-039 Gabumon, P-206 Digital Gate Open, BT16-082 Ukkomon, EX4-038 Agumon, BT12-059 Agumon, LM-034 Wisteria Memory Boost!, BT22-099 Kuremi Detective Agency, BT22-094 Yuugo Kamishiro, EX5-015 Gabumon (X Antibody)
- **Effect text:** "Reveal the top 3 cards of your deck." (and similar for N=3/4)
- **What's missing:** `Game.revealed_cards` vec and `select_reveal` helper exist (RUST_PYTHON_PARITY §3.4) but nothing populates the pool from the top of the deck. No `ctx.reveal_top(player, n)`.
- **Suggested API shape:** `ctx.reveal_top(player, n: u8) -> Vec<CardSource>` — pops top N from deck, pushes to `game.revealed_cards`, fires `OnReveal`. Returns the revealed handles for composable callback chains.
- **Workaround:** None — BLOCKED. Every reveal-and-search Option/Tamer/OnPlay effect in the pool needs this.
- **Related:** RUST_PYTHON_PARITY.md §3.4 (tensor slot exists, no writer).

### Move revealed card to hand / resolve reveal pool
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** Every reveal-using card above.
- **Effect text:** "Add 1 Digimon card with [Greymon] in its name … to your hand."
- **What's missing:** `select_reveal` returns an index but there is no `ctx.add_revealed_to_hand(index)`, `ctx.return_revealed_to_deck_bottom(indices_in_order)`, `ctx.return_revealed_to_deck_top(indices_in_order)`, or `ctx.clear_revealed()`. Effects with phrasing "Add 1 X **and** 1 Y" need multi-pick (see selection gap).
- **Suggested API shape:** `ctx.add_revealed_to_hand(player, reveal_index)`, `ctx.return_revealed_to_deck_bottom(player, order: Vec<usize>)`, `ctx.return_revealed_to_deck_top(player, order: Vec<usize>)`. Paired `ctx.resolve_reveal_pool()` that sends anything un-dispatched back to deck.
- **Workaround:** None — BLOCKED.
- **Related:** Reveal-top-N gap; multi-pick selection gap below.

### Place card on top / bottom of deck in player-chosen order
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX4-039 Gabumon, EX4-038 Agumon, BT12-059 Agumon, P-206, BT22-099 — every multi-reveal card with "Place the rest on top/bottom of the deck in any order"
- **Effect text:** "Place the remaining cards at the bottom of your deck in any order." / "Place the rest on top of your deck in any order."
- **What's missing:** No `SelectionKind::Order` / ordering prompt. The existing `select_reveal` is single-pick. Auto-ordering (arbitrary) hides a player choice with real game state implications — violates §17.
- **Suggested API shape:** `SelectionKind::Ordering { pool: Vec<usize>, destination: DeckEnd }`, exposed via `ctx.select_deck_order(prompt, pool, destination, callback: Fn(Vec<usize>))`. Or compose via repeated single-picks that each append to the destination.
- **Workaround:** Auto-order is an approximation; rejected under §17.
- **Related:** Multi-select / ordering family.

### Trash opponent security top N from effect (outside attack)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-013 WarGreymon (inherited), BT17-015 WarGreymon (inherited), EX4-073 Omnimon Alter-B
- **Effect text:** "trash the top card of your opponent's security stack." / "trash the top 2 cards of your opponent's security stack."
- **What's missing:** `ctx.trash_security_top(player, count)` helper that pops top N, routes to trash, fires `OnLoseSecurity`, cleans `face_up_security`. Distinct from `resolve_security_card` (attack-driven).
- **Suggested API shape:** `ctx.trash_security_top(of_player, count: u8) -> u8` returning actual trashed.
- **Workaround:** Raw pop mis-handles `OnLoseSecurity` + face_up_security bookkeeping. BLOCKED.
- **Related:** RUST_PYTHON_PARITY.md §2.5k (face_up_security), §2.5m (event surface).

### Add top security card to own hand
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** ST20-15 Island of Adventure
- **Effect text:** "[Main] Add your top security card to the hand."
- **What's missing:** No `ctx.move_security_to_hand(player, security_index)`. Must fire `OnLoseSecurity`, clean `face_up_security`, route to hand, emit `security_moved` event.
- **Suggested API shape:** `ctx.move_security_to_hand(player, security_index)`.
- **Workaround:** None — BLOCKED.
- **Related:** `OnLoseSecurity` event surface.

### Place permanent as top/bottom of security (face-up / face-down)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX9-021 Omnimon Alter-S, ST20-15 Island of Adventure, EX4-060 Omnimon Alter-S
- **Effect text:** "place this Digimon as your top security card." / "place this card face up as the top security card." / "place this Digimon at the bottom of your security stack face down."
- **What's missing:** `mark_security_face_up` only flips visibility on an already-in-stack card. No primitive inserts a field permanent (or a hand card) into the security stack at the top/bottom with a face-up/face-down flag, and no `OnPlaceSecurity` firing site.
- **Suggested API shape:** `ctx.place_permanent_to_security(target, position: SecurityEnd, face_up: bool)` — pops permanent, pushes (or trashes sources per DCGO), fires `OnLeaveField` + `OnPlaceSecurity`.
- **Workaround:** None — BLOCKED.
- **Related:** `OnPlaceSecurity` timing declared but unused.

### Recovery +N (Deck) — deck top → security top
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT13-012 GeoGreymon
- **Effect text:** "＜Recovery +1 (Deck)＞"
- **What's missing:** `ctx.recover_security_from_deck(player, count)` — pops the top N cards of the deck, pushes onto security top, fires `OnPlaceSecurity`. Not wired today.
- **Suggested API shape:** `ctx.recovery_from_deck(player, count: u8) -> u8` returning actual placed.
- **Workaround:** None — BLOCKED.
- **Related:** "Place permanent as top/bottom of security".

### Shuffle security stack
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT13-012 GeoGreymon
- **Effect text:** "Then, shuffle your security stack."
- **What's missing:** `ctx.shuffle_security(player)` has no analogue; `player.shuffle_deck` covers only the deck. Needs access to `game.rng`; must clear `face_up_security` entries whose positions are now unknown.
- **Suggested API shape:** `ctx.shuffle_security(player)`.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_PYTHON_PARITY.md §3.3, §2.5k.

### Trash from hand (by index)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-089 Mirei Mikagura, EX5-015 Gabumon (X Antibody)
- **Effect text:** "By trashing 1 card with the [Holy Beast], [Angel], … trait from your hand, ＜Draw 2＞" / "trash 1 card in your hand."
- **What's missing:** `ctx.trash_from_top` exists for deck; no `ctx.trash_from_hand(player, hand_index)`. `select_hand` gives the index but the callback has no matching mutator.
- **Suggested API shape:** `ctx.trash_from_hand(player, hand_index) -> Option<CardSource>`. Fires `OnTrash`.
- **Workaround:** Raw `hand.remove` / `trash.push` violates curated-API rule (§7 anti-pattern).
- **Related:** `OnTrash` timing exists but no firing site.

### Play card from digivolution sources (materials) without paying cost
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX9-021 Omnimon Alter-S, AD1-025 Omnimon (Partition), EX4-060 Omnimon Alter-S, BT22-015 Omnimon (Decode)
- **Effect text:** "play 1 card with [Greymon] in its name … from this Digimon's digivolution cards without paying the costs."
- **What's missing:** `select_material` picks a source but cannot remove it and instantiate it as a new battle-area permanent for free. No `ctx.play_from_materials(of_permanent, source_index)`.
- **Suggested API shape:** `ctx.play_from_materials(target, source_index)` — removes source from stack, creates new permanent, fires OnPlay, skips memory payment.
- **Workaround:** None — BLOCKED. Related to Decode and Partition gaps.
- **Related:** "Decode keyword" and "Partition keyword" gaps.

### Digivolution-stack reorder — move top source to bottom (and general reorder)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT23-008 Greymon, BT23-018 Garurumon
- **Effect text:** "By placing this Digimon's top stacked card as its bottom digivolution card, …"
- **What's missing:** `Permanent.card_sources` is editable in engine but `EffectContext` has no `move_source_to_bottom(target, source_index)` / `reorder_sources(target, from, to)` helper. Cost is mandatory and rearranges stack composition (affects inherited effects + DP math).
- **Suggested API shape:** `ctx.move_source_to_bottom(target, source_index)`.
- **Workaround:** None — BLOCKED (faithful cost).
- **Related:** none.

### Digivolve (effect-driven) ignoring requirements at fixed or zero cost
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-013 WarGreymon, BT17-015 WarGreymon, BT17-027 MetalGarurumon, BT22-026 MetalGarurumon, BT17-095 Miraculous Mega Knight, EX9-019 / EX9-012 / AD1-001 / AD1-010 (observer-driven free digivolve from hand)
- **Effect text:** "1 of your [Gabumon] may digivolve into [MetalGarurumon] in your hand, ignoring its digivolution requirements and without paying the cost."
- **What's missing:** `Game::digivolve_from_hand` validates color/level against `CardData.evo_costs`; no effect-callable helper digivolves a battle-area permanent onto a target card in hand (or in a material pile) with override cost and `can_digivolve` bypass. Must still fire `WhenDigivolving`, install source-card lifecycle, optionally bypass digivolve-draw.
- **Suggested API shape:** `ctx.digivolve_override(from_hand_index, target: PermanentHandle, cost: u16, ignore_requirements: bool)` + `ctx.digivolve_onto_free(base, via_card_source, ignore_requirements: bool)`.
- **Workaround:** None — BLOCKED.
- **Related:** `Game::digivolve_onto`, RUST_PYTHON_PARITY.md §4.5a.

### DNA digivolve from hand via effect (both materials or one hand + one field)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095 Miraculous Mega Knight, BT22-017 Gabumon (inherited EOT), BT22-008 Agumon (inherited EOT), BT17-019 Gabumon (inherited EOT), BT17-007 Agumon (inherited EOT), AD1-009 BlitzGreymon (EOT into named hand card), AD1-012 CresGarurumon (defender-side reactive DNA)
- **Effect text:** "That Digimon and a card in the hand may DNA digivolve into a Digimon card with [Omnimon] in its name in the hand." / "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand."
- **What's missing:** DNA digivolve today is driven only by Main-phase hand mask and is further blocked by empty `dna_costs` (RUST_PYTHON_PARITY §4.5b). No `ctx.offer_dna_digivolve_into_hand(...)` that fires outside Main, no support for granted DNA-digivolve permission (inherited aura granting the action to the whole board), no way to target a specific named card in hand ignoring per-card DNA requirements.
- **Suggested API shape:** `ctx.offer_dna_digivolve(of_player, attacker_filter, hand_filter, cost_override: Option<u16>)` + `ModifierType::GrantDnaDigivolveFromHand` for aura-grants. Depends on §4.5b data.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_PYTHON_PARITY.md §4.5a, §4.5b.

### Blast DNA / Blast Digivolve from hand (Counter window)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-078 Omnimon (Blast DNA Digivolve), EX10-010 BlackWarGreymon (Blast Digivolve, Ace)
- **Effect text:** "[Hand] [Counter] ＜Blast DNA Digivolve ([WarGreymon] + [MetalGarurumon])＞" / "[Hand] [Counter] ＜Blast Digivolve＞"
- **What's missing:** RUST_PYTHON_PARITY §2.3 supports `blast_digivolve = true` for single-target blast; Blast **DNA** Digivolve pairs two field materials with a hand card and has no scanner. Ace-card Blast Digivolve additionally requires Ace Overflow bookkeeping (separate gap).
- **Suggested API shape:** Extend `combat::try_enter_counter` hand-scan to check `Effect.blast_dna_digivolve = true` and iterate battle-area pairs via `has_valid_dna_targets`; stack the counter card on the fused stack.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_PYTHON_PARITY §2.3, DNA digivolve from hand gap, Ace Overflow gap.

### Alternate digivolution source registration (alt-digi scripting channel)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-017 Gabumon, BT22-026 MetalGarurumon, EX9-021 Omnimon Alter-S, ST20-10 Agumon, BT15-101 MetalGarurumon
- **Effect text:** (ST20-10) "this Digimon can digivolve into [WarGreymon] in the hand for a digivolution cost of 4, ignoring digivolution requirements" / `AddSelfDigivolutionRequirementStaticEffect` in DCGO.
- **What's missing:** Python's engine carries `_alt_digi_*` attributes on effects; Rust has no scripting data channel. No `Effect::alt_digivolve(...)` builder, no mask emission for conditional alt-digivolve bits, no `ignore_requirements` flag on the digivolve validator.
- **Suggested API shape:** `Effect::alt_digivolve(card).from_zone(Zone).name_filter(...).cost(n).ignore_requirements(bool).condition(...)` consulted by the Main-phase mask digivolve loop.
- **Workaround:** None — BLOCKED.
- **Related:** "Digivolve ignoring requirements at fixed cost".

### Detect DNA digivolve origin within WhenDigivolving
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX9-021 Omnimon Alter-S
- **Effect text:** "[When Digivolving] If DNA digivolving, your opponent's effects don't affect this Digimon for the turn."
- **What's missing:** `EffectContext` in a `WhenDigivolving` process has no `ctx.was_dna_digivolve() -> bool`. Python carries an `is_dna` context key. `OnDnaDigivolve` is a separate timing in Rust but DCGO branches inside WhenDigivolving.
- **Suggested API shape:** Add `digivolve_was_dna: Option<bool>` on the trigger context fed into `EffectContext`; accessor `ctx.was_dna_digivolve()`.
- **Workaround:** None faithful.
- **Related:** RUST_PYTHON_PARITY §2.5g (context-args pattern).

### BeforePayCost cost-reduction scanning (play and digivolve)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-027 MetalGarurumon, BT17-015 WarGreymon, BT8-097 Crimson Blaze (self-scaling by opp-Digimon count), BT5-092 Nokia Shiramine (Tamer suspend-to-reduce), BT23-008 Greymon, BT22-094 Yuugo Kamishiro, BT23-018 Garurumon, ST21-13 Matt Ishida & T.K. Takaishi
- **Effect text:** "When this card would be played, if you have a Tamer with [Matt Ishida] in its name, reduce the play cost by 3." / "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play." / "By suspending this Tamer, reduce the digivolution cost by 1."
- **What's missing:** `calculate_play_cost` / `calculate_digivolve_cost` do not scan `BeforePayCost` effects on (a) the card being played itself, (b) battle-area permanents, (c) Tamers — with a condition closure that inspects a `PlayContext { card, player, base_cost }`. `Effect::cost_reduction(n)` is a static field; not dynamic. Also missing: activated suspend-as-cost / return-to-bottom-as-cost payment shapes that feed into the reduction.
- **Suggested API shape:** Fire `EffectTiming::BeforePayCost` before `pay_memory` with `PlayContext`; effects can mutate `current_cost: Cell<i32>` or call `ctx.reduce_play_cost(n)`. Add `.pay_cost_suspend_self()` / `.pay_cost_return_self_to_deck_bottom()` builder hooks for activated replacement costs.
- **Workaround:** None — BLOCKED. Python's Issue 24 shows the scan path must be condition-guarded; do it right from the start in Rust.
- **Related:** RUST_PYTHON_PARITY §1.1, §4.7e (DigiXros shares this shape), CLAUDE.md memory "BeforePayCost cost_reduction leak".

### Ace Overflow memory penalty
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-078 Omnimon, BT17-095 Miraculous Mega Knight, EX10-010 BlackWarGreymon, ST20-11 WarGreymon
- **Effect text:** "Ace Overflow ＜-5＞ (As this card moves from the field or under a card to an area other than those, lose 5 memory.)"
- **What's missing:** No `Keyword::Ace(i8)` / `Keyword::AceOverflow(i8)`; no timing `OnLeaveFieldOrStackToOther` firing on every zone-transition path (delete / return-to-hand / return-to-deck / go-to-security). Mandatory memory swing on source-position (buried material) removal, not just top-card removal.
- **Suggested API shape:** `Keyword::AceOverflow(i8)` + data field `CardData.ace_overflow: Option<i8>` + enqueue hook in every Permanent / CardSource zone-transition path.
- **Workaround:** None — BLOCKED. Faking with `OnDeletion` misses return-to-hand/deck paths.
- **Related:** "Return permanent to hand/deck" sibling gaps, "OnLeaveField cause discrimination".

### Delay keyword — place-then-later-trash activated ability
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095 Miraculous Mega Knight, P-206 Digital Gate Open, LM-034 Wisteria Memory Boost!, BT22-099 Kuremi Detective Agency, BT23-096 Comet Hammer
- **Effect text:** "＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)"
- **What's missing:** `Keyword::Delay` variant; per-permanent `placed_turn` gate; mask bit emitted only from turn N+1 onward; execution path that trashes the Option and runs the delayed effect; Option-in-battle-area residency (see Option play flow).
- **Suggested API shape:** `Keyword::Delay` + `Effect::delay_ability(card)` builder treating body as `MainOnField` with implicit trash-self cost + `turn_played < turn_count` gate.
- **Workaround:** None — BLOCKED.
- **Related:** "Option card play flow".

### Evade keyword + player-elected deletion prevention
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT15-101 MetalGarurumon, AD1-012 CresGarurumon, AD1-014 MetalGarurumon
- **Effect text:** "＜Evade＞ (When this Digimon would be deleted, you may suspend it to prevent that deletion.)"
- **What's missing:** No `Keyword::Evade` variant; `delete_permanent` / `delete_permanent_with_effects` are atomic with no pre-delete interrupt phase. No replacement-effect infra for deletion-class events (also needed for Armor / Fortitude / Barrier).
- **Suggested API shape:** `Keyword::Evade` + `attempt_delete(target, reason) -> bool` that enqueues an optional `EffectTiming::WouldBeDeleted` prompt for each applicable keyword; accepted prompt suspends and cancels.
- **Workaround:** None — BLOCKED (hiding the choice violates §17).
- **Related:** "Replacement effect: prevent battle deletion by paying a cost" (EX5-015).

### De-Digivolve N execution primitive
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX4-073 Omnimon Alter-B, BT23-096 Comet Hammer, EX9-019 WereGarurumon: Sagittarius Mode (inherited)
- **Effect text:** "＜De-Digivolve 3＞ 1 of your opponent's Digimon. (Trash up to 3 cards from the top. You can't trash past level 3 cards.)"
- **What's missing:** `Keyword::DeDigivolve(u8)` enum variant exists but no `ctx.de_digivolve(target, n)` helper. Must trash top sources until N trashed or first source with `level ≥ 3`.
- **Suggested API shape:** `ctx.de_digivolve(target: PermanentHandle, n: u8) -> u8` returning actual trashed; fires `OnTrash`, refreshes top card.
- **Workaround:** None — BLOCKED.
- **Related:** enum variant exists, executor absent.

### Partition keyword + provenance-filtered leave-field replacement
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** AD1-025 Omnimon
- **Effect text:** "＜Partition ([WarGreymon] & [MetalGarurumon])＞ (When this Digimon with each of the specified digivolution cards would leave the battle area other than by your own effects or by battle, you may play 1 each of the specified cards without paying the costs.)"
- **What's missing:** `Keyword::Partition` exists as enum variant; no inspection. Requires (a) leave-field replacement hook (acts before the move), (b) provenance filter ("other than by own effects or battle"), (c) play-from-materials-free (sibling gap).
- **Suggested API shape:** `Effect::on_partition(card).names(&[...]).process(...)` fired from the same hook as OnLeaveField replacement, gated by provenance.
- **Workaround:** None — BLOCKED.
- **Related:** "Play from materials without paying cost", "OnLeaveField cause discrimination".

### Native printed keyword parsing (Raid / Jamming / Blocker / Rush / Blitz / Security A.)
- **Severity:** 🟡 PARTIAL — *primitive-with-fidelity-cost*
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-078 Omnimon (native Raid, Blocker), BT23-018 Garurumon (Jamming), P-182 WarGreymon (Security A.+1, Blocker), AD1-001 Greymon (Raid), AD1-010 Garurumon (Jamming), BT17-095 (Delay), others.
- **Effect text:** "＜Raid＞ … ＜Blocker＞ … ＜Jamming＞"
- **What's missing:** `CardData` has no parsed `keywords: Vec<Keyword>` field; static keywords live only inside `effect_text: String`. Combat / mask / security-attack calculation honor only modifier-granted keywords, so face-printed keywords never fire on vanilla-keyword cards.
- **Suggested API shape:** Add `CardData.keywords: Vec<Keyword>` populated by cards.json ingest (or a load-time parser over `effect_text`) + `Permanent::has_printed_keyword(kw) -> bool`. Update combat's Jamming / can-be-blocker / security-attack-modifier paths to union printed + granted.
- **Workaround:** Declaring `Effect::declarative(card).grant_keyword(...)` at registration is brittle (grants expire on leave; native keywords persist) and doesn't activate until an effect-processing hook fires. Accepted as a temporary measure but not faithful.
- **Related:** RUST_PYTHON_PARITY §2.1b, §2.5f, §4.3b.

### Raid "may switch target" OnAttack retarget interrupt
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT23-008 Greymon, EX10-010 BlackWarGreymon
- **Effect text:** "＜Raid＞ (When this Digimon attacks, you may switch the target of attack to 1 of your opponent's unsuspended Digimon with the highest DP.)"
- **What's missing:** Raid is modeled as a Main-phase targeting mask expansion (RUST_PYTHON_PARITY §4.4) but the *mid-attack* "you may retarget" interrupt window is not wired. Combat has no `RaidTiming` / `OnAttackRetarget` phase and no `ctx.redirect_attack(new_target)`.
- **Suggested API shape:** New `GamePhase::RaidTiming` parked after OnAttack fires; `combat::try_enter_raid` installs an optional selection over tied-for-highest-DP unsuspended Digimon; on resolution rewrite `PendingAttack.effective_target`.
- **Workaround:** Main-phase Raid mask covers the targeting half but silently drops the mid-attack switch.
- **Related:** §4.4, §2.3 interrupt state machine.

### Redirect attack target from effect (cross-cutting primitive)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** AD1-012 CresGarurumon
- **Effect text:** "you may change the attack target to 1 of your Digimon."
- **What's missing:** `ctx.redirect_attack(new_target: AttackTarget) -> bool` usable while `Game.pending_attack.is_some()`. Block already does internal rewrite; card scripts need the lever exposed.
- **Suggested API shape:** As above.
- **Workaround:** None — BLOCKED.
- **Related:** Raid interrupt gap.

### Option card [Main] play flow (resolve-and-trash)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095, P-206, BT8-097 Crimson Blaze, LM-034, BT22-099, ST2-13 Hammer Spark, ST20-15 Island of Adventure, BT23-096 Comet Hammer, BT1-090 Gravity Crush, EX1-068 Ice Wall!
- **Effect text:** `[Main]` effects on cards with `card_kind: Option`.
- **What's missing:** RUST_ENGINE_API.md §9 explicit known gap. `play_from_hand` pushes a `Permanent` regardless of kind; no branch on `CardKind::Option`, no `OptionMain` resolution pipeline, no "resolve-then-trash" disposition. `EffectTiming::OptionMain` / `OptionSecurity` exist in the enum but are never fired.
- **Suggested API shape:** Branch in `play_from_hand` on Option kind: drain `OptionMain` effect queue, route to trash (or to battle_area for Delay/Ace lingering, or to security for place-as-security variants).
- **Workaround:** None — BLOCKED.
- **Related:** RUST_ENGINE_API.md §9; Delay / Ace / place-as-security sibling gaps.

### Option card persistent placement in battle area
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095, P-206, LM-034, BT22-099, BT23-096
- **Effect text:** "Then, place this card in the battle area." (Delay / Ace Option residency)
- **What's missing:** Even with an Option play flow, Options that linger on the field (Delay, Ace Option) need a distinct "place as Option permanent" disposition + a mask/action-slot that lets their activated abilities fire later. `Permanent.placed_turn` already exists; needs wiring to `CardKind::Option` residency.
- **Suggested API shape:** `ctx.place_option_in_battle_area()` turning the current Option resolution into a lingering permanent of `CardKind::Option`.
- **Workaround:** None — BLOCKED.
- **Related:** Option play flow, Delay keyword.

### Script-driven color-requirement bypass for Options
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095, P-206, LM-034, BT22-099, BT23-096, ST20-15
- **Effect text:** "You can ignore this card's color requirements." / "Red also meets this card's color requirements."
- **What's missing:** Mask's `option_color_match_available` is unconditional. `CardData` has no `match_color_requirement: Option<Condition>` hook and there's no `Effect::color_bypass(card).condition(...)` builder. Rust over-masks these Options — the mask never emits play bits when color requirements aren't met, even when the bypass clause would allow it.
- **Suggested API shape:** `Effect::color_bypass(card).condition(|rctx| bool)` consulted by mask via `Game::effect_ignores_color(card_id, player)`.
- **Workaround:** None — BLOCKED (RL agent literally cannot pick the play).
- **Related:** RUST_PYTHON_PARITY §4.2b.

### Security effect "return this card to hand / place as security"
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095 Miraculous Mega Knight, P-206 Digital Gate Open
- **Effect text:** "[Security] You may play 1 Digimon card … . Then, add this card to the hand." / "Then, place this card in the battle area."
- **What's missing:** Security loop trashes the revealed card unless `pending_security.played` is raised by `play_from_security`. No `ctx.return_revealed_security_to_hand()` (card goes to hand, not trash/field) and no `ctx.place_revealed_security_on_field()` (Option-as-permanent disposition).
- **Suggested API shape:** `ctx.return_revealed_security_to_hand()`, `ctx.place_revealed_security_on_field()` — both set `pending_security.disposition` explicitly.
- **Workaround:** None — BLOCKED.
- **Related:** Option play flow.

### Granted triggered ability — attach an `Effect` to another permanent
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX1-068 Ice Wall!, BT22-084 Nokia Shiramine (aura-like DP grant by name — adjacent), BT5-093 Tai Kamiya & Matt Ishida (Security A.+1 to all Omnimon), ST21-13 Matt Ishida & T.K. Takaishi (Rush aura)
- **Effect text:** "All of your opponent's Digimon gain \"[When Attacking] lose 2 memory\" until the end of their next turn."
- **What's missing:** `ModifierRegistry` carries only scalar `ModifierType` values + `grant_keyword`. No primitive attaches a full `Effect` (timing + condition + process) to another permanent with bounded expiry. Python has `effect_grant_ability`.
- **Suggested API shape:** Extend `ModifierRegistry` (or add a sibling `GrantedEffectRegistry`) to hold `(target: PermanentHandle, effect: Arc<Effect>, expiry: Expiry)`. `enqueue_from_permanent` also walks `granted_effects[target]` when building the fire list. Expose `ctx.grant_effect(target, effect, expiry)`.
- **Workaround:** None faithful.
- **Related:** Named-target aura gap, Expiry variants gap.

### Named-target declarative aura (DP and keyword grants filtered by name/trait/level)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-084 Nokia Shiramine (+1000 DP to Greymon/Garurumon/Omnimon), BT5-093 Tai Kamiya & Matt Ishida (Security A.+1 to Omnimon), ST21-13 Matt Ishida & T.K. Takaishi (Rush to Lv5+ ADVENTURE Digimon), BT17-095 (observer filter), ST20-15 (security zone aura)
- **Effect text:** "[All Turns] All your Digimon with [Greymon], [Garurumon] or [Omnimon] in their names get +1000 DP." / "gain ＜Rush＞"
- **What's missing:** `Effect::declarative(card).dp_modifier(n)` buffs only the source permanent and consumes a static integer. No primitive for aura-style static DP/keyword grants to **other** permanents filtered by name/trait/level, re-evaluated as the field changes. Mask/tensor/combat queries must consult active auras at read time, not bake fixed modifiers.
- **Suggested API shape:** `Effect::aura(card).target_filter(|rctx, h| bool).grant_keyword(Keyword).dp_modifier(n)` consulted by `effective_dp`, `has_keyword`, and mask at query time. Alternative: explicit `ModifierRegistry::query_aura` pass.
- **Workaround:** Iteratively applying per-permanent modifiers on every state change leaks on new plays and can't be revoked when the source leaves. Not faithful.
- **Related:** Granted-triggered-ability gap, Native keyword parsing.

### Declarative aura sourced from security zone
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** ST20-15 Island of Adventure
- **Effect text:** "[Security] [All Turns] All of your level 3 or higher Digimon get +2000 DP."
- **What's missing:** Tensor / mask / modifier passes iterate only `battle_area` permanents. No "active aura sourced from a face-up security card" query path; `ctx.source_permanent` is `Option<PermanentHandle>` with no security-source variant.
- **Suggested API shape:** Promote face-up security entries to effect sources; extend DP / source-aggregation walks to include face-up security; add `SecuritySource { player, security_index, card_index }` variant on effect-source handles.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_PYTHON_PARITY §3.3.

### Variable / computed static DP modifier (formula per-count)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** P-182 WarGreymon (+1000 per color across own Digimon + Tamers)
- **Effect text:** "[All Turns] This Digimon gets +1000 DP for each color Digimon and Tamers have."
- **What's missing:** `EffectBuilder::dp_modifier(n: i32)` is static; `source_dp_contribution` consumes a fixed integer. No `dp_modifier_fn(|&EffectReadContext| -> i32)` accessor.
- **Suggested API shape:** Add `EffectBuilder::dp_modifier_fn(Arc<dyn Fn(&EffectReadContext) -> i32 + Send + Sync>)` stored in a new optional field and consulted when present.
- **Workaround:** None — BLOCKED (approximating violates §17).
- **Related:** RUST_PYTHON_PARITY §3.1.

### Digivolution-stack name overlay ("has all names of Lv.N cards in materials")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-102 Greymon
- **Effect text:** "[All Turns] This Digimon has all the names of level 3 and lower cards in its digivolution cards."
- **What's missing:** `Permanent::contains_card_name` already walks the stack for self-checks but external name lookups on this permanent see only the top card's printed name. No "virtual name overlay" mechanism synthesizing additional names for external queries.
- **Suggested API shape:** `Effect::declarative(card).name_overlay_from_sources(|src, data| bool)`; update name-lookup surfaces to union overlays.
- **Workaround:** None faithful for external observers.
- **Related:** none.

### Source-scoped CannotBeAffected (opponent-sourced only)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX9-021 Omnimon Alter-S, AD1-009 BlitzGreymon, EX10-010 BlackWarGreymon, ST20-11 WarGreymon
- **Effect text:** "your opponent's effects don't affect this Digimon for the turn."
- **What's missing:** `ModifierType::CannotBeAffected` is a coarse flag. `ModifierEntry` carries `source_player` but effect-application sites don't consult it; no scope discriminator between "opponent-sourced" and "own-sourced" (or "opponent-Digimon-effects-only" variant).
- **Suggested API shape:** Either `ModifierType::CannotBeAffectedByOpponent`, or `ModifierEntry::scope: Option<SourceScope>` honored at every effect-application site.
- **Workaround:** Applying unconditional `CannotBeAffected` also blocks controller's own buffs. Not faithful.
- **Related:** RUST_PYTHON_PARITY §4.7x.

### Attack-without-suspending for effect-granted MayAttack
- **Severity:** 🟡 PARTIAL — *primitive-with-fidelity-cost*
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT20-102 Omnimon (X Antibody), AD1-009 BlitzGreymon
- **Effect text:** "gain ＜Rush＞ for the turn and attack without suspending."
- **What's missing:** The no-suspend attack path is wired only for Vortex / Overclock (`begin_attack_overclock`). `ModifierType::MayAttack` in `EndOfTurnAction` goes through the standard `begin_attack`, which suspends. No `ModifierType::MayAttackWithoutSuspending` variant or `no_suspend: bool` bit on `ModifierEntry`.
- **Suggested API shape:** Either add `ModifierType::MayAttackWithoutSuspending` or thread a `no_suspend` flag into `begin_attack_impl`'s suspension branch.
- **Workaround:** Grant Rush + MayAttack — faithfully wrong (still suspends).
- **Related:** RUST_PYTHON_PARITY §4.6b.

### MayAttack scoped to player-target only
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-081 Tai Kamiya & Matt Ishida
- **Effect text:** "[End of Your Turn] 1 of your Digimon with [Omnimon] in its name may attack a player."
- **What's missing:** `ModifierType::MayAttack` mask emission covers both Digimon and player targets; no variant for "attack a player only."
- **Suggested API shape:** `ModifierType::MayAttackPlayer` or a `target_scope` field on `MayAttack` honored by mask.
- **Workaround:** Granting generic MayAttack over-grants Digimon-target attacks the card text disallows. Violates §17.
- **Related:** RUST_PYTHON_PARITY §4.6c.

### StartOfYourTurn timing firing (enum exists, never fired)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-019 Gabumon, BT5-093 Tai Kamiya & Matt Ishida, BT21-102 Tai Kamiya, BT15-020 Gabumon
- **Effect text:** "[Start of Your Turn] …"
- **What's missing:** `Game::begin_turn` does not enqueue `EffectTiming::StartOfYourTurn`. Variant exists but no drainer. Also no `StartOfOpponentsTurn` firing.
- **Suggested API shape:** `Game::fire_start_of_your_turn(player)` mirroring `fire_end_of_your_turn`, called from `begin_turn` after Draw, before Breeding.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_ENGINE_API.md §9.

### StartOfYourMainPhase timing
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-019 Gabumon, BT22-084 Nokia Shiramine, BT22-089 Mirei Mikagura, BT17-007 Agumon, BT15-020 Gabumon
- **Effect text:** "[Start of Your Main Phase] …"
- **What's missing:** No `EffectTiming::StartOfYourMainPhase` variant (distinct from `StartOfYourTurn` which fires before Draw/Breeding). Python distinguishes the two.
- **Suggested API shape:** Add variant + enqueue hook in `Game::enter_main_phase` (new fn).
- **Workaround:** Folding into `StartOfYourTurn` is wrong — fires before Draw.
- **Related:** StartOfYourTurn gap.

### OnLeaveField observer firing + cause discrimination
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-095, BT22-015 Omnimon (Decode "other than in battle"), AD1-025 Omnimon (Partition "other than by own effects or battle"), EX4-060 Omnimon Alter-S ("other than by one of your effects")
- **Effect text:** "When this Digimon would leave the battle area other than in battle, …" / "… other than by one of your effects"
- **What's missing:** `EffectTiming::OnLeaveField` variant exists but no firing site; `OnDeletion` fires uniformly with no cause/provenance. Need `LeaveCause { Battle, OwnEffect, OpponentEffect, Rules }` threaded through all exit paths (`delete_permanent`, `return_to_hand`, `return_to_deck`, security-move) and exposed on `EffectContext` for observer / replacement conditions.
- **Suggested API shape:** `Game::fire_on_leave_field(leaver, cause, source_player)` invoked at every exit path; `TriggerSource::ForeignPermanentObservers` fan-out; `EffectContext.leave_cause: Option<LeaveCause>`. Also "would leave field" replacement variant (`WouldLeaveField`) with cancel semantics.
- **Workaround:** None — BLOCKED.
- **Related:** Ace Overflow, Partition, Decode, Self-replacement on leave-field gaps all depend on this.

### OnAllyPlayed / OnEnterFieldAnyone observer fan-out
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-005 Tsumemon, EX9-066 Tai Kamiya & Matt Ishida, BT17-081 Tai Kamiya & Matt Ishida, EX9-019, EX9-012, AD1-001, AD1-010, EX4-061 Matt Ishida & Tai Kamiya
- **Effect text:** "When any of your Digimon are played / When you play a [Gabumon] or [Agumon] …"
- **What's missing:** `play_from_hand` → `fire_on_play` fires OnPlay for the played card only. No fan-out to other permanents' `OnEnterFieldAnyone` observers, and — critically — no fan-out to **hand-resident** effects (needed for EX9-019/EX9-012/AD1-001/AD1-010 whose hand cards listen for ally plays). `EffectTiming::OnEnterFieldAnyone` declared, never fired.
- **Suggested API shape:** After `fire_on_play`, enqueue `OnEnterFieldAnyone` via `TriggerSource::PlayerBattleArea(player)` AND via a new `TriggerSource::HandObserver { controller }` that scans hand cards for matching observer effects. `EffectContext` exposes `triggering_permanent` / `triggering_card`.
- **Workaround:** None — BLOCKED.
- **Related:** OnDigivolve fan-out (sibling).

### OnDigivolve / OnDnaDigivolve observer fan-out
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX4-039 Gabumon, EX4-061 Matt Ishida & Tai Kamiya, EX4-003 Tsunomon, BT17-081, EX9-019, EX9-012, AD1-001, AD1-010, EX4-003 Tsunomon
- **Effect text:** "[Your Turn] [Once Per Turn] When one of your other Digimon digivolves, …" (and variants)
- **What's missing:** `EffectTiming::OnDigivolve` / `OnDnaDigivolve` variants declared; the digivolve code path enqueues only `WhenDigivolving` on the digivolving card itself, never broadcasts to observers. No `TriggerSource::PlayerBattleAreaExcluding { except }` for "other Digimon" filter, and no hand-observer fan-out.
- **Suggested API shape:** Broadcast `OnDigivolve` via `PlayerBattleArea(controller)` + hand observer fan-out after a successful digivolve; expose digivolved permanent to observer context.
- **Workaround:** None — BLOCKED.
- **Related:** OnAllyPlayed sibling; "Free-digivolve-from-hand on trigger" depends on this.

### OnSecurityCheck / opponent-security-removed observer
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT14-001 Koromon
- **Effect text:** "[Your Turn] When a card is removed from your opponent's security stack, ＜Draw 1＞"
- **What's missing:** `EffectTiming::OnSecurityCheck` variant exists but is never enqueued. DigiEgg inherited effect on a field permanent needs the observer to fire across all effect-carriers on the controller's side. Also: non-combat security removal (effect-driven) needs the same enqueue.
- **Suggested API shape:** Fire `OnSecurityCheck` in `resolve_security_card` + at every `player.security.pop`/`remove` site; `EffectContext.triggering_defender / triggering_card` available.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_PYTHON_PARITY §2.5b.

### OnSuspend observer firing (self + ally)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT15-101 MetalGarurumon (self), BT13-012 GeoGreymon (inherited — ally Tamer suspend)
- **Effect text:** "[All Turns] [Once Per Turn] When this Digimon becomes suspended, you may unsuspend it." / "When one of your red or yellow Tamers becomes suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less."
- **What's missing:** `EffectTiming::OnSuspend` variant declared; no enqueue site. Every suspend-mutating path (`ctx.suspend`, combat attack declaration, Alliance declaration, Force/May-attack suspend) must fire it.
- **Suggested API shape:** Enqueue `TriggerSource::PermanentSuspended { perm, previous_state }` at every mutation site; drainer dispatches `OnSuspend` self + ally observer fan-out.
- **Workaround:** None — BLOCKED.
- **Related:** Observer fan-out family.

### OnHatch / OnMoveFromBreeding trigger
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-093 Tai Kamiya & Kari Kamiya (OnHatch), BT16-082 Ukkomon (OnMoveFromBreeding), P-123 Ukkomon (OnMoveFromBreeding)
- **Effect text:** "When you hatch in the breeding area, …" / "When one of your Digimon moves from the breeding area to the battle area, …"
- **What's missing:** `Game::hatch` and `Game::move_from_breeding` mutate state silently. No `EffectTiming::OnHatch` / `OnPromoteFromBreeding` variants and no enqueue sites. Also needed: `ctx.hatch(player)` helper (today only `Game::hatch`, not exposed through `EffectContext`).
- **Suggested API shape:** Add variants + enqueue at both sites. Expose `ctx.hatch(player) -> bool`.
- **Workaround:** None — BLOCKED.
- **Related:** none.

### EndOfAttack / WhenAttacking timings firing
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX9-021 Omnimon Alter-S (EndOfAttack), EX4-073 Omnimon Alter-B (WhenAttacking), EX1-068 Ice Wall! (WhenAttacking granted), ST20-11 WarGreymon (WhenAttacking)
- **Effect text:** "[End of Attack] You may play 1 card with [Greymon] …" / "[When Attacking] By trashing up to 3 level 6 or higher cards …"
- **What's missing:** Both `EffectTiming::EndOfAttack` and `EffectTiming::WhenAttacking` are enum variants but never fired by combat. `Effect::when_attacking(card)` builder constructor doesn't exist. DCGO fires WhenAttacking per attack-start (distinct from OnAttack mandatory declaration).
- **Suggested API shape:** In `combat.rs::advance_pending_attack`, after OnAttack fires, enqueue `WhenAttacking` on the attacker; enqueue `EndOfAttack` at the tail of attack resolution before EndOfBattle modifier clear. Add matching builder constructors.
- **Workaround:** None — BLOCKED (OnAttack is mandatory; WhenAttacking is optional — conflating loses fidelity).
- **Related:** RUST_ENGINE_API.md §9.

### Ally-observer triggering context (attacker / defender / played card accessor)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT14-001 Koromon, BT22-005 Tsumemon, EX4-039, EX4-061, EX9-066, EX9-019, and every observer-trigger card above.
- **Effect text:** "When one of your Digimon with the [Unidentified] or [CS] trait is played …"
- **What's missing:** `EffectContext` has no `triggering_permanent` / `triggering_card` / `triggering_defender` / `leave_cause` accessors. Observers can't inspect the event they're responding to.
- **Suggested API shape:** Extend `EffectContext` with `triggering_permanent: Option<PermanentHandle>`, `triggering_card: Option<CardSource>`, `triggering_player: Option<PlayerId>`, `triggering_defender: Option<PermanentHandle>`, populated per-enqueue.
- **Workaround:** None — BLOCKED.
- **Related:** RUST_PYTHON_PARITY §2.5g.

### Delayed one-shot turn-scheduled trigger
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT1-090 Gravity Crush
- **Effect text:** "At end of turn, lose 2 memory."
- **What's missing:** A scheduler for one-shot triggers installed from a resolving Option effect; survives the source leaving all zones; fires once at the scheduled phase.
- **Suggested API shape:** `ctx.schedule_delayed_trigger(timing, once: true, effect: Effect)` storing to a per-player list consumed at phase transition.
- **Workaround:** None — BLOCKED.
- **Related:** Option play flow.

### Replacement effect: prevent battle deletion by paying a cost
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX5-015 Gabumon (X Antibody)
- **Effect text:** "[All Turns] [Once Per Turn] When this Digimon … would be deleted in battle, by returning 2 non-Digi-Egg cards from your trash to the bottom of the deck, prevent that deletion."
- **What's missing:** No pre-deletion interrupt in `combat.rs::resolve_pending_battle`; deletion is atomic. `CannotBeDestroyedByBattle` modifier is unconditional and can't gate on paying a cost.
- **Suggested API shape:** New `EffectTiming::WouldBeDeletedInBattle`; `resolve_pending_battle` enqueues per-combatant with `cause = Battle`, suspends the combat state machine on a `PendingSelection`, paired `ctx.prevent_deletion()` primitive.
- **Workaround:** None — BLOCKED.
- **Related:** Evade keyword sibling gap; general replacement-effect infrastructure.

### Suspend-self as activation cost (pay-cost builder hook)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX9-066 Tai Kamiya & Matt Ishida, EX4-061 Matt Ishida & Tai Kamiya, BT17-081 Tai Kamiya & Matt Ishida, BT21-102 Tai Kamiya, BT17-093 Tai Kamiya & Kari Kamiya, BT5-092 Nokia Shiramine, BT13-012 GeoGreymon, ST21-13
- **Effect text:** "by suspending this Tamer, …"
- **What's missing:** `ctx.suspend` exists but the cost shape — "prompt controller to consent, pay cost iff suspend succeeds, otherwise skip effect" — isn't modeled. `Effect::optional()` handles yes/no but doesn't bind to a state-mutating cost atomic with the effect.
- **Suggested API shape:** `EffectBuilder::pay_cost_suspend_self()` + generalized `.pay_cost(|ctx| bool)` that aborts the process if returns false.
- **Workaround:** Check `is_suspended` in condition + manually suspend in process; scatters cost logic, muddy cost-vs-effect atomicity.
- **Related:** BeforePayCost family.

### Return-self-to-bottom-of-deck as activation cost
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-089 Mirei Mikagura, BT17-093 Tai Kamiya & Kari Kamiya, BT22-094 Yuugo Kamishiro, BT21-102 Tai Kamiya
- **Effect text:** "By returning this Tamer to the bottom of the deck, …"
- **What's missing:** Depends on "Return permanent to bottom of deck" primitive + the cost-builder hook above.
- **Suggested API shape:** `.pay_cost_return_self_to_deck_bottom()` on `EffectBuilder`.
- **Workaround:** None — BLOCKED.
- **Related:** Return-to-bottom-of-deck gap; suspend-as-cost gap.

### Multi-target permanent selection (pick N distinct)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT15-101 MetalGarurumon (3 opp Digimon/Tamers), AD1-014 MetalGarurumon (N per 2 tamer-colors), ST20-11 WarGreymon (N per 2 tamer-colors), BT20-102 Omnimon (X Antibody) (cross-player pick-one)
- **Effect text:** "3 of your opponent's Digimon and Tamers can't suspend until the end of their turn."
- **What's missing:** `select_*` helpers are single-pick. No N-pick with per-pick filter, no "pick one of either player" variant.
- **Suggested API shape:** `ctx.select_multi_permanent(prompt, of_player, count, is_optional, filter, on_resolve)` + `ctx.select_any_permanent(prompt, filter, callback)` spanning both sides.
- **Workaround:** Callback chaining loses "distinct" invariant without manual bookkeeping.
- **Related:** Budgeted multi-select below.

### Budgeted multi-select ("delete up to N cost-worth")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX4-073 Omnimon Alter-B
- **Effect text:** "Then, delete up to 6 play cost's total worth of their Digimon."
- **What's missing:** New `SelectionKind::BudgetedMulti` parameterized by budget + cost-fn + per-pick callback + optional stop.
- **Suggested API shape:** `ctx.select_budgeted(budget, cost_fn, is_optional_stop, callback_per_pick)`.
- **Workaround:** Auto-delete lowest-cost violates §17.
- **Related:** Multi-target selection.

### Multi-pick from revealed pool with per-category filters
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-017 Gabumon, EX4-039 Gabumon, EX4-038 Agumon, BT12-059 Agumon, P-206 Digital Gate Open, EX5-015 Gabumon (X Antibody)
- **Effect text:** "Add 1 Digimon card with [Greymon] and 1 Tamer card with [Tai Kamiya] among them to your hand."
- **What's missing:** `select_reveal` is single-pick; multi-pick with per-slot filters and sequential state-shrinking must be chained manually, and there's no `add_revealed_to_hand` to connect the picks to the outcome. Auto-picking first legal card in each category violates §17.
- **Suggested API shape:** `select_reveal_multi(categories: Vec<(filter, optional)>, callback)` or documented callback-chaining idiom with `add_revealed_to_hand`.
- **Workaround:** None faithful today.
- **Related:** Reveal-top-N + add-revealed-to-hand gaps.

### Self-stack material trash by filter (up to N sources)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX4-073 Omnimon Alter-B
- **Effect text:** "By trashing up to 3 level 6 or higher cards in this Digimon's digivolution cards, for each card trashed, activate the effect below."
- **What's missing:** `select_material` is single-pick + doesn't trash. Needs up-to-N multi-pick with filter + per-trash sub-effect loop + `ctx.trash_material(target, source_index)`.
- **Suggested API shape:** `ctx.select_materials_multi(target, max, filter, callback_per_pick, on_finish)` + `ctx.trash_material(target, source_index)`.
- **Workaround:** Auto-trash top N violates §17.
- **Related:** Budgeted multi-select.

### Expiry::EndOfOpponentsNextTurn (duration spans opponent's entire next turn)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX1-068 Ice Wall!, AD1-014 MetalGarurumon (cross-turn CannotUnsuspend)
- **Effect text:** "until the end of their next turn."
- **What's missing:** `Expiry` has `EndOfTurn`, `EndOfOpponentsTurn`, `EndOfAttack`, `EndOfBattle`, `UntilLeaveField`, `Permanent` — but no "end of target's NEXT turn" variant. Played on the controller's own turn, `EndOfOpponentsTurn` fires at the wrong anchor.
- **Suggested API shape:** Add `Expiry::EndOfTargetsNextTurn` or `Expiry::EndOfTurnAfter { player, turns_forward: u8 }` resolved by modifier sweep consulting turn-rotation.
- **Workaround:** None — BLOCKED.
- **Related:** Modifier registry.

### Trait parsing in `CardData`
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-005 Tsumemon ([Unidentified]/[CS]), BT22-089 Mirei Mikagura ([CS]/[Holy Beast]/[Angel]/etc.), ST20-10 Agumon (Tamer trait union), BT22-099 Kuremi Detective Agency ([CS]), BT22-094 Yuugo Kamishiro ([CS]), BT22-084 Nokia Shiramine (named-trait aura), ST21-13 ([ADVENTURE])
- **Effect text:** "with the [Unidentified] or [CS] trait" and similar
- **What's missing:** `CardData.traits` exists as `Vec<String>` per resolve_deck, but `CardSource::has_trait(name)` / `Permanent::has_trait(name)` surface is not guaranteed available from every observer/filter closure, and traits aren't parsed from `effect_text` of cards that reference them (printed text only has keyword-style tags that need normalized parsing for reliable matching).
- **Suggested API shape:** Ensure `CardSource::has_trait(data, name)` and `Permanent::has_trait(data, name)` exist; document case-insensitivity; ensure cards.json ingest populates traits.
- **Workaround:** Manual iteration over `card_data.traits` in closures — verbose but likely viable once ingest is verified.
- **Related:** RUST_PYTHON_PARITY §2.1b.

### Per-permanent OPT activation recording (EffectContext sugar)
- **Severity:** 🟡 PARTIAL — *ergonomics / sugar*
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT23-008 Greymon, BT15-020 Gabumon, and any `[Once Per Turn]` clause with compound sub-effects
- **Effect text:** "[Main] [Once Per Turn] …"
- **What's missing:** RUST_ENGINE_API §13 flags "`ctx.record_activation()` helper would be a nice follow-up". Today, effects with sub-selections that decouple cost-payment from resolution can't control OPT counter timing cleanly.
- **Suggested API shape:** `ctx.record_activation()` and `ctx.activation_count()` keyed on the slot.
- **Workaround:** Reach into `Permanent::record_activation` — works, violates curated-API discipline.
- **Related:** RUST_ENGINE_API §13.

### Dual-timing composite clause ("[When Digivolving] [When Attacking] …")
- **Severity:** 🟡 PARTIAL — *ergonomics / sugar*
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** ST20-11 WarGreymon, BT15-020 Gabumon, others
- **Effect text:** "[When Digivolving] [When Attacking] Delete 1 of your opponent's lowest DP Digimon."
- **What's missing:** `EffectBuilder::process` closure is `Fn + Send + Sync + 'static` (not Clone); a single closure can't be installed in two `Effect` records without manual `Arc`. Ergonomics.
- **Suggested API shape:** `EffectBuilder::on_timings(&[EffectTiming])` stamping multiple `Effect` records sharing an `Arc`'d process closure.
- **Workaround:** Duplicate closure body in two Effects; viable, risk of drift.
- **Related:** `effect.rs`.

### Aggregate filter helpers (lowest DP / lowest level / highest DP with tie-break)
- **Severity:** 🟡 PARTIAL — *ergonomics / sugar*
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-013 WarGreymon (lowest DP), BT22-026 MetalGarurumon (lowest level), AD1-012 CresGarurumon (lowest level), ST20-11 (lowest DP), EX10-010 (Raid highest DP tie-break)
- **Effect text:** "Delete 1 of your opponent's Digimon with the lowest DP."
- **What's missing:** `select_opponent_permanent` accepts a filter; scripts must pre-compute min/max externally. No convenience helpers. Works today via inline iteration.
- **Suggested API shape:** `ctx.select_opp_permanent_by_min(|perm| extractor, prompt, callback)` / `_by_max`.
- **Workaround:** Inline iteration is faithful.
- **Related:** none.

### Decode keyword (play from own digivolution stack without paying cost on non-battle leave)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-015 Omnimon
- **Effect text:** "＜Decode (Red/Black Lv.3)＞ (When this Digimon would leave the battle area other than in battle, you may play 1 Red or Black Level 3 Digimon card from its digivolution cards without paying the cost.)"
- **What's missing:** Combines (a) `OnLeaveFieldNonBattle` replacement timing, (b) `ctx.play_from_materials` free-play, (c) select-source helper with filter. All are unbuilt.
- **Suggested API shape:** Composed across existing gaps.
- **Workaround:** None — BLOCKED.
- **Related:** OnLeaveField cause, Play-from-materials, Selection family.

### Grant attack permission after WhenDigivolving (inline unsuspend / "this Digimon may attack")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT22-015 Omnimon ("Then, this Digimon may attack.")
- **Effect text:** "Then, this Digimon may attack."
- **What's missing:** After WhenDigivolving resolves, let the freshly-digivolved permanent attack this turn ignoring summoning sickness and (for this specific clause) memory-sign. `ModifierType::MayAttack` wired only for `EndOfTurnAction`; Rush covers summoning sickness but not negative memory; Blitz allows negative memory but semantically wrong keyword.
- **Suggested API shape:** `ModifierType::MayAttackThisTurn` granting attack now regardless of memory sign + turn_played.
- **Workaround:** Blitz grant is PARTIAL (wrong keyword).
- **Related:** RUST_PYTHON_PARITY §4.3 / §4.6c.

### CannotPlayDigimonByEffect modifier (distinct from CannotPlayFromHand)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT8-097 Crimson Blaze
- **Effect text:** "Your opponent can't play Digimon by effects until the end of their turn."
- **What's missing:** `CannotPlayFromHand` covers player-initiated plays; does NOT cover effect-driven plays (`ctx.play_from_hand_free`, future `effect_play_from_deck`, `play_from_trash_free`). Needs a separate modifier + checkpoint at every effect_play_* site.
- **Suggested API shape:** `ModifierType::CannotPlayDigimonByEffect` keyed by player; gated at every effect-initiated play site.
- **Workaround:** None — BLOCKED.
- **Related:** Free-play gaps.

### If-effect-didn't-resolve branch ("If this effect didn't return, …")
- **Severity:** 🟡 PARTIAL — *primitive-with-fidelity-cost*
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** EX9-066 Tai Kamiya & Matt Ishida, BT16-082 Ukkomon (optional hatch tail)
- **Effect text:** "You may return 1 Digimon card … If this effect didn't return, ＜Draw 1＞"
- **What's missing:** Builder's `optional()` is pre-prompt; if declined the whole process is skipped with no else-branch. No `ctx.was_optional_declined()` / `on_decline` hook. `PendingSelection.on_decline` field exists but no builder exposes it.
- **Suggested API shape:** Expose `select_*_with_decline(..., on_decline)`; or `select_*`'s callback takes `Option<usize>`.
- **Workaround:** Track via closure-captured bool; depends on callback firing on decline, which is not guaranteed.
- **Related:** Selection family.

## Deferred — verification / test coverage only

Items where the existing primitive **likely works** but no behavioral test covers the specific pathway. Not engine gaps; filed here so they surface when the archetype moves to `/batch-implement-cards-rust` and a faithful DebugRunner test must be written. Do not count toward the BLOCKING / PARTIAL tallies above.

- **Tamer play-from-security pipeline** — `ctx.play_from_security` was written against `CardKind::Digimon`; `CardKind::Tamer` routing through the same path + subsequent `[Your Turn]` / `[All Turns]` observers is unverified. Cards: BT17-081, BT22-089, BT5-092, EX9-066, ST20-15, EX4-061. See RUST_PYTHON_PARITY §2.5a, §2.5j.
- **Option multi-color match semantics** — RUST_PYTHON_PARITY §4.2 implements color match; verify multi-color Options require at least one matching own-side permanent **per** printed color (intersection), not any-one (union). Card: BT17-095. See RUST_PYTHON_PARITY §4.2, §4.2b.
- **Conditional inherited DP based on top-card name** — fully expressible today via `Effect::inherited(card).dp_modifier(n).condition(|ctx| ctx.source_permanent()...)`. Confirm the per-source walker passes the correct `source_permanent` into the read context. Cards: BT12-059, BT23-008.

## Resolved gaps

(none yet)
