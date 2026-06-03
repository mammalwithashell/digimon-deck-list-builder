# Medusamon — Model

> Archetype-model artifact produced by `/archetype-interaction-test-author`
> (Phases 0–3). Durable, reviewable system model of the **Medusamon** (Red
> Reptile/Dragonkin LIBERATOR) archetype. Sources cited inline: DCGO C# path
> (`$BASE_DCGO/Assets/Scripts/CardEffect/...`) and/or `general_rule.pdf` §16
> rule numbers (keyword semantics) — DCGO + the PDF outrank the card-text JSON
> per CLAUDE.md source priority. Pool resolved with
> `python code/tools/resolve_deck.py "Medusamon" --json` (141 decklists, 64
> unique cards). Per-card DSL verdicts read from
> `qa/qa-reports/validated_cards_dsl.json`; card-text source:
> `data/cards.json`.

## The central engine (read this first)

Medusamon is a **Petrification Token pressure engine**. Its axis is gifting the
opponent **White 3000-DP Petrification Tokens** — tokens the opponent
*controls and owns* — whose `[On Deletion]` trashes the top of *their own*
security stack. The deck then pressures the opponent's security through two
complementary loops:

1. **Token feed loop:** EX11-012 / BT21-029 / BT24-017 Medusamon bodies place
   Petrification Tokens on the opponent's field. When those tokens are deleted
   (by any effect, or as a sticky-body cost) the opponent's own security is
   trashed. This is the backbone — the engine requires no attack to burn
   security.

2. **Security-removed punisher loop:** Once a card leaves the opponent's
   security stack, standing **punisher permanents** fire: BT18-087 Owen
   Dreadnought (`[All Turns]` suspend → delete ≤4000 DP Digimon), BT21-025
   Lamiamon (`[Your Turn][OPT]` trash top security when a Reptile/Dragonkin
   attack target changes), BT24-018 Styracomon (`[All Turns][OPT]` delete 1
   opponent Digimon when security is removed), and various Elizamon/Dimetromon
   inherited effects (gain memory on security removal). Each security removal
   fans into board-removal and more removal.

3. **Sticky-body replacement:** EX11-012 and BT24-018 both carry
   `[All Turns]` would-leave-by-deletion replacements that prevent a Medusamon
   body from leaving by consuming a Token as a cost. This forces the opponent
   to either not delete (and face ongoing token-poison) or delete (triggering
   the On-Deletion security burn), creating a lose-lose pressure loop.

4. **Raid + redirect:** BT24-011 / BT24-017 / EX11-008 install or grant
   `<Raid>` on Dragonkin Digimon, redirecting attacks to the opponent's
   highest-DP unsuspended Digimon. This bridges into BT21-025 Lamiamon's
   `on_attack_target_change` security-trash clause — the attack redirect is
   the trigger source.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT24-001 Gigimon (Lv2 egg) | engine | inherited `[Your Turn][OPT]` on security removal → delete opp ≤3000 DP Digimon |
| BT21-001 Gigimon (Lv2 egg) | engine | inherited `[Your Turn][OPT]` on security removal → digivolve a Dragonkin/Reptile in hand at cost -1 |
| BT5-008 Gaossmon (Lv3) | tech | DP aura to all own Reptile Digimon |
| BT21-007 Agumon (Lv3) | tech | filler Reptile Lv3 body |
| BT21-008 Elizamon (Lv3) | enabler | `[On Play]` reveal 3 → add 1 Reptile/Dragonkin + 1 LIBERATOR to hand; inherited security-remove → +1 memory |
| BT23-005 Elizamon (Lv3) | enabler | `[Your Turn]` digivolve cost -1 into Reptile/Dragonkin |
| BT24-008 Elizamon (Lv3) | enabler | `[On Play]` trash 1 Reptile/Dragonkin/LIBERATOR from hand → Draw 2; inherited security-remove → +1 memory |
| EX11-008 Elizamon (Lv3) | enabler | `[When Moving][On Play]` grant `<Raid>` to 1 Reptile/Dragonkin Digimon; inherited security-remove → +1 memory |
| EX4-006 Guilmon (Lv3) | filler | Reptile Lv3 |
| BT21-015 Cyclonemon (Lv4) | removal | `[On Play][WD]` delete opp ≤4000 DP; `[Security]` re-play self |
| BT21-017 Dimetromon (Lv4) | enabler | `[WD]` free-play 1 Owen Dreadnought if ≤1 Tamers; inherited security-remove → +1 memory |
| BT24-011 Cyclonemon (Lv4) | `<Raid>` engine | `<Rush><Raid>`; inherited `<Raid>` |
| BT24-012 Dimetromon (Lv4) | protection | `<Blocker>`; `[All Turns]` return self to hand → Reptile/Dragonkin doesn't leave; inherited security-remove → +1 memory |
| P-189 Dimetromon (Lv4) | security tech | `[Security]` free-play LIBERATOR ≤4; `<Progress>`; inherited security-remove → +1 memory |
| BT21-024 Cyberdramon (Lv5) | tech | Cyborg filler |
| BT21-025 Lamiamon (Lv5) | punisher | `<Progress>`; `[Your Turn][OPT]` when any Reptile/Dragonkin attack target changes → trash opp top security; inherited on security removal → free-play ≤5000 DP Reptile/Dragonkin from hand |
| BT24-016 Lamiamon (Lv5) | cheat-evolve | `[Hand][Main]` if Owen on field, place Dimetromon from trash under Elizamon → digivolves into Lamiamon for cost 3; inherited on security removal → free-play ≤5000 DP Reptile/Dragonkin |
| BT21-029 Medusamon (Lv6) | punisher | `<Security A.+1><Progress>`; `[WD][EoA][OPT]` delete opp lowest DP; `[All Turns][OPT]` when opp Digimon deleted or security removed → opp plays 1 Petrification Token |
| BT24-017 Medusamon (Lv6) | apex payoff | `<Raid><Progress><Piercing>`; `[WD]` delete opp lowest DP, then (by returning 2 opp trash to deck) opp plays 2 Petrification Tokens + `+2000 DP` per opp Digimon until their turn |
| EX11-012 Medusamon (Lv6) | sticky-body + token | `<Rush><Progress>`; `[WD][EoA]` delete opp ≤ own DP Digimon + (by returning 1 opp trash) opp plays 1 Petrification Token; `[All Turns]` by deleting 1 Token → self doesn't leave |
| BT24-018 Styracomon (Lv7) | apex/control | `<Progress><Piercing><Blocker><Armor Purge>`; `[WD]` trash 1 opp security + may unsuspend; `[All Turns][OPT]` on security removal → delete 1 opp Digimon; `[All Turns][OPT]` when own Reptile/Dragonkin would leave → delete opp lowest DP to prevent |
| BT18-087 Owen Dreadnought (Tamer) | punisher | `[SoT]` memory to 3 if ≤2; `[All Turns]` on opp security removal → suspend → delete opp ≤4000 DP Digimon |
| BT21-081 Owen Dreadnought (Tamer) | enabler | `[SoMP]` +1 memory if opp has Digimon; `[EoT]` suspend → Reptile/Dragonkin gains `<Piercing>` + attacks |
| BT24-082 Owen Dreadnought (Tamer) | enabler | `[SoMP]` bottom self → free-play Owen; `[Your Turn]` when Reptile/Dragonkin digivolves, suspend → +3000 DP + may attack |
| EX11-054 Owen Dreadnought (Tamer) | draw/DP | `[SoT]` memory to 3; `[All Turns]` when Reptile/Dragonkin played/digivolves → suspend → Draw 1 + Progress Digimon +3000 DP |
| BT24-089 Unique Emblem: Blazing Conductor (Option) | ramp | `[Main]` free-play Elizamon or Owen from hand/trash; Delay: when Owen suspends → trash Blazing Conductor → Reptile/Dragonkin digivolves into Reptile/Dragonkin+LIBERATOR at cost -3 |
| EX7-074 Vortex Resonance (Option) | ramp | `[Main]` reveal 3 → add 1 LIBERATOR to hand + Digimon digivolves into hand card at cost -4; color-bypass with LIBERATOR on board |
| P-151 Digimon Liberator (Option) | search | `[Main]` reveal 3 → add 1 LIBERATOR + free-play ≤3-cost LIBERATOR; color-bypass |

(Lower-frequency tech/splash: BT14-017 Dinorexmon, BT20-016 Paildramon,
BT20-102 Omnimon X, BT9-112 DeathXmon, EX10-010 BlackWarGreymon,
BT21-026 WarGreymon, BT21-072 Arresterdramon:SM, LM-021 bond-Agumon,
BT8-094 Digimon Emperor, BT21-093 Raging Serpentine, LM-027/LM-033/LM-045/
LM-051 Memory Boost variants, BT24-001/BT21-001 DigiEgg engines, etc.)

## Digivolution lines

- **BT24-001 / BT21-001 Gigimon (Lv2 egg) → BT21-008 / BT24-008 / EX11-008 /
  BT23-005 Elizamon (Lv3) → BT21-017 / BT24-012 / P-189 Dimetromon (Lv4) →
  BT21-025 / BT24-016 Lamiamon (Lv5) → BT21-029 / BT24-017 / EX11-012
  Medusamon (Lv6) → BT24-018 Styracomon (Lv7)** — the Red Reptile/Dragonkin
  LIBERATOR spine. Each layer adds Petrification Token plays and security-removal
  reaction triggers.
- **BT24-011 Cyclonemon (Lv4)** — a standalone Rush+Raid Dragonkin that acts as
  a `<Raid>` vehicle for Lamiamon's `on_attack_target_change` trigger; digivolves
  from Red Lv3 bodies.
- **Cheat line (BT24-016 Lamiamon):** with Owen on field, place a Dimetromon
  from trash under an Elizamon → digivolves into Lamiamon for cost 3, ignoring
  requirements. Enables fast access to the Lv5 punisher package.
- **Memory floor:** BT18-087 Owen Dreadnought sets memory to 3 at `[SoT]` if
  ≤2, smoothing the curve. Multiple Elizamon prints and Dimetromon inherited
  effects also gain +1 memory on each security removal, creating a snowball
  loop where each punish enables the next.

## Named combos

### C1 — EX11-012 Petrification security loop (sticky body)

- **Cards:** `EX11-012` Medusamon.
- **Expected mechanical outcome:** EX11-012 is on P0's field and has gifted a
  Petrification Token to the opponent (P1 is the token's owner). When an
  opponent effect tries to delete EX11-012, the `[All Turns]` would-leave
  replacement fires: P0 (the controller, per DCGO `selectPlayer: card.Owner`)
  must choose a Token to delete as cost. Paying this cost deletes P1's
  any-owner Petrification Token, which keeps EX11-012 on P0's field (sticky
  body) and simultaneously fires the token's `[On Deletion]` → trash the
  *token's owner's* (P1's) top security card. Net result: EX11-012 remains on
  P0's field, P1's token count drops by 1, and P1 loses 1 security card.
- **Rules/keyword basis:** would-leave replacement timing `general_rule.pdf`
  §16; deletion `general_rule.pdf` §6-2; `[On Deletion]` fires after the
  permanent moves to trash. DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX11/Red/EX11_012.cs`
  (`CanSelectPermanentCondition => p.IsToken`, no owner qualifier;
  `selectPlayer: card.Owner`). The gap G-EX11-012-TOKEN-SHIELD-OWN-ONLY
  (shield used to restrict `select_own_permanent`) was fixed so the shield can
  reach the opponent-owned token.
- **Rank:** highest — the Petrification cross-card loop (gift a token to the
  opponent, then the token burns the opponent's own security) is the archetype's
  defining system-level fact, invisible to per-card tests. Regression coverage
  for the above-named gap fix.

### C2 — BT24-017 Medusamon when-digivolving payoff package

- **Cards:** `BT24-017` Medusamon.
- **Expected mechanical outcome:** BT24-017's `[When Digivolving]` fires a
  3-step chain: (1) delete the opponent's lowest-DP Digimon (opp field −1);
  (2) by returning exactly 2 cards from the opponent's trash to the bottom of
  their deck (cost), the opponent plays 2 Petrification Tokens (opp tokens
  +2); (3) BT24-017 gains +2000 DP × (current opponent Digimon count, counting
  the 2 newly-spawned Tokens) until the opponent's turn ends. With 2 tokens
  spawned this is at minimum +4000 DP. If the opponent has fewer than 2 trash
  cards after the delete, the return-2 cost is unpayable and steps (2) and (3)
  are skipped entirely (DCGO `if (Enemy.TrashCards.Count >= 2)` gate).
- **Rules/keyword basis:** when-digivolving keyword window `general_rule.pdf`
  §16; "by returning ... as cost" timing (cost before reward); DP modifier
  with counted value resolved at modifier-grant time. DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_017.cs`.
- **Rank:** highest — BT24-017 is the apex payoff and this is the full
  3-part chain (delete → token spawn → DP boost) that per-card tests never
  link, including the unpayable-cost gate that collapses the tail.

### C3 — Raid redirect fires Lamiamon attack-target-change trash security (BT21-025)

- **Cards:** `BT21-025` Lamiamon, `BT24-011` Cyclonemon (`<Raid>` attacker).
- **Expected mechanical outcome:** P0 fields Lamiamon and a Dragonkin `<Raid>`
  attacker (BT24-011). When the Raid attacker declares an attack on the player
  and P0 switches the target via `<Raid>` to the opponent's highest-DP
  unsuspended Digimon, the attack-target change fires BT21-025's
  `[Your Turn][OPT] on_attack_target_change` clause (gated on: the attacker is
  P0-owned and Dragonkin) → trash the opponent's top security card (opp
  security −1), once per turn.
- **Rules/keyword basis:** `<Raid>` switch window `general_rule.pdf` §16
  (Raid); attack-target-change observer timing; BT21-025 gates on the
  *attacker's* trait (card text: "any of YOUR Reptile/Dragonkin trait Digimon's
  attack targets change"). DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_025.cs` (Permanent
  condition: own Reptile/Dragonkin attacker; `SetGarbageAnEnemy` security
  trash). DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_011.cs`
  (Raid grant).
- **Confirmed engine gap (G-ATC-EVENT-TARGET-IS-NEW-TARGET):** driven through
  the real `<Raid>` combat path this combo currently FAILS — the
  `event_target_owner` / `event_target_trait_has` predicate family resolves the
  `OnAttackTargetChange` "event target" to the *new attack target* (the
  redirected-to Digimon, opponent-owned and non-Dragonkin) rather than the
  *attacker whose target changed* (P0-owned Dragonkin). Per DCGO, the clause
  must gate on the attacker. Filed to `docs/RUST_ENGINE_GAPS.md`. The test is
  `#[ignore]` with the faithful expected outcome preserved verbatim.
- **Rank:** high — the `<Raid>` keyword on one card is the sole trigger source
  for Lamiamon's security trash on another; this is the cross-card bridge
  per-card tests never exercise end-to-end.

### C4 — Styracomon self-feeding security chain (BT24-018)

- **Cards:** `BT24-018` Styracomon.
- **Expected mechanical outcome:** BT24-018 digivolves onto P0's field. Its
  `[When Digivolving]` may trash 1 chosen opponent security card (opp security
  −1). That security removal immediately arms and fires BT24-018's own
  `[All Turns][OPT] on_opponent_security_removed` clause (same Digimon, same
  turn), which may delete 1 of the opponent's Digimon (opp battle area −1).
  Result: a single digivolve can both trash a security card *and* delete a
  Digimon, using the card's own digivolve effect as the trigger source for its
  own standing reaction clause.
- **Rules/keyword basis:** `[When Digivolving]` and `[All Turns]` are
  evaluated in the same turn; the security removal from the WD clause satisfies
  the "when security stack is removed from" condition for the [All Turns]
  observer. `general_rule.pdf` §16 (when-digivolving window + observer timing).
  DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_018.cs`.
- **Rank:** high — Styracomon's own digivolve security-trash is the enabler for
  its own `on_opponent_security_removed` deletion. The self-feeding chain cannot
  be seen by two separate per-clause tests.

### C5 — Owen punishes opponent security removed by token deletion (BT18-087)

- **Cards:** `BT18-087` Owen Dreadnought (Tamer), `EX11-012` Medusamon
  (Petrification Token feed).
- **Expected mechanical outcome:** BT18-087 (unsuspended) is on P0's field
  alongside a Petrification Token gifted to the opponent. When the opponent-
  owned Petrification Token is deleted, its `[On Deletion]` trashes the
  opponent's top security (P1 security −1). That security removal fires
  BT18-087's `[All Turns]` clause: by suspending Owen (cost), delete 1 of the
  opponent's Digimon with ≤4000 DP (opp battle area −1; Owen becomes
  suspended). Unhappy path: if Owen is already suspended when the security is
  removed, the suspend cost is unpayable and the punisher does not fire — but
  the token's `[On Deletion]` security burn is independent and still happens.
- **Rules/keyword basis:** security-removed observer timing; "by suspending this
  Tamer" = suspend-as-cost (cost paid before reward). `general_rule.pdf` §16.
  DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/BT18/Red/BT18_087.cs`;
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX11/Red/EX11_012.cs`; token On
  Deletion in `code/digimon-engine/src/cards/tokens/petrification.rs`.
- **Rank:** high — the Petrification engine (gift token → token deletion burns
  opp security → Owen's punisher fires) is the cross-card combination that
  makes Owen relevant; per-card tests for Owen and EX11-012 are independent and
  never link this chain.

## Playstyle

- **Class:** midrange control with a security-erosion combo core. Not raw
  aggro; instead the deck builds a Petrification pressure board and forces the
  opponent into a "can't win" loop: letting tokens live means they arm
  Medusamon's sticky body while eventually blocking attacks (tokens are White,
  can't suspend); deleting them burns the opponent's own security.
- **Memory curve:** BT18-087 Owen floors memory to 3 each turn. Elizamon prints
  and Dimetromon inherited effects each provide +1 memory per security removal,
  so the combo snowballs: every security burn funds the next digivolve step.
- **Resilience:** EX11-012 and BT24-018 both carry `[All Turns]`
  would-leave replacements, making the archetype's key bodies sticky without
  requiring hand resources beyond spare Tokens. BT24-012 Dimetromon adds a
  second protection layer (return self → Reptile/Dragonkin doesn't leave).
- **Security pressure axis:** the deck has no single attack-and-check win
  condition. Instead it depletes the opponent's security *from the field*:
  Medusamon bodies deposit tokens that trash security on deletion; Styracomon
  trashes security on digivolve; Lamiamon trashes security when a Raid redirect
  fires; Owen deletes blockers after each security removal. The opponent runs
  out of security before a clean attack lands.

## Win conditions

1. **Petrification + security removal chain:** populate the opponent's field
   with Petrification Tokens → force deletions (via BT24-017 WD, BT24-018 WD,
   EX11-012 EoA, BT21-029 WD) → each deletion burns the opponent's security →
   Owen and Styracomon fan out further board removal → push the final security
   check(s) with `<Piercing>` or `<Security A.+1>`.
2. **Raid pressure:** BT24-017 + BT24-011 / EX11-008 Elizamon grant `<Raid>` to
   Dragonkin attackers. Raid switches to the highest-DP unsuspended Digimon
   (removing blockers), fires Lamiamon's security-trash, and gives BT24-017 a
   DP bonus scaled to the tokens the same WD just spawned.
3. **Sticky-body attrition:** EX11-012 and BT24-018 don't die easily. The
   opponent must either: (a) not delete them (letting the Petrification loop
   accumulate), or (b) delete them (paying the token On-Deletion security cost)
   while P0 chooses any-owner tokens as the sticky-body cost, including tokens
   already on the opponent's field. Either path erodes the opponent's resources.

## Ranked interactions to test

1. **C1 EX11-012 sticky body deletes opponent token keeps self and burns
   security** — regression for G-EX11-012-TOKEN-SHIELD-OWN-ONLY; tests that
   the `select_any_permanent { kind: token }` (no owner restriction) reaches
   the opponent-owned token AND that the token's On-Deletion burns the token's
   *owner's* security. Highest value: invisible to per-card tests.
2. **C2 BT24-017 when-digivolving payoff package (happy + unpayable-cost paths)**
   — the 3-part chain and the cost-gate collapse; the DP boost counts the
   newly-spawned tokens.
3. **C3 Raid redirect fires Lamiamon security trash** — `<Raid>` as the
   trigger source for BT21-025; currently `#[ignore]` pending
   G-ATC-EVENT-TARGET-IS-NEW-TARGET engine fix.
4. **C4 Styracomon self-feeding security chain** — own WD security-trash
   immediately arms own `on_opponent_security_removed` delete; self-feeding
   loop invisible to two separate per-clause tests.
5. **C5 Owen punishes opponent security removed by token deletion (happy +
   already-suspended path)** — Petrification token chain → Owen punisher,
   with the already-suspended gate proving the cost-dependency.
