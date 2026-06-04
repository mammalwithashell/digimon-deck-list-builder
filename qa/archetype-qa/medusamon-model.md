# Medusamon — Model

> Durable system-level model for the `/archetype-interaction-test-author` capstone.
> Authored 2026-05-30. Pool resolved via `resolve_deck.py "Medusamon"` (141 decklists,
> 63 unique cards). Card text from `cards.json`; behavior verified against DCGO C#
> (`$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/Red/<CARD_ID>.cs`) and
> `general_rule.pdf` §16 (keywords). This model drives
> `code/digimon-engine/tests/archetypes/medusamon.rs`.

Medusamon is a **Red [LIBERATOR] / [Reptile]·[Dragonkin]** aggro-combo deck. Its
whole engine is built on one trigger family — **"[When] your opponent's security
stack is removed from"** — and a self-feeding loop that *creates* opponent
security removal on demand via **Petrification Tokens**.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT21-001 Gigimon (egg) | engine | Inherited [Your Turn][OPT]: on opp-security-removed, free digivolve into [Reptile]/[Dragonkin] (cost −1). |
| BT21-008 Elizamon (L3) | enabler | [On Play] reveal-3, add a [Reptile]/[Dragonkin] + a [LIBERATOR]. INH: gain 1 memory on opp-security-removed. |
| BT24-008 Elizamon (L3) | enabler | [On Play] trash a trait card → Draw 2. INH: gain 1 memory on opp-security-removed. |
| BT21-017 Dimetromon (L4) | enabler | [When Digivolving] play Owen free (≤1 Tamer). INH: gain 1 memory on opp-security-removed. |
| BT24-012 Dimetromon (L4) | tech/engine | Blocker; [All Turns] return self to hand to stop an ally [Reptile]/[Dragonkin] leaving by opp effects. INH: gain 1 memory. |
| BT21-025 Lamiamon (L5) | engine | Progress; [Your Turn][OPT] on any of your [Reptile]/[Dragonkin]'s **attack-target change**, trash opp top security. INH: on opp-security-removed, free-play ≤5000 trait Digimon. |
| BT24-016 Lamiamon (L5) | payoff/ramp | [Hand][Main] cheat into play under Elizamon (cost 3, w/ Owen); [When Digivolving/Attacking] opp places hand→bottom security, then trash their top security. |
| BT24-017 Medusamon (L6) | **payoff** | Raid/Progress/Piercing; [When Digivolving] delete opp lowest-DP, then opp plays **2 Petrification Tokens**; +2000 DP per opp Digimon. |
| EX11-012 Medusamon (L6) | payoff | Rush/Progress; [When Digivolving][End of Attack] delete ≤self-DP, opp plays **1 Petrification Token**; survives by deleting a Token. |
| BT24-018 Styracomon (L7) | **payoff** | Progress/Piercing/Blocker/Armor Purge; [All Turns][OPT] on opp-security-removed, delete 1 of their Digimon; would-leave wall via opp-lowest-DP delete. |
| BT24-011 Cyclonemon (L4) | enabler | **Rush + Raid** — the attacker that mechanically forces an attack-target change. |
| BT24-082 Owen Dreadnought (Tamer) | **engine** | On a [Reptile]/[Dragonkin] digivolve, suspend Owen → that Digimon +3000 DP & **may attack**; recursion via return-self/replay. |
| BT21-081 Owen Dreadnought (Tamer) | engine | Memory ramp; [End of Turn] suspend → grant a trait Digimon Piercing + an attack. |
| BT21-093 Raging Serpentine (Option) | removal | Delete opp highest-DP; [All Turns] on opp-security-removed → Delay free digivolve. |
| BT24-089 Unique Emblem (Option) | engine | Free-play Elizamon/Owen; on Owen suspend → Delay free digivolve (cost −3). |
| P-103 / P-035 / LM-027 | enabler | Red dig/ramp Options with Delay digivolve tails. |
| BT8-097 Crimson Blaze (Option) | tech | Cost scales with opp board; lock opp plays + wipe all ≤6000 DP. |

## Digivolution lines

- **Egg → L7:** Gigimon (BT21-001) → Elizamon L3 (BT21-008/BT24-008) → Dimetromon
  L4 (BT21-017/BT24-012) → Lamiamon L5 (BT21-025/BT24-016) → Medusamon L6
  (BT24-017/EX11-012) → Styracomon L7 (BT24-018). All Red, all [LIBERATOR] except
  the non-trait Options; the line is mono-trait ([Reptile] low, [Dragonkin] high)
  so every "any of your [Reptile]/[Dragonkin]" condition is trivially satisfied.
- **Cheat path:** BT24-016 Lamiamon places a Dimetromon from trash under an
  Elizamon and digivolves for cost 3 ignoring requirements (needs Owen in play).
- **Tamer engine:** Owen Dreadnought (BT24-082 / BT21-081) sits beside the line
  feeding memory, DP, and extra attacks.

## Named combos

### Petrification security-trash loop  *(signature)*
- Cards: BT24-017 Medusamon (token source), **Petrification Token**, BT24-018 Styracomon (payoff).
- Expected mechanical outcome: A Petrification Token is created **on the
  opponent**. When that token is deleted, its `[On Deletion]` trashes **the
  token-controller's (= opponent's) top security card**. That security removal is
  an opp-security-removed event, which fires the Medusamon line's
  `[When] opponent's security stack is removed from` payoffs (e.g. BT24-018's
  optional delete). So *deleting our own gift to the opponent* both strips their
  security and triggers our removal — the engine's flywheel.
- Rules/keyword basis: token `[On Deletion]` → `EffectContext::trash_top_security(owner)`
  where `owner = ctx.player` = the token's controller
  (`src/cards/tokens/petrification.rs`); `trash_top_security` calls
  `fire_security_removed_observers` (`effect_context/mod.rs:2416`), so the
  downstream opp-security-removed trigger **auto-chains** within the same
  deletion. DCGO `BT24_017.cs` (`PlayPetrificationToken` on `card.Owner.Enemy`),
  `BT24_018.cs` (`OnLoseSecurity` → optional delete). `general_rule.pdf` §17-1-3
  (deletion), rule 25 (OnDeletion fires post-trash).
- Rank: **1** — BT24-017 freq 122, BT24-018 freq 136; this is the deck's identity
  and is *not* covered by any per-card test (none deletes a token and checks the
  security/chain consequence).

> Faithfulness note: the token's `[Your Turn] This Digimon can't suspend` rider
> is **not yet implemented** (`petrification.rs` gap — only the OnDeletion clause
> ships). Tests must not assert can't-suspend.

### Owen digivolve buff + extra attack
- Cards: BT24-082 Owen Dreadnought + any [Reptile]/[Dragonkin] ally.
- Expected mechanical outcome: when one of your Digimon digivolves into a
  [Reptile]/[Dragonkin], Owen **suspends itself**, grants that Digimon **+3000 DP
  for the turn**, and offers it an **immediate optional attack**. Gated on the
  digivolve target's trait — a non-trait target leaves Owen unsuspended with no
  buff and no attack.
- Rules/keyword basis: DCGO `BT24_082.cs` (`OnEnterFieldAnyone` → suspend self,
  `ChangeDigimonDP +3000 UntilEachTurnEnd`, `SelectAttackEffect`).
  `general_rule.pdf` §15 ([Your Turn] / attack timing). The clause is optional
  ("by suspending this Tamer") so an outer accept/decline gate installs first.
- Rank: **2** — BT24-082 freq 134; the buff+attack is a 2-card interaction
  (Owen + the digivolving ally) the per-card test pins in isolation but the
  trait-gate flip is the system fact.

### Raid target-change → Lamiamon security trash
- Cards: BT24-011 Cyclonemon (Raid attacker) + BT21-025 Lamiamon (payoff).
- Expected mechanical outcome: a **Raid** attacker switching its attack target to
  the opponent's highest-DP unsuspended Digimon is an **attack-target change**,
  which fires BT21-025 Lamiamon's `[Your Turn][OPT]` clause to **trash the
  opponent's top security**. Gated on the event-source being a [Reptile]/[Dragonkin]
  Digimon — a plain attacker's target change trashes nothing.
- Rules/keyword basis: `general_rule.pdf` §16-22 Raid (switch to highest-DP
  unsuspended; optional; controller picks among ties). DCGO `BT21_025.cs`
  (`OnAttackTargetChanged` → `IDestroySecurity` fromTop), `BT24_011.cs` (Raid).
  Engine fires `EffectTiming::OnAttackTargetChange` with `TriggerSource::EventObserved`
  carrying the attacker; clause-2 condition gates on the attacker's owner+trait.
- Rank: **3** — BT21-025 freq 116, BT24-011 freq 128; couples the deck's pressure
  keyword (Raid) to its security-trash engine.

### Security-removal feeds the memory engine  *(the snowball)*
- Cards: BT24-008 Elizamon + BT24-012 Dimetromon (inherited memory) + any security remover.
- Expected mechanical outcome: a **single** opponent-security removal pays out the
  inherited "[Your Turn][OPT] → gain 1 memory" **once per buried source**, so two
  stacked inherited sources net **+2 memory** from one removal. The Petrification
  token loop is itself a security-removal source that feeds this ramp.
- Rules/keyword basis: BT24-008 / BT24-012 inherited (OnOpponentSecurityRemoved,
  [Your Turn], once_per_turn); `effect_context/mod.rs:2416`
  (`fire_security_removed_observers` reaches buried inherited sources).
- Rank: **4** — the deck's snowball; per-card tests only ever see a single source
  (+1), so the additive stacking and the token→memory coupling are system-only.

### Lamiamon inherited free-play (≤5000) on opp-security-removed
- Cards: BT21-025 Lamiamon (inherited) + a ≤5000 [Reptile]/[Dragonkin] in hand + a security remover.
- Expected mechanical outcome: when opp security is removed, Lamiamon's inherited
  lets you play a ≤5000 trait Digimon from hand **free**. DP-gated: a >5000 card is
  filtered out. Driving it via a **real** security removal exercises the inherited
  dispatch the per-card test (`bt21_025.rs::clause3`) leaves `#[ignore]`d.
- Rules/keyword basis: DCGO `BT21_025.cs` (inherited OnLoseSecurity → play-free
  ≤5000); `general_rule.pdf` §15.
- Rank: **5** — BT21-025 freq 116; un-ignores a real dispatch path; DP gate is the
  system fact.

### EX11-012 token-shield  *(was a faithfulness gap — now FIXED)*
- Cards: EX11-012 Medusamon + its opponent-side Petrification Token.
- Expected mechanical outcome (faithful): when EX11-012 would leave, it deletes
  **any** Token — in practice the opponent's gifted Petrification Token — to
  survive, which (via the token's [On Deletion]) also trashes the opponent's
  security (a nested loop).
- **Gap → fix:** the DSL originally used `select_own_permanent { kind: token }`
  (own-only); since EX11-012 only mints **opponent**-side tokens, the shield could
  never fire. DCGO `EX11_012.cs` uses `permanent.IsToken` (any owner). Fixed by
  changing the would-leave cost selector to `select_any_permanent { kind: token }`
  (scans both battle areas, controller picks) — a one-word YAML change; the DSL
  verb already existed. Gap `G-EX11-012-TOKEN-SHIELD-OWN-ONLY` is RESOLVED; the
  test `ex11_012_survives_by_deleting_opponents_petrification_token` now passes.
- Rank: **6** — confirmed cross-card bug the per-card test missed, surfaced by the
  interaction suite and fixed.

### Lamiamon cheat-in chains into its own security swap
- Cards: BT24-016 Lamiamon + Owen Dreadnought + Elizamon (field) + Dimetromon (trash).
- Expected mechanical outcome: the `[Hand][Main]` cheat-in (place Dimetromon under
  Elizamon, digivolve into Lamiamon for cost 3 ignoring requirements) is an
  effect-initiated digivolve that **fires Lamiamon's own `[When Digivolving]`
  swap** — the opponent places a hand card as bottom security and their top
  security is trashed (net security 0, hand −1, trash +1). One activation ramps a
  Lv.5 onto the board AND pressures security. Gated on all four pieces.
- Rules/keyword basis: DCGO `BT24_016.cs` ([Hand][Main] activated digivolve +
  WhenDigivolving as_selecting_player → place_on_security → trash_top_security).
- Rank: **7** — the per-card test never checks that the cheat-in *triggers* the
  swap; this verifies the cross-clause chain (and confirms effect-initiated
  digivolve fires WhenDigivolving).

### Cheat-in nets +2 memory from the two buried inherited sources
- Cards: BT24-016 Lamiamon + **BT24-082 Owen** + **BT24-008 Elizamon** + **BT24-012 Dimetromon** (all real).
- Expected mechanical outcome: the cheat-in builds the stack
  **[Dimetromon, Elizamon, Lamiamon]**, so Dimetromon and Elizamon are both
  digivolution sources. When Lamiamon's chained `[When Digivolving]` swap trashes
  the opponent's top security, **both** buried inherited "gain 1 memory on
  opp-security-removed" clauses fire → **+2 memory**.
- Verified with the real cards: net memory after the activation is **−1**
  (digivolution costs 3, then +2 from the inherits). −2 would mean only one
  inherited fired; −3 none — so the single absolute assertion proves both fire.
  This is folded into the Combo-7 happy test (`cheat_in_chains_swap_and_nets_two_memory_from_real_inherits`),
  not a separate synthetic-control test.
- Rules/keyword basis: BT24-008 + BT24-012 inherited (`gain_memory: 1` on
  `on_opponent_security_removed`, [Your Turn]); BT24-016 `effect_initiated_digivolve`
  fires WhenDigivolving (clause 2 swap → `trash_top_security`).
- Rank: **7 (memory facet)** — the real-card payoff flagged in review; my first
  pass used synthetic name-only Elizamon/Dimetromon (no inherited) and so never
  saw the memory. Whole suite re-authored to use the real implemented cards.

### Lamiamon security swap feeds the memory engine
- Cards: BT24-016 Lamiamon (swap) + BT24-008 Elizamon (buried inherited memory).
- Expected mechanical outcome: the swap's incidental top-security trash is an
  opp-security-removed event that pays out a buried Elizamon inherited (+1 memory)
  — the on-attack security pressure is another faucet into the ramp.
- Rules/keyword basis: DCGO `BT24_016.cs` (swap → trash_top_security);
  BT24-008 inherited (OnOpponentSecurityRemoved, [Your Turn]).
- Rank: **8** — couples a payoff's incidental security removal to the inherited
  engine; system-only.

### (Dropped — logged, not authored)
- **Medusamon DP self-pump (BT24-017):** the +2000-per-opp-Digimon boost counting
  its own 2 minted tokens is already fully pinned by
  `bt24_017.rs::bt24_017_full_sequence_two_trash_two_tokens_dp_boost` (+4000 with
  the 2 tokens as the only opp Digimon). No marginal coverage.
- **Styracomon would-leave wall (BT24-018):** opp-lowest-DP-delete-to-stay is
  pinned by `bt24_018.rs` would-leave tests. No marginal coverage.
- **Lamiamon cheat-in (BT24-016):** place-Dimetromon-under-Elizamon ramp — covered
  per-card; high authoring cost (stack surgery) for low system novelty.
- **BT24-016 security-swap chain:** opp-places-then-trash plus a buried inherited
  free-play — strong candidate for a future run.
- **Dimetromon protection wall (BT24-012):** the "by your opponent's effects" cause
  gate (own-effect / Battle do NOT protect) is already pinned per-card in
  `bt24_012.rs` (OwnEffect + Battle negatives). No marginal coverage.

## Playstyle

- **Class:** aggro-combo. Tempo deck that snowballs once a security-removed engine
  piece is online.
- **Memory curve:** low rookies + Owen ramp, then a single big digivolve into
  Medusamon/Styracomon that pays for itself via tokens and extra attacks.
- The deck *manufactures* the opp-security-removed condition (Petrification
  tokens, Lamiamon target-change, Medusamon/Lamiamon on-attack security trash)
  rather than waiting for combat to supply it.

## Win conditions

- Direct security pressure with Raid/Progress/Piercing attackers (Cyclonemon,
  Medusamon, Styracomon) bypassing blockers and chaining security checks.
- Petrification tokens force the opponent to bleed their own security on each
  token deletion, accelerating the clock while feeding our memory/free-play/
  free-digivolve engine.
- Styracomon (Armor Purge + would-leave wall) as a near-unkillable closer.

## Ranked interactions to test

1. **Petrification security-trash loop** — deleting an opponent token trashes their
   security and auto-fires a Medusamon-line payoff. The deck's identity; zero
   per-card coverage of the cross-card consequence.
2. **Owen digivolve buff + extra attack** — trait-gated +3000 & extra attack on
   digivolve; the tempo engine.
3. **Raid target-change → Lamiamon security trash** — couples Raid pressure to the
   security-trash engine, gated on the attacker's trait.
4. **Security-removal memory snowball** — one removal × N buried inherited sources =
   N memory; the token loop feeds it.
5. **Lamiamon inherited free-play (≤5000)** — DP-gated free play on opp-security-
   removed; exercises the dispatch the per-card test leaves ignored.
6. **EX11-012 token-shield** — *(known-failing)* faithful survival-by-deleting-an-
   opponent-token; surfaced the `select_own_permanent` owner gap.
7. **Lamiamon cheat-in → security swap** — the [Hand][Main] ramp auto-fires its own
   [When Digivolving] swap; cross-clause chain.
8. **Lamiamon security swap → memory** — the swap's incidental security trash feeds
   the buried inherited ramp.
