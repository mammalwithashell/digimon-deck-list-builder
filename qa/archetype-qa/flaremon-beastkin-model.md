# Flaremon / Beastkin (BT25 slice) — Model

Slice authored by `/archetype-interaction-test-author` as an explicit `--cards`
list (not a `deck_library.json` archetype). The four cards are the BT25
`[Beastkin][Iliad][TS]` "Three Sovereigns" beast line — every one is a Lv.4/5
beast carrying an *intra-archetype digivolve engine*: a `[When Digivolving]` /
attack / win trigger that, when its gate is met, performs a **second**
digivolution into a named beast partner ([Apollomon] / [Crescemon] / [Marsmon] /
[Callismon]). All four are IMPLEMENTED and per-card-green
(`cards_behavioral/bt25/bt25_0{16,17,24,54}.rs`, 37 tests pass).

Source priority (CLAUDE.md): printed card face (`BT25-0xx.webp`) + DCGO C#
(`$BASE_DCGO/Assets/Scripts/CardEffect/BT25/<Colour>/BT25_0xx.cs`) outrank
cards.json; the YAML headers carry the per-card DCGO crosschecks.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT25-024 Lekismon (Blue Lv.4, DP 5000) | engine / enabler | `[OP][WD] <Draw 1>`; `[Your Turn]` when your Digimon are **played/digivolve and any are red**, may digivolve into **[Crescemon] in the TRASH**, cost −1. Inherited `<Jamming>`. |
| BT25-016 GrapLeomon (Red Lv.5, DP 7000) | payoff / engine | `[OP][WD]` 1 of your Digimon gets **+3000 DP** for the turn, then 1 may attack; `[All Turns]` when a **13000+ DP** Digimon attacks, may digivolve into **[Marsmon]/[Callismon]** in hand free. Inherited `<Security A. +1>`. |
| BT25-017 Flaremon (Red Lv.5, DP 7000) | payoff / removal | `[OP][WD]` may attack, then by trashing 1 hand card **delete 1 opp Digimon ≤7000 DP**; `[Your Turn]` when your Digimon are **played/digivolve and any are BLUE**, may digivolve into **[Apollomon]** in hand, cost −2. Inherited `<Security A. +1>`. |
| BT25-054 GreatGrizzlymon (Green Lv.5, DP 7000) | control / payoff | `<Blocker>`; `[OP][WD]` **taunt** 1 opp Digimon (gives it `[Start of Your Main Phase] This Digimon attacks.` until their turn ends); `[All Turns]` **when it wins a battle**, may digivolve into **[Callismon]/[Marsmon]** in hand free. Inherited `[OPT]` on battle-win → trash opp top security. |

## Digivolution lines

- All four take a Lv.4 `[TS]`-trait alt-digivolve (cost 2 for Lekismon at Lv.3,
  cost 3 for the three Lv.5s) plus the standard same-colour Lv.3/Lv.4 evo block.
- Each "engine" clause produces a *second* digivolution into a named beast Mega
  partner (Apollomon / Crescemon / Marsmon / Callismon) — the partners are
  cross-set Megas, modelled here as **synthetic evolution targets** (no
  `[When Digivolving]` of their own) because no combo fires a partner's printed
  effect (lazy, no eager closure).

## Named combos

### F1 — GrapLeomon pump arms the 13000-DP free-digivolve observer
- Cards: BT25-016 (×2 — one pump source, one observer), a 10000-DP attacker.
- Expected outcome: GrapLeomon's `[OP]` **+3000** pump lifts a 10000-DP attacker
  to exactly **13000**; when that attacker attacks, the observer GrapLeomon's
  `[All Turns]` clause **offers a free digivolve** into a hand partner (memory
  unchanged). Without the pump reaching the attacker it stays 10000 → no offer.
- Rules/keyword basis: DP modifiers fold into the `≥13000` comparison
  (`general_rule.pdf` §11 DP; §16 digivolve timing); DCGO `BT25_016.cs`
  (`OnPlay` +3000 `UntilEachTurnEnd`; `OnAllyAttack` DP≥13000 → skippable free
  digivolve).
- Rank: 1 (the pump is the *only* enabler of the threshold; highest payoff
  centrality).

### F2 — Playing the blue beast (Lekismon) arms Flaremon's blue gate
- Cards: BT25-017 Flaremon (observer on field), BT25-024 Lekismon (blue entry),
  [Apollomon] in hand.
- Expected outcome: playing Lekismon (Blue) is a "your Digimon played, any blue"
  event → Flaremon's `[Your Turn]` `if any of them are blue` clause **offers its
  digivolve into [Apollomon]**. A **red** beast entry does NOT arm it.
- Rules/keyword basis: card image BT25-017 (gate is **blue**, cost −2) + DCGO
  `BT25_017.cs` `OnEnterFieldAnyone`; BT25-024 is a Blue Lv.4 (card image /
  cards.json color=Blue). `general_rule.pdf` §16 play-vs-digivolve event timing.
- Rank: 2 (couples the deck's blue tempo card to a red payoff's ramp; the
  archetype's blue/red colour-pair is exactly what makes both `[Your Turn]`
  gates live).
- **Model correction during authoring:** the initial draft had Flaremon's gate
  as *red* (confusing it with Lekismon's red gate). The card face is **blue** —
  Flaremon watches blue, Lekismon watches red. Tests assert the blue gate.

### F3 — GreatGrizzlymon taunt → forced attack → battle-win payoffs
- Cards: BT25-054 GreatGrizzlymon (`<Blocker>`), an opp Digimon (taunted),
  [Callismon] in hand; a carrier Mega for the buried-inherited half.
- Expected outcome: the taunt FORCES an opponent Digimon to attack into
  GreatGrizzlymon, which (7000 DP, Blocker) wins. The win fires **(A)** the
  top-card `[All Turns]` **free digivolve** (memory unchanged) and **(B)**, when
  GreatGrizzlymon is buried as a digivolution *source*, the inherited `[OPT]`
  **trash opp top security**. A plain security attack (not a Digimon battle)
  fires neither.
- Rules/keyword basis: DCGO `BT25_054.cs` (`OnPlay` taunt `UntilOwnerTurnEnd`;
  `OnEndBattle` win → skippable free digivolve; inherited `OnEndBattle` win →
  `IDestroySecurity` top). Inherited effects fire only from beneath a stack
  (`general_rule.pdf` digivolution-card / inherited-effect rules) — so A and B
  are split across two stack positions.
- Rank: 3 (the control plan: taunt feeds the win engine).

## Playstyle

Midrange beast tempo. Blue Lekismon ramps/cycles and (with a red entry on board)
loops into Crescemon; red GrapLeomon/Flaremon are the aggressive payoffs (pump +
attack, removal); green GreatGrizzlymon is the control valve (Blocker + taunt +
win-loop). The `[TS]`-trait alt-digivolve and the free "into [Marsmon]/
[Callismon]/[Apollomon]" digivolves give the deck repeated cost-skipping tempo.

## Win conditions

Free-digivolve chains into the Mega beasts (Marsmon / Callismon / Apollomon) for
board dominance, plus `<Security A. +1>` on the Lv.5s and GreatGrizzlymon's
inherited security-trash-on-win to push extra checks.

## Ranked interactions tested

1. **F1** — GrapLeomon pump ⇒ 13000 threshold ⇒ observer free-digivolve. (2 tests:
   happy + pump-elsewhere unhappy.)
2. **F2** — blue Lekismon entry ⇒ Flaremon Apollomon offer. (2 tests: blue happy +
   red unhappy.)
3. **F3** — taunt ⇒ win ⇒ free-digivolve (top-card) + trash-security (buried).
   (4 tests: taunt-install, top-card win digivolve, buried win security-trash,
   security-attack-is-not-a-battle-win negative.)

### Interactions considered but NOT separately authored (logged, not dropped)
- **Lekismon red-gate → Crescemon-from-trash loop** (BT25-024 self): the red
  `[Your Turn]` digivolve into a TRASH [Crescemon] is fully covered by the
  per-card test `bt25_024.rs` (offer-on-red / no-offer-on-blue / no-Crescemon).
  It is a *single-card* clause (no second slice card participates), so it adds no
  cross-card system fact beyond the per-card suite — not re-authored here.
- **Flaremon trash→delete ≤7000** (BT25-017 self): single-card removal, covered
  by `bt25_017.rs`. No cross-card amplifier in this 4-card slice (e.g. no slice
  card raises an opponent's deletable window), so no interaction test.
- **GrapLeomon "may attack" → its own 13000 observer in one play**: folded into
  F1 (the pump+attack is the same clause); F1 drives the attack explicitly rather
  than via the optional in-clause attack to keep the assertion deterministic.

## Notes for reviewers

- Cross-set partners are **synthetic** (`make_*` fixtures with level/colour/DP/
  evo-cost only). No cross-set card implementation was pulled — none of F1/F2/F3
  fires a partner's printed effect.
- F2's gate colour was corrected from red→blue against the card image during
  authoring (see F2 above).
- F3's two win-payoffs are split by stack position because inherited effects only
  apply when the carrier is a digivolution source — a genuine engine rule, not a
  test workaround.
