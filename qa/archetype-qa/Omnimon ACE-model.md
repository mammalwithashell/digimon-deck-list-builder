# Omnimon ACE — Model

> Durable system model for the **Omnimon ACE** archetype (slug `omnimon_ace`),
> authored by `/archetype-interaction-test-author` (Phase 2). Sources are cited
> inline per the family convention: printed text (`data/cards.json` /
> `card_overrides.json`), `general_rule.pdf` §16 (keyword/timing), and the
> battle-tested DCGO C# at
> `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD>.cs`. Per CLAUDE.md
> source priority, `general_rule.pdf` (canonical) + DCGO outrank the card-text
> JSON.
>
> Pool resolved via `python code/tools/resolve_deck.py "Omnimon ACE" --json`
> (1 decklist). Per-card DSL verdicts read-only from
> `qa/qa-reports/validated_cards_dsl.json` (all combo cards = IMPLEMENTED).
> The interaction tests live in
> `code/digimon-engine/tests/archetypes/omnimon_ace.rs`.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT17-095 Miraculous Mega Knight (Option, Red+Blue, cost 2) | **engine / payoff-enabler** | [Main] free-play 1 [Agumon]/[Gabumon] from hand or trash, then seat self as a `<Delay>`; the Delay later DNA-digivolves an own L6 Greymon/Garurumon + a hand L6 into an [Omnimon] from hand; inherited [Security] free-plays a [Tai Kamiya]/[Matt Ishida] Tamer and returns self to hand. |
| BT17-078 Omnimon (Digimon, L7, White) | **payoff** | DNA-digivolve [On Play][When Digivolving]: bottom-deck the whole same-level opp cohort, then delete 1 opp Digimon. |
| BT20-102 Omnimon (X Antibody) (Digimon, L7, Red+Blue) | **payoff** | [On Play][When Digivolving] (gated on an [Omnimon]/[X Antibody] **source**): protect 1 Digimon per player, delete all other Digimon, then bottom-deck 1 opp Digimon. Also `<Raid>/<Piercing>/<Blocker>` + an [EOT][OPT] Rush attack. |
| EX4-073 Omnimon Alter-B (Digimon, L7) | payoff (alt) | Sibling Omnimon top-end; not exercised in the authored combos (BT17-078 / BT20-102 are the modelled payoffs). |
| BT17-015 / BT14-101 WarGreymon (Digimon, L6) | **DNA material** | The Greymon-named L6 half of the Omnimon DNA recipe; also the BT17-095 Delay's leaving subject. |
| BT17-027 / BT15-101 MetalGarurumon (Digimon, L6) | **DNA material** | The Garurumon-named L6 half of the Omnimon DNA recipe. |
| BT17-007 / BT12-059 / ST20-10 Agumon, BT17-019 / EX4-039 Gabumon (Digimon, L3) | enabler (recursion target) | The [Agumon]/[Gabumon] bodies BT17-095 [Main] free-plays from hand or trash. |
| BT17-102 Greymon (Digimon, L4) | enabler | Mid-line body toward WarGreymon. |
| BT17-081 Tai Kamiya & Matt Ishida, EX4-061, BT17-093, BT21-102, ST15-14 Tamers | enabler / tech | [Tai Kamiya]/[Matt Ishida]-named Tamers — the BT17-095 inherited [Security] free-play targets; memory engine. |
| BT16-082 Ukkomon, P-130 Lui Ohwada | tech | Support / colour fixing. |
| BT14-001 Koromon (DigiEgg) | egg | Breeding-line root. |

## Digivolution lines

- **Red Agumon line:** Koromon (BT14-001) → Agumon (BT17-007 / etc.) → Greymon
  (BT17-102) → WarGreymon (BT17-015 / BT14-101, L6).
- **Blue Gabumon line:** → Gabumon (BT17-019 / EX4-039) → … → MetalGarurumon
  (BT17-027 / BT15-101, L6).
- **DNA top-end:** WarGreymon **+** MetalGarurumon → **Omnimon (BT17-078)** or,
  over an [Omnimon] source, **Omnimon (X Antibody) (BT20-102)** (alt-path:
  digivolve from an "Omnimon"-named source for cost 2 — `BT20-102.yaml`
  `alt_paths`). DNA digivolve = jogress: two L6 materials merge under the L7.

## Named combos

### Combo 1 — Miraculous Mega Knight [Main]: free Agumon/Gabumon + arm the Delay
- Cards: BT17-095 + a free-played [Agumon] body (BT17-007).
- Expected mechanical outcome: playing BT17-095 pays only its own use cost (2);
  the [Main] free-plays 1 [Agumon]/[Gabumon] from hand **or trash** for 0
  memory ("without paying the cost"); then the mandatory "place this card in the
  battle area" tail seats BT17-095 itself as a `<Delay>` Option permanent (it is
  **not** trashed).
- Rules/keyword basis: printed [Main]; `<Delay>` `general_rule.pdf` §16-16;
  DCGO `BT17_095.cs` Clause A (`OptionSkill`: per-zone `canNoSelect: true`
  free-play via `PlayPermanentCards(payCost:false)`, then
  `PlaceDelayOptionCards`).
- Rank: high (the deck's central engine; play-frequency 1.0, payoff-central).
- **Status (2026-06-02): BLOCKED on an engine gap.** BT17-095 is a *Standard*
  Option (no `kind: delay` clause); it seats itself via the DSL
  `place_self_as_delay_option` step inside its [Main] body. On the **real**
  `Game::play_option_from_hand` lifecycle the Option card is moved into the
  single-occupancy `pending_option` slot *before* the [Main] body runs, so
  `place_self_as_delay_option_permanent` (whose non-security branch scans only
  the controller's hand/trash) finds nothing and **no-ops**; `dispose_option`
  then trashes the Standard Option. Net: on the path the deck actually uses,
  BT17-095 goes to **trash**, not to the battle area as a Delay, and (in the
  declining variant) trash goes +1. This is reproducible — see
  `docs/RUST_ENGINE_GAPS.md` → **G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH**.
  The per-card test `bt17_095.rs` sidesteps the gap by driving the [Main] via
  `Game::activate_hand_main` (which leaves the card in hand and never runs the
  Option disposal lifecycle, so the place-self step finds the card in hand). An
  *interaction* test must exercise the real play path; the two Combo-1 tests are
  therefore `#[ignore]`d pending the gap, not weakened.

### Combo 2 — Miraculous Mega Knight `<Delay>`: leave-trigger → Omnimon DNA from hand
- Cards: BT17-095 (seated Delay) + BT17-015 WarGreymon (the leaving L6) +
  BT17-027 MetalGarurumon (L6 hand partner) + BT17-078 Omnimon (result in hand).
- Expected mechanical outcome: with BT17-095 seated as a Delay, when an own L6
  [Greymon]/[Garurumon] would leave the battle area **outside of a battle**, the
  `<Delay>` fires: BT17-095 trashes itself (Delay cost), then the leaving L6 + a
  hand L6 DNA-digivolve into the [Omnimon] card from hand. The merged Omnimon's
  stack = leaving-subject sources ++ [hand L6 partner] ++ [Omnimon]; the leaving
  subject is **consumed into the merge** (replacement Cancelled, not trashed);
  hand −2.
- Rules/keyword basis: printed [All Turns] `<Delay>` leave-watcher; "outside of a
  battle" gate; DNA-digivolve with the second material drawn from hand. DCGO
  `BT17_095.cs` Clause B (`WhenRemoveField` + `!IsByBattle` + `SetJogress`
  with the hand-card partner materialised). Engine: the closed
  `effect_initiated_dna_digivolve_with_hand_partner` primitive
  (G-DSL-DNA-FROM-HAND-PARTNER, resolved 2026-05-20).
- Rank: high. **Status: GREEN.**
- Unhappy path: a **battle-cause** leave does NOT trigger the Delay (the
  "outside of a battle" gate). **Status: GREEN.**

### Combo 3 — Miraculous Mega Knight inherited [Security]: off-turn Tamer free-play
- Cards: BT17-095 (in the defender's security) + BT17-081 Tai Kamiya & Matt
  Ishida (the free-played Tamer).
- Expected mechanical outcome: when BT17-095 is security-checked, its inherited
  [Security] free-plays 1 [Tai Kamiya]/[Matt Ishida] card from hand **or trash**
  (0 memory), then adds BT17-095 itself to the **hand** (not trash).
- Rules/keyword basis: printed inherited [Security]; DCGO `BT17_095.cs` Clause C
  (`SecuritySkill` + `PlayPermanentCards(payCost:false)` + `AddThisCardToHand`).
  Driven through the real combat/security-check path (`attack_player`) — the
  `enqueue_triggered(SecuritySkill, …)` shortcut is a no-op for inherited
  [Security] clauses after PR #490.
- Rank: medium. **Status: GREEN.**
- Unhappy path: with no eligible Tamer anywhere, nothing is played but the
  mandatory add-to-hand tail still returns BT17-095 to hand. **Status: GREEN.**

### Combo 4 — Omnimon (BT17-078) DNA digivolve: same-level cohort wipe + delete
- Cards: BT17-078 + BT17-015 WarGreymon + BT17-027 MetalGarurumon.
- Expected mechanical outcome: DNA-digivolving the two L6 materials into BT17-078
  fires its [When Digivolving] (DNA path): choose 1 opp Digimon → return **all**
  opp Digimon of that same level to the bottom of the deck, then delete 1 opp
  Digimon. The whole same-level cohort is swept, plus a separate delete.
- Rules/keyword basis: printed "[On Play][When Digivolving] If DNA digivolving,
  …"; DCGO `BT17_078.cs` (`IsJogress`-gated bottom-deck + delete). Driven via the
  engine's own `effect_initiated_dna_digivolve` primitive (`dna_origin=true`).
- Rank: high. **Status: GREEN.**
- Unhappy path: a **non-DNA** play of BT17-078 does NOT get the body (DNA-origin
  gate). **Status: GREEN.**

### Combo 5 — BT20-102 X-Antibody: protect-1-per-player mass deletion + bottom
- Cards: BT20-102 Omnimon (X Antibody) over a BT17-078 Omnimon **source**.
- Expected mechanical outcome: digivolving into BT20-102 over a stack whose
  digivolution **cards** contain [Omnimon]/[X Antibody] fires [When Digivolving]:
  choose 1 of **both** players' Digimon to protect, delete every other Digimon
  (yours and opponent's), then return 1 surviving opp Digimon to the bottom of
  the deck. The controller may choose to protect **BT20-102 itself** on their
  side — so it survives its own wipe while an unprotected own ally is deleted.
- Rules/keyword basis: printed text; DCGO `BT20_102.cs` (`IsOmniOrXAntiSource`
  gate scanning *sources only* + protect-1-per-player wipe + `PutLibraryBottom`).
  Engine: `self_digivolution_sources_contain_name` (sources-only scan, excludes
  the carrier's own top-card name — G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY,
  resolved 2026-05-19) + `for_each` exclude-binding wipe
  (G-FOR-EACH-EXCLUDE-BINDING, resolved 2026-05-20).
- **Faithfulness note (drives the test fix):** the printed text lets the
  controller protect *any one* of their own Digimon. The per-side protect-pick is
  the player's choice, so an interaction test must drive the own-protect pick
  **deterministically to BT20-102 itself** (via `encode_attack(0, slot)` for
  BT20-102's battle-area slot) before it may assert "BT20-102 survives, the
  unprotected ally is deleted". Asserting that outcome while driving with a
  blind first-valid resolver is over-specified — either own Digimon may legally
  be the protected one (the original bug the Reviewer flagged).
- Rank: high. **Status: GREEN after the deterministic-protect fix.**
- Unhappy path: a BARE BT20-102 (no [Omnimon]/[X Antibody] in its digivolution
  **sources** — its own top-card name is excluded from the sources-only scan)
  does NOT fire the body. **Status: GREEN.**

## Playstyle

- **Class:** midrange/combo. The deck ramps two colour lines to a pair of L6
  jogress materials and converts them into an Omnimon board-clear, with
  BT17-095 acting as a reusable value engine (cheap body recursion + a Delay
  that recoups an Omnimon for free from a leaving L6, plus security-side tempo
  on defence).
- **Memory curve:** the Tamers (Tai Kamiya / Matt Ishida family) supply the
  memory engine; BT17-095 [Main] is cheap (cost 2) and refunds its body for
  free, so the Option's net swing is just its own cost.

## Win conditions

- Land an Omnimon ([BT17-078] level-cohort wipe, or [BT20-102] X-Antibody
  protect-1 mass deletion) to clear the opponent's board, then close with the
  Omnimon body (BT20-102 carries `<Raid>/<Piercing>/<Blocker>` and an [EOT]
  Rush attack). BT17-095's Delay can re-buy an Omnimon for free off a leaving
  WarGreymon/MetalGarurumon.

## Ranked interactions to test

1. **Combo 4** — BT17-078 DNA cohort-wipe + delete (high payoff, deterministic
   board diff). GREEN.
2. **Combo 5** — BT20-102 X-Antibody protect-1 mass deletion + bottom (high
   payoff; the source-gate and per-side protect are the subtle facts). GREEN
   after the deterministic-protect fix.
3. **Combo 2** — BT17-095 Delay leave-trigger → Omnimon DNA from hand (the
   four-card chain a per-card test can't express). GREEN.
4. **Combo 3** — BT17-095 inherited [Security] off-turn Tamer free-play + return
   to hand. GREEN.
5. **Combo 1** — BT17-095 [Main] free-play + Delay-seat. **BLOCKED** on
   G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH (the place-self step does not
   compose with the real Option-play disposal lifecycle); both Combo-1 tests are
   `#[ignore]`d pending the engine fix rather than weakened or routed through the
   `activate_hand_main` bypass.

### Interactions considered but NOT separately authored (logged, not silently dropped)
- EX4-073 Omnimon Alter-B as an alternate payoff: redundant with the BT17-078 /
  BT20-102 payoff coverage; its distinct body is per-card-test territory, not a
  cross-card combo.
- BT12-059 Agumon [On Play] reveal-4 search: an enabler (single-card draw
  engine), covered by per-card testing; no multi-card interaction beyond feeding
  the lines already modelled.
