# Recording games from the installed DCGO client (MelonLoader)

Goal: record games played in the **real, installed DCGO client** (including PvP) as the same
JSONL the Unity-project recorder writes (`docs/DCGO_RECORDING_SCHEMA.md`), so they replay through
`dcgo-replay` and can seed training data. The installed client is the official build, so PvP keeps
working; a self-built client would not match the online game version.

Decision (2026-11-08): **MelonLoader mod**, as DCGO-Tweaks does.

## What the install actually is

`DCGO_Application/Game/DCGO.exe` is an **IL2CPP** build (`GameAssembly.dll` + `il2cpp_data`, no
`Assembly-CSharp.dll`). Consequences:

- The Unity-project recorder is compiled *into* the game and hooked inside ~29 game methods
  (CLAUDE.md rule 27). A player install cannot be edited that way. Each hook has to be re-expressed
  as a **Harmony patch** over the interop assemblies MelonLoader generates
  (`MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll`, ~65 MB, game class names intact).
- MelonLoader 0.7.2+ supports this build (DCGO-Tweaks runs on it). Verified here with v0.7.3.
- Names are readable in `global-metadata.dat`: of the recorder's hook sites only the recorder's own
  additions (`GameRecorder`, `HarnessAuto`, `DetermineAltPath`) are absent, as expected.
- The shipped build can differ from the DCGO submodule the engine's card scripts were read from.
  The recorder must be written against the shipped signatures (`signatures.txt`, below), and card
  behaviour in a recording is still compared to our engine, not to the submodule.

## What exists (spike 2)

`code/dcgo-melon-recorder/` - a MelonLoader mod (netstandard2.0; the machine has no net6 targeting
pack and MelonLoader hosts .NET 6). It writes **raw events only**, not schema rows yet. It:

1. dumps every member (methods, fields, properties, nested types, base chain) of the hooked types
   to `UserData/DcgoRecorder/signatures.txt`, plus a scan of all game types named like
   `Select*` / `*Action` / `*Effect` / `*Skill`;
2. Harmony-patches **18** methods with a postfix that appends one JSON line (method name, boxed
   arguments, UTC time, sequence number) to `UserData/DcgoRecorder/raw-<session>.jsonl`.

Proven on a **sandbox copy** of `Game/` (`D:\DCGO_mod_sandbox\Game`): MelonLoader boots the game,
generates the interop assemblies, loads the mod, and Harmony patches all 18 methods. **Not yet
proven:** that they *fire* during a real game (needs a game played in the sandbox), and nothing
has been installed into the real `DCGO_Application`.

### Why this is easier than first feared

Every selection result and every action reaches the game through a **plain, non-coroutine
method that takes raw data** - the calls the game uses to sync a match between two clients, so
they run for both seats:

| Unity-recorder row | Hook in the shipped build |
|---|---|
| `action` (local UI) | `TurnStateMachine.QueueMainPhaseAction(Player, MainPhaseAction)` |
| `action` (opponent) | `TurnStateMachine.QueueMainPhaseAction_Internal(int playerID, byte packetId, byte[] bytes)` |
| AI / script mirrors | `SetActSkill(int,int)`, `SetActCardSkill(int,int)`, `SetAttackingPermaent(int,int)` |
| `selection` card pick | `SelectCardEffect.SetTargetCardAndIndicies(int playerID, int[] CardIDs, int[] Indicies)` |
| `selection` hand pick | `SelectHandEffect.SetTargetHandCards(int playerID, int[] CardIDs)` |
| `selection` permanent pick | `SelectPermanentEffect.SetTargetFrames(int playerID, bool[] isTurnPlayer, int[] UnitIndex)` |
| `selection` attack target | `SelectAttackEffect.SetAttackTarget(int playerID, bool isTurnPlayer, int permanentIndex)` |
| `selection` count | `SelectCountEffect.SetCount(int playerID, int selectedCount)` |
| `selection` DigiXros | `SelectDigiXrosClass.SetTargetDigiXrossIndex(int playerID, int targetIndex)` |
| generic int / bool | `UserSelectionManager.SetIntForPlayer` / `SetBoolForPlayer` |
| `game_start` / `game_end` | `TurnStateMachine.StartGame()` / `EndGame(Player Winner, bool Surrendered, string)` / `Surrender(int)` |

`MainPhaseAction` subclasses carry plain properties (`PlayCardAction.CardIndex` / `TargetFrameID` /
`BurstTamerFrameID` / `AppFusionFrameIDs` / `JogressEvoRootsFrameIDs`;
`AttackPermanentAction.PermanentIndex` / `AttackTargetPermanentIndex`), enough to rebuild the
2192-space id with the Unity project's `ActionEncoder` logic.

Still unknown: how to get `action_detail` (cost paid, alt path), `effect_activation`, and PvP
`reveal` rows. `UseCardEffect` is not a type name in this build. The raw log from a real game will
show what is available before any of these is designed.

Correction (2026-11-08): an earlier version of this note said the dump showed "no declared methods"
for the `Select*Effect` classes. That was wrong - the first dump had the methods and I filtered
the output too narrowly when reading it.

## Plan

| Phase | Deliverable | State |
|---|---|---|
| 1 | Mod loads, Harmony patches apply on the IL2CPP build | done (sandbox) |
| 2 | Probes fire in a real game; the raw event log shows the whole stream | next - needs a game played in the sandbox (`DCGO.exe --melonloader.hideconsole`) |
| 3 | `game_start` / `initial_state` / `action` rows from `QueueMainPhaseAction` + mulligan + `game_end`, written as schema-v1 JSONL | |
| 4 | Selection rows from the 6 result setters above; `action_detail` (cost, alt path); `effect_activation` | smaller than first thought: no coroutine wrapping for selections |
| 5 | PvP reveals (draw / security / mill of the opaque opponent) | |
| 6 | Replay a mod-recorded bot game through `dcgo-replay`; compare against a Unity-recorder game of the same deck | acceptance test |
| 7 | Install into the real `DCGO_Application/Game` (back it up first) | last |

A partial recorder (actions only) is not useful: the engine needs the selection rows to resolve
card effects, so phase 4 is the real milestone.

## Build and run (sandbox)

```bash
# one-time: copy Game/ (0.7 GB) and unpack MelonLoader v0.7.3 into the copy
dotnet build code/dcgo-melon-recorder -c Release -p:GamePath="D:\DCGO_mod_sandbox\Game"
# run with the MelonLoader console hidden: a visible console that gets clicked enters Windows
# "Select" mode and freezes the game
D:\DCGO_mod_sandbox\Game\DCGO.exe --melonloader.hideconsole
```

Logs: `Game/MelonLoader/Latest.log`, `Game/UserData/DcgoRecorder/`.

MelonLoader was downloaded from the official release page
(`MelonLoader.x64.zip` v0.7.3, 20,155,622 bytes, SHA-256
`5b2b2f3d1cd42b59ec886c5bdc2663edae87a0097a4f4a8f58c0965a99dda416`, matching the digest GitHub
publishes for that asset).

## Status (2026-11-08): scripted bot match works in the sandbox

Proven in the sandbox copy (`D:\DCGO_mod_sandbox\Game`, real install untouched):

- The mod loads, patches all 18 probes, and the auto-walk reaches a running bot match
  (`GManager.IsAI`, `You.PlayerID`) from a cold start.
- `TurnStateMachine.EndGame` and `TurnStateMachine.Surrender` rows are written to
  `UserData/DcgoRecorder/raw-<session>.jsonl`.
- **The sandbox needs `Assets/` as a sibling of `Game/`** (`D:\DCGO_mod_sandbox\Assets\Decks`). Saved
  decks (`Assets/Decks/*.txt`) live there, not under `Game/`; without it the deck list holds only
  "Random deck" and no match can start. Copy `Decks` (19 small files); `Textures` (0.6 GB) is not needed
  because card images are fetched over HTTPS.
- Selecting a deck: clicking a `DeckInfoPrefab(Clone)/BackGround` sets `ContinuousController.BattleDeckData`.
  The `SelectButton` click did NOT start the match; calling `SelectBattleDeck.OnClickSelectButton_BotMatch()`
  directly did (after `SelectBattleMode.StartSelectBattleDeck(isAI: true)`). The scene then switches to
  `BattleScene`.

Not yet proven: the other 16 hooks. After 90 s in `BattleScene` the match was still at `TurnCount == 0`
and `StartGame` had not fired: the game waits on a human prompt (first-player / mulligan UI). A scripted
surrender therefore only exercises `EndGame`/`Surrender`.

Next: create `UserData/DcgoRecorder/play.flag` (the auto-walk then leaves the match to a human instead of
surrendering), play a bot game by hand, and check that `StartGame`, `SetBreedingPhase`,
`QueueMainPhaseAction*`, `Select*` and `UserSelectionManager.Set*ForPlayer` rows appear. Then write the
converter from raw rows to schema-v1 and replay a mod-recorded game through `dcgo-replay`.

### Findings from the first scripted games (2026-11-08, later)

The auto-walk now also clicks "First Player" and "Keep Hand" (names `LargeButton02Yellow`, `NotSelectButton`;
note the opponent-side `OpponentFirstObject/LargeButton02Yellow` can be hit first - it is harmless but
unintended). A wide call-logger (`wide-<session>.txt`: first 3 calls of every method of the selection
classes and `TurnStateMachine`) showed which hooks really fire:

- **Fire:** `SetRedraw(playerID, bool)` (the mulligan decision, both seats - added to the named probes),
  `EndGame`, `Surrender`, `Init`, `OffFieldCardTarget`/`OffHandCardTarget`, `SelectCountEffect.SetPreferMin`.
- **Silent although the game ran:** every `IEnumerator`-returning method (`StartGame`, `BreedingPhase`,
  `MainPhase`, ...) - IL2CPP inlines the tiny iterator factories, so a postfix on them never runs. The
  recorder must hook the plain setters, or the iterator state classes' `MoveNext`
  (`_StartGame_d__23` etc.).
- **Not yet seen** because nobody made a breeding/main-phase/attack decision: `SetBreedingPhase`,
  `QueueMainPhaseAction*`, `SetAttackingPermaent`, `Select*` result setters, `PassTurn`.

So the raw log is currently: mulligan decisions (both seats) + game end. The `game_start` row (deck orders,
first player) has no hook yet; it needs the iterator `MoveNext` or reading `GManager`/`Player` deck state at
`BattleScene` load. A converter before those exist would emit recordings `dcgo-replay` cannot start.

### First mod-recorded game through `dcgo-replay` (2026-11-08)

`code/dcgo-melon-recorder/convert_raw.py` converts the raw rows to a v1 recording: `game_start` from the
first-mulligan `Snapshot` row (deck = opening hand + library, top first; egg decks from `DigitamaLibraryCards`),
mulligan -> actions 0/1, declined breeding and main-phase pass -> action 62, `Surrender` -> `game_end`.
Anything it cannot encode (hatching, played cards, attacks, any selection) truncates the recording
(`--allow-truncated`) instead of being skipped. Scripted games reach this state by pressing "Next Phase"
(`Btn_Skill00`) every 5 s, and end themselves when a selection prompt opens that the script cannot answer.

Result of replaying `qa/dcgo-mod-recordings/mod_game1.jsonl` (rebuild `dcgo-replay` first: a stale binary
reports "built for v1"): the recording parses, `Game` starts from the recorded decks, both mulligans replay
(2/4 steps), then **`actor_mismatch` at step 2**: the engine expects the bot (P1, first player) to act, but
the recording only contains the local seat. **The bot seat's decisions are not hooked.** They do not pass
through `SetBreedingPhase` / `QueueMainPhaseAction*` / `PassTurn` (those fired only for the local seat); the
only bot-seat rows seen are `UserSelectionManager.SetIntForPlayer(1, n)` with no prompt context. The
Unity-project recorder gets them from in-source "AI mirror" hooks inside the main-phase coroutine
(`TurnStateMachine`, rule 27); in the IL2CPP build the equivalent is the compiler-generated lambdas
(`_MainPhase_b__72_*`, `_SetMainPhase_b__73_*`) and `AISelectionUtility`, none of which is decoded yet.

Practical consequence: a **human-vs-bot** recording from this mod is missing every bot decision, so it
cannot replay. Human-vs-human (PvP) games do not have this gap in principle (both seats' decisions arrive
through the same network setters), but that is untested.

### Bot-seat decisions: what is and is not hookable (2026-11-08, evidence)

Board snapshots at each pass (`Snapshot.PassTurn`/`Snapshot.EndGame`, with `breeding`/`battle` lists) from a
scripted game where only "Next Phase" is pressed for the local seat:

| after | seat 1 (bot) breeding | seat 1 battle area | hooked bot rows |
|---|---|---|---|
| mulligans (turn 0) | empty | empty | - |
| bot turn 1 + my turn 2 | `BT26-004` (hatched) | `BT16-082` (played) | **none** |

So the bot hatches and plays cards with **no** hit on `SetBreedingPhase`, `SendShouldHatch`,
`QueueMainPhaseAction*`, `PassTurn`, `SetPlayCard`, `set_PlayCard`, `set_AttackingPermanent`,
`set_UseCardEffect`, `set_Passed` (all hooked; they fire only for the local seat). The most likely reason is
IL2CPP inlining: the bot brain lives inside the giant `_MainPhase_d__72.MoveNext` coroutine, where trivial
auto-property setters are inlined into the caller, so a Harmony postfix on the setter never runs
(`set_DefendingPermanent` fires only from the non-inlined sites). `set_TargetFrameID` is UI-hover noise and
was dropped.

Not-inlined hook candidates for the bot's plays, all unverified: `_PlayCard_d__45.MoveNext` (state 0) on
`PlayCardClass` (fields: `CardSources`, `_targetPermanent`, `_burstTamerFrameID`, `_isBreedingArea`,
`_fixedCost`, `_reducedCost`), the attack coroutine behind `GManager.attackProcess`, and the hatch step of
`_BreedingPhase_d__28.MoveNext`. Each needs its own probe run (about 7 minutes) and then a card-to-hand-slot /
frame-to-slot translation ported from `ActionEncoder.cs`. Until those exist, a human-vs-bot recording from this
mod cannot replay (`actor_mismatch` at the first bot decision). Human-vs-human games are the untested but more
promising target: both seats' decisions arrive through the same network setters.

### First partial pass of a mod-recorded game (2026-11-09)

Hooking `PlayCardClass..ctor` (the card a seat is playing, plus that seat's hand and slot) and
`Permanent.AddCardSource` makes the bot's plays visible even though the setters it uses are inlined. With the
converter's bot-seat reconstruction (`convert_raw.py`), `qa/dcgo-mod-recordings/mod_game2.jsonl` replays as
**partial_pass: 4/4 steps, 0 divergences** - both mulligans (the bot redrew), the bot's breeding PASS
(**inferred**: nothing fires for it) and the bot's play of `EX11-071` from hand slot 0, accepted by the engine's
legality check and memory cost. It stops there because the next bot decision is a `SelectCardEffect` selection
with no prompt context (`SetTargetCardAndIndicies(playerID, indices, ...)`), which the converter refuses to
guess; the recording is written without a `game_end` row so the harness reports it as partial, not a full pass.

Known holes, in the order they block a human-vs-bot game: bot selections (costs, ETB targets), bot hatching
(the first run hatched `BT26-004` with no constructor row), bot digivolution (`target` field), bot attacks and
blocks, the bot's implicit end-of-turn PASS being inferred rather than observed, and the iterator-class
`Dispose` patches that crash the game (patch `MoveNext` only).

### Sandbox crash explained (2026-11-09)

The silent game deaths ~4 s into the bot's first turn were `System.StackOverflowException` (Windows Event Log:
Application Error, exception code `0xc00000fd`; WER `CLR20r3`, P9 `System.StackOverflowException`). Cause: the
diagnostic "wide" call-logger wrapped ~190 methods, including the `PlayCardClass` checks the game calls deeply
during the bot's digivolve chains; the extra Harmony/trampoline frames overflowed the stack. Wide probes are now
opt-in (`UserData/DcgoRecorder/wide.flag`); without them a scripted game completes (3 turns, `auto: done`).
Check `Get-WinEvent -LogName Application` for `Application Error`/`Windows Error Reporting` before blaming a probe.

Still open: in that completed run the bot played `EX13-030`, used option `P-105`, hatched an egg and digivolved
in the breeding area (board snapshots show it), but the `PlayCardClass..ctor` probe produced no rows (its
IL2CPP patch backend init logs a warning and is unreliable), only a bare `Permanent.AddCardSource`. So the
bot-seat reconstruction needs a more dependable play hook than the constructor.

### First full PASS of a mod-recorded game (2026-11-09)

`qa/dcgo-mod-recordings/mod_game3.jsonl` (raw log `raw-20261109T011025Z.jsonl`, scripted bot match, 3 turns, then a
surrender) replays through `dcgo-replay` as **PASS: 8/8 steps, 0 divergences, winner matches**. Two of its rows
are converter inferences and are marked `"inferred": true`:

- the bot's breeding **HATCH** on turn 1 (inferred from its egg deck shrinking 4 -> 3 between two snapshots;
  the engine accepted it);
- the bot's main-phase **PASS** at the end of turn 1 (not needed: paying `ST20-10`'s cost crossed the memory
  gauge and ended the turn, so the engine was already waiting on the other seat - the replay skipped the row).

Observed, not inferred: both mulligans, the bot's play of `ST20-10` (`PlayCardClass.PlayCard` hook, hand slot 1),
the local seat's breeding PASS and main PASS, and the surrender (`CONCEDE_GAME`, action 93).

Replay-side changes this needed (engine `runners/replay.rs`, `dcgo_recording.rs`): `ActionRow.inferred` /
`StepSpec.inferred` - an inferred row is skipped, and listed in `ReplayDriver::inferred_skipped()`, ONLY when the
engine is already waiting on a different seat (never when it accepts the actor); and `CONCEDE_GAME` is exempt from
the on-decision-player check (a surrender is available to either seat at any time). Skipped inferred rows are added to
the report's `skipped_actions` (mod_game3 reads `skipped_actions: 1`; its summary line says "recorded DCGO play(s)
skipped as never-completed", which is imprecise wording for this case). Rows that the engine ACCEPTED (the inferred
HATCH) are not counted, so also read the converter's `inferred:` lines. Tests: `dcgo-replay/tests/integration.rs`
(`integration_concede_by_the_seat_not_on_decision_passes`, `integration_inferred_row_for_the_wrong_seat_is_skipped`,
`integration_observed_row_for_the_wrong_seat_still_diverges`). Corpus after the change: 2076 pass / 22 triaged /
0 fail / 34 partial, unchanged.

Still not covered: bot digivolution (`target` set), bot attacks/blocks, bot effect selections (count / hand /
DigiXros), bot costs that need a choice, security checks, and any hatch that is not the first hatch of the game
(the egg-count rule assumes the local seat never hatches). Every longer game will hit one of these.

### Batch of unattended games, and the cards they forced (2026-11-09)

`qa/dcgo-mod-recordings/batch_games.sh`-style batches (launch -> scripted walk -> convert -> replay) produced four
games in a row. With the post-mulligan `initial_state` row (the mod polls `DoneStartGame` and snapshots both seats'
zones; a bot mulligan redraw is otherwise invisible and made one game fail on a stale deck order), count / permanent /
hand selections decoded, and digivolution targets decoded (`DIGIVOLVE_START + hand*15 + field`, field 14 = breeding
area), **all four games and `mod_game3` replay as full PASS** (9, 7, 6, 9 and 8 steps, each ending in the observed
surrender). `mod_game2` (recorded before `initial_state`) stays a 4-step partial.

The one game that first failed (`illegal_action`: bot digivolves a hatched `ST21-01` Tsunomon into `ST20-02` Biyomon
via its printed "[Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0") exposed that the Adventure starter decks had NO card
scripts, so the engine played them as vanilla cards. 30 cards were authored (ST20-01..09/12/13, ST21-01..12/14/15,
AD1-003/007/008/020/022: workflow `author-adventure-starter-cards`, 986 + 38 behavioural tests green; a wrong
ST20-05 expectation was corrected against rule 14-2-3, "Security Digimon aren't deleted even when they lose a
battle"). Two of them needed new vocabulary, `distinct_colors_gte { of, filter, n }` (see `qa/dsl-vocab-gaps.md`,
G-DSL-DISTINCT-COLORS-FILTERED-GTE, resolved). Corpus after all of this: 2132 recordings, 2076 pass / 34 partial /
22 triaged / 0 fail, unchanged.

Not yet covered by the converter: bot attacks and blocks, security-check choices, bot effect selections that need
prompt context beyond count / hand / permanent / card picks, DigiXros / DNA, any second hatch (the egg-count rule
assumes the local seat never hatches), and the bot's turn-end PASS is still inferred.

### Game 4 failure explained; fallback-int channel decoded (2026-11-09, evening)

- **A pass the bot never made.** One unattended game failed (`illegal_action`, engine waiting on a bot prompt) because the
  scripted walk surrendered mid-effect (bot had just played `ST24-04`, whose [On Play] needs selections) and the converter then
  appended an inferred bot main PASS "at end of game". The converter no longer synthesises a pass when the game ends by
  surrender; the concede is the next row.
- **`UserSelectionManager.SetIntForPlayer` = schema prompt `generic_int`** (bot choice among an effect's branches, e.g. `BT17-015`
  WarGreymon "activate 1 of the effects below"). A first guess of `MultipleSkills` replayed but then diverged; `generic_int`
  replays cleanly (the adapter indexes it against the live prompt's branch list).
- Tally after this: **ten mod-recorded games (nine unattended + `mod_game3`) replay as full PASS, 0 divergences.** Still inferred:
  the bot's hatch and turn-end pass; still undecoded: bot attacks/blocks, security-check choices, DigiXros/DNA, second hatch,
  and prompts whose answers do not reach a hooked setter.
- The scripted walk can fail to find the Keep Hand button within its 60 s window ("auto: giving up") and then leaves the game for
  a human; a human-driven game's `SetBreedingPhase(0, true)` (hatch) is not encoded yet and truncates the recording.

## PvP recording, model seats and sim playback (2026-11-10)

All three run on the sandbox copies (`D:\DCGO_mod_sandbox`, `D:\DCGO_mod_sandbox2`); the real install is untouched.

### PvP recording

`UserData/DcgoRecorder/auto.txt` = `host` (or `join`) and the shared dir. The host creates a Photon room and writes
`room.txt`; the joiner enters it; both pick a deck, ready, and the host's room starts the battle. Each client simulates
both decks, so each raw log converts to a full recording (`convert_raw.py`); the first PvP recordings replay with 0
divergences.

### A model plays a seat (`dcgo-agent`)

`UserData/DcgoRecorder/agent.txt` = a request dir; run
`digimon-engine-cli --pool all --cards-json data/cards.json dcgo-agent --dir <dir> --model <checkpoint.onnx>
--converter code/dcgo-melon-recorder/convert_raw.py` (EX13 campaign checkpoints, e.g. `checkpoint8.onnx`). At each
decision of its seat the mod writes `request-<id>.json`; the agent converts the raw log, replays it through the engine
(`ReplaySession::from_dcgo`) and answers in DCGO terms; the mod performs the answer (mulligan button, `SendShouldHatch`,
`QueueMainPhaseAction(PlayCardAction / AttackPermanentAction)`, selection setters). In PvP, selection answers go out as
`photonView.RPC(<setter>, RpcTarget.All, ...)` like DCGO's own UI, so both clients resolve the prompt.

### Playing back a game our engine played

1. `digimon-engine-cli ... sim-record --model <onnx> --deck0 d0.json --deck1 d1.json --seed N --out sim.json` plays the
   model on both seats in our engine and writes a native recording.
2. `python code/dcgo-melon-recorder/make_playback_plan.py sim.json plan.txt` writes the first player and each seat's
   main / egg deck in DCGO's top-first order (`hand + security + reversed(library)`: DCGO deals the hand from the top 5,
   then security one card at a time onto the FRONT of its list; checked on a pre/post-mulligan snapshot pair).
3. Both sandboxes: `playback.txt` = the plan path, `agent.txt` = one request dir each, `auto.txt` = host / join.
4. One `dcgo-agent --script sim.json` per request dir. The agent finds DCGO's current state in the recording by a
   fingerprint (turn, phase, memory, every zone in order) and answers with the recorded decision; a state the
   recording never reached is reported as `diverged`.

The mod's shuffle postfixes replace the n-th deck build of a game (master main, master eggs, joiner main, joiner eggs:
the fixed order in `CardObjectController`) with the plan's cards, looked up by id in `ContinuousController.CardList`,
and reorder a mulligan reshuffle to the plan. The host picks the first player with
`RoomManager.OnClickFirstPlayerIndexIDButton`.

### Things that broke along the way (each fixed)

- **Replay running ahead of DCGO.** A play packet is logged when the play is *queued*; DCGO resolves it ~20 s later
  (animations). Converting the packet at queue time let the agent answer prompts DCGO had not opened. Plays are now
  taken only from `PlayCardClass.PlayCard` rows (logged at resolution, with the resolved hand slot and target, on
  every client), and the mod only queues a turn action while DCGO's main-phase loop is idle.
- **Attacks were never recorded.** `SetAttackingPermaent` (called by `AttackPermanentAction.Execute`) is inlined in
  the shipped build and never reaches its hook. Attacks are decoded from the attack packet, which now carries both
  seats' `GetFieldPermanents()` order (`boards`) so `remap_board_slots` maps DCGO indices onto engine slots.
- **Digivolution cost choice.** DCGO asks it with `SelectCountEffect` over the candidate COSTS ("Which digivolution
  cost do you pay?"); the answer is `SetCount(player, cost)`, not an index.
- **Selection picks were never read.** The agent writes pretty-printed JSON (`"picks": [`); the mod looked for
  `"picks":[` and sent every answer empty.
- **Turn 0 after the mulligans.** `TurnPlayer` is stale there; `SendShouldHatch` answers for the turn player, so a
  hatch sent then went to the other seat.
- **A missing card shows up as a DCGO-only prompt.** P-248 (Veemon) had no engine implementation: DCGO prompted its
  start-of-main trash and our sim never did. Every card in a playback deck must be implemented first.
- **The local seat answers through DCGO's own UI.** For the local seat DCGO opens its card-selection panel and waits
  for the human (`OpenSelectCardPanel`); a setter call is queued but not read while the panel is open, and an answer
  sent before the panel opens is consumed by a LATER prompt. The mod now clicks the panel (`OnClickHandCard` +
  `OnClickEndSelectButton` / `OnClickNotSelectButton`), clicks DCGO's command menus by label (zone menus "From Hand",
  cost menus "3", top/bottom), and answers trigger order (`MultipleSkills`) in the same panel by the trigger's source
  card. Hand picks (`SelectHandEffect`) and yes/no (`OptionalSkill`) read the answer queue, so their RPC works.
- **Attack indices** are positions in `GetFieldPermanents()` at DEQUEUE time; the mod resolves them by card id +
  occurrence in that list and only once the board has held still. Attacks are recorded from `SetAttackingPermaent`
  (it fires when DCGO executes the attack; it never fired earlier only because those attacks were never executed).
- **Engine bugs the playback found** (fixed): P-248 had no implementation (DCGO prompted its start-of-main trash);
  a DUAL card checked from security was not a Security Digimon (CR 4.3 13-1-7 / 4-6-1) - Siriusmon EX12-018 did not
  battle. `tests/combat/end_of_turn_target_kind.rs::recorded_monarchlizamon_attack_on_paishu_token_progresses` is
  ignored: its fixture game relied on that bug and needs re-recording.
- **"Can't be deleted in battle" blocked effect deletions** (fixed 2026-11-11): `EffectContext::add_modifier` builds
  its entry with `ModifierEntry::simple`, which set no cause filter, so `CannotBeDestroyedByBattle` (AD1-011
  Paildramon's [When Digivolving]) cancelled Omnimon EX12-037's "Delete 1" too. `simple` now gives that modifier
  its intrinsic `Battle` filter (`modifiers::intrinsic_cause_filter`). Test
  `cards_behavioral/ex12/ex12_037.rs::ex12_037_delete_ignores_battle_only_protection`. The sim game was re-recorded
  (`sim1b`: identical for 94 decisions, then ST9-06 is deleted as DCGO does).

### Answering DCGO's prompts during playback (2026-11-11)

Every answer for the local seat goes through DCGO's own UI, and only once that UI is ready. An answer that arrives
early or twice sits in DCGO's selection queue and a LATER prompt consumes it. The rules the mod follows:

- **Card panel** (`SelectCardPanel`): click only after the panel's card list (`Parent`) has been active for 1 s. The
  panel is shown before its setup coroutine ends, and closing it then (End Selection) deactivates the panel, which
  kills that coroutine: DCGO then waits forever on a selection with no UI. Never click more cards than the panel's
  `_maxCount` (an extra click REPLACES the last pick; DCGO opens a second panel for the rest), and never click a card
  already in `_preSelectedHandCardList` (it would deselect it). A panel that is still open 12 s after the answer is
  driven again. Some hand picks ("Trash 1 card in your hand") come in this panel, not as clickable hand cards.
- **Hand prompts** with no button (a mandatory pick) are open when the hand cards carry `OnClickAction`. The End
  Selection button appears only after the click, so a hand answer is driven again when its menu is still open 12 s
  later.
- **Optional effects** (`OptionalSkill`, a lone "Use" button plus the back button). The engine models most "you may"
  effects as the pick itself (a pick means use), so the mod clicks "Use" and then answers the pick. When the engine
  has an explicit optional-trigger prompt (`SelectionKind::Replacement`, accept = 59), the agent sends category
  `optional`. The answer RPC `OptionalSkill.SetUseOptional` is now probed and stamped. When DCGO asks about an
  optional effect while the engine wants a turn action (the engine resolved the "may" without a decision, e.g. no
  legal target), the mod declines it, and that row carries no stamp and is not replayed.
- **Forced picks**: DCGO resolves a mandatory pick whose candidate count equals the max by itself
  (`SelectPermanentEffect` "forced selection"), while the engine still asks. The mod answers a permanent prompt only
  when field cards carry a click target. Otherwise the agent's next request is the next prompt.
- **Permanent picks carry DCGO's field order**: `SetTargetFrames`' `UnitIndex` is a `GetFieldPermanents()` position,
  which is not the engine's battle-area slot (DCGO `[BT16-025, AD1-011, BT16-085, LM-058]` against engine
  `[BT16-085, AD1-011, BT16-025]`). The row now logs `boards`, and the converter passes them as `board_p0/1` so
  `remap_selection_targets` maps the pick onto engine slots.
- **Effect plays that play nothing** (`PlayCardClass.PlayCard` with no cards and no hand) are skipped by the
  converter.

### A full sim game played back in DCGO (2026-11-11)

`sim1b` (cp8 vs cp8, EX12 Gammamon/Greymon vs BT12 Veemon/Wormmon, seed 1, 124 decisions, seat 0 wins on turn 12)
plays back in two PvP DCGO clients from mulligan to the winning attack. DCGO's `EndGame` comes at the same attack,
and the host's recording of the played-back game, converted with `convert_raw.py`, replays through `dcgo-replay`
as a full PASS: every decision is legal and the winner is the same. One full run takes about 45 minutes.

Getting there, besides the answering rules above:

- **An engine bug** (Omnimon's "Delete 1" was blocked by battle-only protection, see above). The first recording,
  `sim1`, was made with that bug. `sim1b` is the same game re-recorded with the fix (identical for 94 decisions).
- **DCGO asks orders and confirmations the engine does not.** It lists triggers whose condition is already false
  ("if your hand has 7 or fewer"), asks Assembly once per material element, and asks "Use?" for optional effects
  the engine folds into the pick. The mod answers these itself (the engine's effect first, decline the extra
  Assembly panels, confirm "Use"), and the replay skips the rows they leave when the engine has nothing parked.
- **The script agent** (`--script`) also matches a recorded decision whose recorded state is already the state
  after it. The native replay folds an optional yes/no into the forced pick before it (Ukkomon BT16-082's "you may
  hatch"), so the live state waiting on that decision appears nowhere in the recording.
- `DCGO EndGame` becomes a `game_end` row (winner from the mod's id, or the turn player when the other seat's
  security is empty). A trigger-order pick made on the other client is shared through `chosen-<pid>.txt` beside the
  engine-action stamps, so both clients' recordings name the effect.

Run it: make the plan (`make_playback_plan.py sim1b.json plan.txt`), point both sandboxes' `playback.txt` at it,
start two `dcgo-agent --script sim1b.json` (one per seat dir), then launch the host and, 20 s later, the joiner.

### More archetypes played back: what they found (2026-11-11/12)

Three more cp8 sim games (seed 1, decks in `qa/dcgo-mod-recordings/playback/deck_*.json`):

| Match | Result |
|---|---|
| Toho Braves vs TS Jupitermon (`sim_toho_jup`, 86 decisions) | Plays back to the same winner (seat 1, turn 11); both clients' recordings replay as a full PASS. Took ten runs. |
| Medusamon vs Rocks (`sim_med_rocks`) | Blocked at DCGO step 58 by a DCGO bug: Sunarizamon EX8-047's rule-granted [Mineral] trait is missing in DCGO (`qa/rules/dcgo_quirks.md`, Q-DCGO-RULE-TRAIT-MISSING). |
| Glowing Dawn vs DNA Omnimon (`sim_gd_dna`, re-recorded after each engine fix, last 56 steps) | Plays back to the same winner (seat 1); both clients' recordings replay as a full PASS. |

Engine bugs found and fixed:

- **Tsunomon BT24-003 offered a card that cannot digivolve.** An effect digivolve from hand/trash now only offers
  cards that can digivolve onto the target (`digimon-dsl` pass `digivolve_candidates`, DCGO `CanPlayCardTargetFrame`).
- **Turn end after the breeding phase (CR 4.3 6-1-4-1).** A breeding-phase effect that pays memory over to the
  opponent's side ends the turn; the engine used to open the main phase. Pinned by `rules_conformance`
  `ch06_turn_flow`.
- **Native replay double-applied a forced pick.** The one-card ordering auto-answer ran for native recordings too,
  which log that pick themselves, so the recorded pick landed on the next decision (a main-phase id decodes for the
  turn player: P0's "place the last card" played P1's BT24-090 a step early). Now DCGO recordings only. It silently
  corrupted the states the playback agent matches against.

More engine bugs, from Glowing Dawn vs DNA Omnimon:

- **EX9-066 Tai Kamiya & Matt Ishida** ("You may return 1 ... If this effect didn't return, <Draw 1>") was a
  Return/Draw branch choice whose return branch drew nothing when it returned nothing. Now one optional trash pick,
  then the return or the draw.
- **BT25-049 / BT25-081** filtered already-suspended targets out of "suspend 1 ..." (Q&A Q845 allows them).
- **BT25-090** "by suspending this Tamer, you may place the top 2 ..." had no suspend-only choice (CR 15-7-5).
- **ST23-15 e-Pulse**'s [Security] ("Activate this card's [Main] effects") trashed the card instead of placing it in
  the battle area, so its [Start of Your Main Phase] never came.
- **Replay:** DCGO's "Don't activate these effects" (-1) now uses the engine's decline-all, and declines an optional
  gate DCGO lists among other effects; a declined permanent pick with null arrays converts as a cancel.

DCGO asks things the engine does not, and the mod now answers them:

- **Replacement order** is DCGO's "Multiple effects" card panel; the agent names each branch's source card.
- **One effect listed twice** (BT24-040's "[On Play] [When Digivolving]" activated by BT24-102): the mod takes the
  first; the pick is stamped stray (-2) and the converter drops it.
- **A lone optional trigger** gets only DCGO's "Use?"; the mod answers it with the engine's order-pick stamp.
- **Zone menus** ("From hand / From trash / Don't play", EX12-074's [Security]): the decline is stamped PASS; a zone
  choice is held back by the agent until the card pick follows it.
- **The non-turn seat's card panel** opens a moment late; answering through the setter first left it open.
- **[Main] activation index:** the engine's field-effect sub-slot (2 = [Main], 3 = <Link>) is mapped to DCGO's
  positional skill index, and back via `activate_link` on the packet row.
- **Hand picks** wait for DCGO's End Selection after the click.
- **A declined effect listed with others** in DCGO's "Multiple effects" panel is answered with No Selection.

Open question, not changed: official Q&A Q845 (ST18-10) and Q1782 say an already-suspended Digimon can be chosen for a
"suspend 1 ..." effect (it just isn't suspended by it). BT25-049 and BT25-081 now allow that (DCGO agrees). Five more
non-cost suspends still require an unsuspended target in both the engine and DCGO: BT25-055, BT26-035, BT26-036,
BT26-044, EX13-076. Following the ruling there would diverge from DCGO; it needs a decision.

Known open: `tests/replay_empty_replacement_window.rs` fails its recorded digest at step 18 on this branch. It
predates this work: it is unaffected by the candidate pass and the turn-end fix, and does not use the replay runner.

### Where playback stopped earlier (2026-11-10, historical)

`sim1` (cp8 vs cp8, EX12 Gammamon/Greymon vs BT12 Veemon, 124 decisions) plays back through turn 4: mulligans, plays,
digivolves with cost menus, attacks into Security Digimon, reveal picks, a forced one-card bottom order, trigger order
and Siriusmon's zone menu + first hand pick. It stops at Siriusmon's second source: DCGO asks the zone once, takes up
to 2 cards in ONE selection and then top/bottom once; our engine asks card by card ("place another ...", with its own
top/bottom per card). Card text: "place up to 2 ... from your hand or trash as this Digimon's top or bottom
digivolution cards". That is a rules / implementation question for EX12-018, not a playback one; every such mismatch
shows up the same way (a DCGO prompt shape the engine does not share), which is what the playback is for.
