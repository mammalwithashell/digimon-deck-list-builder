# Witchelny (EX13 slice) — Model

Slice cards: EX13-025 Candlemon, EX13-029 FlameWizardmon, EX13-033 Mistymon.
All three are `IMPLEMENTED` in `qa/qa-reports/validated_cards_dsl.json`
(per-card suites `code/digimon-engine/tests/cards_behavioral/ex13/ex13_0{25,29,33}.rs`).

Out-of-slice EX13 partners that the line naturally runs with, all already
`IMPLEMENTED` (no pull needed): EX13-004 DemiMeramon (egg; inherited
[When Attacking] digivolve into a [Witchelny]-text card at -1, then trash top
security) and EX13-037 Dynasmon (Lv.6 [Witchelny]-route top end, used as the
carrier for Mistymon's inherited unsuspend).

No DCGO C# exists for any of the three at the pinned submodule; the printed
text (official Bandai DB bundles `data/card_bundles/EX13-0xx.md`) governs. The
Candlemon/FlameWizardmon inherited leave-prevention mirrors DCGO
`BT18/Yellow/BT18_030.cs` (BT18-030 Candlemon inherited: WhenRemoveField,
OPT, optional, opponent-effect gate, cost = trash top security).

Rules basis (`general_rule.pdf`, via `docs/digimon-rules/digest.md`):
- 15-7-3/15-7-4/15-7-5 — "By X, Y" optional processing conditions: the player
  chooses to pay; the cost may be paid even if the result can't happen.
- 15-8-3-2 — trigger-type effects can't activate mid-processing; they wait
  as pending activation until the current effect finishes.
- 15-4-5-2/3 — derived triggers activate before still-pending ones.
- 15-8-5 — "would leave" immediate-type replacement effects interrupt right
  before their cause.

## Card pool & roles
| Card | Role | One-line function |
|---|---|---|
| EX13-025 Candlemon | enabler / engine | (Rule) Trait [Witchelny]; [Start of Your Main Phase] if 3+ security: trash top/bottom security, Draw 1, +1 memory; then if ≤2 security, may place 1 [Witchelny]-text card from hand as bottom security. Inherited [All Turns][OPT] leave-prevention for a [Dynasmon]/[Witchelny]-text carrier (trash top security) |
| EX13-029 FlameWizardmon | removal | Lv.3 w/[Witchelny] route cost 2; <Armor Purge>; [WD][WA][OPT] by trashing top security: opp Digimon -4000; after, if ≤3 security, delete 1 opp Digimon ≤4000 DP. Same inherited leave-prevention as Candlemon |
| EX13-033 Mistymon | payoff / engine | Lv.4 w/[Witchelny] route cost 3; <Barrier>; [OP][WD] may place a [Witchelny]-text card from hand as bottom security, then by trashing top security 1 own Digimon may attack; [All Turns][OPT] when your security is removed from: opp Digimon -6000, then if ≤3 security delete 1 ≤6000 DP. Inherited [All Turns][OPT] when your security is removed from: may unsuspend |
| EX13-004 DemiMeramon (partner) | enabler (egg) | Inherited [WA][OPT] digivolve into a [Witchelny]-text card from hand at -1; if it did, trash top security |
| EX13-037 Dynasmon (partner) | payoff | Lv.5 w/[Witchelny] route cost 3; [OP][WD][WA][OPT] trash top security, +10000 |

Fillers / opponents are effectless vanilla ST3 Digimon: ST3-02 Salamon
(Lv.3 3000), ST3-03 Tapirmon (Lv.3 4000), ST3-06 Gatomon (Lv.4 5000),
ST3-10 Magnadramon (Lv.6 12000).

## Digivolution lines
- (DemiMeramon, Yellow Lv.2) → Candlemon (Yellow Lv.2 / 0) → FlameWizardmon
  (Lv.3 w/[Witchelny] in text / **2**, else Yellow/Red Lv.3 / 3) → Mistymon
  (Lv.4 w/[Witchelny] / **3**, else 4) → Dynasmon (Lv.5 w/[Witchelny] / **3**).
- Candlemon's (Rule) Trait makes it a [Witchelny]-text card, so it unlocks the
  discounted FlameWizardmon route; a non-[Witchelny] Lv.3 pays the printed 3.

## Named combos
### C1 Witchelny discount route into FlameWizardmon removal
- Cards: EX13-025, EX13-029
- Outcome: FlameWizardmon digivolves onto Candlemon for 2 (Witchelny route);
  [WD] pays top security (4→3), -4000 on a 12000 opp, and with 3 security
  deletes a 4000-DP opp Digimon.
- Basis: printed alt route + [WD] text; 15-7-4.
- Unhappy path (C1b): on a vanilla yellow Lv.3 (Salamon) only the printed
  cost-3 circle applies.
- Rank: 1 (the line's core tempo play).

### C2 FlameWizardmon's cost feeds Mistymon's observer (double removal)
- Cards: EX13-029, EX13-025, EX13-033
- Outcome: Mistymon on the field; FlameWizardmon's [WD] cost trashes a
  security card → after FW resolves (15-8-3-2), Mistymon's [All Turns][OPT]
  fires: -6000 + delete ≤6000. One digivolve removes two opponent Digimon.
- Rank: 2.

### C3 Mistymon [WD]: placed security funds the attack cost and keeps the threshold
- Cards: EX13-025 (Witchelny card to place, and the stack base), EX13-029, EX13-033
- Outcome: Mistymon digivolves onto FlameWizardmon for 3; places Candlemon
  from hand as bottom security (3→4), pays the top (4→3) so a Digimon attacks;
  the payment fires Mistymon's own observer at 3 security → -6000 + delete.
- Rank: 3.

### C4 Stacked inherited saves feed Mistymon's observer
- Cards: EX13-025 + EX13-029 (sources), EX13-033 (top)
- Outcome: Opponent-effect deletion of the Mistymon stack is prevented by a
  source inherited (trash top security); that removal fires Mistymon's
  observer (delete). A second opponent removal the same turn is stopped by the
  OTHER source (distinct OPTs); a third goes through.
- Basis: DCGO BT18_030.cs inherited; 15-8-5.
- Rank: 4.

### C5 Candlemon's main-phase trash triggers Mistymon and recycles a Witchelny card
- Cards: EX13-025, EX13-033, EX13-029 (placed card)
- Outcome: 3 security → trash 1, Draw 1, +1 memory; 2 left → place
  FlameWizardmon as bottom security; Mistymon's observer fires off the trash.
- Rank: 5.

### C6 DemiMeramon mid-attack climb into FlameWizardmon
- Cards: EX13-004, EX13-025, EX13-029
- Outcome: Candlemon (on DemiMeramon) attacks → inherited digivolves into
  FlameWizardmon for 2-1=1, trashes top security (5→4); FW [WD] (derived
  trigger) pays again (4→3) → -4000 + delete ≤4000.
- Rank: 6.

### C7 Mistymon inherited unsuspends Dynasmon after its attack trash
- Cards: EX13-033 (source), EX13-037 (top), EX13-025/EX13-029 (lower sources)
- Outcome: Dynasmon attacks (suspends); its [WA] trashes top security →
  Mistymon's inherited "may unsuspend" readies Dynasmon.
- Rank: 7.

## Playstyle
- Midrange/control: spends its own security as a resource (every clause pays
  or trashes security) to buy removal, then refills via placements. Memory
  curve is cheap thanks to the [Witchelny] routes (0 → 2 → 3 → 3).

## Win conditions
- Board control through repeated -DP + delete triggered by its own security
  loss, closing with Dynasmon / extra attacks from Mistymon.

## Ranked interactions to test
1. C1 — core discount route + threshold removal (and the non-Witchelny unhappy path).
2. C2 — cross-card security-removal observer chain.
3. C3 — placement-funded cost staying under the ≤3 threshold.
4. C4 — stacked inherited saves (distinct OPTs) feeding the observer.
5. C5 — Candlemon start-of-main + Mistymon observer + recycle placement.
6. C6 — egg-driven mid-attack climb with derived [WD].
7. C7 — inherited unsuspend under the Lv.6 top end.

Not selected (logged): FlameWizardmon <Armor Purge> interplay with the
inherited save (Armor Purge is a self-only would-be-deleted keyword already
covered per-card); Mistymon <Barrier> in battle (per-card); Dynasmon Assembly
-5 from three [Witchelny] cards (Dynasmon belongs to the Royal Knights slice and
its Assembly is covered in `royal_knights_ex13.rs`).
