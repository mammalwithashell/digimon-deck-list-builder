# ST-5 Machine Black — Model

System-level model of the ST-5 *Starter Deck Machine Black* (Greymon →
Machinedramon). Built by the `/archetype-interaction-test-author` scout. All 16
unique cards (ST5-01 … ST5-16) are implemented as DSL YAML
(`code/digimon-engine/cards/st5/`) and audited faithful (AUDITED-OK,
`qa/qa-reports/2026-05-29-starter-decks-st1-6-faithfulness-audit.md`).

Source priority: printed text (`cards/st5/<ID>.json`) → `general_rule.pdf` §16
/ §12 (canonical) → DCGO C# (`DCGO/Assets/Scripts/CardEffect/ST5/Black/`,
battle-tested). Deck quantities below are from `data/deck_library.json`
("Machine Black", single decklist: 50 deck cards + 4× Tai tamer = 54 entries).

## Card pool & roles

| Card | Qty | Role | One-line function |
|------|-----|------|-------------------|
| ST5-01 Kapurimon (Lv2 egg) | 4 | engine (DP) | Inherited: `[Your Turn]` while the bearer has `<Blocker>` it gets +1000 DP. (`ST5_01.cs` = `ChangeSelfDPStaticEffect 1000` gated on `IsOwnerTurn && HasBlocker`.) |
| ST5-02 Jazamon (Lv3) | 4 | body (vanilla) | 2c/4000 Data rookie; digivolution material, no effect. |
| ST5-03 Agumon (Lv3) | 4 | payoff (wall) | Printed `<Blocker>`. Primary cheap Blocker (cost 3, alt-digi 0 from Lv2). |
| ST5-04 ToyAgumon (Lv3) | 4 | engine (draw) | Inherited: `[End of Opp Turn]` if opponent didn't attack with a Digimon this turn, Draw 1. (`ST5_04.cs`: `AttackCount == 0` gate.) |
| ST5-05 Commandramon (Lv3) | 4 | body | 4c/5000 vanilla rookie; sturdy material. |
| ST5-06 Greymon (Lv4) | 4 | engine (draw) | Same inherited draw engine as ST5-04 (`[End of Opp Turn]` no-attack → Draw 1). Higher-level carrier of the same engine. |
| ST5-07 Jazardmon (Lv4) | 4 | body (vanilla) | 5c/6000 Champion; material into Ultimates. |
| ST5-08 DarkTyrannomon (Lv4) | 2 | payoff (wall) | Printed `<Blocker>` + `[When Attacking] Lose 2 memory` (drawback). Higher-DP Blocker. |
| ST5-09 MetalGreymon (Lv5) | 4 | enabler (Blocker grant) | `[When Digivolving]` 1 of your Digimon gains `<Blocker>` until end of opp's next turn. (`ST5_09.cs` `GainBlocker`.) |
| ST5-10 MetalTyrannomon (Lv5) | 4 | body (beater) | 6c/9000 vanilla Ultimate; large body. |
| ST5-11 Megadramon (Lv5) | 2 | payoff (wall) | Inherited `<Blocker>` (`ST5_11.cs` `BlockerSelfStaticEffect isInherited`). Grants Blocker to whatever it's stacked under. |
| ST5-12 Machinedramon (Lv6 Mega) | 2 | payoff (finisher) | `[When Digivolving]` up to 2 of your Digimon gain `<Reboot>` until end of opp's next turn. (`ST5_12.cs` `GainReboot`, max 2.) |
| ST5-13 BlitzGreymon (Lv6 Mega) | 2 | payoff (finisher) | `<Security A. +1>` + `[Main] <Digi-Burst 2>`: 1 of your Digimon +4000 DP until end of opp's next turn. (`ST5_13.cs`.) |
| ST5-14 Tai Kamiya (Tamer) | 4 | engine (untap loop) | `[Opponent's Turn]` when you use `<Blocker>` to suspend one of your Digimon, you may suspend this Tamer to unsuspend 1 of your Digimon. Security: play free. (`ST5_14.cs`, `OnTappedAnyone` + `IsBlock`.) |
| ST5-15 Laser Eye (Option) | 4 | tech (removal) | `[Main] <De-Digivolve 1>` 2 of opponent's Digimon. Security: same. (`ST5_15.cs` `IDegeneration`, up to 2 targets.) |
| ST5-16 Dark Side Attack (Option) | 2 | tech (removal) | `[Main]` Delete 1 of opponent's Digimon with play cost ≤ 7. Security: same. (`ST5_16.cs` `cost ≤ 7` gate.) |

## Digivolution lines

Mono-Black. Egg gates everything with the Kapurimon inherited Blocker-DP buff.

- **Greymon line (Blocker / removal core):**
  Kapurimon (Lv2 egg) → Agumon (Lv3, printed Blocker; alt-digi 0) → Greymon
  (Lv4, draw engine; cost 2 from Lv3) → MetalGreymon (Lv5, grants Blocker;
  cost 3 from Lv4) → **Machinedramon** (Lv6, Reboot; cost 3 from Lv5) or
  **BlitzGreymon** (Lv6, Digi-Burst +4000; cost 4 from Lv5).
- **Tyranno line (wall body):**
  Kapurimon → DarkTyrannomon (Lv4, printed Blocker; via Lv3) → MetalTyrannomon
  (Lv5 beater) → Machinedramon / BlitzGreymon.
- **Dragon line (Megadramon inherited Blocker):**
  Kapurimon → Jazamon (Lv3) → Jazardmon (Lv4) → Megadramon (Lv5, inherited
  Blocker) → Machinedramon / BlitzGreymon.
- Colour/level gates: all evolutions require `level_eq: N, color_is: black`;
  costs per `evo_costs` (e.g. Machinedramon alt-path cost 3 from any Lv5 Black,
  ST5-12 YAML `alt_paths`).

## Named combos

### Combo A — Blocker / Tai untap loop (the deck's signature)
- Cards: **ST5-14 Tai Kamiya** + any Blocker — **ST5-03 Agumon** (printed),
  ST5-08 DarkTyrannomon (printed), ST5-11 Megadramon (inherited), or a Digimon
  granted Blocker by ST5-09 MetalGreymon.
- Expected mechanical outcome: opponent attacks → owner blocks with the Blocker
  (the Blocker **suspends**, attack target switches to it, §12-1-7-1). Blocking
  suspends the blocker fires Tai's `[Opponent's Turn]` trigger; owner **may**
  suspend Tai (cost) to **unsuspend** that Digimon. The blocker is now ready
  again and, because it can suspend, is eligible to **block a second attack the
  same turn** (§12-1-4: a Digimon that can't suspend can't block). Net per cycle:
  one extra block per Tai (1 Tai → +1 block; up to 4 Tai → up to +4 re-readies).
  Board state asserted: blocker `suspended → unsuspended`, Tai
  `unsuspended → suspended`, and a second block is legal afterward.
- Rules/keyword basis: Blocker `general_rule.pdf` §16-4 (+ §12-1 Blocking,
  §12-1-7-1 "block declaration and suspends 1 of their Digimon", §12-1-4
  can't-suspend → can't block); Tai `ST5_14.cs` (`OnTappedAnyone` gated by
  `CanTriggerWhenPermanentSuspends` + `IsBlock` + `IsOpponentTurn`; activation
  `CanActivateSuspendCostEffect`; `Mode.UnTap`). Engine YAML `ST5-14.yaml`
  (`on_attack_target_change`, `attack_target_change_reason: blocker`,
  `activation_cost: suspend_self`, then `unsuspend`).
- Rank: **1** (Tai ×4 + Agumon ×4 = highest play frequency; central to the
  deck's control-wall identity; multi-card, opponent-turn timing — exactly the
  cross-card surface per-card TDD misses).

### Combo B — Kapurimon Blocker-DP synergy under Tai's wall
- Cards: **ST5-01 Kapurimon** (egg, inherited +1000 DP while bearer has Blocker)
  + a Blocker (ST5-03 / ST5-08 / ST5-11 inherited / ST5-09-granted) [+ ST5-14
  Tai to keep the wall ready].
- Expected mechanical outcome: a Digimon hatched from / stacked over Kapurimon
  that currently **has `<Blocker>`** gets **+1000 DP** during the **owner's
  turn** only. Combined with the wall, the blocker survives bigger attacks /
  trades up. Asserted: DP delta = +1000 iff (`your_turn` AND bearer has Blocker);
  delta disappears on opponent's turn (the buff is `[Your Turn]`-gated — a
  faithfulness edge), and disappears if the Blocker keyword is removed
  (e.g. after De-Digivolve strips ST5-11's inherited Blocker).
- Rules/keyword basis: `ST5_01.cs` (`ChangeSelfDPStaticEffect 1000`,
  `IsOwnerTurn && HasBlocker`); Blocker §16-4. Engine YAML `ST5-01.yaml`
  (`scope: inherited`, `active_when: {your_turn, has_keyword: Blocker}`,
  `dp_modifier: 1000`).
- Rank: **2** (Kapurimon ×4 — every line runs it; the `[Your Turn]`-only window
  and the keyword-conditional toggle are subtle, easy to mis-implement as
  always-on).

### Combo C — Machinedramon Reboot persistent wall
- Cards: **ST5-12 Machinedramon** + up to 2 of your Digimon (ideally Blockers /
  the Tai-loop wall).
- Expected mechanical outcome: on digivolving into Machinedramon, choose up to 2
  of your Digimon; each gains `<Reboot>` until end of opponent's next turn. On
  the opponent's unsuspend phase those Digimon **unsuspend** (mandatory,
  §16-10-4), so a Blocker that blocked / a beater that attacked is ready to
  block again on the opponent's turn — overlaps with Tai (Reboot re-readies for
  free; Tai re-readies on a per-block trigger). Asserted: both chosen Digimon
  carry the Reboot grant; after the opponent's unsuspend phase both are
  unsuspended; grant expires at end of opponent's next turn.
- Rules/keyword basis: Reboot §16-10 (persistent; unsuspend during opponent's
  unsuspend phase; mandatory 16-10-4; simultaneous with turn player's unsuspend
  16-10-5). `ST5_12.cs` (`GainReboot`, `max 2`, `EffectDuration.UntilOpponentTurnEnd`).
  Engine YAML `ST5-12.yaml` (`select_count_capped_multi max:2`, `grant_keyword
  Reboot expiry: end_of_opponents_turn`).
- Rank: **3** (Machinedramon ×2 — lower frequency, top-end finisher; the
  "up to 2" + expiry + opponent-unsuspend-phase timing is a real interaction,
  but it lands late and overlaps Combo A's payoff).

### (Lower-priority) Combo D — removal removing a Blocker source
- Cards: **ST5-15 Laser Eye** (De-Digivolve 1 ×2) on opponent, OR opponent's
  Laser Eye / De-Digivolve on **ST5-11 Megadramon**: De-Digivolving a stack that
  relies on Megadramon's *inherited* Blocker strips the Blocker, breaking the Tai
  loop and Kapurimon's +1000. Useful as the **unhappy path** for Combos A/B.
- Rules basis: De-Digivolve §16-11 (can't trash past Lv3); `ST5_15.cs`
  `IDegeneration`.
- Rank: **4** (tech / disruption; valuable as a negative-test rider on A/B, not
  a standalone combo).

## Playstyle

- **Class:** Black control wall. Not aggro — `[When Attacking] Lose 2 memory`
  on DarkTyrannomon and the whole Blocker package are defensive.
- **Tempo / memory curve:** sit behind cheap Blockers (Agumon cost 3, alt-digi
  free from egg), pass turns letting the opponent crash into walls. The ToyAgumon
  / Greymon `[End of Opp Turn]` draw engine **rewards a stalled opponent**
  (`AttackCount == 0` → Draw 1), so the deck profits from the opponent *not*
  attacking — a virtuous loop with the Blocker/Tai wall that discourages attacks.
- **Memory:** removal Options (Laser Eye 4, Dark Side Attack 5) and the Megas
  (Machinedramon alt-digi 3, BlitzGreymon alt-digi 4) are the memory sinks;
  Tai's loop costs no memory (it costs suspending Tai).

## Win conditions

- **Grind + removal:** out-card the opponent with the draw engine while Laser Eye
  (De-Digivolve) and Dark Side Attack (delete ≤7 cost) clear threats; eventually
  push through with MetalTyrannomon (9000) / the Megas.
- **BlitzGreymon close:** `<Security A. +1>` (checks 2 security) + Digi-Burst
  +4000 to swing in for a multi-security hit once the board is stable.
- **Machinedramon lock:** Reboot turns the wall into a persistent blocker that's
  ready on both turns, denying the opponent any safe attack window.

## Ranked interactions to test

1. **Combo A — Blocker + Tai untap loop** (rank 1): the deck's defining engine,
   opponent-turn triggered, multi-card, and exactly the kind of cross-card timing
   a per-card test can't express. Assert the block→suspend→Tai-trigger→
   unsuspend→second-block sequence (and the unhappy path: no Tai = no re-ready).
2. **Combo B — Kapurimon Blocker-DP synergy** (rank 2): high frequency (every
   line), subtle `[Your Turn]`-only + keyword-conditional toggle. Assert +1000
   only when (your turn ∧ has Blocker); 0 otherwise (opponent's turn, or Blocker
   stripped).
3. **Combo C — Machinedramon Reboot wall** (rank 3): late-game persistent-wall
   payoff; assert "up to 2" selection, grant present, unsuspend on opponent's
   unsuspend phase, and end-of-opponent's-next-turn expiry.

Dropped (logged, not selected for authoring): **Combo D** De-Digivolve-breaks-
Blocker is folded into Combo A/B as an unhappy-path rider rather than a standalone
test; pure vanilla bodies (ST5-02/05/07/10) and the two removal Options' raw
single-card behavior (ST5-15/16 on a lone target) are covered by per-card
behavioral tests, not interaction tests; ST5-13 Digi-Burst +4000 as a solo buff
is single-card (only becomes an interaction if paired with the wall to win a
specific battle — candidate rank-5 if a fourth test is wanted).
