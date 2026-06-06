# Callismon / Dark Animal (BT25 slice) — Model

Slice cards (as given): **BT25-082 BlackGatomon**, **BT25-058 Callismon**,
**BT25-095 Paradise Colosseum**. All three are implemented as YAML DSL cards
(`code/digimon-engine/cards/bt25/`) with green per-card behavioral tests
(`code/digimon-engine/tests/cards_behavioral/bt25/{bt25_058,bt25_082,bt25_095}.rs`).

This is a **slice**, not a full archetype: three TS / "Iliad" cards that share a
Bear-Brothers (Marsmon / Callismon) + Three-Musketeers flavor. The combos below
are the cross-card interactions that the per-card tests cannot express.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT25-095 Paradise Colosseum (Option, Red/Green, cost 3) | engine / payoff-amp | In security: red/green [TS] get +2000 DP, and **+`<Rush>` while you have a [Marsmon] or [Callismon]**. [Main]: swap bottom security for self (face-up) + play a red/green [TS] Digimon for −3. |
| BT25-058 Callismon (Digimon, Lv.6, Green/Black, DP 13000) | payoff / control | `<Reboot>/<Blocker>/<Fortitude>`; OP/WD/WA suspend + unsuspend-lock; **[All Turns] when *effects* play/digivolve a Digimon → De-Digivolve 1 opp + may battle**. |
| BT25-082 BlackGatomon (Digimon, Lv.4, Purple/Black, DP 6000) | enabler | OP/WD free-play a [Three Musketeers]-text Tamer (if ≤1 Tamer); **[All Turns] while a TM-text Tamer is out → may digivolve into a TM-trait Digimon for cost 4, ignoring reqs**; inherited WA place-TM-source → Draw 1. |

## Digivolution lines

- **Callismon**: std Lv.5 Green / Cost 5; alt-source Lv.5 **[TS]** / Cost 4
  *ignoring requirements* (printed xros_req). Off the Bear-Brothers TS line.
- **BlackGatomon**: std Lv.3 Purple / Cost 3; alt-source Lv.3 w/[Three
  Musketeers] text **or** [TS] trait / Cost 2 (requirement-respecting);
  `[All Turns]` **into**-path: BlackGatomon → any TM-trait Digimon, cost 4,
  ignore reqs, gated on a TM-text Tamer on field.

## Named combos

### C1 — "Bear Brothers aura: Paradise Colosseum buffs Callismon (+2000 DP & Rush)"
- Cards: BT25-095 (in security, face-up), BT25-058 (on field).
- Expected mechanical outcome: with PC face-up in P0 security and Callismon on
  P0's field, Callismon (green, [TS]) gets **13000 → 15000 DP** and gains
  `<Rush>`. The `<Rush>` grant is self-satisfying: PC's keyword gate is
  "while you have a [Marsmon] **or** [Callismon]", and Callismon **is** the
  Callismon-named permanent, so Callismon's own presence opens its own Rush.
- Rules/keyword basis: PC `[Security][All Turns]` ChangeDPStaticEffect(+2000) +
  RushStaticEffect gated on `EqualsCardName("Marsmon"|"Callismon")`
  (`$BASE_DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_095.cs` regions
  "All Turns - Security DP" / "Rush"). `<Rush>` = attack the turn it enters
  (`general_rule.pdf` §16 keyword glossary). Security-scope aura materializes
  only from a **face-up** security source (`IsExistInSecurity(card, false)`;
  ST20-15 / BT24-090 idiom).
- Rank: HIGH (PC is the slice's marquee payoff; the Marsmon/Callismon gate is
  the printed reason the slice is named).

### C2 — "Paradise Colosseum Main → Callismon effect-play observer punishes"
- Cards: BT25-095 (in hand, [Main]), BT25-058 (on field), a red/green [TS]
  Digimon in hand (neutral fixture), an opponent Digimon with sources.
- Expected mechanical outcome: PC's [Main] plays a red/green [TS] Digimon **by
  effect** (cost −3). That **effect-initiated play** fires Callismon's
  `[All Turns]` observer → a De-Digivolve-1 prompt on an opponent Digimon (+ an
  optional battle). A *normal hand play* of the same Digimon does **not** fire
  it.
- Rules/keyword basis: Callismon `[All Turns]` CanUseCondition =
  `CanTriggerOnPermanentPlay && IsByEffect` (BT25_058.cs region "All Turns");
  PC Main plays via `SelectHandEffect.Mode.PlayForCost` = effect-initiated
  (BT25_095.cs region "Main Effect"); engine marks `effect_initiated: true` for
  `PlaySource::ByEffect` (`game_actions.rs::play_from_hand_with_cost` → fires
  play-event triggers). De-Digivolve stops at level 3 (`general_rule.pdf`
  De-Digivolve keyword).
- Rank: HIGH (the only inter-card *chain* in the slice — PC enables, Callismon
  punishes; both per-card tests fire these clauses in isolation only).

### C3 — "BlackGatomon Tamer-anchor unlocks its own [All Turns] into-path"
- Cards: BT25-082 (on field / freshly played), a [Three Musketeers]-text Tamer
  (neutral fixture), a TM-trait Digimon in hand (neutral fixture).
- Expected mechanical outcome: BlackGatomon's `[On Play]` free-plays the TM-text
  Tamer (≤1 Tamer gate). Once that Tamer is on the field, BlackGatomon's
  `[All Turns]` **into**-path condition ("while you have a TM-text Tamer") is
  satisfied, so the cost-4 ignore-reqs digivolve-into-a-TM-Digimon path is now
  legal. The clause *anchors itself*: the OP play is what unlocks the AT path.
  Control: without the TM Tamer on field, the into-path condition is unmet.
- Rules/keyword basis: BT25_082.cs region "Shared OP/WD"
  (AdditionalActivateCondition: TM-text Tamer in hand && battle-area Tamer ≤ 1,
  PlayForFree) + region "All Turns"
  (AddSelfDigivolutionRequirementStaticEffect cost 4, ignoreDigivolutionReq,
  condition: self-on-field && a TM-text Tamer permanent). Both reference the
  same `HasText("Three Musketeers")` Tamer predicate — the OP output feeds the
  AT gate.
- Rank: MEDIUM (keeps BlackGatomon in the slice; a true cross-clause dependency
  within one card, but no second *card's* effect fires).

## Playstyle
- Class: midrange / control-tempo. PC is a security-anchored aura engine + a
  ramp-y Main; Callismon is a top-end De-Digivolve/lock payoff; BlackGatomon is
  a Tamer-anchor enabler that thins into the TS / Three-Musketeers shell.
- Memory curve: PC Main is a swingy −3 play that hands the opponent tempo;
  Callismon's lock + De-Digivolve are designed to claw it back.

## Win conditions
- Stick Callismon (Reboot blocker, +2000 DP & Rush under PC) and grind the board
  with the per-effect-play De-Digivolve observer while the unsuspend-lock denies
  the opponent's defense.

## Ranked interactions to test
1. **C1** — PC security aura buffs the *real* Callismon (+2000 DP & self-gated
   Rush). HIGH: marquee payoff; cross-card identity the per-card test fakes with
   a synthetic Marsmon.
2. **C2** — PC [Main] effect-play fires Callismon's [All Turns] observer (with a
   normal-play control). HIGH: only inter-card chain in the slice.
3. **C3** — BlackGatomon OP Tamer-anchor unlocks its own [All Turns] into-path
   (with a no-Tamer control). MEDIUM.

### Interactions considered and NOT authored (logged, not silently dropped)
- **BlackGatomon → Callismon digivolve line**: dropped — Callismon's traits are
  Dark Animal / Iliad / **TS** (no "Three Musketeers"), and BlackGatomon is
  Lv.4; neither BlackGatomon's TM-trait into-path nor Callismon's Lv.5-TS
  alt-source connects the two. No legal in-slice line exists.
- **PC inherited [Security] free-play → Callismon observer**: dropped as
  redundant with C2 — the security-effect play is also effect-initiated, so it
  would fire the same Callismon observer; C2 already pins that path via the
  [Main] play and the normal-play control. (Would only add a duplicate assertion.)
- **Callismon OP/WD/WA suspend-lock interactions**: dropped — single-card
  behavior already fully covered by `bt25_058.rs` Section 3; no second slice card
  modifies or feeds it.
- **Cross-set closure**: none required. No authored combo fires a *cross-set*
  card's printed effect — all cross-set / cross-archetype pieces (TM-text Tamer,
  TM-trait Digimon, red/green TS Digimon, opponent Digimon) are **synthetic
  neutral fixtures**, so no cross-set card implementation was pulled in (lazy, no
  eager closure).
