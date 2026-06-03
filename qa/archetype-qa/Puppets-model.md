# Puppets — Model

> System-level model authored by `/archetype-interaction-test-author` (Phase 2).
> Sources cited inline: DCGO C# paths under
> `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`, card text
> from `data/cards.json`, rules from `Digimon TCG resources/general_rule.pdf`.
> Pool resolved from `data/deck_library.json` ("Puppets", 34 decklists, 62
> unique cards). Static gate (Phase 4): deck legal (4 eggs / 50 main), coverage
> 62/62 = 100% implemented, 5/5 smoke games clean.

## What the deck is

A **Yellow [Puppet]-trait token-engine / control** deck built on the Cinderella /
fairy-tale Shoemon line. Its identity is a **self-deletion value loop**: it
floods the board with disposable **[Familiar] Tokens** (Digimon/Yellow/3000 DP,
`[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn`), then
deliberately deletes its own tokens via the **Overclock** keyword to (a) take
free extra attacks, (b) draw cards off Arisa Kinosaki, (c) stack -DP onto the
opponent's board until things hit 0 DP and die (rule 17-1-3-1: a Digimon at 0
or less DP is deleted). Deletion is a *resource*, not a cost.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| ST19-03 / EX7-024 / BT22-029 / P-134 / EX11-019 Shoemon | enabler | Lv3 base; search ([Puppet]+[LIBERATOR]), cost-reduction, or token-on-deletion |
| ST19-08 / EX7-025 / BT22-032 / P-165 ShoeShoemon | engine | Lv4; On-Deletion / When-Dig free-play a Lv3 [Puppet]; Overclock (ST19-08) |
| ST19-11 / EX7-027 / BT22-036 Chaperomon | engine | Lv5; Overclock; leave-prevention by deleting a Token/[Puppet] |
| ST19-12 / EX7-030 / BT22-040 / EX11-024 Cendrillmon | payoff/engine | Lv6; Overclock; plays [Familiar] Tokens; BT22-040/EX11-024 refire When-Dig on deletion |
| BT22-042 Nyabootmon | payoff (win-con) | Lv7; Overclock; When-Dig free-plays a Puppet + mass -DP scaling on board width |
| EX11-020/EX9-024 Hanimon, EX11-021/EX9-027 Kokeshimon, EX11-022/EX9-032 Karakurumon, EX11-023/EX9-033 Kaguyamon | engine | Yellow/Purple Kaguya line; Scapegoat resilience, trash recursion, removal |
| BT22-088 / EX11-060 / EX7-063 / ST19-14 / EX9-067 / P-136 Arisa Kinosaki | engine (Tamer) | Memory floor, draw-on-deletion, Rush grant, replay loop |
| EX11-061 Mirai Kinosaki | engine (Tamer) | Memory ramp; free Lv3 Puppet on Puppet-digivolve |
| BT13-101 Miki & Megumi | enabler (Tamer) | Free PawnChessmon; draw on B/Y play |
| ST19-04/05 PawnChessmon | enabler | Trash-a-Puppet → Draw 2; Reboot |
| BT22-098 Fable Waltz / P-229 Narrative Ronde | enabler (Option) | Free Shoemon/Arisa, search, Delay digivolve-discount |
| LM-035 Amber / P-037 Yellow Memory Boost!, LM-029 Yellow Scramble | enabler (Option) | Ramp / search |
| BT9-033 Pillomon | tech | `Players can't play Digimon by effects` (mirror / lock) |

## Digivolution lines

- **Shoemon line (mono-Yellow):** Kyaromon/Nyaromon (Lv2 egg, ST19-01/BT22-002/
  BT15-003) → Shoemon (Lv3, evo 0) → ShoeShoemon (Lv4, evo 2) → Chaperomon
  (Lv5, evo 3) → Cendrillmon (Lv6, evo 3) → Nyabootmon (Lv7, evo 4). Mono-Yellow
  gates throughout; Shoemon's cheap evo-0 plus EX7-024's `-1` cost reduction
  and Fable Waltz's `-3` Delay make the climb fast.
- **Kaguya line (Yellow/Purple):** Shoemon/Hanimon (Lv3) → Kokeshimon (Lv4) →
  Karakurumon (Lv5) → Kaguyamon (Lv6). Adds Scapegoat resilience + lowest-level
  removal + trash recursion. Mirai Kinosaki enables it.

## Named combos

### C1 — Overclock value loop (Cendrillmon + Arisa + Familiar Token)
- Cards: ST19-12 **Cendrillmon** (Overclock + When-Dig plays 2 Familiar Tokens),
  EX11-060 **Arisa Kinosaki**, the **[Familiar] Token**.
- Expected mechanical outcome: at end of your turn, Cendrillmon's Overclock
  deletes 1 Familiar Token → Cendrillmon gains a free (un-suspending) attack;
  that token deletion fires (a) the Familiar's `[On Deletion]` -3000 DP onto an
  opponent Digimon, and (b) Arisa's `[All Turns]` "when your Token/[Puppet] is
  deleted, by suspending this Tamer, Draw 1". Net per loop: **+1 card drawn,
  one opponent Digimon -3000 DP, Cendrillmon attacks without suspending.**
- Rules/keyword basis: Overclock keyword (`$BASE_DCGO/.../ST19/Yellow/ST19_12.cs`,
  `EX11/Yellow/EX11_060.cs`); deletion → on-deletion timing (`general_rule.pdf`
  §6 deletion, §16 keyword glossary). The suspend-to-draw "may" must surface
  (no-approximations).
- Rank: **1** (signature engine; touches every Puppets deck).

### C2 — Self-sustaining token refill (Overclock deleter + BT22-040 Cendrillmon)
- Cards: ST19-12 **Cendrillmon** (Overclock, the deleter) OR BT22-040's own
  Overclock, and BT22-040 **Cendrillmon** (When-Dig plays a Familiar Token;
  `[All Turns][Once Per Turn]` "when any of your **other** Digimon are deleted,
  you may activate 1 of this Digimon's `[When Digivolving]` effects").
- Expected mechanical outcome: an Overclock deletes a Familiar Token → BT22-040's
  refire observer offers to re-run its When-Dig → **a fresh Familiar Token is
  played**, so the board count is restored. The loop is OPT-gated: a second
  same-turn deletion does **not** re-offer until the turn cycles.
- Rules/keyword basis: `$BASE_DCGO/.../BT22/Yellow/BT22_040.cs`; OnAnyDeletion
  must evaluate the *deleted object's* owner/kind against "your other Digimon"
  (covered structurally by `tests/cards_behavioral/bt22/bt22_040.rs`). This
  combo asserts the **cross-card** wiring Overclock-deletes → refire-replays.
- Rank: **2** (board-sustain engine).

### C3 — Nyabootmon mass-removal payoff (board-width-gated)
- Cards: BT22-042 **Nyabootmon** + a **wide own board** (tokens/Puppets) + an
  opponent Digimon target.
- Expected mechanical outcome: When-Dig "to 1 of your opponent's Digimon, give
  -3000 DP until their turn ends **for each of your Digimon**." With a wide
  board the cumulative -DP drives the target to ≤0 → **deleted** (rule
  17-1-3-1). The *same* effect over a narrow board leaves the target alive —
  the removal is **gated on board width**, the system-level fact a per-card
  test can't express.
- Rules/keyword basis: `$BASE_DCGO/.../BT22/Yellow/BT22_042.cs`; 0-DP deletion
  rule 17-1-3-1 (see memory `project_dp_zero_deletion.md`); -DP "until their
  turn ends" window.
- Rank: **3** (primary win-con / board wipe).

### C4 — Arisa replay & Shoemon recursion (BT22-088)
- Cards: BT22-088 **Arisa Kinosaki** (in play) + a second **Arisa** (in hand) +
  a **Shoemon** (in trash).
- Expected mechanical outcome: `[Start of Your Main Phase]` returning the
  in-play Arisa to the **bottom of the deck**, play the hand Arisa for free;
  then **if you have no Digimon**, play a Shoemon from trash for free. Asserts
  the multi-zone shuffle: deck +1 (old Arisa bottomed), the new Arisa in play,
  Shoemon recurred from trash when the board is empty. Unhappy path: with a
  Digimon already in play the Shoemon clause is skipped.
- Rules/keyword basis: `$BASE_DCGO/.../BT22/Yellow/BT22_088.cs`; "without
  paying the cost" play; conditional second clause.
- Rank: **4** (consistency / recursion engine).

### C5 — Leave-prevention by token sacrifice (Chaperomon inherit + Familiar)
- Cards: ST19-11 / EX7-027 **Chaperomon** inherited `[All Turns][OPT]` "when
  this would leave the battle area other than by your effects, by deleting 1 of
  your Tokens or other [Puppet] Digimon, prevent it from leaving" + a **Familiar
  Token** as the sacrifice.
- Expected mechanical outcome: opponent tries to delete the carrier → you delete
  a Familiar Token instead → **the carrier stays**. Unhappy path: with no
  Token/[Puppet] to delete, the carrier leaves.
- Rules/keyword basis: `$BASE_DCGO/.../EX7/Yellow/EX7_027.cs`; leave-replacement
  timing (`general_rule.pdf` §6 deletion replacement, see rule 25 deletion
  lifecycle). NOTE: this is an **inherited** effect — the carrier must hold
  Chaperomon as a digivolution source; setup cost is higher, so this is the
  lowest-ranked selected combo.
- Rank: **5** (resilience). *Authored only if C1–C4 land cleanly within scope.*

## Playstyle
- Class: **combo-control** with a token swarm. Tempo comes from cheap evo costs +
  Arisa's memory floor (set to 3) and Overclock free attacks; the deck grinds
  card advantage through self-deletion while shrinking the opponent's board.
- Memory curve: Arisa (EX11-060/ST19-14) sets memory to 3 each turn; Mirai
  (EX11-061) and EX7-063 ramp +1 when the opponent has a Digimon.

## Win conditions
1. **Overclock + Alliance extra attacks** punching multiple security checks
   (Cendrillmon EX11-024 / Kaguyamon Alliance add Security A. +1).
2. **Board wipe via stacked -DP** (Nyabootmon, Familiar On-Deletion, Cendrillmon
   When-Attacking) followed by free attacks into open security.
3. **Grind** — Arisa draw engine + Scapegoat resilience outlasting the opponent.

## Ranked interactions to test
1. **C1** Overclock value loop — signature engine, highest coverage. ✅ authored
2. **C2** Self-sustaining token refill — board-sustain cross-card wiring. ✅ authored
3. **C3** Nyabootmon mass-removal — board-width-gated payoff (happy + unhappy). ✅ authored
4. **C4** Arisa replay & Shoemon recursion — multi-zone consistency engine. ✅ authored
5. **C5** Leave-prevention by token sacrifice — **dropped by rank**: the per-card
   suite `cards_behavioral/ex7/ex7_027.rs` already covers leave-prevention
   exhaustively (happy / decline / OPT-lock / token-cost-to-trash /
   own-effect-no-trigger); the cross-card delta is marginal.

## Additional situations (second wave — 2026-05-30)

Beyond the four headline combos, five more cross-card seams were exercised:

| # | Situation | Verdict |
|---|-----------|---------|
| S2 | Karakurumon (EX11-022) temp-Puppet self-deletion → Arisa (EX11-060) Draw 1 — drawback becomes card advantage | ✅ PASS |
| S4 | ST19-14 Arisa grants <Rush> to one effect-played Familiar Token (suspend cost gates the second) | ✅ PASS |
| S5 | BT22-088 Arisa **play-side** draw (mirror of C1's deletion-side) — Draw exactly once over two token plays | ✅ PASS |
| S1 | Pillomon (BT9-033) flood-gate must block effect-played Familiar Tokens | ⛔ FINDING **G-PLAY-TOKEN-FLOODGATE** |
| S3 | Kaguyamon (EX11-023) trash-recursion must fire on a Familiar Token deletion | ⛔ FINDING **G-EX11-023-TOKEN-DELETION** |

## Findings (both confirmed AND fixed — 2026-05-30)

1. **G-PLAY-TOKEN-FLOODGATE** (engine primitive → `docs/RUST_ENGINE_GAPS.md`) —
   **RESOLVED.** `EffectContext::play_token` now consults
   `CannotPlayDigimonByEffect` and no-ops the spawn when the controller carries
   it (every registered token is a Digimon token), matching DCGO's
   `CanPlayAsNewPermanent` → `CanNotPutFieldClass(IsDigimon)`. Pins:
   `s1_…` + `s1b_…` (interaction) and `play_token_blocked…` /
   `play_token_allowed…` (lib unit tests).
2. **G-EX11-023-TOKEN-DELETION** (card-spec → `qa/archetype-qa/engine-gaps.md`) —
   **RESOLVED.** EX11-023's deletion-recursion condition is now
   `any_of: [digimon, token]` (matching sibling cards BT22-040/EX11-060). Pins:
   `s3_…` (interaction), `ex11_023_other_deletion_recursion_fires_on_familiar_token_deletion`
   (per-card), and the strengthened structural assertion.

## Run record (Phase 6 — 2026-05-30)

- Interaction tests: `code/digimon-engine/tests/archetypes/puppets.rs` —
  **11/11 PASS + 2 `#[ignore]`'d findings** (C1–C4 + S2/S4/S5 pass; S1/S3 pin
  confirmed open divergences). Full `--test archetypes` binary: **41/41 PASS,
  2 ignored** (no regressions).
- Static gate: deck legal (4 eggs / 50 main), coverage 62/62 = 100%, 5/5 smoke
  games, combo-presence 4/4. Recorded in
  `qa/qa-reports/archetype_interactions.json`.
- **First-wave authoring note:** the four headline combos surfaced no engine
  bugs (the only fixes were to the *tests* — a trigger-order assumption, the
  `enter_main_phase()` firing, and a Shoemon-`[On Play]`-confounded deck count).
  The second wave then surfaced the two genuine faithfulness divergences above.
- **No engine/card code was edited** (per skill guardrails) — findings filed to
  the trackers; the two pinning tests flip to un-ignored when fixed.
