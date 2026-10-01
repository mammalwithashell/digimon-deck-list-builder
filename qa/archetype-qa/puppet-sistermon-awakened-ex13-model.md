# Puppet (Sistermon Awakened DUALs) — EX13 slice — Model

Slice of the EX13 author-set run: EX13-065 Sistermon Blanc (Awakened),
EX13-066 Sistermon Noir (Awakened), EX13-035 KingEtemon. All three are
[Puppet] cards. No DCGO C# exists for any of them at the base submodule
(`b9a0638cd`), so the official Bandai DB bundles (`data/card_bundles/<ID>.md`)
+ `general_rule.pdf` govern. Interaction tests:
`code/digimon-engine/tests/archetypes/puppet_sistermon_ex13.rs`.

Two independent sub-systems share the slice: the **Sistermon DUAL line**
(white Option faces that also sit on the field as Digimon via `<Arts Digivolve>`)
and the **KingEtemon "Etemon party"** (a yellow Lv.6 that floods the board with
[Chuumon]/[Sukamon]/[Etemon] bodies and turns them into an opponent-wide DP aura).

## Card pool & roles
| Card | Role | One-line function |
|---|---|---|
| EX13-066 Sistermon Noir (Awakened) | payoff + engine | DUAL. Option [Main]: free-play a play cost <=4 [Sistermon] from hand/trash, then `<De-Digivolve 1>` per own Digimon; `<Arts Digivolve>` onto a Sistermon. Digimon: [WD] delete opp Digimon play cost <=4; `<Decode ([Sistermon Noir]/[Sistermon Ciel])>`. |
| EX13-065 Sistermon Blanc (Awakened) | enabler / protection | DUAL. Option [Main]: free-play a <=4 [Sistermon], then 1 opp Digimon -3000 DP per own Digimon. Digimon: `<Decode ([Sistermon Blanc])>`, `<Guard>`. IMPLEMENTED 2026-10-01 (G-NESTED-PARKED-REPLACEMENT RESOLVED; combo tests not yet authored). |
| EX13-035 KingEtemon | payoff | [OP][WD] free-play up to 2 [Chuumon]/[Sukamon]/[Etemon] Digimon (<=6 total cost; +6 by returning 10 such cards from trash). [All Turns] 3+ [Sukamon]/[Etemon] Digimon -> opp Digimon -3000 DP + `<Security A. -1>`. |
| BT6-082 Sistermon Blanc (cross-set) | Arts / alt-path base | Lv.3, play cost 3, [Huckmon] in its text -> Noir's "Lv.3 w/[Huckmon] in text: Cost 3" base; [On Play] `<Draw 1>`. |
| BT6-084 Sistermon Ciel (cross-set) | Option colour enabler / Arts base | White Lv.4; name route for Noir (cost 1). |
| BT23-077 Sistermon Ciel (cross-set) | Decode payload | Lv.4, also named [Sistermon Noir]; [On Play] delete opp Digimon play cost <=4. |
| EX13-027 Chuumon / EX13-028 Sukamon | KingEtemon bodies | play cost 3 Chuumon ([On Play] reveal-3 tutor) / Sukamon (<Blocker>, counts for the aura). |
| EX10-039 ChuuChuumon (cross-set) | KingEtemon body | play cost 4, [Chuumon] in its name — only fits beside a Sukamon under the raised 12 maximum. |

## Digivolution lines
- Noir (Lv.4, no colour circles): [Sistermon Noir]/[Sistermon Ciel] name -> cost 1;
  Lv.3 with [Huckmon] in text (BT6-082, EX13-065) -> cost 3; or free via its own
  Option's `<Arts Digivolve>` (general_rule.pdf `<Arts Digivolve>` keyword; engine
  fix G-ENGINE-ARTS-ALT-PATH-BASE lets the alt-path bases qualify).
- Blanc (Awakened) (Lv.3): [Sistermon Blanc] cost 0; Lv.2 w/[Huckmon] in text cost 1.
- KingEtemon (Lv.6): Yellow Lv.5 / 5, Black Lv.5 / 5, Lv.5 [Sukamon]/[Etemon]-named / 4.
  No [Etemon]/KingSukamon Lv.5 is implemented, so tests use the real vanilla-ish
  yellow Lv.5 ST3-08 MagnaAngemon as the colour-circle base.

## Named combos
### C1 — Noir Option -> De-Digivolve -> Arts -> [WD] delete
- Cards: EX13-066 (Option) + BT6-082 (trash) + BT6-084 (field, colour enabler).
- Expected: Option plays BT6-082 from trash free (its [On Play] draws 1); with 2
  own Digimon the opp Lv.5/4/3 stack is De-Digivolved 2 to its Lv.3 (play cost 3);
  `<Arts Digivolve>` puts Noir onto BT6-082 free (Huckmon-text Lv.3 base) instead
  of trashing; Noir's [WD] deletes the now-play-cost-3 opponent Digimon that was
  play cost 6 when the Option was used. Noir never reaches the trash.
- Basis: printed text (bundle EX13-066, BT6-082); general_rule.pdf `<De-Digivolve>`
  / `<Arts Digivolve>` keywords; [WD] reads the target's current top card.
- Rank: 1 (signature line of the deck: removal that the Option itself enables).

### C1' — unhappy: skip the free play and the [WD] whiffs
- Cards: EX13-066 (Option) + BT6-084 (field) only; the optional free play is declined.
- Expected: with 1 own Digimon the opp stack is De-Digivolved only 1 (to its
  play cost 5 Lv.4); Arts onto BT6-084 (name route) still happens, but Noir's
  [WD] has no play cost <=4 target, so the opponent keeps its Digimon. The
  free-played body is what makes the removal land (De-Digivolve N counts Digimon
  at resolution, after the play).

### C2 — Noir on Ciel -> [WD] delete -> Decode replays BT23-077 -> [On Play] delete
- Cards: EX13-066 (Digimon face, from hand) + BT23-077.
- Expected: digivolve onto BT23-077 for 1 (name route — BT23-077 is also
  [Sistermon Noir]); [WD] deletes one opp play cost <=4 Digimon. When the
  opponent's effect then deletes Noir, `<Decode>` plays BT23-077 from its sources
  free; BT23-077's [On Play] deletes a second opp play cost <=4 Digimon. A play
  cost 5 opp Digimon survives both.
- Basis: general_rule.pdf `<Decode>` (non-battle leave replacement; a played card
  triggers [On Play]); bundle BT23-077.
- Rank: 2.

### C3 — KingEtemon [WD] party -> aura online
- Cards: EX13-035 + 2x EX13-028 (trash) on ST3-08.
- Expected: digivolve for 5; [WD] plays both Sukamon from trash (3+3 = 6) ->
  3 [Sukamon]/[Etemon] Digimon (KingEtemon counts itself) -> every opp Digimon
  -3000 DP and `<Security A. -1>`.
- Rank: 3.

### C3' — unhappy: Chuumon is not a counter
- EX13-027 + EX13-028 instead: Chuumon's [On Play] tutor fires off the free play,
  but the aura stays off (KingEtemon + 1 Sukamon = 2).

### C4 — Return-10 widens the budget (ChuuChuumon + Sukamon = 7)
- Cards: EX13-035 (played) + EX10-039 (hand) + 11x EX13-028 (trash).
- Expected (official Q&A order: return choice first): returning 10 raises the
  maximum to 12, so ChuuChuumon (4) and a Sukamon (3) both land; declining keeps
  6 and the second pick excludes the Sukamon (4+3 > 6).
- Rank: 4.

## Playstyle
- Sistermon: white/black midrange-control; Options are tempo removal that turn
  into bodies (Arts), Decode gives resilience. KingEtemon: yellow go-wide finisher.

## Win conditions
- Sistermon: remove the opponent's board with De-Digivolve + play-cost deletes,
  then swing with resilient Decode bodies.
- KingEtemon: a 13000 DP Mega plus two free bodies, with an aura that shrinks
  every opp Digimon by 3000 and blunts their security checks.

## Ranked interactions to test
1. C1 / C1' — Option + Arts + [WD] (the only way the deck converts a big opp Digimon into a deletable one).
2. C2 — Decode chaining a second [On Play] removal.
3. C3 / C3' — KingEtemon free-play feeding its own aura threshold.
4. C4 — return-10 budget.

## Blocked / dropped
- Unblocked 2026-10-01 (EX13-065 IMPLEMENTED; G-NESTED-PARKED-REPLACEMENT RESOLVED) — still to author: "Blanc (Awakened) Option ->
  -3000 x N DP", "Blanc (Awakened) <Guard> + <Decode> recursion",
  "Blanc (Awakened) Lv.3 -> Noir (Huckmon text, cost 3)".
- Dropped (low value / covered per-card): BT20-084's [Trash] free-digivolve onto
  Noir (name semantics of "[Sistermon Ciel]s" vs Noir's also-treated-as name are
  ambiguous without DCGO); KingEtemon aura counting opp Sukamon (per-card
  `ex13_035_aura_counts_digimon_on_both_fields`).
