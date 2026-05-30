# Rocks — Model

> Archetype-model artifact produced by `/archetype-interaction-test-author`
> (Phase 2). This is a **dry-run exemplar** pinning the model→test→gate flow:
> the "Greymon removal, Koromon-enabled" combo below is authored as an
> interaction test in `code/digimon-engine/tests/archetypes/rocks.rs` and the
> static gates are recorded in `qa/qa-reports/archetype_interactions.json`.
> Sources cited inline (DCGO C# path / `general_rule.pdf` rule).

## Card pool & roles

Rocks is a [Mineral]/[Rock]-trait deck whose engine is **trashing its own
digivolution sources for value** and recurring them. Representative implemented
pieces (per `qa/qa-reports/validated_cards_dsl.json`; 48/56 of the pool
implemented at time of writing):

| Card | Role | One-line function |
|------|------|-------------------|
| BT17-102 Greymon | payoff | `[When Digivolving]` Koromon-gated +3000 DP, then deletes an opponent Digimon with ≤ its DP |
| EX10-028 Golemon | engine | `[On Play][When Digivolving]` trash a Mineral/Rock source for a buff; inherited delete when its source is trashed |
| EX10-034 Blastmon | payoff | `[All Turns][OPT]` when Digimon attack, by trashing 2 sources, board impact |
| EX8-048 Landramon | engine/tech | inherited: when this source is trashed from a Mineral/Rock Digimon, delete an opponent Digimon cost ≤4 |
| EX10-025 Sunarizamon | enabler | `[On Play]` re-bury 2 Mineral/Rock trash cards as bottom sources (refuels the trash-for-value engine) |
| BT14-009 Gotsumon | enabler | low-cost [Rock] rookie / digivolution base |

(The full per-card pool + roles is enumerated from the resolve-deck output; this
table lists the pieces the named combos below reference.)

## Digivolution lines

- **Koromon (Lv.2) → Agumon-name (Lv.3) → BT17-102 Greymon (Lv.4)** — the
  enabler line for the removal combo: Koromon must sit in Greymon's digivolution
  cards for the +3000 buff to fire.
- Rock rookies (e.g. BT14-009 Gotsumon) into the Mineral/Rock mid-game payoffs
  that feed the source-trashing engine.

## Named combos

### Greymon removal, Koromon-enabled  *(authored — `tests/archetypes/rocks.rs`)*

- **Cards:** BT17-102 (Greymon), Koromon (enabler in the stack), an opponent Digimon (target).
- **Expected mechanical outcome:** on Greymon's `[When Digivolving]`, *if* Koromon
  is in its digivolution cards, Greymon gains +3000 DP for the turn (base 5000 →
  8000), **then** deletes 1 opponent Digimon with DP ≤ its (now-buffed) DP. With
  the enabler, a 6000-DP target is inside the window and is deleted; a 12000-DP
  target is outside and survives. **Without** Koromon, Greymon stays at 5000 and
  the 6000-DP target is *spared* — the removal is gated on the enabler.
- **Rules/keyword basis:** `[When Digivolving]` timing window; deletion semantics
  + DP comparison (`general_rule.pdf` §11 DP / §6-2 deletion). DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_102.cs`.
- **Rank:** high — the deck's primary interactive removal; the enabler/target DP
  interplay is exactly the system-level fact no per-card test sees.

### Source-trash value, Golemon  *(candidate — to author)*

- **Cards:** EX10-028 (Golemon) + a Mineral/Rock Digimon with ≥1 digivolution source.
- **Expected mechanical outcome (candidate, pending verification):** Golemon's
  `[On Play]/[When Digivolving]` "by trashing 1 Mineral/Rock source" pays the cost
  off a friendly stack and applies its buff; the trashed source's own inherited
  "when trashed → delete" (e.g. EX8-048 Landramon) then chains. Verify the
  trash-then-inherited-delete ordering against the C# before asserting.
- **Rules/keyword basis:** "by [cost]" = cost paid before the reward;
  inherited-trigger dispatch from a trashed source. DCGO C#:
  `$BASE_DCGO/.../EX10/.../EX10_028.cs`, `.../EX8/.../EX8_048.cs`.
- **Rank:** high — the archetype's value engine; chains across ≥3 cards.

## Playstyle

- **Class:** midrange/control with a combo-removal core. Tempo comes from
  cost-efficient source-trashing rather than raw aggression.
- **Memory curve:** builds a Mineral/Rock board, then converts trashed sources
  into removal + recursion; closes once the opponent's board is suppressed.

## Win conditions

- Grind the opponent's board down with repeated source-trash-fueled deletion
  (Greymon / Landramon / Golemon), then attack through an empty board with the
  surviving Rock payoffs.

## Ranked interactions to test

1. **Greymon removal, Koromon-enabled** — authored (`rocks.rs`); the
   enabler-gated DP window. *(done)*
2. **Source-trash → inherited delete chain (Golemon + Landramon)** — the value
   engine across ≥3 cards; verify trash/delete ordering vs C# first.
3. **Blastmon attack-trigger, by-trashing-2-sources** — OPT payoff; verify the
   cost (trash exactly 2) and the once-per-turn lockout in a multi-attacker turn.

> Capped at the top interaction for this dry-run; combos 2–3 are logged here as
> the next authoring targets (Phase 3 cap, not silently dropped).
