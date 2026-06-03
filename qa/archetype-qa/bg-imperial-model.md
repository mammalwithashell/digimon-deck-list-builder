# BG Imperial (Imperialdramon) — Model

> System-level model authored by `/archetype-interaction-test-author` (capstone),
> 2026-06-02. Inputs: `data/cards.json` printed text, the implemented YAML specs
> under `code/digimon-engine/cards/`, the per-card behavioral tests, and DCGO C#
> (`$BASE_DCGO/Assets/Scripts/CardEffect/...`). Source priority per CLAUDE.md:
> `general_rule.pdf` (canonical) + DCGO C# (battle-tested) > card-text JSON.
>
> This is the durable model that the interaction tests in
> `code/digimon-engine/tests/archetypes/bg_imperial.rs` map to 1:1. The older
> `bg-imperial.md` is a per-card QA verdict table (2026-04-05, batch-fix-cards),
> not this system model.

## Card pool & roles (the implemented Imperialdramon line)

| Card | Role | One-line function |
|------|------|-------------------|
| EX1-014 ExVeemon (Blue Lv.4) | enabler / DNA material | Blue Lv.4 leg of the Paildramon DNA digivolve; inherited Jamming for [Free]/[Imperialdramon]. |
| ST9-09 Stingmon (Green Lv.4) | enabler / DNA material | Green Lv.4 leg of the Paildramon DNA digivolve; play-cost reduction + inherited draw. |
| BT12-022 ExVeemon (Blue Lv.4) | enabler / DNA material | DNA material; gains 1 memory when DNA-digivolving into a green card. |
| BT12-050 Stingmon (Green Lv.4) | enabler / DNA material | DNA material; gains 1 memory when DNA-digivolving into a blue card. |
| ST9-05 Paildramon (Blue+Green Lv.5) | payoff / pivot | [WD] *When DNA digivolving* return 1 opp Digimon DP≤6000 to deck bottom; [WA][OPT] unsuspend self. |
| BT16-025 Paildramon (Blue/Green Lv.5) | payoff / pivot | Partition; [WD] suspend all opp Digimon with ≤ its digivolution cards; *if DNA* they can't unsuspend; [WA][OPT] suspend/unsuspend-self. |
| BT12-028 Paildramon (Blue/Green Lv.5) | payoff / pivot | [WD] trash top-3 digivolution cards of all opp Digimon; *if DNA* 2 opp Digimon w/ no sources can't attack. |
| AD1-011 Paildramon (Blue/Green Lv.5) | payoff / pivot | Partition; [WD] battle-immunity; *if DNA* attack target can't change; [WA] digivolve into Imperialdramon (-2 cost). |
| ST9-06 Imperialdramon Dragon Mode (Blue+Green Lv.6) | payoff | [WD] *you may* play 1 Lv.4-or-lower blue **and** 1 Lv.4-or-lower green Digimon card from its own digivolution cards, free. |
| BT16-028 Imperialdramon Dragon Mode (Blue/Green Lv.6) | payoff | [WD] lock + suspend/unsuspend; [All Turns] free digivolve into Imperialdramon: Fighter Mode when opp plays/digivolves and you have a Tamer. |
| BT16-027 Imperialdramon Fighter Mode (Blue/Green Lv.6) | finisher | Blast Digivolve; [WD] bounce opp by source count; [EoA][OPT] unsuspend + (DM-in-stack) bounce a suspended opp. |
| BT12-031 Imperialdramon Fighter Mode (Blue/Green Lv.6) | finisher | [WD] suspend/return; [All Turns] +1000 DP per color in sources, 2+ colors → Security A.+1 + Blocker. |
| AD1-024 Imperialdramon Fighter Mode (Blue/Green Lv.6) | finisher | Security A.+1 + Blocker; [WD][WA][OPT] bounce lowest-DP opp; reactive suspend/bounce. |
| BT17-077 Imperialdramon Paladin Mode (White/Blue Lv.7) | apex finisher | Blast Digivolve; [WD] mass source-trash + trash-recycle, gain 3 on white Lv.7; [WA] bounce-to-unsuspend. |
| BT16-085 Davis & Ken (Blue/Green Tamer) | engine | free Veemon/Wormmon each turn; memory + DNA source-trash on blue/green digivolve. |
| BT3-093 Davis Motomiya (Blue Tamer) | engine | memory floor to 3; [On Play] dig for a blue + a green Digimon. |

## Digivolution lines (cost / colour gates)

```
DemiVeemon (Bx Lv.2)                Wormmon (Gn Lv.3)
   │ digivolve                          │
Veemon (Blue Lv.3)                  Stingmon (Green Lv.4)
   │ digivolve                          │
ExVeemon (Blue Lv.4) ─────┐   ┌─────────┘
                          ▼   ▼
              DNA digivolve  (Blue Lv.4 + Green Lv.4, cost 0)
                          │
                  Paildramon (Blue+Green Lv.5)        ← ST9-05 / BT16-025 / BT12-028 / AD1-011
                          │ digivolve  (Blue Lv.5, cost 4–5)
            Imperialdramon: Dragon Mode (Lv.6)        ← ST9-06 / BT16-028
                          │ digivolve  (Blue Lv.5/Lv.6, cost 2–5; DM→FM often -2)
            Imperialdramon: Fighter Mode (Lv.6)       ← BT16-027 / BT12-031 / AD1-024
                          │ digivolve  (Blue Lv.6, cost 6)
            Imperialdramon: Paladin Mode (Lv.7)       ← BT17-077
```

Key gates (from the compiled `alt_paths`, cross-checked vs `cards.json` `evo_costs`/`xros_req`):
- **Paildramon DNA digivolve**: materials `{Lv.4, Blue}` + `{Lv.4, Green}`, **cost 0** (ST9-05, BT16-025). This is the archetype's defining DNA colour requirement — it needs *both* a blue and a green Lv.4.
- **Paildramon standard digivolve**: from Lv.4 (BT16-025 any colour, ST9-05 Blue), **cost 4**.
- **Imperialdramon DM standard digivolve**: from **Blue Lv.5**, **cost 4** (ST9-06).

### The rookie/champion line is colour-FLUID, not colour-locked (corrected 2026-06-02)

Digivolution into a Lv.3/Lv.4 in this archetype is **not** purely colour-gated. Two
mechanics let the *entire* line — both DNA legs — grow off a **single egg colour**,
so the deck never needs a green egg or a "Wormmon over an egg":

1. **Named digivolution requirements** (`xros_req`; DCGO `AddSelfDigivolutionRequirementStaticEffect`,
   colour-agnostic — matches the base's *name*, not its colour):
   - **BT16-017 Veemon** — `[Digivolve] [DemiVeemon]: Cost 0` → goes over the **blue egg by name**
     (`BT16_017.cs`: `ContainsCardName("DemiVeemon")`).
   - **BT16-018 ExVeemon** — `[Digivolve] [Veemon]: Cost 2` (by name).
   - **BT16-040 Wormmon** — `[Digivolve] [Minomon]: Cost 0` (by name; NOT a blue egg).
2. **Cross-colour evo-costs** (the lower Digimon's colour ≠ the new card's colour):
   - **BT12-050 Stingmon (Green)** ← **Blue** Lv.3 (or Green) — the green DNA leg off the **blue** Veemon line.
   - **BT12-022 ExVeemon (Blue)** ← **Green** Lv.3 (or Blue) — the blue DNA leg off the **green** Wormmon line.
   - **BT16-017 Veemon (Blue/Red)** ← **Green** Lv.2.
   - **BT16-018 ExVeemon (Blue/Red)** ← **Green** Lv.3.

Consequence: a blue DemiVeemon egg → Veemon (blue) → **ExVeemon (blue)** AND, via BT12-050, →
**Stingmon (green)**; both DNA materials for Paildramon assemble off one egg. So:
- **"Can Wormmon digivolve over a blue egg?" → No** — verified in DCGO across all three pool Wormmon
  (BT12-047/P-118 = Green Lv.2 colour-only; BT16-040 = Red Lv.2 + named over *Minomon*). But this is **moot**:
  the green leg comes from Stingmon off the blue Veemon, not from Wormmon-over-an-egg.

**Engine/implementation gap:** the engine *supports* named digivolution requirements (DSL alt-path
`from: { name_contains: ... }`, checked at `dna_digivolve.rs:942`), but the linchpin cross-colour/named
rookies+champions **BT16-017 and BT16-018 are unimplemented** (no YAML) — so the single-egg colour-fluid
line cannot currently be built in the engine. Logged to `qa/archetype-qa/engine-gaps.md`.

## Named combos

### Combo 1 — "DNA Bounce" (ST9-05)
- Cards: EX1-014 ExVeemon (Blue Lv.4) + ST9-09 Stingmon (Green Lv.4) → ST9-05 Paildramon.
- Expected mechanical outcome: the DNA path satisfies the blue+green colour requirement and reaches Paildramon at cost 0; the `[When Digivolving] *When DNA digivolving*` clause installs an OppField selection and returns an opp Digimon with DP≤6000 to the bottom of its deck (opp field −1, opp deck +1). A *regular* digivolve into the same Paildramon does **not** fire the DNA clause.
- Rules/keyword basis: DNA digivolution (`general_rule.pdf` §6 / §16 digivolve); ST9-05 `when: on_dna_digivolve`; DCGO `ST9_05.cs` (`IsJogress` gate, `PutLibraryBottom`, `DP<=6000`).
- Rank: very high (core payoff, every game; the user's "Paildramon effects work" target).

### Combo 2 — "DNA Lockdown" (BT16-025, meta Paildramon)
- Cards: EX1-014 ExVeemon + ST9-09 Stingmon → BT16-025 Paildramon, vs a board of opp Digimon.
- Expected mechanical outcome: `[When Digivolving]` suspends every opp Digimon with ≤ Paildramon's digivolution-card count; **because** the digivolve was DNA, the `on_dna_digivolve` clause additionally applies a `CannotUnsuspend` modifier to all opp Digimon until end of their turn. A regular digivolve suspends them but applies **no** lock.
- Rules/keyword basis: BT16-025 `when: when_digivolving` (materials_count_lte vs source_material_count) + `when: on_dna_digivolve` (CannotUnsuspend, expiry end_of_opponents_turn); DCGO `BT16_025.cs` (`IsJogress` → `GainCanNotUnsuspendPlayerEffect`).
- Rank: high (the meta Paildramon; demonstrates the DNA-vs-regular system fork).

### Combo 3 — "Colour-Gated Source Replay" (ST9-06)
- Cards: stack `[ExVeemon (Blue Lv.4), Stingmon (Green Lv.4), Paildramon, ST9-06 Imperialdramon Dragon Mode]`.
- Expected mechanical outcome: ST9-06's `[When Digivolving]` *you may* replay **1 blue Lv.4-or-lower** and **1 green Lv.4-or-lower** Digimon card *from its own digivolution cards* free — so the blue ExVeemon and the green Stingmon both re-enter the field (field +2). With a stack that has a blue source but **no** green source, only the blue source replays (the green selection has no candidate).
- Rules/keyword basis: ST9-06 `when: when_digivolving, optional: true`, two `select_own_sources` (color_is blue/green, level_lte 4) + `play_selected_sources_free`; DCGO `ST9_06.cs`.
- Rank: high (the user's "colour requirements" target — the effect is gated on the stack's colours).

### Combo 4 — "Evolution-cost & colour gate" (structural + behavioural)
- Cards: ST9-05 / BT16-025 / ST9-06 alt-paths.
- Expected mechanical outcome: the compiled DNA alt-path for Paildramon enforces the `{Lv.4 Blue}` + `{Lv.4 Green}` material colours at cost 0; the standard digivolve costs 4; ST9-06's standard digivolve is from Blue Lv.5 at cost 4. A DNA attempt that cannot supply both a blue **and** a green Lv.4 material does not complete as DNA.
- Rules/keyword basis: `cards.json` `evo_costs`/`dna_costs`/`xros_req`; compiled `CompiledAltPath`.
- Rank: medium-high (the user's "evolution costs and colour requirements" target).

### Combo 6 — "Digivolution colour gate" (ExVeemon / Stingmon / Wormmon)
- Cards: the rookie/champion DNA legs vs eggs/Lv.3 bases of varying colour.
- Expected mechanical outcome: a standard digivolve is permitted only when the base's colour list contains the evo-cost colour and the level matches (`Game::can_digivolve`). Concretely: **Wormmon (Green, evo Green Lv.2) CANNOT digivolve over a blue DemiVeemon egg** (it needs a green Lv.2), but Veemon (Blue) can; ExVeemon (EX1-014, single Blue evo) is locked to a blue Lv.3 and Stingmon (ST9-09, single Green evo) to a green Lv.3, while the dual-colour BT12-022 ExVeemon / BT12-050 Stingmon accept either colour Lv.3.
- Rules/keyword basis: digivolution colour+level requirement (`general_rule.pdf` §6 digivolve; evo-cost is a (colour, level) pair); `Game::can_digivolve` game.rs:3390 → `matching_evo_cost`.
- Rank: high (directly answers "can Wormmon digivolve over a blue egg?" — **no**).
- **Harness note:** the embedded DSL pack builds CardData from YAML only and `card_data_from_compiled` sets `evo_costs: Vec::new()`, so a real DSL-loaded card carries no printed digivolution cost unless its YAML declares a `digivolve` alt-path. The Imperialdramon line's printed evo-costs live in `cards.json` (production), not the YAML — so this combo exercises the gate with synthetic cards encoding the real printed evo-costs (per-card-test convention, `st9_05.rs::make_lv4_regular_base`). This is a test-fixture limitation, **not** an engine bug.

### Combo 5 — "Effect descriptions bubble to the UI" (cross-cutting)
- Cards: ST9-05 (DNA bounce prompt) + ST9-06 (source-replay prompts).
- Expected mechanical outcome: at each Paildramon/Imperialdramon choice point the engine's `PendingSelection.prompt` (serialized to the UI by `digimon-engine-py` `lib.rs:804/1129` and rendered by `action/explain.rs`) carries the card's descriptive YAML `prompt` text, not an empty/placeholder string.
- Rules/keyword basis: no-approximations policy (rule 17) — every choice surfaces through `pending_selection`; the YAML `prompt`/`summary` fields are the UI descriptions.
- Rank: medium (the user's "descriptions bubble to the UI" target; asserted inline within Combos 1 & 3).

## Playstyle
- Class: **tempo / control-combo**. Blue suspend/unsuspend + bounce-to-deck control the board; green ramp + Davis/Ken memory engine and search keep the DNA line online. Memory curve swings hard on Paildramon → DM → FM.
- Win condition: repeated bounce-to-deck + unsuspend loops (FM/PM) deny the opponent a board while the Imperialdramon line pushes security with Security A.+1, Piercing, and Jamming carried up from the [Free]/[Imperialdramon] inheritances.

## Ranked interactions to test (selected → authored)
1. **Combo 1 — DNA Bounce (ST9-05)** — core payoff, exercises the DNA-only clause + colour requirement + cost + UI prompt.
2. **Combo 2 — DNA Lockdown (BT16-025)** — meta Paildramon; the DNA-vs-regular fork applies a real modifier.
3. **Combo 3 — Colour-Gated Source Replay (ST9-06)** — colour requirement gates the replay; happy + unhappy paths.
4. **Combo 4 — Evolution-cost & colour gate** — alt-path costs/colours pinned.
5. **Combo 5 — UI descriptions** — folded into Combos 1 & 3 as inline prompt assertions.

### Dropped / deferred (logged, not silently truncated)
- **BT12-028 Paildramon** (trash-top-3-sources; *if DNA* 2 opp can't attack) — same DNA-fork shape as Combos 1–2; deferred to avoid redundant coverage.
- **AD1-011 Paildramon** (Partition + battle-immunity + [WA] -2-cost digivolve into Imperialdramon) — strong, but the [WA] self-digivolve spans more machinery; deferred to a later pass.
- **BT16-028 DM → BT16-027/BT12-031 FM** free/cost-reduced digivolve chain — multi-step finisher line; deferred (depends on the Tamer-trigger [All Turns] clause).
- **BT17-077 Paladin Mode** mass source-trash + recycle — apex finisher; deferred (White/Blue, outside the Blue+Green core the user named).
