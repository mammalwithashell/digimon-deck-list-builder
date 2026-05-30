# ST-3 Heaven's Yellow — Model

> Slug `st3-heavens-yellow` · set `st3` · color Yellow · Angemon→Seraphimon control deck.
> System thesis: **stack -DP reduction effects to drop an opponent Digimon to 0 DP → the 0-DP rule check deletes it → inherited "0-DP deletion" observers (Tokomon/Patamon) reward the carrier.** Removal + security recovery + DP-zero payoffs form one engine.
> All 16 cards are DSL YAML, audited faithful (AUDITED-OK, 16/16, no drift) per `qa/qa-reports/2026-05-29-starter-decks-st1-6-faithfulness-audit.md`.
> Sources: printed text `code/digimon-engine/cards/st3/<ID>.json` + `.yaml`; DCGO C# `DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_<NN>.cs`; rules `Digimon TCG resources/general_rule.pdf` (§16 keywords, §17 rule checks); `glossary.pdf`.

## Card pool & roles

| Card | Name | Lv/DP | Kind | Role | Key text (printed) |
|------|------|-------|------|------|--------------------|
| ST3-01 | Tokomon | 2 / — | Digi-Egg (In-Training) | **DP-zero PAYOFF (observer)** | Inh `[Your Turn][Once Per Turn]` when an opp Digimon is deleted by dropping to 0 DP, **this Digimon (carrier) +1000 DP** for the turn |
| ST3-02 | Salamon | 3 / 3000 | Digimon (Rookie) | Vanilla digivolve base | — |
| ST3-03 | Tapirmon | 3 / 4000 | Digimon (Rookie) | Vanilla digivolve base | — |
| ST3-04 | Patamon | 3 / 1000 | Digimon (Rookie) | **DP-zero PAYOFF (observer)** | Inh `[Your Turn][Once Per Turn]` when an opp Digimon is deleted by dropping to 0 DP, **gain 1 memory** |
| ST3-05 | Angemon | 4 / 4000 | Digimon (Champion) | Tempo / memory | Inh `[When Attacking]` if ≥4 security, gain 1 memory |
| ST3-06 | Gatomon | 4 / 5000 | Digimon (Champion) | Vanilla Lv4 (Angewomon base) | — |
| ST3-07 | Unimon | 4 / 6000 | Digimon (Champion) | Wall / Blocker | `<Blocker>`; `[When Attacking]` lose 2 memory |
| ST3-08 | MagnaAngemon | 5 / 7000 | Digimon (Ultimate) | **-DP REDUCER (inherited)** | Inh `[When Attacking]` 1 opp Digimon **-1000 DP** for the turn |
| ST3-09 | Angewomon | 5 / 7000 | Digimon (Ultimate) | **Security recovery** | `[When Digivolving]` if ≤3 security, `<Recovery +1 (Deck)>` |
| ST3-10 | Magnadramon | 6 / 12000 | Digimon (Mega) | Vanilla finisher | — |
| ST3-11 | Seraphimon | 6 / 10000 | Digimon (Mega) | **-DP REDUCER (main)** | `[When Attacking]` 1 opp Digimon **-4000 DP** for the turn |
| ST3-12 | T.K. Takaishi | — | Tamer | **Security-Digimon defense** | `[Opponent's Turn]` your **Security Digimon +2000 DP**; `[Security]` play free |
| ST3-13 | Heaven's Gate | — | Option | Combat trick / pump | `[Main]` 1 of your Digimon +3000 DP; `[Security]` all your Digimon+Security Digimon +5000 DP, return to hand |
| ST3-14 | Heaven's Charm | — | Option | **-DP REDUCER (option)** | `[Main]` 1 opp Digimon **-2000 DP** for the turn; `[Security]` add to hand |
| ST3-15 | Holy Flame | — | Option | **Security-attack debuff** | `[Main]` 1 opp Digimon gains `<Security Atk -3>` (checks 3 FEWER security) until end of their turn; `[Security]` all opp Digimon `<Security Atk -1>` |
| ST3-16 | Seven Heavens | — | Option | **-DP REDUCER (big, option)** | `[Main]` 1 opp Digimon **-10000 DP** for the turn; `[Security]` activate its `[Main]` |

Note: ST3-01/04/05/08 carry inherited text in the JSON `effect_description_eng` field with an empty `inherited_effect_description_eng` — a `data/cards.json` ingest artifact; the YAML correctly scopes them `scope: inherited`. ST3-15 JSON says `<Security A. -3>` "checks 3 additional" but glossary + DCGO (`ST3_15.cs` description) define it as checks 3 **fewer**; the YAML correctly encodes the debuff (`SecurityAttackChange: -3`).

## Digivolution lines

Two parallel Angel lines plus a Holy-Beast splash, all mono-Yellow, digivolve cost via In-Training/level chains:

- **Angemon line (the main control engine):**
  `(Tokomon ST3-01 egg)` → Salamon/Tapirmon (Lv3) → **Angemon ST3-05** (Lv4) → **MagnaAngemon ST3-08** (Lv5, inherited -1000) → **Seraphimon ST3-11** (Lv6, -4000).
- **Angewomon recovery line:**
  Lv3 → **Gatomon ST3-06** (Lv4) → **Angewomon ST3-09** (Lv5, recovery) → Magnadramon ST3-10 (Lv6) [or → Seraphimon].
- **Inherited-source plumbing:** because ST3-08 (-1000) and ST3-01/04 (observers) are *inherited* effects, evolving THROUGH them stacks their inherited effects under the top Digimon. A Seraphimon stacked over MagnaAngemon-over-Tokomon attacks with both -4000 (own) AND inherited -1000, with a Tokomon source underneath ready to read the 0-DP deletion. This is the heart of the combo.

## Named combos

### Combo A — Seven Heavens overkill → Tokomon carrier payoff
- **Cards:** ST3-16 Seven Heavens + ST3-01 Tokomon (as an inherited source under any attacker).
- **Expected mechanical outcome:** Seven Heavens `[Main]` applies -10000 DP to a target opp Digimon of ≤10000 printed DP, dropping it to ≤0. After effect processing, the 0-DP rule check (17-1-3-1) **deletes** it. The deletion fires `on_any_deletion`; the Tokomon-source observer (ST3-01) sees `event_target_owner: opponent`, `event_target_dp_lte: 0`, `your_turn`, once-per-turn → the **carrier Digimon gains +1000 DP** for the turn. A test asserts: a 7000-DP opp Digimon is gone from battle_area, and the Tokomon-source carrier's effective DP is base+1000.
- **Rules/keyword basis:** §17 rule checks — **17-1-3-1** "0 DP Digimon in battle area → deleted." Crucially, **17-1-2-2** ("rule checks are NOT performed during effect processing") means the -10000 lands during processing and the deletion + observer fire at the *following* rule check, not mid-effect. DCGO: `ST3_16.cs` (`ChangeDigimonDP(-10000)`); `ST3_01.cs` `CanUseCondition` gates on `IsExistOnBattleArea` + `IsOwnerTurn` + `CanTriggerOnPermanentDeleted(opp digimon)` + `IsDPZeroDelete(hashtable)`, then `ChangeDigimonDP(card.PermanentOfThisCard(), +1000, UntilEachTurnEnd)`.
- **Rank:** 1 (the canonical archetype loop; single-card-trigger, fully deterministic, exercises 17-1-3-1 + the inherited observer + the "deleted-by-0-DP-specifically" gate).

### Combo B — Stacked -DP attack (Seraphimon + MagnaAngemon inherited) → 0-DP deletion → Patamon memory
- **Cards:** ST3-11 Seraphimon (top, -4000 when attacking) over an inherited **ST3-08 MagnaAngemon** (-1000 when attacking) and/or **ST3-14 Heaven's Charm** (-2000 from hand), plus an inherited **ST3-04 Patamon** source.
- **Expected mechanical outcome:** On Seraphimon's attack, the `[When Attacking]` triggers stack: -4000 (Seraphimon main) + -1000 (MagnaAngemon inherited) for -5000, optionally + -2000 (Heaven's Charm `[Main]` pre-attack) = up to -7000 onto one target, dropping a ~5000–7000-DP blocker/attacker to ≤0 → deleted at the rule check (17-1-3-1). The deletion fires the Patamon-source observer (ST3-04) → **gain 1 memory** (once per turn). A test asserts: target deleted AND active player memory increased by exactly 1 (and only once even if multiple opp Digimon die). Optionally co-assert a Tokomon source also reading +1000 on the same deletion.
- **Rules/keyword basis:** §16 `[When Attacking]` keyword timing (multiple when-attacking effects on the attacking Digimon + its inherited stack all trigger on the same attack); §17-1-3-1 deletion; once-per-turn gating on the observer. DCGO: `ST3_11.cs` (-4000), `ST3_08.cs` (-1000 inherited), `ST3_14.cs` (-2000 `[Main]`), `ST3_04.cs` (`AddMemory(1)` gated by `IsDPZeroDelete` + `IsOwnerTurn` + once-per-turn).
- **Rank:** 2 (multi-card *and* multi-source interaction; verifies inherited -DP stacking + the second observer payoff + once-per-turn semantics — the parts Combo A doesn't touch).

### Combo C — Angewomon recovery wall + T.K. security defense (the control/clock half)
- **Cards:** ST3-09 Angewomon (`[When Digivolving]` recover if ≤3 security) + ST3-12 T.K. Takaishi (`[Opponent's Turn]` Security Digimon +2000) + (optional) ST3-13 Heaven's Gate `[Security]`.
- **Expected mechanical outcome:** Digivolving into Angewomon while at ≤3 security places the top deck card on security (security count +1); a test asserts security count increases by 1 only when the ≤3 gate holds (and does NOT when at 4+). Separately, with T.K. in play on the opponent's turn, each of your **Security Digimon** (a Digimon revealed/checked from security) reads +2000 DP — a test asserts a security-checked Digimon's combat DP is base+2000 only during the opponent's turn and only while T.K. is present.
- **Rules/keyword basis:** §16 `<Recovery +(Deck)>` glossary keyword (place top of deck onto security stack); the "Security Digimon" status during the security-checking step; `[Opponent's Turn]` persistent aura (15-8-2 persistent effect, deactivates off the opponent's turn). DCGO: `ST3_09.cs` (`SecurityCards.Count <= 3` gate → `IRecovery(1)`), `ST3_12.cs` (`ChangeSecurityDigimonCardDPStaticEffect(+2000)` gated on `IsOpponentTurn`).
- **Rank:** 3 (the survival/clock half of the system — recovery loop + security-combat aura; conditional gating worth a cross-card test, but less central than the -DP→delete→payoff loop and partly single-card).

## Playstyle

Mono-Yellow control. Survive early behind cheap walls (Unimon `<Blocker>`, security pump), grind to Lv5/Lv6 Angels, then use **DP reduction as repeatable removal**: shave opponent Digimon to 0 DP each turn (Seraphimon -4000 + MagnaAngemon-inherited -1000 on attack; Heaven's Charm -2000 / Seven Heavens -10000 from hand or security) so the 0-DP rule check deletes them. The inherited Tokomon (+1000 carrier DP) and Patamon (+1 memory) observers turn each 0-DP deletion into a tempo refund, and Angewomon/Heaven's Charm/Heaven's Gate security cards keep the security stack topped up (recovery) while T.K. + Heaven's Gate `[Security]` make the player's own security a defensive wall. The deck wins slowly on attrition rather than burst.

## Win conditions

1. **Attrition removal + chip:** repeatedly delete the opponent's board via -DP→0-DP, then push security with whatever survives, while Patamon's memory refund and Angemon's `[When Attacking]` memory keep tempo even.
2. **Big Angel beatdown:** Seraphimon (10000) / Magnadramon (12000) as hard-to-trade attackers that also remove a blocker on the way in (Seraphimon -4000), so attacks resolve into security.
3. **Outlast via security recovery:** Angewomon `<Recovery>` + Heaven's Charm/Heaven's Gate self-return + T.K.'s +2000 security-Digimon aura extend the security buffer, racing the opponent out of resources.

## Ranked interactions to test

1. **Combo A — Seven Heavens (-10000) → 0-DP deletion → Tokomon-source carrier +1000.** Single-trigger, fully deterministic; exercises 17-1-3-1, the inherited 0-DP observer, and the "deleted *specifically* by dropping to 0 DP" gate (vs. deletion by other means). Highest signal-to-noise.
2. **Combo B — Seraphimon -4000 + MagnaAngemon-inherited -1000 (+ optional Heaven's Charm -2000) stack → 0-DP deletion → Patamon-source +1 memory (once per turn).** Multi-card + multi-inherited-source; verifies -DP stacking on one attack, the second observer payoff, and once-per-turn semantics.
3. **Combo C — Angewomon `<Recovery>` gated at ≤3 security (+ does-not-fire at 4+) and T.K. +2000 Security-Digimon aura active only on opponent's turn.** Conditional cross-card survival engine; lower central importance and partly single-card, but the ≤3 gate and opponent-turn aura window are worth asserting.

**Dropped candidates (not authored as interaction tests):**
- **Holy Flame `<Security Attack -3>` reducing an opp attacker's security checks** — real and useful, but a single-card debuff with no cross-card payoff in this pool; belongs in per-card behavioral tests, not an interaction test.
- **Heaven's Gate `[Security]` mass +5000 + return-to-hand on a defended security check** — interacts only with combat math, not with the -DP/observer engine; single-card security-effect test territory.
- **Angemon ST3-05 `[When Attacking]` gain 1 memory at ≥4 security** — pure single-card conditional; no second card changes its outcome.
- **Unimon `<Blocker>` / lose-2-memory on attack** — vanilla keyword + cost; covered by keyword/per-card tests.
- **Heaven's Charm `[Security]` add-to-hand** — single-card security effect, no combo partner.
