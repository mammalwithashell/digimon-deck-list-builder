# Dracomon / dragon (EX13 slice) — Model

Slice of EX13 authored by the `author-set` workflow: EX13-008, EX13-018, EX13-039,
EX13-021, EX13-041, EX13-044. Payoff EX13-045 Examon (EX13, outside the slice,
already implemented) is pulled in because the slice exists to reach it. Static
harness resolves this pool as the `Examon` archetype (37 unique cards).
Interaction tests: `code/digimon-engine/tests/archetypes/dracomon_ex13.rs`.

Sources: per-card printed text `code/digimon-engine/cards/ex13/<ID>.json` (agrees
with `data/card_bundles/<ID>.md`); DCGO `$BASE_DCGO/Assets/Scripts/CardEffect/EX13/<COLOR>/EX13_0xx.cs`;
`general_rule.pdf` (DNA digivolution, Assembly, battle resolution).

## Card pool & roles

| Card | Status | Role | One-line function |
|---|---|---|---|
| EX13-008 Dracomon (Lv.3 R) | IMPLEMENTED | enabler / engine | [When Moving][On Play] reveal 3, add 1 [Dracomon]/[Examon]-text card; inherited [End of Your Turn] may DNA digivolve with another Digimon into a hand card (DCGO `Red/EX13_008.cs`). |
| EX13-018 Coredramon (Lv.4 B/R) | IMPLEMENTED | engine (draw) | Lv.3 [Dracomon]-name route cost 2; [OP][WD] trash 1 [Dracomon]/[Examon]-text card → Draw 2; [Your Turn] other text-Digimon played → may digivolve into [Examon]-text hand card −2; inherited [Your Turn] +2000 (DCGO `Blue/EX13_018.cs`). |
| EX13-039 Coredramon (Lv.4 G/R) | IMPLEMENTED | engine (recursion) | Same route/observer/inherited; [OP][WD] may return 1 non-egg [Dracomon]/[Examon]-text card from trash (DCGO `Green/EX13_039.cs`). |
| EX13-021 Wingdramon (Lv.5) | **BLOCKED** | bridge / DNA half | "treated as Lv.6 [Slayerdramon] for [Examon]'s DNA" — G-DNA-MATERIAL-TREATED-AS-FOR-TARGET. |
| EX13-041 Groundramon (Lv.5) | **BLOCKED** | bridge / DNA half | "treated as Lv.6 [Breakdramon] for [Examon]'s DNA" — same gap. |
| EX13-044 Breakdramon (Lv.6 G/R) | IMPLEMENTED | payoff / control | Assembly −5 (Lv.5×Lv.4×Lv.3 text cards from trash); Piercing, Blocker; [OP][WD] suspend up to 2 (either side) then lock 2 opponent; face + inherited [All Turns][OPT] own Digimon suspends → a text Digimon battles (DCGO `Green/EX13_044.cs`). |
| EX13-045 Examon (Lv.7, cross-slice) | IMPLEMENTED | finisher | DNA Green Lv.6 + Blue Lv.6 cost 0; DNA [WD] all own +10000 and it attacks; [Your Turn][OPT] wins a battle → play/use cost ≤12 text card from hand or its sources free (DCGO `Green/EX13_045.cs`). |

## Digivolution lines

- Lv.2 red / [Bebydomon] → **Dracomon** (cost 0) → **Coredramon** (cost 2 via "Lv.3 w/[Dracomon] in name"; the red Lv.3 circle also offers cost 3 — a real red Dracomon satisfies both, so the engine asks) → Lv.5 [Wingdramon]/[Groundramon] (*blocked*) → **Breakdramon** (cost 3 from those names, 4 from G/R Lv.5) → **Examon** (cost 5 from G/R/B Lv.6).
- **Breakdramon by Assembly −5** from trash: Lv.5 + Lv.4 + Lv.3, each with [Dracomon]/[Examon] in its text (play for 7).
- **Examon by DNA** (Green Lv.6 + Blue Lv.6, cost 0), reachable at end of turn through Dracomon's inherited effect.

## Named combos

### C1 — Dracomon search → Coredramon name-route → trash-to-draw
- Cards: EX13-008, EX13-018, EX13-039 (as the trashed cost).
- Outcome: reveal finds Coredramon; digivolve onto Dracomon for 2 (both 2 and 3 offered); WD trashes a text card for Draw 2. From a 2-card hand: memory −5, hand 3, 1 trash.
- Basis: DCGO `EX13_008.cs` (SimplifiedRevealDeckTopCardsAndSelect, HasText), `EX13_018.cs` (ContainsCardName cost 2; discard → DrawClass(2)).
- Rank: 1 (every game's opener).

### C2 — Green Coredramon recursion → replayed Dracomon re-searches
- Cards: EX13-039, EX13-008 ×2, EX13-018.
- Outcome: WD returns the trashed Dracomon; replaying it re-fires its [On Play] reveal AND EX13-039's [Your Turn] observer (declined — no [Examon]-text Digimon in hand). Memory −2 −3.
- Basis: DCGO `EX13_039.cs` (SelectCardEffect Root.Trash; OnEnterFieldAnyone), `EX13_008.cs`.
- Rank: 2.

### C3 — Coredramon discard fuels Breakdramon Assembly
- Cards: EX13-018, EX13-008, EX13-039, EX13-044 (+ a Lv.5 text card).
- Outcome: Coredramon's draw cost trashes the Lv.3 piece; Breakdramon plays for 12 − 5 = 7 with all three under it. Unhappy: without the Lv.3 piece it is unreduced (full 12, turn passes).
- Basis: DCGO `EX13_044.cs` AddAssemblyConditionClass (reduceCost 5); `general_rule.pdf` Assembly.
- Rank: 3.

### C4 — Breakdramon suspends its own Coredramon → OPT battle
- Cards: EX13-044, EX13-018.
- Outcome: [OP] suspend may pick your own Digimon; that is "any of your Digimon suspend", so Breakdramon's face OPT lets Coredramon (text names [Dracomon]) battle and delete a 3000 Digimon; lock lands on the opponent; a second own-suspend that turn doesn't re-trigger.
- Basis: DCGO `EX13_044.cs` (Tap over either side; OnTappedAnyone → IBattle).
- Rank: 4.

### C5 — Examon over Breakdramon: attack → inherited battle → win → free Breakdramon
- Cards: EX13-045, EX13-044 (source), EX13-018/008.
- Outcome: Examon attacking suspends it → Breakdramon's INHERITED OPT lets Examon battle another Digimon → 15000 wins → Examon's OPT plays Breakdramon from its own digivolution cards free.
- Basis: DCGO `EX13_044.cs` (inherited OnTappedAnyone), `EX13_045.cs` (CanTriggerWhenWinBattle → PlayPermanentCards payCost:false).
- Rank: 5 (the deck's main value loop).

### C6 — Dracomon inherited EoT DNA → Examon end-of-turn attack
- Cards: EX13-008 (under Breakdramon), EX13-044 (Green Lv.6), ST2-10 Plesiomon (vanilla Blue Lv.6), EX13-045.
- Outcome: at End of Turn the carrier + partner DNA into Examon for 0; both stacks merged; DNA WD gives +10000 (27000 with the Coredramon inherited +2000, still your turn) and forces an attack — 2 security checked (Security A.+1).
- Basis: DCGO `EX13_008.cs` (OnEndTurn DNADigivolvePermanentsIntoHandOrTrashCard), `EX13_045.cs` (AddJogressConditionClass green6+blue6 cost 0; IsJogress attack + DP).
- Rank: 6 (the closer).

### Blocked combos (not authored)
- Coredramon observer → [Groundramon] (EX13-041) / [Wingdramon] (EX13-021) for −2 — BLOCKED on the missing Lv.5s (the only [Examon]-text Lv.5 targets).
- [Groundramon]/[Wingdramon] → Breakdramon alt digivolve cost 3 — BLOCKED (EX13-021, EX13-041).
- Wingdramon-as-[Slayerdramon] / Groundramon-as-[Breakdramon] for Examon DNA — BLOCKED (G-DNA-MATERIAL-TREATED-AS-FOR-TARGET).

## Playstyle
- Midrange combo: Dracomon/Coredramon dig and refill (card advantage + trash setup), Breakdramon tempo-locks the board, Examon closes. Cheap curve (3 → 2 → 7 Assembly), then a free DNA.

## Win conditions
- End-of-turn DNA Examon swinging into security with +10000 and Security A.+1; Breakdramon locks blockers out (can't unsuspend) and its suspend-battle clause clears the board; Examon's win-battle loop replays Breakdramon from its sources for free.

## Ranked interactions to test
1. C1 — opener; pins the dual-cost choice and the in-text search hit.
2. C2 — recursion loop; pins the double trigger on replay.
3. C3 — the trash-fills-Assembly system fact.
4. C4 — self-suspend as an enabler (own-target suspend + OPT).
5. C5 — inherited battle → win → free play from sources.
6. C6 — end-of-turn DNA closer.

Dropped/not selected: the four blocked combos above; Dracomon [When Moving] out of breeding (per-card covered, `ex13_008.rs`); Breakdramon Piercing/Blocker (keyword-only, per-card covered).

## Test-fixture note
DSL-loaded `CardData` carries empty printed text (known — see `docs/RUST_ENGINE_GAPS.md` "invisible to the embedded DSL pack (empty text fields)"), so the suite copies each real card's printed text from its per-card JSON (`with_printed_text`) — every "[Dracomon] or [Examon] in its text" filter scans it in production. Removing that step fails 5 of 7 tests, confirming it is load-bearing, not cosmetic.
