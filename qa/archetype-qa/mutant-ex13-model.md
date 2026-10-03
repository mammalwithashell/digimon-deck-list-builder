# Mutant / Mamemon (EX13 slice) — Model

Slice of the EX13 author-set run: EX13-050 Bokomon, EX13-053 Thundermon,
EX13-054 Nanimon, EX13-031 KingSukamon, EX13-059 BigMamemon, EX13-063
PrinceMamemon. Printed text from the official Bandai DB bundles
(`data/card_bundles/<ID>.md`). **No DCGO C# exists for any of the six**
(`$BASE_DCGO/Assets/Scripts/CardEffect/EX13/...` has no `EX13_0{31,50,53,54,59,63}.cs`),
so printed text + official Q&A + `general_rule.pdf` govern.

Per-card status (`qa/qa-reports/validated_cards_dsl.json`):

| Card | Status |
|---|---|
| EX13-050 Bokomon | IMPLEMENTED |
| EX13-053 Thundermon | IMPLEMENTED |
| EX13-054 Nanimon | IMPLEMENTED |
| EX13-059 BigMamemon | IMPLEMENTED |
| EX13-031 KingSukamon | **BLOCKED** (dsl — base name/colour/DP rewrite payload) |
| EX13-063 PrinceMamemon | IMPLEMENTED (2026-10-01; `G-ASSEMBLY-NO-DISTINCT-BY` resolved) |

## Card pool & roles

| Card | Role | One-line function |
|---|---|---|
| EX13-050 Bokomon | tech / enabler | Lv.3 black. [All Turns] neither player gains memory except from Tamer effects (both players — official Q&A). Inherited <Blocker>. |
| EX13-053 Thundermon | engine | Lv.4 black. [OP][OD] may return ≤3 [Mamemon]-text Digimon from trash to deck top, then delete 1 opp Digimon cost ≤ 3 + returned. (Rule) name treated as including [Mamemon]. Inherited [OD] <De-Digivolve 1>. |
| EX13-054 Nanimon | tech / enabler | Lv.4 black, Rule-granted [Mutant]. [Security] play after battle. [OP][OD] 1 opp Digimon can't attack players until their turn ends. Inherited +1000 DP. |
| EX13-059 BigMamemon | payoff / engine | Lv.5 black. [WD][OD] reveal 3, may play 1 cost ≤7 [Mamemon]-name or [Mutant] Digimon free. [EoYT][OPT] by deleting 1 own [Mamemon]-named Digimon, delete 1 opp lowest play cost Digimon. Inherited EoYT: delete ALL opp lowest. |
| EX13-031 KingSukamon | payoff (BLOCKED) | Lv.5 yellow Sukamon line; base name/colour/DP rewrite. |
| EX13-063 PrinceMamemon | finisher (BLOCKED) | Lv.6 black. Reveal-3 cost ≤10; [OD] delete opp highest; [Mamemon]-named Digimon gain <Blocker>+<Guard>; Assembly −4. |

## Digivolution lines

- Black Lv.2 → **Bokomon** (cost 0) → **Thundermon** / **Nanimon** (Black or Yellow Lv.3, cost 2)
  → **BigMamemon** (Black Lv.4, cost 3) → PrinceMamemon (Black Lv.5, cost 3; BLOCKED).

## Named combos

### C1 — Thundermon's [Mamemon] alias pays BigMamemon's end-of-turn cost (double removal)
- Cards: EX13-059, EX13-053.
- Expected outcome: at end of turn BigMamemon's optional cost offers BOTH
  BigMamemon and Thundermon (Thundermon's (Rule) name includes [Mamemon]).
  Deleting Thundermon: BigMamemon finishes its effect (deletes opp lowest play
  cost), then Thundermon's [On Deletion] returns a [Mamemon]-text card to the
  deck top and deletes a second opp Digimon at cost ≤ 4. Net: −1 own, −2 opp,
  deck +1.
- Basis: printed text EX13-053 (Rule) name, EX13-059 EoYT; `general_rule.pdf`
  15-7 (processing conditions "By X, Y"), 15-8-3-2 (triggers wait as pending
  until the current effect finishes).
- Rank: 1 (the archetype's core removal loop).

### C2 — Thundermon stacks the deck → BigMamemon [When Digivolving] replays it
- Cards: EX13-053 (×2), EX13-059 (×2).
- Expected outcome: Thundermon [On Play] returns Thundermon#2 then BigMamemon#2
  to the top (cap 3+2=5, deletes a cost-5); digivolving BigMamemon draws the
  top (BigMamemon#2), its reveal finds Thundermon#2 and plays it free, whose
  [On Play] fires again (no [Mamemon] text left in trash → cap 3) and deletes a
  cost-3. Memory −4 −3 only.
- Basis: printed text; Thundermon returns in pick order (last = top);
  digivolution draw (rules §6 digivolve procedure).
- Rank: 2.

### C3 — BigMamemon sacrifices itself, carrying Thundermon's inherited De-Digivolve
- Cards: EX13-053 (as source), EX13-059, EX13-054 (reveal hit).
- Expected outcome: EoYT, BigMamemon pays its own cost (it has [Mamemon] in its
  name), deletes the opp lowest-cost Digimon, then BOTH [On Deletion]s trigger
  together: BigMamemon's reveal plays Nanimon (Rule [Mutant]) free → Nanimon
  [On Play] locks an opp Digimon from attacking players; Thundermon's inherited
  <De-Digivolve 1> strips the opp stack.
- Basis: printed text; `general_rule.pdf` 15-8-3 simultaneous triggers (turn
  player orders), 16 <De-Digivolve>.
- Rank: 3.

### C4 — Bokomon → Thundermon: inherited <Blocker> trade kills the attacker
- Cards: EX13-050 (source), EX13-053, EX13-059 (trash fodder).
- Expected outcome: Thundermon digivolved on Bokomon has <Blocker> (and no
  [On Play] on digivolve). Blocking a 5000-DP cost-4 attacker gets Thundermon
  deleted; its [On Deletion] returns 1 [Mamemon] card → cap 4 → deletes the
  attacker. Unhappy path: on a vanilla Lv.3 base there is no block prompt.
- Basis: `general_rule.pdf` §16 <Blocker>; printed text.
- Rank: 4.

### C5 — Nanimon → BigMamemon: +1000 inherited, reveal lands Bokomon's memory lock
- Cards: EX13-054 (source), EX13-059, EX13-050 (reveal hit).
- Expected outcome: BigMamemon on Nanimon is 9000 DP; its reveal plays
  Bokomon ([Mutant], cost 3) free; afterwards a Digimon-sourced memory gain is
  blocked. Control (play declined): the same gain goes through.
- Basis: printed text + official Q&A (EX13-050 affects both players).
- Rank: 5.

### Blocked combos (not authored)
- PrinceMamemon <Guard>/<Blocker> aura over Thundermon/BigMamemon — EX13-063 BLOCKED.
- PrinceMamemon Assembly −4 from a trash of [Mamemon]-text cards — EX13-063 BLOCKED.
- BigMamemon inherited "delete ALL lowest" under PrinceMamemon — EX13-063 BLOCKED.
- PrinceMamemon [OD] chain (reveal ≤10 + delete highest) — EX13-063 BLOCKED.
- KingSukamon Sukamon rewrite + inherited Sukamon-deleted reveal — EX13-031 BLOCKED.

## Playstyle
- Black midrange/control. Board removal on every [On Play]/[On Deletion]/EoYT,
  self-sacrifice turning one's own Mamemon into card advantage; Bokomon as a
  symmetric ramp-denial tech.

## Win conditions
- Out-remove the opponent with repeated Thundermon/BigMamemon deletions, then
  swing with a protected (PrinceMamemon) board.

## Ranked interactions to test
1. C1 — core removal loop + the (Rule) alias as a cost payer.
2. C2 — deck-top setup feeding the reveal; double [On Play].
3. C3 — self-sacrifice + simultaneous [On Deletion] chain (incl. Nanimon's Rule trait as a reveal hit).
4. C4 — Bokomon's inherited Blocker enabling Thundermon's on-death removal.
5. C5 — inherited DP + reveal landing the floodgate.

Dropped (not selected): Nanimon [Security] play-after-battle (single-card,
covered by `ex13_054.rs`); Bokomon floodgate vs Tamer memory (single-card,
covered by `ex13_050.rs`).
