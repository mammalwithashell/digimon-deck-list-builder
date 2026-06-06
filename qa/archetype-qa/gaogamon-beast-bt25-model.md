# Gaogamon / Beast (BT25 slice) — Model

Scope: the BT25 [Beast]-family slice handed to the interaction-test author —
**BT25-008, BT25-009, BT25-021, BT25-048, BT25-012, BT25-013, BT25-023,
BT25-051**. These eight split into two co-resident beast sub-engines that share
the BT25 set and the [Beast] trait but use **different** trait gates:

- **Red/off-colour [Iliad]/[TS] "Three Sovereigns" beast line** — BT25-008
  Coronamon, BT25-009 Bearmon (Red), BT25-048 Bearmon (Green), BT25-012
  Grizzlymon (Red/Green), BT25-013 Firamon (Red), BT25-051 Grizzlymon
  (Green/Yellow). The engine: free/discounted digivolution gated on the **[TS]**
  trait, a **+3000 DP / Raid** combat-pump payoff, and Iliad-recursion. This is
  the same "Beastkin / Three Sovereigns" family as
  `flaremon-beastkin-model.md` (BT25-016/017/024/054) — this slice is the
  Lv.3 enablers + the Red/Green Champion payoffs that feed it.
- **Blue [Beast]/[DATA SQUAD] "Gaogamon" line** — BT25-021 Gaomon, BT25-023
  Gaogamon. The engine: a reveal-3 search that digs the Thomas H. Norstein
  tamer + a Gaogamon-named card, a free tamer-play ramp, and an inherited
  both-players-draw on attack.

Sources cited inline: card images / `cards.json` printed text and the per-card
DCGO crosschecks embedded in each `code/digimon-engine/cards/bt25/BT25-0xx.yaml`
header (DCGO C# at `$BASE_DCGO/Assets/Scripts/CardEffect/BT25/<Colour>/BT25_0xx.cs`).
`general_rule.pdf` §16 governs digivolution / Raid / cost timing.

## Card pool & roles

| Card | Name | Lv | Colour | Role | One-line function |
|------|------|----|--------|------|-------------------|
| BT25-008 | Coronamon | 3 | Red | enabler/engine | [OnPlay/WhenMoving] trash up to 2 [Iliad]/[TS] → draw 1 each; TS Lv.2 cost-0 alt-path. |
| BT25-009 | Bearmon | 3 | Red | engine | [SOMP] if ≤4 memory, **free-digivolve self** into a [Beast]/[Animal]/[Sovereign] (not [Sea Animal]) or [TS] Digimon from hand. |
| BT25-048 | Bearmon | 3 | Green | enabler | [Your Turn] **−1 cost** when THIS digivolves into a [TS] Digimon; inherited draw-on-win. |
| BT25-012 | Grizzlymon | 4 | Red/Green | payoff | [OnPlay/WhenDigivolving] grant a beast ally **Raid + 3000 DP for the turn**. |
| BT25-013 | Firamon | 4 | Red | engine | [OnPlay/WhenDigivolving] trash 1 → recover a red/blue [Iliad] from trash; [Your Turn] on a **blue** play/digivolve, free(-1)-digivolve into [Flaremon]. |
| BT25-051 | Grizzlymon | 4 | Green/Yellow | payoff/tech | <Blocker>; [OnPlay/WhenDigivolving] a beast ally **+3000 DP until opp turn ends**; inherited draw-on-win. |
| BT25-021 | Gaomon | 3 | Blue | enabler | [OnPlay] reveal 3 → add 1 [Thomas H. Norstein]/[DATA SQUAD] + 1 [Gaogamon]-named; inherited both-players-draw on attack. |
| BT25-023 | Gaogamon | 4 | Blue | engine/ramp | [OnPlay/WhenDigivolving] if ≤1 Tamers, **free-play a Thomas H. Norstein** from hand; inherited both-players-draw on attack. |

## Digivolution lines

- **Red TS line**: (Lv.2 [TS] base) → **BT25-008 Coronamon** / **BT25-009
  Bearmon** (Lv.3, TS Lv.2 cost-0 alt-path) → **BT25-012 Grizzlymon** /
  **BT25-013 Firamon** (Lv.4, TS Lv.3 cost-2 alt-path; 012 `ignore_requirements`).
  Off-colour mirrors: **BT25-048 Bearmon** (Green Lv.3) and **BT25-051
  Grizzlymon** (Green/Yellow Lv.4).
- **Blue DATA SQUAD line**: (Wanyamon / Lv.2 DATA SQUAD) → **BT25-021 Gaomon**
  (Lv.3) → **BT25-023 Gaogamon** (Lv.4, DATA SQUAD Lv.3 cost-2 alt-path).

The two lines' cost gates differ: the Red/Green line discounts on the **[TS]**
trait (BT25-048's −1, the alt-paths' cost-0/2), the Blue line on **[DATA
SQUAD]**.

## Named combos

### C1 — Bearmon SOMP free-digivolve into the Grizzlymon payoff (Red TS engine)
- Cards: **BT25-009 Bearmon** + **BT25-012 Grizzlymon** (the real payoff as the
  hand evolution target).
- Expected outcome: at [Start of Your Main Phase] with ≤4 memory, Bearmon
  free-digivolves (memory unchanged) into Grizzlymon from hand; the digivolve is
  a `WhenDigivolving` event, so Grizzlymon's grant-clause **immediately installs
  a Raid+3000 target selection** — one trigger (SOMP) chains into the payoff's
  trigger. Stack grows to ≥2 (Bearmon buried under Grizzlymon).
- Rules/keyword basis: SOMP optional ("may"), free digivolve §16; Grizzlymon
  `when:[on_play, when_digivolving]`. DCGO BT25_009.cs OnStartMainPhase →
  DigivolveIntoHandOrTrashCard; BT25_012.cs OnEnter/WhenDigivolving GainRaid+DP.
- Rank: HIGH — the core engine→payoff bridge of the Red line.

### C2 — Bearmon SOMP gating: >4 memory or no eligible hand card is a no-op
- Cards: **BT25-009 Bearmon** (+ a non-eligible / eligible hand target).
- Expected outcome: with 5 memory the SOMP never installs a prompt; with ≤4
  memory but only a [Sea Animal] in hand it is a no-op (excluded). System fact:
  the engine only fires when *both* the memory gate and a legal hand target hold.
- Basis: `active_when: {your_turn, memory_lte:4}`; filter excludes [Sea Animal].
- Rank: MED — the unhappy path that proves the C1 gate.

### C3 — BT25-048 Bearmon's [Your Turn] −1 cost discounts a digivolve into a [TS] result
- Cards: **BT25-048 Bearmon** (the cost-reducer, as digivolve **base**) + a
  [TS]-trait Lv.4 hand result.
- Expected outcome: digivolving the Bearmon-topped stack into a [TS] Digimon
  spends **one less memory** than the printed evo cost; digivolving it into a
  **non-[TS]** result spends full cost (the gate is the *result's* [TS] trait,
  and `source_is_cost_target_permanent` ties the reduction to THIS base).
- Basis: `cost_reduction … when_any_ally_digivolves_into:{trait_has:TS},
  condition:{source_is_cost_target_permanent}`; DCGO BT25_048.cs
  ChangeDigivolutionCostStaticEffect(-1). (Printed text omits DCGO's green gate;
  YAML follows printed text per source-priority.)
- Rank: HIGH — the discount engine; cross-clause cost interaction per-card
  tests pin structurally but not as a live memory swing through a real digivolve.

### C4 — Grizzlymon Raid+3000 pump turns a sub-threshold beast into a Raid attacker
- Cards: **BT25-012 Grizzlymon** + a beast ally on field.
- Expected outcome: the [OnPlay] grant gives the chosen beast ally **Raid** and
  **+3000 DP for the turn**; the +3000 expires at end of turn. (Pairs with the
  Beastkin model's 13000-DP observers — the pump is how a 10000-DP beast crosses
  thresholds.) Mandatory select: if any eligible beast exists you MUST pick one.
- Basis: `grant_keyword Raid end_of_turn + add_dp_modifier 3000 end_of_turn`;
  DCGO BT25_012.cs. Raid §16-… (switch attack target to highest-DP unsuspended).
- Rank: MED — payoff is per-card-pinned; the cross-card value is the +DP window,
  asserted as a net DP swing + expiry.

### C5 — BT25-051 Grizzlymon: Blocker + cross-turn +3000 DP defensive wall
- Cards: **BT25-051 Grizzlymon** + a beast ally.
- Expected outcome: 051 enters with <Blocker>; its grant gives a beast ally
  **+3000 DP until the opponent's turn ends** (survives into the opponent's turn,
  unlike 012's end-of-turn pump). System fact: the +3000 persists across the
  turn boundary — the defensive mirror of C4's offensive pump.
- Basis: `add_dp_modifier 3000 expiry: end_of_opponents_turn`; self `Blocker`;
  DCGO BT25_051.cs ChangeDigimonDP UntilOpponentTurnEnd.
- Rank: MED — distinguishes the two Grizzlymon printings by expiry window.

### C6 — Firamon's [Your Turn] blue-play observer chains a blue entry into a free-1 Flaremon
- Cards: **BT25-013 Firamon** + a **blue** Digimon entering.
- Expected outcome: playing/digivolving a **blue** Digimon on your turn (with a
  [Flaremon] in hand) arms Firamon's [Your Turn] clause to digivolve **into
  Flaremon, cost −1**; a **non-blue** entry does NOT arm it. Cross-card: one
  card's entry fires another's digivolve observer. (Mirrors the Beastkin model's
  F2 from the Flaremon side; here Firamon is the observer.)
- Basis: `when:[on_enter_field_anyone,on_digivolve], condition:{event_target_owner:you,
  event_card_color_has:[blue]}`; DCGO BT25_013.cs OnEnterFieldAnyone blue gate.
- Rank: HIGH — the Firamon ramp engine; genuinely cross-card.

### C7 — Firamon's [OnPlay] trash-1 recovers a red/blue [Iliad] from trash
- Cards: **BT25-013 Firamon** + a red/blue [Iliad] Digimon seeded in trash.
- Expected outcome: on play, by trashing 1 hand card you may return a red or
  blue [Iliad] Digimon from trash to hand — hand net 0 (trash 1, gain 1), trash
  net 0 (gain the cost card, lose the recovered card). A **non-Iliad** or a
  green/other-colour trash card is **not** a legal recovery target.
- Basis: clause-0 `select_hand cost:true → trash → select_trash{Iliad, red|blue}
  → add_to_hand_from_trash`; DCGO BT25_013.cs SelectHand Discard → SelectCard
  Trash AddHand (Iliad red||blue).
- Rank: MED — recursion engine; an unhappy-path colour/trait gate worth pinning.

### C8 — Gaomon reveal-3 digs the Thomas tamer AND the Gaogamon payoff
- Cards: **BT25-021 Gaomon** (+ a [DATA SQUAD] card, a [Gaogamon]-named card, a
  plain card in deck).
- Expected outcome: [OnPlay] reveals top 3, adds **one** [Thomas H.
  Norstein]/[DATA SQUAD] card **and one** [Gaogamon]-named card to hand, bottoms
  the rest — the two-bucket search that assembles the Blue line's next play. The
  plain card is NOT added. (Per-card test pins this; the interaction angle is
  feeding C9.)
- Basis: `reveal_top_deck 3 → select_reveal_buckets(2) → add ×2 → bottom rest`;
  DCGO BT25_021.cs SimplifiedRevealDeckTopCardsAndSelect.
- Rank: MED — already per-card-covered; included as the setup half of C9.

### C9 — Gaogamon free-plays Thomas H. Norstein when Tamer-light
- Cards: **BT25-023 Gaogamon** + a synthetic **Thomas H. Norstein** Tamer in
  hand.
- Expected outcome: [OnPlay/WhenDigivolving], **if ≤1 Tamers**, may free-play a
  Thomas H. Norstein from hand (Tamer count rises, **no memory paid**); with the
  Tamer gate already ≥2 on field the clause does NOT offer. System fact: the
  ramp is gated on the controller's own Tamer count, not on hand contents.
- Basis: `active_when:{count_lte:{tamer, owner:you}, n:1}, optional:true →
  select_hand(name "Thomas H. Norstein") → play_from_hand_free`; DCGO
  BT25_023.cs PlayForFree gated on Tamer count ≤1. (Thomas's own printed effect
  is NOT fired by this clause → synthetic Tamer suffices; no cross-set pull.)
- Rank: HIGH — the Blue line's signature ramp.

## Playstyle

Midrange beast tempo. The Red/TS line free-/cheap-digivolves up the curve
(BT25-009 SOMP, BT25-048 −1, the TS alt-paths) and converts the tempo into
combat pressure via the Grizzlymon pumps (Raid+3000 / +3000-til-opp-end) and
Firamon's blue-fed Flaremon ramp. The Blue line is a draw-and-search shell
(Gaomon dig, both-players-draw, Thomas ramp) that smooths the curve. Memory
curve sits low (the SOMP gate wants ≤4 memory; free plays don't spend).

## Win conditions

- Beast beatdown: stack Raid + DP pumps onto a single attacker to punch through
  blockers/security; the inherited draw-on-win (BT25-048/051) and Gaogamon
  both-players-draw refuel the hand for the next pump.
- Tempo snowball: free digivolves + free Thomas plays keep memory advantage,
  ending games before the opponent stabilises.

## Ranked interactions to test (selected)

1. **C1** — Bearmon SOMP → Grizzlymon payoff trigger chain (engine→payoff bridge).
2. **C3** — BT25-048 −1 cost on a [TS]-result digivolve (live memory swing).
3. **C6** — Firamon blue-play observer → free-1 Flaremon (cross-card entry → digivolve).
4. **C9** — Gaogamon free-plays Thomas when Tamer-light (Blue-line ramp gate).
5. **C4/C5** — Grizzlymon pumps (Raid+3000 end-of-turn vs +3000-til-opp-end), distinguished by expiry.
6. **C7** — Firamon trash-1 → recover red/blue [Iliad] (recursion gate).
7. **C2** — Bearmon SOMP negative gating (memory / eligibility).
8. **C8** — Gaomon two-bucket dig (already per-card-covered; setup for C9).

Dropped from authoring (logged, not silently truncated): **C8** (fully covered
by `cards_behavioral/bt25/bt25_021.rs` — no cross-card surface beyond feeding
C9, which is authored directly). The both-players-draw inherited on BT25-021/023
is per-card-covered and adds no cross-card interaction, so it is not given its
own interaction test.
