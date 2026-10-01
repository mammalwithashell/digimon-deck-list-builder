# Chronicle (EX13 slice) — Model

Slice cards: EX13-006 Dorimon, EX13-049 Dorumon, EX13-055 Raptordramon,
EX13-057 Grademon, EX13-072 Kota Domoto. All five are `IMPLEMENTED` in
`qa/qa-reports/validated_cards_dsl.json` (per-card suites in
`code/digimon-engine/tests/cards_behavioral/ex13/ex13_0{06,49,55,57,72}.rs`).
EX13-060 Alphamon (another EX13 slice, `IMPLEMENTED`) is the natural Lv.6
payoff and is pulled in as a real card for the top-end combo.

No DCGO C# exists for any of the five at the pinned DCGO commit; the printed
text (official Bandai DB bundles `data/card_bundles/EX13-0xx.md`) governs, with
DCGO BT20/Black/BT20_053.cs (BT20-053 Grademon, the BT20 printing of the same
line) as the behavioural sibling for the "if during an attack" rider.

## Card pool & roles
| Card | Role | One-line function |
|---|---|---|
| EX13-006 Dorimon | engine (egg) | Inherited [End of Your Turn][OPT]: pay 1 → unsuspend 1 [X Antibody]/[Chronicle] Digimon |
| EX13-049 Dorumon | enabler | Digivolves from [Dorimon] for 0; [When Moving]/[On Play] reveal 3 → add 1 [X Antibody]/[Chronicle]; inherited [When Attacking][OPT] -2000 |
| EX13-055 Raptordramon | engine | [Dorumon]/Lv.3 [Chronicle] route cost 2; [OP][WD] -3000; [When Attacking] may digivolve into a [Chronicle] card from hand/trash (printed cost); inherited <Barrier> |
| EX13-057 Grademon | payoff | [Raptordramon]/Lv.4 [Chronicle] route cost 3; [OP][WD] 1 [XA]/[Chronicle] Digimon gains <Reboot>+<Blocker> until opp turn end, +immunity to opp Digimon effects and +5000 if during an attack; [End of Attack][OPT] may digivolve into a [Chronicle] card from hand/trash; inherited [All Turns][OPT] leave-prevention for any own [Chronicle] Digimon (trash top security) |
| EX13-072 Kota Domoto | engine (tamer) | [Start of Your Main Phase] trash 1 [Chronicle] card → Draw 1 + 1 memory; [Your Turn] when own [Chronicle] Digimon attacks, suspend → use 1 [X Antibody]/[Chronicle] Option from hand at -1 cost; [Security] play free |
| EX13-060 Alphamon (out-of-slice payoff) | payoff | [Grademon]/Lv.5 [Chronicle] route cost 4; [When Digivolving] -8000 until opp turn end |

## Digivolution lines
- Dorimon (Lv.2 egg, Black) → Dorumon (cost 0 via [Dorimon]) → Raptordramon
  (cost 2 via [Dorumon]) → Grademon (cost 3 via [Raptordramon]) → Alphamon
  EX13-060 (cost 4 via [Grademon]). Printed circles (Black/Yellow, 1/3/4/5) are
  always the more expensive alternative; the named routes are the line's tempo.
- Mid-attack climbing: Raptordramon [When Attacking] → Grademon;
  Grademon [End of Attack] → Lv.6 [Chronicle]. Both pay the printed digivolution
  cost (no reduction) from hand OR trash.

## Named combos
### C1 Breeding hatch-and-search
- Cards: EX13-006, EX13-049 (+ EX13-055 as the search hit)
- Outcome: Dorumon digivolves onto Dorimon in the breeding area for 0; moving
  it out fires [When Moving] → reveal 3 → Raptordramon to hand, rest to deck.
- Basis: printed alt route "[Dorimon]: Cost 0"; [When Moving] fires on breeding
  → battle move (`general_rule.pdf` breeding-phase move); card text EX13-049.
- Rank: 1 (every game's opener).

### C2 Stacked DP shred
- Cards: EX13-049 (as inherited source), EX13-055
- Outcome: Raptordramon digivolves onto Dorumon via the cost-2 route; [WD]
  -3000 on a 5000 DP Digimon, then its attack fires Dorumon's inherited
  [When Attacking] -2000 → 0 DP → deleted.
- Basis: card text; 0-DP rule deletion (`general_rule.pdf` DP 0 → deleted).
- Rank: 2.

### C3 Mid-attack climb into Grademon from trash
- Cards: EX13-049 (source), EX13-055, EX13-057
- Outcome: Raptordramon attacks; [When Attacking] digivolves into Grademon from
  the trash paying the [Raptordramon] route (3); Grademon's [When Digivolving]
  resolves *during the attack*, so the chosen Digimon gets <Reboot>, <Blocker>,
  immunity to opponent Digimon effects and +5000 (7000 → 12000). Inherited
  <Barrier> from Raptordramon and Dorumon's -2000 still apply from the stack.
- Basis: card text; DCGO BT20_053.cs `attackProcess.IsAttacking` gate.
- Rank: 1 (the line's signature turn).

### C4 Kota discard fuels the trash climb
- Cards: EX13-072, EX13-055, EX13-057
- Outcome: Kota [Start of Your Main Phase] trashes Grademon (Draw 1, +1 memory);
  Raptordramon's [When Attacking] then digivolves into Grademon from the trash.
- Basis: card text (trash is a legal zone for both effects).
- Rank: 3.

### C5 Grademon End-of-Attack climb into Alphamon
- Cards: EX13-057, EX13-060
- Outcome: Grademon attacks; [End of Attack] digivolves into Alphamon from hand
  via the [Grademon] route (4); Alphamon [WD] -8000 deletes a 5000 DP Digimon.
- Basis: card text EX13-057 / EX13-060.
- Rank: 2.

### C6 Grademon's inherited shield under Alphamon
- Cards: EX13-057 (source), EX13-060 (carrier), EX13-049 (protected ally)
- Outcome: another [Chronicle] Digimon (Dorumon) about to be deleted stays by
  trashing the top security card; a non-[Chronicle] ally is NOT protected.
- Basis: card text; BT24-101 replacement shape.
- Rank: 3.

### C7 Dorimon re-arms the attacker
- Cards: EX13-006 (bottom source), EX13-049, EX13-055
- Outcome: Raptordramon attacks (suspended); at end of turn the inherited
  Dorimon clause pays 1 and unsuspends it (a ready blocker on the opponent's turn).
- Basis: card text EX13-006.
- Rank: 4.

### C8 Kota attack trigger + Raptordramon climb in one attack
- Cards: EX13-072, EX13-055, EX13-057 (+ a [Chronicle] Option)
- Outcome: one Raptordramon attack opens two triggers; both resolve: Kota is
  suspended to use a [Chronicle] Option at -1, and Raptordramon digivolves into
  Grademon from hand.
- Basis: card text; simultaneous-trigger ordering (`general_rule.pdf` §
  processing order — turn player orders own triggers).
- Rank: 4. The Option is a synthetic effectless [Chronicle] Option: no real
  [Chronicle]/[X Antibody] Option (BT9-109, BT20-095, P-204) is implemented.

## Playstyle
- Midrange climb: a Black/Yellow line that gains value mid-attack (two free
  re-digivolves per turn from hand/trash), shreds DP and turns its attacker into
  an immune, rebooting blocker. Memory curve leans on cheap named routes
  (0 → 2 → 3 → 4).

## Win conditions
- Repeated mid-attack climbs into Alphamon with <Barrier>/leave-prevention
  protecting the board, attacking through DP-shredded blockers.

## Ranked interactions to test
1. C3 — the line's signature attack-time climb + rider.
2. C1 — opener consistency.
3. C2 — DP math across two cards.
4. C5 — top-end climb into the payoff.
5. C6 — cross-card inherited protection.
6. C4 — trash enabling.
7. C7 — inherited re-arm across a 3-card stack.
8. C8 — two-trigger attack resolution.

All 8 authored; none dropped. Not modelled (no implemented pieces): Kota using
[X Antibody] BT9-109 / [Chronicle] Options BT20-095 / P-204 with their printed
effects (cards unimplemented), and BT20-056 Alphamon (PARTIAL).
