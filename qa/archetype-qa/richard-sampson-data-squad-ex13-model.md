# Richard Sampson / DATA SQUAD (EX13 slice) — Model

Slice cards: EX13-003 Kyaromon, EX13-026 Kudamon, EX13-030 Reppamon,
EX13-032 Chirinmon, EX13-071 Richard Sampson. Interaction suite:
`code/digimon-engine/tests/archetypes/richard_sampson_data_squad_ex13.rs`.

Sources: printed text from the official Bandai DB bundles
(`data/card_bundles/EX13-0{03,26,30,32,71}.md`); DCGO has **no** C# for any
slice card at `b9a0638cd` (noted in each YAML header), so printed text +
official Q&A + `general_rule.pdf` govern. Keyword refs: <Barrier> §16
(would-be-deleted-in-battle replacement, trash top security); optional
processing conditions rule 15-7-2 / 15-7-4 (Reppamon Q&A "Yes, you can.").

## Card pool & roles
| Card | Status | Role | One-line function |
|---|---|---|---|
| EX13-003 Kyaromon | IMPLEMENTED | engine (egg) | Inherited [Your Turn][OPT] when your security is removed from, the carrier may digivolve into a [Kentaurosmon]-name / [Holy Beast] hand card, cost −1. |
| EX13-026 Kudamon | IMPLEMENTED | enabler | [On Play]/[When Moving] reveal 3: add 1 HB/RK/DS card to hand + place 1 face down at the BOTTOM of a [DATA SQUAD] Tamer (Q&A); rest bottom. Inherited [WA][OPT] opp Digimon <Security A. −1> until their turn ends. |
| EX13-030 Reppamon | IMPLEMENTED | engine / defence | <Barrier> + inherited <Barrier>. [OP][WD][WA][OPT] by trashing top security, may play [Richard Sampson] from hand/trash free (cost payable with no Richard — Q&A). |
| EX13-032 Chirinmon | IMPLEMENTED (2026-10-01) | payoff | All clauses authored; would-leave saves unblocked by G-TOP-STACKED-CARD-TO-SECURITY (RESOLVED). |
| EX13-071 Richard Sampson | **BLOCKED** (hybrid, partial) | engine (tamer) | SOMP/On Play stash + memory, [Security] play authored; [Main] Kudamon→Kentaurosmon blocked on G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST. |

Cross-set cards used (all IMPLEMENTED DSL): EX13-036 Kentaurosmon (Lv.6
yellow, Kyaromon's name target), EX12-046 Shishimamon (Lv.5 yellow [Holy
Beast], yellow Lv.4 circle cost 4 — the only implemented Lv.5 yellow Holy
Beast), ST24-14 Yoshino Fujieda & Keenan Crier (a real non-blocked [DATA SQUAD]
Tamer; Richard is BLOCKED and BT25-087 Thomas is BLOCKED), ST1-02 Biyomon /
ST2-10 Plesiomon (vanilla filler / opponent bodies).

## Digivolution lines
- Kyaromon (Lv.2 yellow DATA SQUAD egg) → Kudamon (cost 0: yellow Lv.2 circle
  or Lv.2 w/[DATA SQUAD]) → Reppamon (cost 2) → Chirinmon (cost 3, BLOCKED) →
  Kentaurosmon (EX13-036, cost 3 from yellow / HB / DS Lv.5).
- Off-line Lv.5: Shishimamon (EX12-046, yellow Lv.4 circle cost 4).

## Named combos
### C1 — Kudamon breeding move feeds a [DATA SQUAD] Tamer
- Cards: EX13-003, EX13-026, ST24-14 (+ EX13-030 / EX13-036 as reveal hits)
- Outcome: moving Kudamon (on Kyaromon) out of breeding fires [When Moving];
  of 3 revealed, 1 eligible to hand, 1 eligible face down at the bottom of the
  DS Tamer, the ineligible card to deck bottom. No DS Tamer → add-only branch.
- Basis: EX13-026 printed text + Q&A; ST23-06 Gekkomon DCGO idiom.
- Rank: high (the slice's only card-flow engine that is fully implemented).

### C2 — Reppamon [When Digivolving] security trash wakes Kyaromon
- Cards: EX13-003, EX13-026, EX13-030, EX12-046
- Outcome: Reppamon digivolves on Kudamon/Kyaromon (cost 2); paying its
  optional cost (no Richard available — legal per Q&A) trashes 1 security,
  which fires Kyaromon's inherited trigger: the stack digivolves into
  Shishimamon from hand at 4−1 = 3. Declining the cost → no trigger.
- Basis: EX13-030 Q&A / rule 15-7-4; EX13-003 printed text.
- Rank: high (self-damage-as-ramp is the deck's core tempo engine).

### C3 — Reppamon shared [Once Per Turn] + Kudamon inherited on attack
- Cards: EX13-026, EX13-030
- Outcome: after Reppamon's WD cost is paid, its WA (same OPT clause) does not
  re-offer on the same turn's attack, while Kudamon's inherited WA still gives
  the opponent's Digimon <Security A. −1>.
- Basis: EX13-030 (one clause, three timings, one OPT); EX13-026 inherited.
- Rank: medium.

### C4 — Inherited <Barrier> self-trash wakes Kyaromon into Kentaurosmon
- Cards: EX13-003, EX13-026, EX13-030, EX12-046, EX13-036
- Outcome: Shishimamon (on Reppamon/Kudamon/Kyaromon) loses a battle on your
  turn; Reppamon's inherited <Barrier> trashes 1 security instead of deletion,
  which fires Kyaromon: digivolve into Kentaurosmon at 3−1 = 2. On the
  opponent's turn the same Barrier save does NOT wake Kyaromon ([Your Turn]).
- Basis: <Barrier> §16; EX13-003 [Your Turn] gate.
- Rank: high (the deck's route to its Lv.6 payoff).

### Blocked combos (not authored)
- Reppamon [OP/WD/WA] plays [Richard Sampson] free — EX13-071 BLOCKED.
- Kudamon reveal stashes under Richard — EX13-071 BLOCKED.
- Richard SOMP stash → Chirinmon trashes a Tamer face-down card to unsuspend — EX13-071 + EX13-032 BLOCKED.
- Richard [Main] Kudamon → Kentaurosmon (ignore level, −1) — EX13-071 BLOCKED (G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST).
- Chirinmon WD unsuspend + opponent [When Digivolving] lock — EX13-032 BLOCKED.
- Chirinmon / Kentaurosmon-carrier "top stacked card to security" leave save — EX13-032 now IMPLEMENTED (2026-10-01); combo test not yet authored.
- Kyaromon → Chirinmon off a security trash — EX13-032 BLOCKED.

## Playstyle
- Yellow security-as-resource midrange: trades its own security (Reppamon
  cost, <Barrier>) for tempo (Kyaromon cost-reduced digivolves, Chirinmon
  unsuspend) and refills it via Chirinmon/Kentaurosmon top-card saves.

## Win conditions
- Land Kentaurosmon (EX13-036) cheaply via Kyaromon / Richard's [Main], then
  -7000 sweeps + placing opposing Digimon into security.

## Ranked interactions to test
1. C2 — core engine, two prompts across three cards.
2. C4 — Lv.6 route + [Your Turn] system gate.
3. C1 — tamer stash flow.
4. C3 — OPT shared across timings.
Dropped: none beyond the blocked list (slice is small; all four authored).

## Run record (2026-10-01)
- 7/7 interaction tests green (`--test archetypes richard_sampson_data_squad_ex13`).
- Finding filed: G-ENGINE-OPT-SPENT-TRIGGER-IN-TRIGGER-ORDER (`docs/RUST_ENGINE_GAPS.md`), surfaced by C3 —
  the spent Reppamon [WA] is still offered as a TriggerOrder entry (it does nothing when picked), so the outcome
  is correct but the menu contains a choice that changes nothing.
