# ST-4 Giga Green — Model

Insectoid/Plant (Vegetation/Fairy) Green starter deck. Core identity: **suspend the opponent's board as tempo**, convert those suspensions into memory and removal, and apply **security pressure on battle wins**. Suspend is both an offensive lock (suspended Digimon can be attacked directly, 16-6/11-1-3) and the trigger fuel for the deck's value engine.

Sources consulted (priority order): printed text `code/digimon-engine/cards/st4/<ID>.json`; `Digimon TCG resources/general_rule.pdf` §16 + `glossary.pdf` (Digi-Burst 16-13, Piercing 16-6, Suspend / attack flow 11-1-3, 11-2); DCGO C# `DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_<NN>.cs`. YAML specs `code/digimon-engine/cards/st4/<ID>.yaml`. Engine suspend dispatch verified at `code/digimon-engine/src/game.rs` `Game::suspend` (lines 2894–2948).

## Card pool & roles

| Card | ID | Lvl | Role | Effect (printed) |
|------|----|----|------|------|
| Motimon | ST4-01 | 2 | In-Training | Inh [Your Turn] while lvl 6+, +1000 DP |
| Floramon | ST4-02 | 3 | Rookie (vanilla) | — (digivolution body) |
| Tentomon | ST4-03 | 3 | Rookie / dig | [On Play] reveal top; add if green Digimon, else bottom |
| Palmon | ST4-04 | 3 | Rookie | Inh [When Attacking] vs opp Digimon, +2000 DP this turn |
| Kunemon | ST4-05 | 3 | Rookie (5000 body) | — |
| Togemon | ST4-06 | 4 | Champion | Inh [When Attacking] vs opp Digimon, +2000 DP this turn |
| Kuwagamon | ST4-07 | 4 | Champion (6000 body) | — |
| Kabuterimon | ST4-08 | 4 | Champion | \<Blocker\>; [When Attacking] lose 2 memory |
| Okuwamon | ST4-09 | 5 | Ultimate (7000 body) | — |
| Lillymon | ST4-10 | 5 | Ultimate / dig | [When Digivolving] reveal top 5; add 1 lvl6+ Digimon; bottom rest |
| MegaKabuterimon | ST4-11 | 5 | Ultimate | Inh [Your Turn][OPT] on deleting opp Digimon in battle & surviving, trash top of opp security |
| Rosemon | ST4-12 | 6 | Mega / lock | [When Digivolving] 1 opp Digimon can't attack or block until end of their next turn |
| HerculesKabuterimon | ST4-13 | 6 | Mega / finisher | \<Piercing\>; [Main] \<Digi-Burst 2\> suspend 1 opp Digimon |
| Izzy Izumi | ST4-14 | Tamer | Memory engine | [Your Turn] when an opp Digimon becomes suspended, may suspend this Tamer to gain 1 memory; [Security] play free |
| Needle Spray | ST4-15 | Option | Suspend enabler | [Main] suspend 1 opp Digimon; [Security] do Main, then add to hand |
| Electro Shocker | ST4-16 | Option | Suspend payoff (bounce) | [Main] return 1 opp **suspended** Digimon to hand; [Security] do Main |

Notes:
- **ST4-16 text discrepancy (flagged, not blocking):** DCGO `ST4_16.cs` describes "Trash all of the digivolution cards of that Digimon" in addition to the bounce, but the printed/JSON text and YAML are bounce-only ("Return 1… to its owner's hand"). Per source priority printed text governs, and returning a permanent to hand already clears its whole stack, so the YAML (`return_to_hand`) is faithful; DCGO's extra clause is a stale/over-specified description, not a behavioral target.
- **Mandatory-targeting drift (prior audit, minor):** ST4-13/15/16 suspend/return steps carry `optional: true` in YAML; printed text is "Suspend 1" / "Return 1" (mandatory when a legal target exists). Tracked in `qa/qa-reports/2026-05-29-starter-decks-st1-6-faithfulness-audit.md`. Interaction tests must **select** the target (not decline) to exercise the real combo path.

## Digivolution lines

- **Insectoid (security-pressure / finisher line):** Motimon (2) → Tentomon (3) → Kabuterimon (8) / Kuwagamon (7) → MegaKabuterimon (11) / Okuwamon (9) → HerculesKabuterimon (13). MegaKabuterimon's inherited security-trash rides under HerculesKabuterimon; HercKabu provides Piercing + the Digi-Burst suspend.
- **Vegetation/Fairy (lock / dig line):** Floramon (2)/Palmon (4) → Togemon (6) → Lillymon (10) → Rosemon (12). Lillymon digs for the lvl6+ (HercKabu/Rosemon); Rosemon installs the attack/block lock.
- Both lines feed the same memory engine (Izzy) and the same suspend payoffs (Electro Shocker, direct attacks into suspended Digimon).

## Named combos

### Combo A — Needle Spray → Izzy memory ramp
- **Cards:** Needle Spray (ST4-15) + Izzy Izumi (ST4-14).
- **Expected mechanical outcome:** Needle Spray's [Main] suspends 1 opponent Digimon → the suspend transition fires `OnSuspend` → Izzy's [Your Turn] trigger is offered → player elects to suspend Izzy → **+1 memory**. Net: a 2-cost option that suspends *and* refunds 1 memory, so the suspend is nearly free.
- **Rules / keyword basis:** `Game::suspend` emits `OnSuspend` only on the unsuspended→suspended transition (`game.rs:2920` `if already { return; }`), so it fires exactly once. Izzy condition lowers to `event_target_owner: opponent` + `event_target_kind: digimon` + `source_is_unsuspended: true` (`ST4-14.yaml`; predicates in `code/digimon-dsl/src/predicate.rs`). Activation cost `suspend_self` taps Izzy; because Izzy is a Tamer (not opponent's Digimon) the self-suspend does **not** re-satisfy the condition — no loop. DCGO `ST4_14.cs` (`OnTappedAnyone` → `CanTriggerWhenPermanentSuspends` on opponent battle-area Digimon → `SuspendPermanentsClass(Izzy).Tap()` → `AddMemory(1)`) and `ST4_15.cs` (`SelectPermanentEffect … Mode.Tap`).
- **Rank:** 1 (signature engine; cleanest, fully cross-card, deterministic).

### Combo B — Rosemon lock also feeds Izzy; and HercKabu Digi-Burst suspend feeds Izzy
- **Cards:** Izzy Izumi (ST4-14) + a second suspend source — primarily HerculesKabuterimon (ST4-13) `[Main] <Digi-Burst 2>` suspend; secondarily any other enabler.
- **Expected mechanical outcome:** HercKabuterimon pays Digi-Burst 2 (trash 2 of its digivolution cards) and suspends 1 opponent Digimon → `OnSuspend` fires → Izzy offers suspend-self → +1 memory. Confirms the memory engine is **source-agnostic**: any opponent-Digimon suspension (option, Digi-Burst, or otherwise) triggers Izzy, not just Needle Spray. (Note: Rosemon's lock is "can't attack or block," *not* a suspend, so Rosemon does **not** feed Izzy — see Dropped.)
- **Rules / keyword basis:** Digi-Burst 16-13 (trash X digi cards as a cost). `Game::suspend` is called by HercKabu's `suspend` step regardless of effect source, so the same `OnSuspend` path fires. DCGO `ST4_13.cs` (`IDigiBurst(card,2).DigiBurst()` then `SelectPermanentEffect … Mode.Tap`). YAML `ST4-13.yaml` (`select_own_sources min/max 2` → `trash_selected_sources` → `select_opponent_permanent` → `suspend`).
- **Rank:** 2 (validates the engine generalizes across suspend sources; also exercises the Digi-Burst cost path).

### Combo C — Suspend → Electro Shocker bounce (suspend payoff)
- **Cards:** Needle Spray (ST4-15) **or** HercKabu Digi-Burst (ST4-13) as the enabler + Electro Shocker (ST4-16).
- **Expected mechanical outcome:** Enabler suspends an opponent Digimon; Electro Shocker can then legally target it ("return 1 opponent **suspended** Digimon to hand") and bounce it (whole stack to hand). Without a prior suspension Electro Shocker has no legal target. Demonstrates the suspend→removal payoff chain and Electro Shocker's targeting gate (`is_suspended: true` filter).
- **Rules / keyword basis:** `ST4-16.yaml` filter `is_suspended: true`; DCGO `ST4_16.cs` `CanSelectPermanentCondition` requires `permanent.IsSuspended`. Bounce = `return_to_hand`.
- **Rank:** 3 (real and asymmetric — the suspend gates a removal that's otherwise dead — but a 2-card chain with a hard target gate rather than a value loop).

### Combo D (secondary) — MegaKabuterimon inherited security trash on a battle win
- **Cards:** HerculesKabuterimon (ST4-13) carrying MegaKabuterimon (ST4-11) as a digivolution source.
- **Expected mechanical outcome:** When the stacked Digimon attacks, deletes an opponent's Digimon in battle, and survives, MegaKabuterimon's inherited [Your Turn][OPT] trashes the top of the opponent's security. Combined with HercKabu's \<Piercing\> (which performs the normal security check when it deletes a blocker/Digimon and survives), a single attack into a suspended blocker can strip **two** security (Piercing check + MegaKabu trash). This is the security-pressure win-rate driver.
- **Rules / keyword basis:** Piercing 16-6 (security check at end of attack when it deletes and survives). MegaKabu once-per-turn, your-turn, "deletes in battle and survives." `ST4-11.yaml` `on_any_deletion` + `source_deleted_battle_opponent: true` + `trash_top_security`; DCGO `ST4_11.cs` (`OnEndBattle` → `CanTriggerWhenDeleteOpponentDigimonByBattle(isOnlyWinnerSurvive:true)` → `IDestroySecurity(enemy,1,fromTop)`). HercKabu Piercing via `ST4_13.cs` `OnDetermineDoSecurityCheck` / `PierceSelfEffect`.
- **Rank:** 4 (genuine multi-card stacking interaction, but depends on a combat-resolution setup and the inherited-source attachment; more fragile to author than A–C).

## Playstyle

Tempo-control. Early: dig with Tentomon/Lillymon for the lvl6+ payoffs; develop the Insectoid or Vegetation line. Mid: drop **Izzy** as a standing engine, then chain suspend effects (Needle Spray, later HercKabu Digi-Burst) each turn — every opponent-Digimon suspension is a +1 memory window, so the deck out-tempos by suspending blockers and *also* ramping. Suspend doubles as offense: a suspended Digimon can be attacked directly (11-2), and HercKabu's Piercing + MegaKabu's inherited trash convert those attacks into security loss. Electro Shocker and Rosemon are the hard-control tools: Electro Shocker bounces a suspended threat (best aimed at a digivolved stack), Rosemon shuts off the opponent's best attacker/blocker for a full turn cycle.

## Win conditions

1. **Security attrition** — HercKabuterimon (Piercing) under a MegaKabuterimon source trashing/Piercing through security on battle wins; backed by suspend-enabled direct attacks into a locked-down board.
2. **Tempo + memory snowball** — Izzy converts the deck's frequent suspensions into a memory lead, deploying threats ahead of curve while the opponent's board is suspended/locked and can't profitably block.
3. **Lethal swing** — once security is depleted, a suspended/locked opponent board (Needle Spray + Rosemon/Electro Shocker) can't block the finishing attack.

## Ranked interactions to test

1. **A — Needle Spray → Izzy +1 memory** (ST4-15 + ST4-14). Assert: after Needle Spray suspends an opponent Digimon, Izzy's `OnSuspend` trigger surfaces; accepting it suspends Izzy and adds exactly 1 memory. Negative check: Izzy does **not** re-trigger off its own self-suspend; a second already-suspended target yields no new trigger.
2. **B — HercKabu Digi-Burst suspend → Izzy +1 memory** (ST4-13 + ST4-14). Assert: paying Digi-Burst 2 and suspending an opponent Digimon fires the same Izzy trigger and grants +1 memory — engine is source-agnostic across suspend producers, and the Digi-Burst cost (2 sources trashed) resolves first.
3. **C — Suspend → Electro Shocker bounce** (ST4-15 or ST4-13 + ST4-16). Assert: Electro Shocker has **no** legal target with no suspended opponent Digimon; after an enabler suspends one, Electro Shocker can target only the suspended Digimon and bounces its whole stack to hand.
4. **(secondary) D — Piercing + MegaKabu inherited double security trash** (ST4-13 carrying ST4-11). Assert: a battle win where HercKabu deletes a Digimon and survives triggers both the Piercing security check and MegaKabu's inherited top-security trash (once per turn), removing two security from one attack.

### Dropped candidates
- **Rosemon → Izzy:** Rosemon's lock is "can't attack or block," not a suspend — it does **not** emit `OnSuspend`, so it does not feed Izzy. (Kept only as a separate control line, not an Izzy combo.)
- **Tentomon/Lillymon reveal-search as an "interaction":** single-card deck-dig value effects with no cross-card dependency; per-card coverage already exists. Not an interaction test.
- **Palmon/Togemon inherited +2000 attacking buff:** generic combat math, no archetype-specific cross-card synergy beyond enabling a battle win that any attacker enables; folded into Combo D's setup rather than its own test.
- **Needle Spray / Izzy / Electro Shocker [Security] effects:** security-source resolution is single-card behavior covered per-card; the suspend-from-security → Izzy chain is a variant of Combo A and not worth a separate authored test (timing differs but the engine path is identical).
- **Izzy self-suspend loop:** confirmed impossible (self-suspend target is a Tamer / own card, fails `event_target_kind: digimon` + `event_target_owner: opponent`); asserted as a negative within Combo A rather than its own combo.
