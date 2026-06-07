# DNA Omnimon — Model

System-level model of the **DNA Omnimon** archetype (resolved canonical name:
`DNA Omnimon`, 110 decklists, 98 unique cards). This is a durable archetype
*system* model authored per the `/archetype-interaction-test-author` skill
(Phases 0-3). It is NOT a per-card faithfulness doc.

Source priority (CLAUDE.md): official `general_rule.pdf` (canonical; keyword
semantics §16, DNA digivolution §8-2) + DCGO C# (battle-tested) OUTRANK the
card-text JSON. DCGO C# at
`DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`.

**Coverage caveat (corrected 2026-06-03).** The archetype is NOT fully
implemented. The static coverage gate **FAILS**: 66/98 = 67% implemented
(threshold 85%), with **1 failing card** (AD1-004) and **31 `unknown`-status
cards** (`qa/qa-reports/archetype_interactions.json` → `coverage_gate`). Per the
skill guardrail, `unknown`-status cards are **not** "implemented". The named
**combo payoff/enabler/material cards** (EX9-021, BT17-095, BT17-078, BT17-015,
BT17-027, BT22-013, BT22-026, BT22-084, BT22-017, BT22-008, AD1-025, EX4-060,
EX9-012, EX9-019) ARE confirmed `IMPLEMENTED` in
`qa/qa-reports/validated_cards_dsl.json` (archetype "DNA Omnimon", gap_kind
null) — which is why the **combo-presence** gate passes. But several
digivolution-line *connector* cards are NOT implemented and appear in the
coverage-gate `unknown_cards` list — notably **BT14-014 (MetalGreymon)**,
**BT15-024 (Garurumon)**, and **EX9-014 (Gabumon)**. They are line context, not
combo pieces, and the interaction tests below do not depend on them; they are
marked `(unimplemented)` where cited so the model does not over-state coverage.

---

## Card pool & roles

Only the meaningful / high-frequency core (full pool of 98 includes low-freq
splashes). Frequency = number of the 110 decklists containing the card.

| Card | Freq | Role | One-line function |
|------|------|------|-------------------|
| EX9-021 Omnimon Alter-S | 107 | **payoff (primary)** | Lv.7; DNA from Blue Lv.6 + Red Lv.6 (cost 0); when DNA → self unaffected by opp effects for the turn + delete ALL opp Digimon with the highest level; End-of-Attack replays 2 sources + re-secures self. |
| BT17-095 Miraculous Mega Knight | 96 | **enabler/Option (key)** | [Main] free-play 1 [Agumon]/[Gabumon] from hand or trash + stays in battle area as a Delay; [All Turns] Delay: when a Lv.6 [Greymon]/[Garurumon] would leave outside battle, that Digimon + 1 hand card may DNA digivolve into an [Omnimon] in hand. |
| BT22-084 Nokia Shiramine | 95 | engine/Tamer | Start-of-turn memory floor → 3 (if ≤2); free-play [Agumon]/[Gabumon] if ≤1 Digimon; passive +1000 DP to [Greymon]/[Garurumon]/[Omnimon]. Enables BT22-013/026 cheap Lv.6 lines. |
| BT17-027 MetalGarurumon | 90 | payoff/engine | Cost −3 with [Matt Ishida]; On Play/Digivolve choose: opp can't unsuspend, OR free-digivolve an [Agumon]→[WarGreymon]. Omnimon inherit: unsuspend on attack. |
| BT22-017 Gabumon | 90 | enabler (search) | On Play reveal top 3, add 1 "[Omnimon] in text" + 1 [CS] card. Inherit [End of Your Turn]: this + another Digimon may DNA digivolve into a hand card. |
| BT17-015 WarGreymon | 89 | payoff/engine | Cost −3 with [Tai Kamiya]; On Play/Digivolve choose: delete opp ≤8000 DP, OR free-digivolve a [Gabumon]→[MetalGarurumon]. Omnimon inherit: trash opp top security. |
| BT22-008 Agumon | 89 | enabler (recursion) | On Play return 1 [Greymon]/[Garurumon]/[Omnimon] from trash. Inherit [End of Your Turn]: DNA digivolve into a hand card. |
| BT22-013 WarGreymon | 89 | payoff (accel) | [Hand][Main] with [Nokia]: an [Agumon] digivolves into this for cost 6. When Digivolving: free [Gabumon]→[MetalGarurumon], OR delete opp lowest DP. |
| BT22-026 MetalGarurumon | 88 | payoff (accel) | [Hand][Main] with [Nokia]: a [Gabumon] digivolves into this for cost 6. When Digivolving: free [Agumon]→[WarGreymon], OR bounce opp lowest level. |
| BT17-078 Omnimon | 86 | **payoff (Blast)** | [Hand][Counter] Blast DNA Digivolve ([WarGreymon]+[MetalGarurumon]); Raid/Blocker; When DNA Digivolving → bounce all opp Digimon sharing a chosen level + delete 1 opp Digimon. |
| BT17-081 Tai Kamiya & Matt Ishida | 84 | engine/Tamer | [All Turns] suspend → gain memory per Greymon-name + per Garurumon-name on play/digivolve; [End of Turn] an [Omnimon] may attack a player. |
| BT22-015 Omnimon | 84 | payoff | Blocker; Decode (Red/Black & Blue/Yellow Lv.3); On Play/Attack delete opp lowest DP; When Digivolving bounce per 2 same-level stack cards + self may attack. |
| EX4-039 Gabumon | 73 | enabler (search) | On Play reveal top 3, add 1 [Garurumon] + 1 [Agumon]/[Greymon]/[Omnimon]. Inherit: gain 1 memory on ally digivolve. |
| EX9-066 Tai Kamiya & Matt Ishida | 58 | engine/Tamer | On Play return Greymon/Garurumon/Omnimon from trash (else draw 1); [All Turns] suspend → memory per Greymon/Garurumon name on play/digivolve. |
| ST20-10 Agumon | 52 | enabler | [Your Turn] when opp has a 10000+ DP Digimon (or 3+ Tamer colors), digivolve into [WarGreymon] for cost 4 ignoring requirements. |
| EX4-038 Agumon | 51 | enabler (search) | On Play reveal top 3, add 1 [Greymon] + 1 [Gabumon]/[Garurumon]/[Omnimon]. |
| BT22-089 Mirei Mikagura | 51 | engine/Tamer | DigiXros / memory engine (splash). |
| BT16-082 Ukkomon | 49 | enabler | White rookie ramp/search (Royal Knight support). |
| P-206 Digital Gate Open | 39 | enabler/Option | [Main] reveal top 3, add 1 Digimon + 1 Tamer; Delay: play a Tamer cost −4. |
| BT8-097 Crimson Blaze | 27 | tech/Option | [Main] opp can't play Digimon by effects this turn + delete all opp ≤6000 DP; cost −1 per opp Digimon. |
| ST20-15 Island of Adventure | 28 | tech/Option | Security/[Main] all your Lv.3+ get +2000 DP; recurs security; free Tamer from security. |
| BT22-099 Kuremi Detective Agency | 8 | engine/Option | [Main] add 1 [CS] card from top 3; Delay: gain 2 memory. |
| LM-034 Wisteria Memory Boost! | 14 | engine/Option | [Main] add 1 blue/red Digimon from top 3; Delay: gain 2 memory. |
| AD1-001/009/010/012/025 (Adventure line) | 17/17/17/17/3 | alt payoff line | Greymon/BlitzGreymon/Garurumon/CresGarurumon free-evo chains → AD1-025 Omnimon (Partition) and EX9-021 via [End of Turn] DNA. |
| EX9-012/013/019 (Alterous/Blitz/WereGaru SM) | 18/4/21 | engine | Free cross-line digivolve chains (Greymon↔Garurumon triggers) feeding EX9-021. |
| EX4-060 Omnimon Alter-S | 6 | payoff (alt) | When Digivolving delete ≤8000 + bounce Lv.6+; replaces self from security with BlitzGreymon+CresGarurumon on leave. |
| AD1-025 Omnimon | 3 | payoff (alt) | DNA WarGreymon+MetalGarurumon; bounce opp with ≤ own sources + delete 1; Partition; opp-leave trashes Option + top security. |

---

## Digivolution lines

Two-tribe (Greymon / Garurumon) converging into Omnimon-name Lv.7s via **DNA
digivolution** (`general_rule.pdf` §8-2 — multiple field Digimon become the
digivolution cards of one new Digimon; the new card is a fresh card that may
attack the same turn, §8-2-2-1-6).

- **Greymon line (Red):** Koromon/Tsunomon eggs → Agumon (BT22-008 / EX4-038 /
  ST20-10) → Greymon (BT17-102 / BT23-008) → MetalGreymon (BT14-014
  *(unimplemented — coverage-gate `unknown_cards`)* / EX9-012) → **WarGreymon**
  (BT17-015 / BT22-013, Lv.6 Red) / BlitzGreymon.
- **Garurumon line (Blue):** Wanyamon/Tsunomon eggs → Gabumon (BT22-017 /
  EX4-039 / EX9-014 *(unimplemented — coverage-gate `unknown_cards`)*) →
  Garurumon (BT15-024 *(unimplemented — coverage-gate `unknown_cards`)* /
  BT23-018 / EX9-019 WereGaru SM) → **MetalGarurumon** (BT17-027 / BT22-026,
  Lv.6 Blue) / CresGarurumon.
- **DNA convergence into Omnimon (Lv.7):**
  - **EX9-021 Omnimon Alter-S** ← DNA: **Blue Lv.6 + Red Lv.6**, cost 0
    (`stacks_unsuspended: true`). The widest DNA gate (any blue 6 + any red 6).
  - **BT17-078 Omnimon** ← Blast DNA / DNA: **[WarGreymon] + [MetalGarurumon]**.
  - **AD1-025 Omnimon** ← DNA: [WarGreymon] + [MetalGarurumon] (Partition).
  - **EX4-060 Omnimon Alter-S** ← from Blue Lv.6 standard digivolve.
- **Free-digivolve sub-engine:** WarGreymon/MetalGarurumon "choose" arms
  (BT17-015 / BT17-027 / BT22-013 / BT22-026) chain the *other* tribe's Lv.6
  out for free, assembling both DNA materials in one turn.

---

## Named combos

### Combo A — DNA Omnimon Alter-S blowout (Blue 6 + Red 6 → EX9-021)
- Cards: **EX9-021** (+ any Red Lv.6 e.g. BT17-015/BT22-013 and any Blue Lv.6
  e.g. BT17-027/BT22-026 as the two materials; opp Digimon as victims).
- Expected mechanical outcome: stack a Blue Lv.6 + Red Lv.6 into EX9-021 (cost
  0); EX9-021 enters with both Lv.6 as its digivolution cards (≥2 sources). DNA
  origin → EX9-021 gains opponent-effect immunity until end of turn. Then **ALL
  opponent Digimon tied for the highest level are deleted** (board diff: every
  max-level opp Digimon → trash; EX9-021 present, unsuspended, may attack this
  turn per §8-2-2-1-6). The two materials stay as sources (consumed from field).
- Rules/keyword basis: `general_rule.pdf` §8-2 (DNA digivolution), §8-2-2-1-6
  (new DNA Digimon may attack same turn); DCGO `EX9/Blue/EX9_021.cs` lines
  30-96 (GetJogress Blue6+Red6 cost 0), lines 100-189 (DNA-gated immunity +
  unconditional delete-highest). Engine: `cards/ex9/EX9-021.yaml`
  (`alt_paths kind: dna_digivolve`, `grant_effect_immunity`, `for_each` over
  `level_matches_aggregate{highest_level}` + `delete_permanent`).
- Rank: **1** (107/110 — the defining payoff; highest play-frequency × payoff).

### Combo B — Miraculous Mega Knight Delay → reactive DNA Omnimon (Option)
- Cards: **BT17-095** (Option) + a Lv.6 [Greymon]/[Garurumon] on field + an
  [Omnimon]-name Lv.7 in hand (e.g. **BT17-078** / EX9-021 / AD1-025).
- Expected mechanical outcome: (1) [Main] free-plays an [Agumon]/[Gabumon] from
  hand/trash (board diff: +1 rookie, no memory paid) and BT17-095 is placed in
  the battle area as a Delay Option. (2) Later, when a Lv.6 [Greymon]/[Garurumon]
  *would leave the battle area outside of battle* (deletion, bounce, trade), the
  Delay triggers: that leaving Lv.6 + a Lv.6 card from hand DNA digivolve into
  an [Omnimon]-name Lv.7 in hand (board diff: BT17-095 trashed; the leaving
  Digimon is *consumed as a DNA material* instead of going to trash; new Omnimon
  on field with both as sources; Omnimon's When-Digivolving fires).
- Rules/keyword basis: §8-2 (DNA digivolution); §16 `<Delay>` (trash-after-
  placing-turn activation). DCGO `BT17/Red/BT17_095.cs` lines 17-157 (Main
  free-play + PlaceDelayOptionCards), lines 162-484 (WhenRemoveField Delay:
  level-6 Greymon/Garurumon leaving outside battle → select Lv.7 [Omnimon] +
  field permanent + hand Lv.6 → PlayCardClass.SetJogress). Engine:
  `cards/bt17/BT17-095.yaml`.
- Rank: **2** (96/110 — top Option, the FOCUS line: Option enabling a reactive
  DNA play; converts a removed Lv.6 into a fresh Omnimon).

### Combo C — Blast DNA Omnimon off opponent's turn (BT17-078 Counter)
- Cards: **BT17-078** (hand) + a **[WarGreymon]** + a **[MetalGarurumon]** on
  field (e.g. BT17-015 + BT17-027).
- Expected mechanical outcome: at Counter timing (e.g. when attacked), Blast DNA
  Digivolve — the WarGreymon and MetalGarurumon become BT17-078's digivolution
  cards without paying cost. When DNA Digivolving: choose 1 opp Digimon; **bounce
  ALL opp Digimon of that same level to deck bottom**, then **delete 1 opp
  Digimon** (board diff: same-level opp Digimon → deck bottom, plus 1 more
  deleted). BT17-078 has Raid + Blocker. As a Counter, this is a defensive
  blow-out played on the opponent's turn.
- Rules/keyword basis: §16-30 `<Blast DNA Digivolve>` (1 field Digimon + 1
  specified hand card → DNA digivolve at Counter timing without cost); §8-2.
  DCGO `BT17/White/BT17_078.cs`. Engine: `cards/bt17/BT17-078.yaml`
  (`alt_paths kind: blast_dna_digivolve` materials WarGreymon+MetalGarurumon,
  + `kind: dna_digivolve`).
- Rank: **3** (86/110 — the Counter-timing DNA payoff; high disruption).

### Combo D — Free cross-tribe Lv.6 assembly → both DNA materials in one turn
- Cards: **BT17-015** (or BT22-013) WarGreymon "free-digivolve [Gabumon]→
  [MetalGarurumon]" arm + a [Gabumon] on field + [MetalGarurumon] in hand
  (BT17-027/BT22-026); symmetrically BT17-027's "[Agumon]→[WarGreymon]" arm.
- Expected mechanical outcome: digivolve into WarGreymon (When-Digivolving),
  choose the second arm → a [Gabumon] on field **digivolves into [MetalGarurumon]
  in hand, ignoring requirements, free**. Board diff: now a WarGreymon (Red Lv.6)
  AND a MetalGarurumon (Blue Lv.6) on field in one turn for one tribe's worth of
  memory — exactly the two materials Combo A / Combo C need. Then DNA into
  EX9-021 / BT17-078.
- Rules/keyword basis: §8-1 (digivolution, ignore-requirements by effect); §8-2
  (subsequent DNA). DCGO `BT17/Red/BT17_015.cs` + `BT17/Blue/BT17_027.cs`
  (On Play/Digivolve two-option ActivateClass; arm 2 = free cross-line
  digivolve). Engine: `cards/bt17/BT17-015.yaml`, `cards/bt17/BT17-027.yaml`.
- Rank: **4** (89/89 — the setup engine that makes Combos A/C castable; high
  centrality but is a *setup*, slightly below the payoffs).

### Combo E — Nokia accel into cheap Lv.6 (BT22-013/026 cost-6 line)
- Cards: **BT22-084 Nokia Shiramine** + **BT22-013 WarGreymon** (or **BT22-026
  MetalGarurumon**) + an [Agumon]/[Gabumon] on field.
- Expected mechanical outcome: with Nokia in play, BT22-013's [Hand][Main]:
  an [Agumon] **digivolves into BT22-013 for digivolution cost 6**, ignoring
  requirements (board diff: Lv.3 → Lv.6 in one jump for 6 memory, no Lv.4/5
  needed); Nokia also gives the resulting Greymon/Omnimon +1000 DP and floors
  memory to 3 at start of turn. When-Digivolving then free-chains the other
  tribe's Lv.6 (BT22-013 arm 1). Yields a Lv.6 + a free Lv.6 → DNA material pair.
- Rules/keyword basis: §8-1 (effect-driven digivolve, ignore requirements);
  §rule for "If you have <named Tamer>". DCGO `BT22/Red/BT22_013.cs`,
  `BT22/Blue/BT22_026.cs`, `BT22/.../BT22_084.cs`. Engine:
  `cards/bt22/BT22-013.yaml`, `cards/bt22/BT22-026.yaml`, `cards/bt22/BT22-084.yaml`.
- Rank: **5** (84-95/110 — the dominant modern acceleration enabler).

### (Dropped, ranked below cap)
- **Combo F — Tai & Matt memory ramp loop (BT17-081 / EX9-066):** suspend Tamer
  → gain memory per Greymon-name + per Garurumon-name on every play/digivolve;
  fuels the multi-play DNA turns. Rank 6. Dropped: pure memory engine, not a
  board-diff combo; better as a per-card assertion.
- **Combo G — AD1 Adventure free-evo chain → AD1-025 / EX9-021 [End of Turn] DNA
  (AD1-001/009/010/012/025):** Greymon/Garurumon free-digivolve off each other's
  plays, then BlitzGreymon/CresGarurumon [End of Turn] "2 Digimon may DNA into
  Omnimon Alter-S". Rank 7. Dropped: lower frequency (3-17), and the
  cross-tribe-assembly idea is already covered by Combo D.
- **Combo H — Crimson Blaze board wipe + lockout (BT8-097):** delete all opp
  ≤6000 + opp can't play Digimon by effects this turn, then resolve a DNA
  Omnimon unimpeded. Rank 8. Dropped: tech Option (27/110), tangential to the
  DNA line.

---

## Playstyle

- **Class:** midrange/combo with a heavy control top-end. Tempo: spend the early
  turns searching (BT22-017/EX4-038/EX4-039 reveal-3) and ramping
  (Nokia/Tai&Matt/Memory Boosts), then convert into a one-turn DNA Omnimon that
  simultaneously wipes the board and pressures security.
- **Memory curve:** floors (Nokia → 3) + memory-on-play/digivolve (Tai & Matt,
  EX4-039 inherit) + free-digivolve arms collapse a normally-expensive Lv.7 DNA
  turn into a single explosive sequence.
- **Resilience:** opponent-effect immunity (EX9-021 DNA arm) and reactive DNA
  (BT17-095 Delay, BT17-078 Counter, Partition/Decode replays on EX4-060/AD1-025/
  BT22-015) make the Omnimon hard to answer; removed Lv.6 are re-used as DNA fuel.

## Win conditions

1. **DNA Omnimon board wipe + beatdown:** EX9-021 deletes all highest-level opp
   Digimon (Combo A) / BT17-078 mass-bounces by level (Combo C), then the 15000
   DP Omnimon attacks (often Raid/Blocker, +1000 from Nokia) and inherits trash
   opp security each attack (BT17-015/BT22-013 Omnimon inherits) → race security
   to 0.
2. **Reactive Omnimon value loop:** Partition/Decode/Delay replays keep an
   Omnimon (or its materials) on board through removal, grinding the opponent out.

## Ranked interactions to test

Authored in `code/digimon-engine/tests/archetypes/dna_omnimon.rs`. Each test
maps 1:1 to a combo below and exercises the **real card abilities** (no
bypassing engine helper substitutes for a named card).

1. **Combo A** (EX9-021 DNA blowout) — AUTHORED. Fire the real DNA digivolve
   (`effect_initiated_dna_digivolve` over a Blue Lv.6 + Red Lv.6 pair) and
   assert the delete-ALL-highest-level board diff + the opponent-effect
   immunity flag. Unhappy path: a *standard* (non-DNA) digivolve grants no
   immunity. Highest priority (FOCUS: DNA payoff line).
2. **Combo B** (BT17-095 Option Delay → reactive DNA) — AUTHORED. The
   `G-DSL-DNA-FROM-HAND-PARTNER` gap that previously omitted the DNA-into-Omnimon
   body was **CLOSED 2026-05-20** (verb
   `effect_initiated_dna_digivolve_hand_partner`), so the FOCUS Option line is
   now exercisable: assert the leaving Lv.6 [Greymon] is consumed as a DNA
   material under a merged [Omnimon] (NOT trashed). Unhappy path: an *opponent's*
   leaving Lv.6 does not fire the Delay (subject filter `replacement_subject_is_mine`).
3. **Combo C** (BT17-078 Blast DNA Counter) — AUTHORED. Drive the real Counter
   Blast-DNA route (`begin_attack` → select BT17-078 result → field WarGreymon +
   hand MetalGarurumon materials) and assert same-level bottom-deck + the
   mandatory extra-delete prompt on the opponent's turn. Unhappy path: broad
   [Greymon]/[Garurumon]-named materials do NOT satisfy the exact
   WarGreymon+MetalGarurumon Blast marker.
4. **Combo D** (free cross-tribe Lv.6 assembly) — AUTHORED. Fire BT17-015's real
   [On Play] branch-1 arm (select a field [Gabumon] + a hand [MetalGarurumon] →
   `effect_initiated_digivolve` free, ignore reqs) and assert a Blue Lv.6
   MetalGarurumon now sits alongside the Red Lv.6 WarGreymon — the DNA material
   pair Combo A/C need. Unhappy path: no [Gabumon] on field → the branch produces
   no second Lv.6.
5. **Combo E** (Nokia cost-6 Lv.6 jump) — **`#[ignore]`d / BLOCKED on
   `G-ACTIVATED-DIGIVOLVE-EXECUTION`.** BT22-013's [Hand][Main] cost-6 jump is
   encoded only as a `kind: activated_digivolve` alt-path, which has **no engine
   execution route** (`qa/archetype-qa/engine-gaps.md` → the gap lists BT22-013
   as a residual card). The cost-6 jump cannot be played or behaviorally driven,
   so its named board diff cannot be produced faithfully. Additionally the model's
   original "no Nokia → illegal" unhappy-path claim does NOT hold even
   structurally: BT22-013.yaml does not populate the (now-RESOLVED)
   `AltPathSpec.condition:` to gate on Nokia Shiramine, so the Nokia precondition
   is unenforced — a card-local authoring follow-up, distinct from the execution
   gap. The combo's per-card *components* that ARE executable (the [When
   Digivolving] delete / free-digivolve branches) are already covered by
   `tests/cards_behavioral/bt22/bt22_013.rs`; no faithful interaction test can be
   authored for the named jump until the execution route lands.
