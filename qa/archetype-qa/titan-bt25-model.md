# Titan (BT25 slice) — Model

Capstone archetype model for the **Titan / [TS]** slice of BT25, authored by
`/archetype-interaction-test-author`. Slice cards (the eight named in the
request):

| Card | Name | Status | Why |
|------|------|--------|-----|
| BT25-006 | Dorimon | **IMPLEMENTED** | inherited opp-turn unsuspend engine |
| BT25-068 | Deltamon | **IMPLEMENTED** | on-suspend De-Digivolve payoff |
| BT25-071 | Orochimon | **IMPLEMENTED** | lockdown + on-suspend reveal-play ramp |
| BT25-019 | UltimateBrachiomon | **IMPLEMENTED** | OP/WD highest-DP removal, Reboot/Blocker |
| BT25-069 | Raremon | **BLOCKED (dsl)** | trash→link verb missing (`qa/dsl-vocab-gaps.md`) |
| BT25-080 | Witchmon | **BLOCKED (engine)** | `OnDiscardHand` trigger + `played_by_effect` (`docs/RUST_ENGINE_GAPS.md`) |
| BT25-073 | Dragomon | **BLOCKED (hybrid)** | link-trash-as-cost + would-leave replacement |
| BT25-083 | LadyDevimon | **BLOCKED (hybrid)** | bottom-source picker + trash-option-as-cost |

Per-card verdicts read-only from `qa/qa-reports/validated_cards_dsl.json`
(report `batch-implement-cards-rust-dsl`, 2026-06-06). **Four of the eight slice
cards are BLOCKED**, so any combo naming a BLOCKED card is dropped at Phase-4
precondition gating (logged in §"Ranked interactions" below) — this skill never
authors a test that cannot pass.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT25-006 Dorimon | engine (inherited) | [Opp Turn][OPT] trash 1 hand card → **unsuspend** 1 of your [Titan] Digimon when an opponent Digimon attacks |
| BT25-068 Deltamon | payoff / engine | `<Collision>`; [All Turns][OPT] **on self-suspend** → De-Digivolve 1 opp Digimon; inherited +1000 DP |
| BT25-071 Orochimon | ramp / control | [On Play/When Digivolving] lock 1 opp Digimon/Tamer; [All Turns][OPT] **on self-suspend** reveal 3, play a cost≤4 [TS] free, bottom the rest |
| BT25-019 UltimateBrachiomon | apex payoff | `<Reboot>`+`<Blocker>`; [On Play/When Digivolving] delete 1 highest-DP opp Digimon; EoT memory-gated effect immunity |

The slice is a **suspend-engine deck**: several payoffs trigger *when this
Digimon suspends* (Deltamon's De-Digivolve, Orochimon's reveal-play ramp), and
Dorimon is the *re-arm* engine — it **unsuspends** a Titan on the opponent's
turn so it can be suspended again (block) and re-fire its on-suspend payoff.
`<Reboot>` (Brachiomon) is the same theme at the apex: it unsuspends on the
opponent's turn for free, so it can `<Blocker>`-block every turn.

## Digivolution lines

All BT25 Titan Lv.4+ digivolve along their printed Black/Purple line **or** via
an alt-path gated on a **[TS]-trait** base (cost-reduced):

- Lv.3 [TS] → **Deltamon** (Lv.4, cost 2 alt) → … → **Orochimon** (Lv.5, Lv.4
  [TS] cost 3 alt).
- Lv.5 [TS]/Dinosaur → **UltimateBrachiomon** (Lv.6, cost 4 alt).
- Dorimon (Lv.2) is a **digivolution source** (inherited engine), not a payoff —
  it sits under any Titan and supplies the opp-turn unsuspend.

(Interaction tests use synthetic Lv.3/Lv.4/Lv.5 stack bases for these
prerequisites — the cross-set lower-line cards are not needed to fire the
implemented effects, so they are not pulled, per the lazy-closure rule.)

## Named combos

### T1 — Dorimon unsuspends a Titan payoff (trait gate spanning two cards)
- Cards: **BT25-006 Dorimon** (engine), **BT25-068 Deltamon** (target).
- Expected mechanical outcome: on the opponent's turn, when an opponent Digimon
  attacks, Dorimon's inherited clause trashes 1 hand card and **unsuspends
  Deltamon** — because Deltamon carries the **[Titan]** trait it is a legal
  target. A non-Titan ally is *not* a legal unsuspend target (the gate is
  cross-card: Dorimon names the trait, Deltamon supplies it).
- Rules/keyword basis: unsuspend = "set to active" (`general_rule.pdf` §10
  suspend/unsuspend states); [Opponent's Turn]/[Once Per Turn] timing.
  DCGO `BT25_006.cs` (`SelectPermanent UnTap`, own-[Titan] gate, opp-turn).
- Rank: **high** (Dorimon is the slice's defining engine; the unsuspend is the
  setup for every on-suspend payoff).

### T2 — Dorimon re-arms Deltamon's De-Digivolve across the opponent's turn
- Cards: **BT25-006 Dorimon**, **BT25-068 Deltamon**.
- Expected mechanical outcome: Deltamon's on-suspend De-Digivolve is
  [Once Per Turn] **per turn**. It fires on P0's turn (suspend #1, removes 1
  opp source), then on the *opponent's* turn Deltamon — unsuspended by Dorimon —
  is re-suspended and the De-Digivolve fires **again** (a second opp source
  removed), because the OPT counter resets at the turn boundary. The system fact
  a per-card test misses: the *count* of De-Digivolves across a turn cycle is
  governed by how many times the suspend-engine can re-arm, which is a
  two-card property (Dorimon's unsuspend + the turn boundary).
- Rules/keyword basis: [Once Per Turn] resets each turn (`general_rule.pdf`
  §OPT); De-Digivolve "trash the top card, stop at Lv.3" — DCGO `BT25_068.cs`
  `Mode.Degenerate`, `CanTriggerWhenSelfPermanentSuspends`.
- Rank: **high** (the marquee Titan grind engine).

### T3 — Orochimon on-suspend reveal-play chains into a [When Digivolving] payoff
- Cards: **BT25-071 Orochimon** (ramp), a cost≤4 [TS] Digimon **with its own
  on-play/when-digivolving payoff** played from the reveal.
- Expected mechanical outcome: when Orochimon suspends it reveals 3, and the
  player **plays a cost≤4 [TS] Digimon for free**; that free play is itself a
  *play event*, so the played card's own `[On Play]` payoff fires off
  Orochimon's clause — a free-tempo card doubling as a trigger platform.
- Rules/keyword basis: a play from an effect is still a play and fires `[On
  Play]` (`general_rule.pdf` play timing); DCGO `BT25_071.cs`
  `SimplifiedRevealDeckTopCardsAndSelect`, `play payCost:false`.
- Rank: **medium-high** (the ramp engine's hidden value).
- NOTE: the implemented Titan reveal-targets in this slice (Deltamon, Witchmon)
  are cost ≥4/blocked; rather than pull a cross-set TS payoff *just* to fire a
  second card's effect, T3 uses a **synthetic cost-4 [TS] Digimon with no own
  trigger** and asserts the *free play itself* (the Orochimon-owned mechanical
  outcome). The "chains into another payoff" claim is therefore asserted at the
  play-event level (field +1, no cost paid) — the cross-card fire is structural,
  not a second printed effect, so no extra card is pulled (lazy closure honored).

### T4 — UltimateBrachiomon apex removal lands off the suspend engine's tempo
- Cards: **BT25-019 UltimateBrachiomon**, **BT25-068 Deltamon** (board context).
- Expected mechanical outcome: Brachiomon's [On Play/When Digivolving] deletes
  the opponent's **highest-DP** Digimon. With Deltamon's De-Digivolve having
  *shrunk* an opponent's stack (lowering its level but not its printed DP), the
  highest-DP gate still resolves against the *current* DP board — the removal
  picks the genuine top-DP target, independent of the De-Digivolve. This pins
  that the two payoffs compose without interfering (De-Digivolve changes
  *level/sources*, Brachiomon's gate reads *DP*).
- Rules/keyword basis: highest-DP selection gate (`dp_gte aggregate
  highest_dp`); deletion (`general_rule.pdf` deletion). DCGO `BT25_019.cs`
  `IsMaxDP` Destroy.
- Rank: **medium** (apex payoff; composition check).

### (dropped) Witchmon hand-trash removal loop — **BLOCKED on BT25-080**
- Would pair Witchmon's inherited "when your hand is trashed, delete a Lv.4≤ opp"
  with Dorimon's "by trashing 1 hand card" cost. **Dropped**: BT25-080 is BLOCKED
  (engine gap `OnDiscardHand`). Routed to backlog; not authored.

### (dropped) Raremon trash-link recursion — **BLOCKED on BT25-069**
- Would pair Raremon's "link a [TS] card from trash" with Dragomon's link-cost.
  **Dropped**: BT25-069 and BT25-073 are both BLOCKED. Not authored.

### (dropped) LadyDevimon Three-Musketeers bottom-source draw — **BLOCKED on BT25-083**
- Out of the Titan suspend-engine theme and BLOCKED. Not authored.

## Playstyle
- Class: **midrange suspend-engine / grind control**. Uses Dorimon + `<Reboot>`
  to act on both players' turns; De-Digivolve + highest-DP removal to attrition
  the opponent's board; Orochimon to ramp the next threat for free.
- Tempo: builds Lv.4–5 Titans, then leverages opp-turn unsuspend/blocker windows.
- Memory curve: alt-path cost reductions ([TS] base) keep the Lv.4/5/6 climb
  cheap; Brachiomon's EoT immunity protects the apex on the swing-back turn.

## Win conditions
- Grind the opponent out of board (repeat De-Digivolve + highest-DP deletion),
  then close with a protected `<Reboot>`/`<Blocker>` UltimateBrachiomon that
  attacks every turn while ignoring opponent effects under the memory gate.

## Ranked interactions to test
1. **T1** Dorimon unsuspends a Titan payoff (trait gate spans two cards) — high.
2. **T2** Dorimon re-arms Deltamon's De-Digivolve across the turn boundary — high.
3. **T3** Orochimon on-suspend reveal-play is a free-play trigger platform — med-high.
4. **T4** Brachiomon highest-DP removal composes with De-Digivolve tempo — medium.

Dropped (BLOCKED-card-gated, logged above): Witchmon hand-trash loop (BT25-080),
Raremon/Dragomon trash-link recursion (BT25-069/073), LadyDevimon TM draw
(BT25-083).
