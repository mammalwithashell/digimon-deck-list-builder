# Veedramon / CS (EX13 slice) — Model

Slice: EX13-017 Veemon, EX13-019 Veedramon, EX13-022 AeroVeedramon,
EX13-067 Nokia Shiramine, EX13-069 Rina Shinomiya, EX13-074 Rie Kishibe.
Interaction suite: `code/digimon-engine/tests/archetypes/veedramon_cs_ex13.rs`.

Sources: printed text `code/digimon-engine/cards/ex13/<ID>.json` (official Bandai DB
bundles `data/card_bundles/<ID>.md` agree); DCGO
`DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_0{17,19,22,69}.cs` (EX13-067 / EX13-074
have no DCGO script at b9a0638cd); `general_rule.pdf` §16 keywords (Jamming, Draw),
security-effect / "play" rules.

## Card pool & roles
| Card | Role | One-line function |
|---|---|---|
| EX13-017 Veemon | enabler | [On Play]/[When Moving] reveal 3, add a [Veedramon]-text or [Royal Knight] card; inherited OPT "would leave by opponent's effect → suspend instead" on a [Veedramon]-name carrier |
| EX13-019 Veedramon | engine | Lv.4, <Jamming>; [When Attacking][OPT] play a [Veedramon]-text Tamer from hand for 2 less |
| EX13-022 AeroVeedramon | engine / payoff | Lv.5; [OP]/[WD]/[WA] shared OPT free [Veedramon]-text Tamer; [All Turns][OPT] own Tamer played → 1 opp Digimon/Tamer can't suspend until their turn ends; inherited OPT "this [Veedramon]-name Digimon suspends → may unsuspend" |
| EX13-069 Rina Shinomiya | engine | [Start of Main] +1 memory with a [Veemon]/[Veedramon]-name Digimon; [Your Turn] own Digimon unsuspends → suspend Rina: Draw 1, then may digivolve into a [Veedramon]-name hand card for 2 less; [Security] play |
| EX13-067 Nokia Shiramine | tech (CS partner tamer) | [Start of Main] +1 memory if opp has a Digimon; [Your Turn] own Digimon digivolves with ≤1 Digimon → suspend: play [Gabumon] (if [Greymon]) / [Agumon] (if [Garurumon]) free from hand/trash; [Security] play |
| EX13-074 Rie Kishibe | tech (Knightmon) | **BLOCKED** (engine gap, no YAML) — not in any authored test |

Cross-set pieces used (all IMPLEMENTED/AUDITED DSL): BT13-030 UlforceVeedramon (Lv.6
[Veedramon]-name top, no [When Attacking]), ST1-03 Agumon, BT23-008 Greymon (CS),
ST2-03 Gabumon, ST1-02 Biyomon (vanilla filler/opponent).

## Digivolution lines
- (Lv.2) → Veemon EX13-017 (Blue Lv.2 / 0, or Lv.2 w/[CS] / 0) → Veedramon EX13-019
  (Blue Lv.3 / 2, or Lv.3 w/[CS] / 2) → AeroVeedramon EX13-022 (Blue Lv.4 / 3, or Lv.4
  w/[CS] / 3) → UlforceVeedramon (BT22-025 Lv.5 w/[CS] / 3; BT13-030 Blue Lv.5 / 3).
- Nokia sub-line: Agumon → Greymon BT23-008 (Lv.3 w/[Agumon]-name or [CS] / 2).

## Named combos
### C1 Veemon search → Veedramon on curve
- Cards: EX13-017, EX13-019 (+ EX13-069 as an alternative legal hit).
- Outcome: Veemon [On Play] reveals 3; both Veedramon (name) and Rina ([Veedramon] in
  text) are eligible, Biyomon is not; Veedramon added, rest to bottom; Veedramon then
  digivolves onto that Veemon for 2.
- Basis: DCGO EX13_017.cs `SimplifiedRevealDeckTopCardsAndSelect(HasText("Veedramon")||RoyalKnight)`; EX13_019.cs alt digivolve cost 2.
- Rank: 1 (opener every game).

### C2 Veedramon attack → Rina for 1
- Cards: EX13-019 (on a Veemon stack), EX13-069, EX13-067 (negative).
- Outcome: [When Attacking] offers only the [Veedramon]-text Tamer (Rina, not Nokia);
  Rina enters for 3−2 = 1 memory; the attack continues to security.
- Basis: EX13_019.cs OnAllyAttack SelectHandEffect(IsTamer && HasText) cost −2.
- Rank: 2.

### C3 AeroVeedramon WD → free Rina → own lock fires
- Cards: EX13-022, EX13-019, EX13-017, EX13-069; EX13-067 (OPT negative).
- Outcome: digivolve for 3; [When Digivolving] plays Rina free; that Tamer play fires
  Aero's own [All Turns] lock on an opponent Digimon (CannotSuspend, expiry end of
  opponent's turn). A second Tamer (Nokia, paid) the same turn: lock OPT spent.
- Basis: EX13_022.cs shared OP/WD/WA hash; OnEnterFieldAnyone own Tamer →
  GainCantSuspendUntilOpponentTurnEnd.
- Rank: 3.

### C4 Unsuspend engine: attack → Aero inherited unsuspend → Rina draw + cheap digivolve
- Cards: BT13-030 (top) over EX13-022 / EX13-019 / EX13-017, EX13-069, second EX13-017
  on field, EX13-019 in hand.
- Outcome: attacking suspends Ulforce → Aero's inherited "may unsuspend" → that
  unsuspend is "any of your Digimon unsuspend" → Rina suspends: Draw 1, then the second
  Veemon digivolves into Veedramon for 2−2 = 0. Net: attacker unsuspended, Rina
  suspended, memory unchanged, hand 1 → 2, Veemon→Veedramon stack.
- Basis: EX13_022.cs OnTappedAnyone inherited; EX13_069.cs OnUnTappedAnyone →
  DrawClass(1) → DigivolveIntoHandOrTrashCard(reduce 2).
- Rank: 4 (the deck's card-advantage engine).

### C5 Resilience chain: opponent removal → Veemon suspend-instead → Aero unsuspend → Rina draw
- Cards: EX13-017 (inherited), EX13-022 (inherited), BT13-030 top, EX13-069.
- Outcome: an opponent's deletion effect on the Ulforce stack is replaced by
  suspending it (Veemon inherited); that suspension triggers Aero's inherited unsuspend,
  which triggers Rina's Draw 1. Ulforce survives untapped. A second opponent deletion
  the same turn is not prevented (Veemon OPT spent).
- Basis: EX13_017.cs WhenRemoveField IsByEffect(opponent) suspend cost; general_rule
  replacement effects; EX13_022.cs / EX13_069.cs as above.
- Rank: 5.

### C6 Security Rina → AeroVeedramon lock on the opponent's turn
- Cards: EX13-069 (security), EX13-022 (field).
- Outcome: opponent's attack checks Rina → [Security] plays it → "any of your Tamers
  are played" ([All Turns]) → Aero locks one opponent Digimon (can't suspend until
  the end of that same opponent turn — it can no longer attack).
- Basis: EX13_069.cs PlaySelfTamerSecurityEffect (PlayPermanentCards → enters field);
  EX13_022.cs OnEnterFieldAnyone.
- Rank: 6.

### C7 Nokia: Agumon → Greymon with a lone Digimon → free Gabumon
- Cards: EX13-067, ST1-03, BT23-008, ST2-03; ST1-02 (negative).
- Outcome: with 1 Digimon, Greymon digivolving lets Nokia suspend and play Gabumon from
  trash free. With 2 Digimon, nothing triggers.
- Basis: printed text + official Q&A (no DCGO script).
- Rank: 7.

### C8 CS Tamer ramp at Start of Main
- Cards: EX13-069, EX13-067, EX13-017, ST1-02 (opponent Digimon).
- Outcome: Veemon on field + an opponent Digimon → both Tamers gain 1 (+2). Without a
  [Veemon]/[Veedramon]-name Digimon only Nokia gains.
- Rank: 8.

## Playstyle
- Blue midrange/tempo; cheap CS curve (Veemon 0 / Veedramon 2 / Aero 3) with Tamers
  deployed for free or cheaply off attacks, generating memory and cards.

## Win conditions
- Repeated attacks from a Veedramon-name top that untaps (Aero inherited) while Rina
  converts every untap into a draw + discounted digivolve; Aero's suspend-lock blunts
  the opponent's swing-back; Veemon's inherited protects the stack from removal.

## Ranked interactions to test
1. C1 — opener consistency (search filter scope "in its text").
2. C2 — Veedramon's discounted Tamer play (filter excludes non-[Veedramon] CS Tamers).
3. C3 — one Tamer play feeds two of Aero's clauses.
4. C4 — the unsuspend→Rina engine (three-card inherited/observer chain).
5. C5 — removal resilience chain (replacement → suspend → unsuspend → draw).
6. C6 — security Tamer + [All Turns] observer on the opponent's turn.
7. C7 — Nokia partner play.
8. C8 — start-of-main ramp.

Dropped / blocked:
- Any Rie Kishibe (EX13-074) combo (Knightmon draw engine, LordKnightmon digivolve) —
  **BLOCKED**: EX13-074 has no YAML (engine gap).
- UlforceVeedramon BT22-025 mode "play blue Tamer ≤4 free" → Rina: not selected (BT22-025
  is outside the slice and the Tamer-play payoff is already covered by C3).

## Run findings (2026-10-01)
- C1–C5, C7, C8: green (9 tests).
- C6: **CONFIRMED engine bug** — `play_pending_security` fires only `OnPlay`, so a
  `[Security] Play this card` Tamer never triggers `OnEnterFieldAnyone` /
  `OnAllyPlayed` observers (Aero's lock does not fire). The model claim stands (DCGO
  `PlaySelfTamerSecurityEffect` → `PlayPermanentCards(activateETB: true)`); the test is an
  `#[ignore]`d reproducer. Routed to `docs/RUST_ENGINE_GAPS.md`
  §G-ENGINE-SECURITY-PLAY-SKIPS-ENTER-FIELD-OBSERVERS.
