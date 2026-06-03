# Omni Nokia — Model

System-level model of the **Omni Nokia** archetype (canonical name resolved by
`python code/tools/resolve_deck.py "Omni Nokia" --json`; 25 decklists, 49 unique
cards in `qa/archetype-qa/omni-nokia/deck_pool.json`). This is the durable
archetype-model artifact for `/archetype-interaction-test-author` (Phases 0–3)
and the traceability anchor for `code/digimon-engine/tests/archetypes/omni_nokia.rs`.

Sources are cited inline: printed text from `data/cards.json`
(`effect_description_eng`), DCGO C# at
`$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`
(underscores, e.g. `BT17_095.cs`), and `general_rule.pdf` rule numbers (keyword
semantics §16). DCGO + `general_rule.pdf` outrank the API-ingested JSON
(CLAUDE.md source priority). Implementation status is read from
`qa/qa-reports/validated_cards_dsl.json` (46/49 pool cards implemented;
BT1-010, EX1-066, ST21-10 not yet in the DSL pool — see static coverage gate).

> Naming: this is the **system model**. The per-card faithfulness doc
> `qa/archetype-qa/DNA_Omnimon.md` is a separate, lower-level artifact and is
> *not* the model the reviewer audits.

---

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| **BT17-095 Miraculous Mega Knight** (Option, freq 25) | **engine / payoff-enabler** | `[Main]` free-play 1 Agumon/Gabumon from hand **or trash** without paying the cost, then place self in the battle area as a Delay; `[All Turns]` when an own Lv6 Greymon/Garurumon-named Digimon would leave the battle area outside a battle, `<Delay>`-trash to DNA-digivolve it + a hand card into an Omnimon-named hand card. |
| **BT17-081 Tai Kamiya & Matt Ishida** (Tamer, freq 22) | **engine (memory)** | `[All Turns]` when one of your Digimon is played or digivolves, by suspending this Tamer: +1 memory if you have a Greymon-named Digimon, +1 if a Garurumon-named; `[EoT][OPT]` an Omnimon-named Digimon may attack a player. |
| **EX9-066 Tai Kamiya & Matt Ishida** (Tamer, freq 18) | **engine (memory) / recursion** | `[On Play]` return a Greymon/Garurumon/Omnimon from trash to hand else `<Draw 1>`; `[All Turns]` suspend-for-memory on play/digivolve (Greymon +1, then Garurumon +1). |
| **BT22-084 Nokia Shiramine** (Tamer, freq 25) | **engine / enabler** | `[Start of Your Turn]` if ≤2 memory, set to 3; `[Start of Your Main Phase][On Play]` if ≤1 Digimon, free-play 1 Agumon/Gabumon from hand without paying the cost; `[All Turns]` **+1000 DP** to all your Greymon/Garurumon/Omnimon-named Digimon. |
| **BT5-092 Nokia Shiramine** (Tamer, freq 10) | **enabler / cost reduction** | `[On Play]` free-play Agumon/Gabumon; `[Your Turn]` suspend → reduce a digivolution into a hand Greymon/Garurumon/Omnimon by 1. |
| **BT22-013 WarGreymon** (Lv6 Red, freq 25) | **payoff / cross-line** | `[Hand][Main]` (Nokia gate) an Agumon digivolves into this for cost 6 ignoring requirements; `[When Digivolving]` activate 1: a Gabumon → MetalGarurumon in hand free ignoring reqs, **or** delete opp lowest-DP; inherited Omnimon-name attack trashes opp top security. |
| **BT22-026 MetalGarurumon** (Lv6 Blue, freq 25) | **payoff / cross-line** | `[Hand][Main]` (Nokia gate) a Gabumon digivolves into this for cost 6; `[When Digivolving]` activate 1: an Agumon → WarGreymon in hand free ignoring reqs, **or** return opp lowest-level to hand; inherited Omnimon-name attack unsuspends self. |
| **BT17-015 WarGreymon** (Lv6 Red, freq 25) | **payoff** | -3 play cost with a Tai Kamiya Tamer; delete opp ≤8000 DP **or** Gabumon → MetalGarurumon free; inherited trash-security on Omnimon attack. |
| **BT17-027 MetalGarurumon** (Lv6 Blue, freq 24) | **payoff** | -3 play cost with a Matt Ishida Tamer; opp can't-suspend lock **or** Agumon → WarGreymon free; inherited unsuspend on Omnimon attack. |
| **BT22-015 Omnimon** (Lv7 Red, freq 25) | **payoff / closer** | DNA cost 0 (Lv6 Greymon-name + Lv6 Garurumon-name); `<Blocker>`, two `<Decode (Lv3)>`; `[On Play][When Attacking]` delete opp lowest-DP; `[When Digivolving]` bottom-deck opp per 2 same-level stack cards, then may attack. |
| **EX9-021 Omnimon Alter-S** (Lv7 Blue, freq 24) | **payoff / closer** | DNA: opp effects can't affect it this turn + delete all opp highest-level; `[End of Attack]` replay materials, then become top security. |
| **BT17-078 Omnimon** (Lv7 Red, freq 22) | **payoff / closer** | `<Blast DNA Digivolve (WarGreymon + MetalGarurumon)>`, `<Raid>`, `<Blocker>`; if DNA, bounce opp same-level + delete 1. |
| **BT20-102 Omnimon (X Antibody)** (Lv7, freq 4) | **payoff / board wipe** | `<Raid>`,`<Piercing>`,`<Blocker>`; keep 1 Digimon each side, delete the rest, then bounce 1 opp. |
| **EX4-073 Omnimon Alter-B** (Lv7 Black, freq 7) | **payoff / removal** | `<De-Digivolve 3>` then delete ≤6 cost; attack trashes Lv6+ materials to delete opp lowest-cost + trash security. |
| **BT22-008 Agumon** (Lv3, freq 25) | **enabler / recursion** | `[On Play]` return a Greymon/Garurumon/Omnimon from trash to hand; inherited `[EoT]` this + another may DNA-digivolve into a hand Digimon. |
| **BT22-017 Gabumon** (Lv3, freq 25) | **enabler / search** | `[On Play]` reveal 3, add 1 Omnimon-text card + 1 `[CS]` card; inherited `[EoT]` DNA-digivolve. |
| **BT17-007 / EX4-038 / ST20-10 / BT12-059 Agumon** | enablers | search Tamer/Greymon-Omnimon lines; rookie bodies for free-play targets. |
| **BT17-019 / EX4-039 / ST21-10 Gabumon** | enablers | Garurumon-line search / rookie bodies. |
| **BT16-082 Ukkomon / BT14-001 Koromon / BT22-005 Tsumemon** | engine (breeding / draw) | breeding reveal-and-add; inherited draw on opp security removal / `[CS]` play. |
| **BT22-094 Yuugo / BT22-089 Mirei / BT22-099 Kuremi Detective Agency** | tech / `[CS]` support / ramp | cost reduction, Tamer-chain + draw, Delay +2 memory. |
| **P-206 Digital Gate Open / LM-034 Wisteria Memory Boost! / ST2-13 / BT1-090** | engine / ramp | reveal-and-add + Delay-ramp / temporary memory. |
| **ST20-15 Island of Adventure** | tech (security) | `[All Turns]` +2000 DP to Lv3+, security swap. |
| **EX4-061 Matt & Tai / P-156 Future Potential!** | enabler / tech | suspend-for-memory + opposite-rookie free-play; ≤3-cost color-matched free play. |

Colors: **Red/Blue** core (Agumon→Greymon→WarGreymon red; Gabumon→Garurumon→MetalGarurumon
blue), with Black splashes for the Omnimon Alter-B / Omnimon (X Antibody) edges.

---

## Digivolution lines

- **Red:** Koromon (BT14-001) → Agumon (BT22-008 / BT17-007 / EX4-038 / ST20-10) →
  Greymon (BT17-102 / BT23-008) → **WarGreymon** (BT17-015 cost 3 from Lv5;
  BT22-013 cost 3 from Lv5, or **cost 6 from an Agumon with Nokia, ignoring
  requirements**).
- **Blue:** Tsumemon (BT22-005) → Gabumon (BT22-017 / BT17-019 / EX4-039 / ST21-10) →
  Garurumon (BT23-018) → **MetalGarurumon** (BT17-027 cost 3; BT22-026 cost 3,
  or cost 6 from a Gabumon with Nokia).
- **DNA → Omnimon (Lv7):** WarGreymon (Lv6 Greymon-name) + MetalGarurumon (Lv6
  Garurumon-name) DNA-digivolve into **Omnimon** (BT22-015 `[DNA Digivolve]` cost
  0, "Stack the 2 specified Digimon and digivolve unsuspended"; BT17-078 `<Blast
  DNA Digivolve>`; EX9-021 Alter-S; EX4-073 Alter-B; BT20-102 X-Antibody). DNA
  digivolution stacks both Digimon into one permanent and the result is a
  different Digimon (`general_rule.pdf` §6/§8 DNA digivolution); both stacks
  become digivolution sources under the result.
- **Cross-line shortcut:** the WarGreymon/MetalGarurumon `[When Digivolving]`
  branch digivolves the *opposite* rookie into the *opposite* Lv6 from hand free,
  ignoring requirements — manufacturing the second DNA piece in one digivolve.
- **Off-stack Omnimon via BT17-095 `<Delay>`:** when an own Lv6 Greymon/Garurumon-
  name leaves the battle area **outside a battle**, the placed BT17-095 trashes
  itself (`<Delay>`: trash after the placing turn to activate, §16) to
  DNA-digivolve that leaving Digimon + a hand card into a hand Omnimon. DCGO
  `BT17/Red/BT17_095.cs` clause B (`WhenRemoveField` observer, `!IsByBattle`,
  owner gate, `SetJogress` into an Omnimon-named hand recipe).

---

## Named combos

### 1. Miraculous Mega Knight — recur an Agumon/Gabumon from trash
- Cards: **BT17-095**, plus an Agumon (**BT22-008**) or Gabumon (**BT22-017**).
- Printed (BT17-095 `[Main]`): "You may play 1 [Agumon] or [Gabumon] from your
  hand or trash without paying the cost. Then, place this card in the battle area."
- Expected mechanical outcome: with an eligible rookie **only in trash**, `[Main]`
  plays it for **0 memory** (own field +1, that card's trash count −1), THEN
  seats BT17-095 itself as a face-up **Delay** Option (Delay-Option count +1) —
  net own field +2. With an eligible rookie in **both** hand and trash the
  union-zone pick must surface ≥2 card options (no auto-select; CLAUDE.md rule 17).
- Rules/keyword basis: "without paying the cost"; `<Delay>` placement §16. DCGO
  `BT17/Red/BT17_095.cs` clause A (union play free + `PlaceDelayOptionCards`),
  `BT22/Red/BT22_008.cs`.
- Rank: **1** (freq-25 Option; the archetype's signature value/tutor engine that
  also seeds combo 2).

### 2. Mega Knight Delay — off-stack Omnimon when a Lv6 leaves outside battle
- Cards: **BT17-095** (placed, past its placing turn) + an own Lv6 Greymon/
  Garurumon-name (**BT22-013 WarGreymon**) + an Omnimon (**BT22-015**) and a DNA
  partner in hand.
- Printed (BT17-095 `[All Turns]`): "When one of your level 6 Digimon with
  [Greymon] or [Garurumon] in its name would leave the battle area outside of a
  battle, `<Delay>` … That Digimon and a card in the hand may DNA digivolve into a
  Digimon card with [Omnimon] in its name in the hand."
- Expected mechanical outcome: the named Lv6 leaving **outside battle** pays the
  Delay cost (BT17-095 trashed: Options −1, trash +1) and DNA-digivolves the
  leaving Lv6 + a chosen hand card into the hand Omnimon — a new Lv7 Omnimon
  permanent with the leaving WarGreymon as a digivolution source; both the Omnimon
  result and the partner leave hand (hand −2).
- Unhappy paths (must NOT fire): (A) an **opponent's** Lv6 leaving (owner gate);
  (B) an **own** Lv6 with a non-Greymon/Garurumon name (name filter); (C) an own
  Lv6 Greymon-name leaving **in battle** (the "outside of a battle" filter).
- Rules/keyword basis: `<Delay>` §16 (incl. not-same-turn); DNA digivolution §6/§8;
  "would leave the battle area" replacement timing. DCGO `BT17/Red/BT17_095.cs`
  clause B.
- Rank: **2** (the archetype's defining trick — a free out-of-sequence Omnimon
  with protection value).

### 3. Nokia free-play into Tai & Matt memory ramp
- Cards: **BT22-084 Nokia Shiramine** + **BT17-081 / EX9-066 Tai & Matt** (both on
  field) + an Agumon/Gabumon in hand.
- Printed: BT22-084 `[Start of Your Main Phase][On Play]` "If you have 1 or fewer
  Digimon, you may play 1 [Agumon] or [Gabumon] from your hand without paying the
  cost." BT17-081 `[All Turns]` "When one of your Digimon is played or digivolves,
  by suspending this Tamer, if you have a Digimon with [Greymon] in its name, gain
  1 memory. If you have a Digimon with [Garurumon] in its name, gain 1 memory."
- Expected mechanical outcome: with ≤1 own Digimon, Nokia free-plays the rookie
  (own field +1, **0 memory**). That play fires Tai & Matt: suspend the Tamer →
  **+1 per present Greymon name** and **+1 per present Garurumon name**. The gate
  is on the Greymon/Garurumon *names*, so an Agumon/Gabumon alone grants **0**; a
  Greymon name present grants **+1**; both names present grant **+2**.
  - System note: with both name-markers on field there are already 2 own Digimon,
    so Nokia's free-play is correctly gated **off** (printed "1 or fewer Digimon");
    the +2 arm is then driven by a real hard-played own-Digimon play event (the
    same trigger Nokia's free-play would have produced), isolating the +2 grant.
- Rules/keyword basis: "by suspending this Tamer" per-trigger cost; name match is
  substring, not trait. DCGO `BT22/Red/BT22_084.cs`, `BT17/Red/BT17_081.cs`.
  (Known BT17-081 simultaneous-double-pay regression is out of scope — single
  play events only.)
- Rank: **3** (the deck's core tempo/ramp engine; ubiquitous).

### 4. Nokia +1000 aura + DNA Omnimon swing
- Cards: **BT22-084 Nokia Shiramine** + **BT22-015 Omnimon**.
- Printed: BT22-084 `[All Turns]` "All your Digimon with [Greymon], [Garurumon] or
  [Omnimon] in their names get +1000 DP." BT22-015 `[On Play][When Attacking]`
  "Delete 1 of your opponent's Digimon with the lowest DP."
- Expected mechanical outcome: while Nokia is on field, BT22-015 Omnimon reads
  **16000** (base 15000 + 1000 aura); the aura must NOT touch the **opponent's**
  Greymon-name (owner-gated) nor own **non-matching** names (a Veemon stays at
  base). Paired with BT22-015's own `[On Play]` delete-lowest-DP to clear a
  blocker ahead of the swing — exercised through the card's real trigger.
- Rules/keyword basis: continuous DP modifier with owner+name filter; "delete with
  the lowest DP" selection. DCGO `BT22/Red/BT22_084.cs` (`ChangeDPStaticEffect`),
  `BT22/Red/BT22_015.cs`.
- Rank: **4** (the closing DP buff + board control).

### 5. WarGreymon/MetalGarurumon branch — free cross-line digivolve
- Cards: **BT22-013 WarGreymon** / **BT22-026 MetalGarurumon**.
- Printed (both `[When Digivolving]`): "Activate 1 of the effects below: ・1 of
  your [Agumon]/[Gabumon] may digivolve into [the opposite Lv6] in the hand,
  ignoring digivolution requirements and without paying the cost. ・[BT22-013]
  Delete opp lowest-DP / [BT22-026] Return opp lowest-level to hand."
- Expected mechanical outcome: choosing the digivolve branch lets a named partner
  digivolve into the opposite Lv6 from hand **free, ignoring requirements** — a
  second Lv6 enters with **no memory spend** (the base permanent's top card becomes
  the Lv6; hand −1). Choosing the OTHER branch resolves only the removal arm
  (BT22-026 bounce: opp field −1, opp hand +1) and leaves the Agumon + the hand
  Lv6 untouched. **Exactly one** branch resolves per "Activate 1 of the effects
  below" (mutual exclusion).
- Rules/keyword basis: single-choice branch; ignore-requirements + without-cost
  digivolve. DCGO `BT22/Red/BT22_013.cs`, `BT22/Blue/BT22_026.cs`.
- Rank: **5** (builds the DNA board two Lv6s deep in one digivolve; fuels
  combos 2 & 4).

### Dropped (ranked but under the cap — logged per Phase 3)
- **P-206 Digital Gate Open Delay → cost-reduced Tamer** (freq 4): strong
  Option-chaining ramp but lower frequency; the cost-reduction Delay is
  engine-heavier to assert. Pieces implemented.
- **BT17-015 / BT17-027 Tamer cost reduction** (−3 with Tai/Matt): freq 25 but
  folded into combo 5's line; has green per-card coverage.
- **BT22-099 Kuremi / LM-034 Wisteria** Delay → +2 memory: Option ramp engines,
  lower payoff-centrality than the Omnimon combos.
- **BT22-008 / BT22-017 inherited `[EoT]` DNA-digivolve** into a hand Omnimon: a
  fourth route to Omnimon, overlaps combos 2 & 5; dropped to keep the cap.

---

## Playstyle

**Combo-midrange / tempo-ramp.** Nokia (BT22-084) keeps memory floored at 3 and
free-plays rookies; the Tai & Matt Tamers (BT17-081 / EX9-066) convert every play
and digivolve into memory, so the deck climbs the red/blue lines and DNA-digivolves
into Omnimon a turn or two ahead of fair cost. Option cards (BT17-095, P-206,
LM-034, BT22-099) double as tutors, ramp, and — for BT17-095 — a recursion /
off-stack-Omnimon engine. Memory curve: floor-to-3 each turn keeps the player
proactive; the deck spends in bursts to land a Lv7 with removal attached. Every
multi-pick (BT17-095 union-zone pick, the Delay accept + DNA partner pick, the
WG/MG branch choice, the delete/bounce targets) is surfaced to the action space
(CLAUDE.md rule 17 — no auto-selection).

## Win conditions

- Land **Omnimon** (BT22-015 / BT17-078 / EX9-021 / EX4-073 / BT20-102), use its
  `[On Play]`/`[When Digivolving]` removal (delete lowest-DP, DNA-bounce, board
  wipe, De-Digivolve) to strip the opponent's board, then swing with `<Blocker>`
  behind Nokia's +1000 aura. Omnimon-named inherited attacks (BT22-013 trash
  security, BT22-026 unsuspend) and BT22-015's `<Decode>` redundancy press the
  advantage; EX9-021 Alter-S converts a hit into a re-set Omnimon on security.
- Memory ramp + free plays out-tempo the opponent and present multiple Lv7
  threats; BT17-095's Delay recovers an Omnimon when one leaves outside battle.

## Ranked interactions to test

1. **Mega Knight `[Main]` from trash** (combo 1) — Option recursion + Delay
   placement + the from-hand/from-trash zone choice (no auto-select).
2. **Mega Knight `<Delay>` off-stack Omnimon** (combo 2) — the DNA digivolve fires
   on an own Lv6 Greymon/Garurumon leaving outside battle; the three **unhappy
   paths** (opponent's Digimon; own non-matching name; leave-in-battle) do NOT fire.
3. **Nokia free-play → Tai & Matt memory ramp** (combo 3) — rookie plays for 0;
   the Tamer grants memory only per Greymon/Garurumon names present (rookie alone
   grants 0; +1 / +2 with names).
4. **Nokia +1000 aura selectivity** (combo 4) — buff hits own
   Greymon/Garurumon/Omnimon names only, never the opponent's nor own
   non-matching, paired with an Omnimon removal swing.
5. **WarGreymon/MetalGarurumon branch digivolve** (combo 5) — free cross-line
   digivolve (branch 0) vs forced removal (branch 1); exactly one branch resolves.
