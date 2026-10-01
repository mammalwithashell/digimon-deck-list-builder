# Examon / holy warrior (Royal Knights Assembly) [EX13] — Model

Slice of the EX13 author-set run. Cards: EX13-001, -014, -015, -016, -020,
-023, -036, -037, -043, -045, -060, -061, -062, -064, -077. Interaction tests:
`code/digimon-engine/tests/archetypes/royal_knights_ex13.rs`. Printed text is
from the official Bandai DB bundles (`data/card_bundles/<ID>.md`); DCGO C#
exists at the base-repo submodule only for EX13-001, -015, -020, -045, -062
(`DCGO/Assets/Scripts/CardEffect/EX13/<Color>/EX13_0xx.cs`). For the rest,
printed text + `general_rule.pdf` govern.

## Card pool & roles
| Card | Name | Status | Role | One-line function |
|---|---|---|---|---|
| EX13-001 | Gigimon | IMPLEMENTED | enabler | Inherited [Your Turn][OPT]: red Tamer played → carrier may digivolve into a [Growlmon]/[Gallantmon]-named hand card −2 |
| EX13-020 | Magnamon | IMPLEMENTED | engine | Blocker/Armor Purge; OP/WD/WA +1000 per trash colour then −4000 per 5000 DP; printed + inherited [End of Your Turn][OPT] unsuspend a [Free]/[Royal Knight] Digimon |
| EX13-014 | Jesmon | IMPLEMENTED | payoff/engine | WD/WA free [Huckmon]-text Option ≤5; [All Turns][OPT] own Digimon played → may delete opp lowest DP + Atho token |
| EX13-015 | Gallantmon | IMPLEMENTED | payoff | Raid/Progress/Blocker; OP/WD/WA/Counter OPT delete opp ≥12000, else trash their top security; 9000-DP leave-replacement |
| EX13-023 | UlforceVeedramon | IMPLEMENTED (2026-10-01) | payoff | stacked-card protection aura (G-ENGINE-STACKED-CARD-RETURN-PROTECTION RESOLVED) |
| EX13-036 | Kentaurosmon | IMPLEMENTED | payoff | [Security]/OP −7000 (all at ≤6 total security); WD trash most-security top → re-run [Security]; place Digimon as security |
| EX13-037 | Dynasmon | IMPLEMENTED | payoff/engine | OP/WD/WA trash own top security +10000, ≤3 → trash theirs; [All Turns][OPT] security removed (either stack) → opp −12000, ≤3 → Recovery +1 |
| EX13-043 | Leopardmon | IMPLEMENTED | enabler | OP/WD suspend any Digimon then bottom-deck opp lowest; WD/WA OPT play a [Royal Knight]/[Beast]… card from hand −4 −1 per suspended Digimon |
| EX13-060 | Alphamon | IMPLEMENTED | payoff | [Chronicle] tool-box: WD −8000; Chronicle played → attack + re-fire WD; EoT play Chronicle −6 with Rush |
| EX13-061 | Gankoomon | IMPLEMENTED | payoff | Reboot/Blocker; OP/WD Hinukamuy token + white Digimon immunity; white suspend → free [Huckmon] Option |
| EX13-062 | Craniamon | PARTIAL | payoff | Reboot/Blocker; OP/WD opp-effect immunity; self-suspend OPT → delete ALL opp lowest play cost; self-unsuspend → +3000 (effect-driven only — G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND) |
| EX13-064 | LordKnightmon | IMPLEMENTED | payoff | WD free ≤8 [Knightmon]-text from hand/trash; [Your Turn] Knightmon-text gain Alliance/Piercing; other Digimon/Tamer played → Rush+Collision attack |
| EX13-016 | Omnimon | IMPLEMENTED (2026-10-01) | payoff | same-level source-pair leave-replacement (G-ENGINE-SAME-LEVEL-SOURCE-PAIR-SELECTION RESOLVED) |
| EX13-045 | Examon | IMPLEMENTED | finisher | DNA Green Lv.6 + Blue Lv.6 cost 0; if DNA: attacks + all own +10000, then may battle; win → free ≤12 [Dracomon]/[Examon]-text card |
| EX13-077 | Omnimon: Merciful Mode | IMPLEMENTED (2026-10-01) | finisher | Assembly distinct-colour materials (G-ASSEMBLY-DISTINCT-BY-COLOR resolved) |

## Digivolution lines
- Gigimon (egg) → … → red Lv.5 → **Gallantmon** (Red Lv.5 / 3; −2 via Gigimon's Tamer trigger → 1).
- Green Lv.5 → **Leopardmon** (3) ; Black Lv.5 → **Craniamon** (3) / **Gankoomon** (5) ; Red|Yellow Lv.5 → **Dynasmon** (4) ; Red Lv.5 → **Jesmon** (3).
- Every Lv.6 knight also has an **Assembly -5** from trash (Lv.5×Lv.4×Lv.3 of its sub-family).
- **Examon** = DNA of a green Lv.6 (Leopardmon) + a blue Lv.6, cost 0.
- Lv.3 Blue/Yellow → **Magnamon** (4) / [Veemon] (3) / Assembly -2 [Veemon].

## Named combos
### C1 Takato → Gigimon → Gallantmon
- Cards: EX13-068 (red Tamer), EX13-001, EX13-015.
- Expected: playing the red Tamer lets Gigimon's carrier (red Lv.5) digivolve into Gallantmon for 3 − 2 = 1; Gallantmon's WD finds no ≥12000 target → trash their top security.
- Basis: DCGO `EX13/Red/EX13_001.cs` (reduceCost 2), `EX13/Red/EX13_015.cs` (failedToDelete → trash security); `general_rule.pdf` inherited effects.
- Rank: high (the Gallantmon line's tempo engine).

### C2 Gallantmon security trash → Dynasmon punisher
- Cards: EX13-015, EX13-037.
- Expected: Gallantmon's "didn't delete → trash their top security" is a security removal → Dynasmon gives an opp Digimon −12000 (deleted at the rule check) and Recovery +1 at ≤3 own security. Unhappy: when Gallantmon deletes a ≥12000 Digimon instead, Dynasmon stays dormant.
- Basis: printed text (no EX13_037.cs); EX13-060 Q&A for 0-DP rule check; Recovery §16.
- Rank: high (two red knights' natural cross-synergy).

### C3 Gankoomon under Jesmon
- Cards: EX13-014, EX13-061.
- Expected: one Gankoomon play fires Jesmon's OPT (delete opp lowest DP + Atho token) AND Gankoomon's OP (Hinukamuy token); the token plays don't re-fire Jesmon ([Once Per Turn]).
- Basis: printed text; simultaneous triggers (turn player orders), OPT.
- Rank: medium-high (Huckmon-text sub-engine).

### C4 Leopardmon: suspend first → cheaper [Royal Knight]
- Cards: EX13-043, EX13-062.
- Expected: Leopardmon's two WD clauses trigger together; suspending first makes Craniamon cost 12 − 4 − 1 = 7; playing first costs 12 − 4 = 8 (and overspends 7 memory → turn passes).
- Basis: printed text; `general_rule.pdf` turn player chooses simultaneous-trigger order.
- Rank: high (the slice's main [Royal Knight] cheat-in).

### C5 Craniamon attack wipe + Magnamon wake
- Cards: EX13-062, EX13-020 (printed + inherited copies).
- Expected: attack-suspend → delete all opp lowest-play-cost Digimon; at EoT both Magnamon clauses unsuspend a [Royal Knight] each (Craniamon gets +3000 on unsuspend; the field Magnamon wakes too); both stand as Blockers on the opponent's turn; the +3000 expires with our turn.
- Basis: DCGO `EX13/Black/EX13_062.cs` (OnTappedAnyone / OnUnTappedAnyone), `EX13/Blue/EX13_020.cs`.
- Rank: high (aggressive attack with no defensive cost).

### C6 Leopardmon DNA → Examon + Magnamon wake
- Cards: EX13-043, ST2-10 (vanilla blue Lv.6), EX13-045, EX13-020.
- Expected: DNA cost 0; all own Digimon +10000 (Magnamon 17000, Examon 25000); forced attack (Security A. +1 → 2 checks) then may battle (deletes Birdramon); EoT Magnamon unsuspends the [Royal Knight] Examon.
- Basis: DCGO `EX13/Green/EX13_045.cs` (AddJogressConditionClass, IsJogress), `EX13/Blue/EX13_020.cs`; DNA digivolution rules.
- Rank: high (the closer).

## Playstyle
- Midrange toolbox: one or two Lv.6 knights per game cheated in via Assembly / Leopardmon / Gigimon, protected by Blocker + Magnamon re-standing, closing with Examon DNA or Gallantmon/Dynasmon security pressure.

## Win conditions
- Examon DNA end-game swing (team +10000, attack + battle); Dynasmon/Gallantmon security trash plus Raid/Piercing attacks.

## Ranked interactions to test
1. C4 Leopardmon ordering — authored (2 tests).
2. C6 Examon DNA + Magnamon — authored.
3. C2 Gallantmon → Dynasmon — authored (+ unhappy path).
4. C1 Gigimon → Gallantmon — authored.
5. C5 Craniamon + Magnamon ×2 — authored.
6. C3 Jesmon + Gankoomon — authored.

### Not selected (logged, not silently dropped)
- Alphamon [Chronicle] engine (EX13-060) — needs a cross-set [Chronicle] Digimon/Tamer; no in-slice partner. Per-card tests cover it.
- LordKnightmon + Knightmon-text recursion (EX13-064 + EX13-058) — EX13-058 has no verdict in `validated_cards_dsl.json` ("unknown" ≠ implemented).
- Kentaurosmon WD most-security trash → Dynasmon trigger (EX13-036 + EX13-037) — redundant with C2's security-removal edge.
- Gankoomon / Jesmon white-suspend → free [Huckmon] Option — needs a real implemented [Huckmon]-text Option; deferred.

### Blocked combos
- UlforceVeedramon (EX13-023) CS line / Magnamon wake — EX13-023 now IMPLEMENTED (2026-10-01); combo test not yet authored.
- Omnimon (EX13-016) DNA/Assembly and its leave-replacement — BLOCKED (hybrid).
- Omnimon → Omnimon: Merciful Mode (EX13-016 → EX13-077) — both BLOCKED.

## Run record (2026-10-01)
- 8 interaction tests, all green. No engine findings filed.
- Static harness (`archetype-static-tests "Royal Knights"`, pre-EX13 RK deck-library pool, `--no-write`): deck-legality pass, smoke 3/3 pass, combo-presence 6/8 (the 2 BLOCKED combos), coverage gate on that legacy pool 76/89. Slice coverage: 12/15 implemented (EX13-062 PARTIAL; EX13-016/-023/-077 BLOCKED).
- Test-design note: ending the turn with memory on our side makes the debug opponent auto-pass straight back into our unsuspend phase, masking Magnamon's wake; the tests set memory to −3 first so the opponent's turn waits at Main.
