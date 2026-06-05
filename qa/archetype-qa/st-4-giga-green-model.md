# ST-4 Giga Green — Model

Durable archetype model for the `/archetype-interaction-test-author` capstone run.
Interaction tests: `code/digimon-engine/tests/archetypes/st4.rs`.
Per-card behavioral coverage already green in
`code/digimon-engine/tests/cards_behavioral/st4/mod.rs`.

**System summary.** ST-4 is a Green Insectoid tempo deck whose signature engine is
*suspension as a resource*: suspend an opponent's Digimon and that single state
change simultaneously (a) feeds **Izzy Izumi**'s [Your Turn] memory engine and
(b) unlocks **Electro Shocker**'s bounce, which is *gated* on the suspended
state. Suspension comes from three sources — **Needle Spray** (option),
**HerculesKabuterimon** (Digi-Burst 2), and Needle Spray's Security effect. The
deck also runs a deck-dig sub-theme (Tentomon / Lillymon) and a separate lockdown
tool (**Rosemon**, which does *not* suspend — a deliberate contrast with the
suspend engine).

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| ST4-01 Motimon (DigiEgg) | enabler | INH [Your Turn] +1000 DP while carrier is Lv.6+. |
| ST4-02 Floramon (Rookie) | body | Vanilla Lv.3. |
| ST4-03 Tentomon (Rookie) | engine (dig) | [On Play] reveal top; add if green Digimon else bottom. |
| ST4-04 Palmon (Rookie) | enabler | INH [When Attacking] +2000 DP vs an opp Digimon. |
| ST4-05 Kunemon (Rookie) | body | Vanilla Lv.3. |
| ST4-06 Togemon (Champion) | enabler | INH [When Attacking] +2000 DP vs an opp Digimon. |
| ST4-07 Kuwagamon (Champion) | body | Vanilla Lv.4. |
| ST4-08 Kabuterimon (Champion) | tech | Blocker; [When Attacking] −2 memory. |
| ST4-09 Okuwamon (Ultimate) | body | Vanilla Lv.5. |
| ST4-10 Lillymon (Ultimate) | engine (dig) | [When Digivolving] reveal 5, add 1 Lv.6+ Digimon, rest to bottom. |
| ST4-11 MegaKabuterimon (Ultimate) | payoff | INH [Your Turn][OPT] on winning a battle + surviving, trash top of opp security. |
| ST4-12 Rosemon (Mega) | tech (lockdown) | [When Digivolving] 1 opp Digimon CannotAttack+CannotBlock until end of their next turn. |
| ST4-13 HerculesKabuterimon (Mega) | payoff / enabler | Piercing; [Main] <Digi-Burst 2> suspend 1 opp Digimon. |
| ST4-14 Izzy Izumi (Tamer) | engine | [Your Turn][OPT] when an opp Digimon becomes suspended, suspend Izzy → gain 1 memory. [Security] play free. |
| ST4-15 Needle Spray (Option, c2) | enabler | [Main] suspend 1 opp Digimon. [Security] suspend + add to hand. |
| ST4-16 Electro Shocker (Option, c5) | payoff | [Main] return 1 *suspended* opp Digimon to hand. [Security] same. |

## Digivolution lines

- **Insectoid (kuwagamon line):** Motimon → Tentomon/Kunemon → Kuwagamon →
  Kabuterimon → Okuwamon / MegaKabuterimon → HerculesKabuterimon.
- **Plant (palmon line):** Motimon → Floramon → Togemon → Lillymon / (Ult) → Rosemon.
- Both top out at the two Megas (HerculesKabuterimon, Rosemon). HerculesKabuterimon
  feeds the suspend engine via Digi-Burst; Rosemon is a separate lockdown line.

## Named combos

### 1. Suspend → Izzy memory → Electro Shocker bounce (signature 3-card engine)
- Cards: ST4-15 Needle Spray + ST4-14 Izzy Izumi + ST4-16 Electro Shocker.
- Expected mechanical outcome: Needle Spray suspends an opp Digimon (on your turn);
  the OnSuspend event fires Izzy's [Your Turn] optional response → Izzy suspends
  herself and you gain exactly +1 memory; then Electro Shocker, seeing the same
  Digimon is suspended, returns it to the owner's hand (opp field −1, opp hand +1).
- Rules basis: `general_rule.pdf` §16 (suspend/unsuspend timing) + return-to-hand
  (§ "return to hand" routes sources through trash). Izzy's response keys on the
  opponent's Digimon *becoming* suspended (OnSuspend observer). DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_15.cs`, `ST4_14.cs`, `ST4_16.cs`.
- Rank: A (the deck's defining loop).

### 2. Electro Shocker is gated on suspended state (unhappy)
- Cards: ST4-16 Electro Shocker vs an UNSUSPENDED opp Digimon, then after a suspend.
- Expected mechanical outcome: against an unsuspended Digimon, Electro Shocker has
  no legal target — the opp board is untouched (no bounce). After the Digimon is
  suspended (Needle Spray or direct), it becomes a legal target and is bounced.
  The prompt's valid-target set flips: the target's field action is absent in the
  unsuspended case and present once suspended.
- Rules basis: Electro Shocker's printed text restricts to "suspended"; the DSL
  filter is `is_suspended: true`. DCGO C#: `ST4_16.cs`.
- Rank: A (the gating fact is the system point).

### 3. HerculesKabuterimon Digi-Burst suspend → Izzy memory
- Cards: ST4-13 HerculesKabuterimon (≥2 sources) + ST4-14 Izzy Izumi.
- Expected mechanical outcome: HerculesKabuterimon's [Main] <Digi-Burst 2> trashes
  2 of its own digivolution cards and suspends 1 opp Digimon; that suspend feeds
  the same Izzy engine (Izzy suspends herself, +1 memory). Asserts: opp target
  suspended, 2 sources trashed, Izzy suspended, memory +1.
- Rules basis: `general_rule.pdf` §16 Digi-Burst (trash N sources as a cost) +
  suspend timing. DCGO C#: `ST4_13.cs`, `ST4_14.cs`.
- Rank: B (second suspend source into the same engine).

### 4. Rosemon lockdown (and its contrast with the suspend engine)
- Cards: ST4-12 Rosemon (+ ST4-16 Electro Shocker for the contrast).
- Expected mechanical outcome: Rosemon's [When Digivolving] applies CannotAttack +
  CannotBlock to 1 opp Digimon until end of their next turn. Crucially Rosemon does
  NOT suspend — so Electro Shocker, which needs the suspended state, still cannot
  bounce a Rosemon-locked-but-unsuspended Digimon. The two lockdown tools are
  mechanically distinct.
- Rules basis: CannotAttack/CannotBlock modifiers with `end_of_opponents_turn`
  expiry. DCGO C#: `ST4_12.cs`, `ST4_16.cs`.
- Rank: B (tech line; contrast clarifies the engine).

### 5. Izzy is optional / costs a suspend (unhappy)
- Cards: ST4-14 Izzy Izumi (already suspended) + a suspend event.
- Expected mechanical outcome: Izzy's response is "you MAY suspend this Tamer." If
  Izzy is already suspended she cannot pay the cost, so the opp-suspend trigger
  yields no memory (memory unchanged, Izzy stays suspended).
- Rules basis: activation cost `suspend_self`; an already-suspended permanent
  cannot pay it (mirror of ST5-14 Tai's "cannot pay suspend cost"). DCGO C#: `ST4_14.cs`.
- Rank: C (boundary of the engine).

## Playstyle / Win conditions

Tempo: dig with Tentomon/Lillymon to assemble the suspend engine, suspend the
opponent's threats to deny attacks/blocks while Izzy refuels memory, then bounce
the suspended threat with Electro Shocker for hard tempo. HerculesKabuterimon
(Piercing) + MegaKabuterimon (security trash on a winning battle) close out
through security. Rosemon offers an alternate lock when bounce is unavailable.

## Ranked interactions to test (status)

| # | Interaction | Rank | Status |
|---|-------------|------|--------|
| 1 | Suspend → Izzy memory → Electro Shocker bounce | A | `#[test]` — `needle_spray_suspend_feeds_izzy_then_electro_shocker_bounces` (+ Izzy-isolation control) |
| 2 | Electro Shocker gated on suspended state | A | `#[test]` — `electro_shocker_target_set_flips_on_suspended_state` |
| 3 | Hercules Digi-Burst suspend → Izzy memory | B | `#[test]` — `hercules_digi_burst_suspend_feeds_izzy_memory` |
| 4 | Rosemon lockdown + Electro Shocker contrast | B | `#[test]` — `rosemon_lockdown_modifiers` + `electro_shocker_cannot_bounce_rosemon_locked_unsuspended_target` |
| 5 | Izzy optional / costs a suspend | C | `#[test]` — `izzy_already_suspended_gains_no_memory_on_opponent_suspend` |

### Blocked / dropped
- None. All 16 cards are implemented in the DSL with green per-card tests; every
  ranked interaction maps to a `#[test]`.
