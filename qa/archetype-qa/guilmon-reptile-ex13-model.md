# Guilmon / reptile (EX13 slice) — Model

Slice of EX13 authored by the `author-set` workflow: **EX13-007 Guilmon**,
**EX13-048 Kotemon**, **EX13-068 Takato Matsuki**. Two independent sub-engines
that share only the [Reptile] Rookie tier:

- **Red Takato / Guilmon** — Takato Matsuki keeps memory at a floor of 3 and
  refreshes itself at the start of each main phase (return to deck bottom →
  replay a [Takato Matsuki] free), reviving a [Guilmon] from trash when the
  board is empty; Gigimon (EX13-001) converts every red-Tamer play into a
  cost-reduced digivolve into [Growlmon]/[Gallantmon].
- **Black Kotemon / Knightmon** — Kotemon tutors the [Knightmon] package off
  the top 3 and, as an inherited source, shields its Knightmon by sacrificing
  another [Knightmon]-text Digimon, whose own [On Deletion] (EX13-058) refills
  the board with a cost-≤4 Knightmon-text card (Kotemon itself).

Sources: official Bandai DB bundles `data/card_bundles/EX13-0{07,48,68}.md`
(and EX13-001 / EX13-058 / EX13-064 / EX4-006 / EX3-057 YAML headers). **No
DCGO C# exists** for EX13-007, EX13-048 or EX13-068 at DCGO `b9a0638cd`, so
printed text + `general_rule.pdf` govern.

## Card pool & roles
| Card | Role | One-line function |
|---|---|---|
| EX13-068 Takato Matsuki (Tamer, red, 4) | engine | [Start of Your Turn] memory ≤2 → set to 3; [Start of Your Main Phase] by returning self to deck bottom, may play a [Takato Matsuki] from hand free; after, if no Digimon, may play a [Guilmon] from trash free; [Security] play self |
| EX13-007 Guilmon (Lv.3 red Reptile) | enabler | [When Moving][On Play] trash 1 from hand → return a [Gallantmon]-named Digimon or a red Tamer from trash to hand; inherited +2000 to DP-deletion maximums — **BLOCKED** (G-ENGINE-DP-DELETION-MAX-MODIFIER) |
| EX13-048 Kotemon (Lv.3 black Reptile) | enabler / engine | [On Play] reveal 3: add 1 [Knightmon]-text + 1 [Knightmon]-name card; inherited [All Turns][OPT] would-leave (not by your effects) → delete another own [Knightmon]-text Digimon instead |
| EX13-001 Gigimon (egg, cross-slice) | enabler | inherited [Your Turn][OPT] when your red Tamer is played → may digivolve into [Growlmon]/[Gallantmon] from hand, cost −2 |
| EX4-006 Guilmon (cross-set) | body | [Guilmon] revival target for Takato (On Play: Rush only if both trashes ≥20) |
| EX3-057 Growlmon (cross-set) | payoff | digivolves from [Guilmon]-named (cost 2); [When Digivolving] delete opp ≤3000 DP else both mill 2 |
| EX13-058 Knightmon (cross-slice) | payoff / engine | [When Attacking][On Deletion] may play/use a [Knightmon]-text card cost ≤4 from hand free |
| EX13-064 LordKnightmon (cross-slice) | payoff | Kotemon reveal hit ([Knightmon] in name ⊂ text) |

## Digivolution lines
- Red: Gigimon (EX13-001) → Guilmon (Red Lv.2 / 0; EX4-006 from [Gigimon] / 0)
  → Growlmon (EX3-057 from [Guilmon] / 2) → Gallantmon.
- Black: Lv.2 black → Kotemon (Black Lv.2 / 0) → Lv.4 → Knightmon (EX13-058,
  Black Lv.4 / 3 or Lv.4 [Knightmon]-text / 3) → LordKnightmon (EX13-064).

## Named combos
### C1 — Takato reload (memory floor + Tamer refresh + Guilmon revival)
- Cards: EX13-068 ×2, EX4-006 Guilmon.
- Expected mechanical outcome: at the start of our turn with ≤2 memory, the
  on-field Takato sets memory to 3; at start of main phase we pay its cost
  (Takato → deck **bottom**), play the second Takato from hand free, then —
  because we control no Digimon (checked *after* the Tamer play) — play
  Guilmon from trash free. Field: [Takato, Guilmon]; memory stays 3.
- Basis: EX13-068 bundle text + Q&A ("if you don't return this card … you
  can't process the part after 'after'"); `general_rule.pdf` start-of-turn /
  start-of-main-phase timing (§ phase procedure).
- Rank: 1 (the deck's core engine turn).

### C2 — Takato refresh feeds Gigimon (red Tamer played → cost-reduced digivolve)
- Cards: EX13-068 ×2, EX13-001 Gigimon (source of EX4-006 Guilmon), EX3-057 Growlmon.
- Expected mechanical outcome: the refreshed Takato is a *red Tamer played on
  our turn*, so Gigimon's inherited trigger offers Growlmon from hand; digivolve
  cost 2 − 2 = 0, memory unchanged; Growlmon's [When Digivolving] deletes the
  opponent's ≤3000 DP Digimon. The Guilmon-revival branch is skipped (we have a
  Digimon).
- Basis: EX13-001 / EX13-068 / EX3-057 printed text.
- Rank: 2.

### C3 — Kotemon tutors the Knightmon package
- Cards: EX13-048 Kotemon, EX13-064 LordKnightmon, EX13-058 Knightmon.
- Expected: reveal 3 → one card per bucket (text bucket accepts named Knightmon
  too), cross-bucket de-dup, remainder to deck bottom; hand +2.
- Basis: EX13-048 bundle + Q&A (text includes name).
- Rank: 3.

### C4 — Kotemon shield: sacrificed Knightmon replays Kotemon
- Cards: EX13-048 ×2, EX13-058 ×2, EX13-064 (reveal hit).
- Expected: opponent's effect would delete the Knightmon-on-Kotemon stack →
  inherited replacement offered; cost offers only the *other* Knightmon;
  paying keeps the stack; the sacrificed Knightmon's [On Deletion] plays Kotemon
  #2 (cost 3 ≤ 4, [Knightmon] in text) from hand free; Kotemon #2's [On Play]
  tutors LordKnightmon. Field count unchanged, memory unchanged.
- Basis: EX13-048 / EX13-058 printed text; replacement "instead" timing.
- Rank: 4.

### C5 — Lone Knightmon falls, its [On Deletion] still replays Kotemon (C4 unhappy path)
- Cards: EX13-048 ×2, EX13-058.
- Expected: no other [Knightmon]-text Digimon → no replacement offer; the stack
  is deleted; Knightmon's [On Deletion] still plays Kotemon from hand free.
- Rank: 5.

### BLOCKED — C6 Guilmon recursion (EX13-007 + EX13-068)
- EX13-007 Guilmon [On Play] trashing 1 → return a red Tamer ([Takato
  Matsuki]) from trash; and EX13-007's inherited DP-deletion max +2000 feeding
  EX13-010 Growlmon. EX13-007 is BLOCKED (engine gap
  G-ENGINE-DP-DELETION-MAX-MODIFIER) — not authored.

## Playstyle
- Red: midrange/value — memory floor at 3 every turn, a self-refreshing Tamer
  that rebuilds a board from an empty field. Black: defensive grind — tutor +
  sacrifice-shield + refill.

## Win conditions
- Red: Gallantmon removal swings off cheap Gigimon digivolves. Black:
  LordKnightmon / Knightmon board that won't leave.

## Ranked interactions to test
1. C1 Takato reload — the engine turn, exercised through the real turn flow.
2. C2 Takato → Gigimon digivolve — cross-card trigger off an effect-played Tamer.
3. C4 Kotemon shield chain — replacement → deletion → On Deletion → On Play.
4. C5 lone-Knightmon unhappy path.
5. C3 Kotemon tutor with real Knightmon cards.
Dropped: Takato [Security] self-play (per-card covered); Takato decline path
(per-card covered); C6 (BLOCKED on EX13-007).
