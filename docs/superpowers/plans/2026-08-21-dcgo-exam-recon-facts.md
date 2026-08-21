# DCGO Exam — Verified API Facts and Plan Corrections

Recon output, 2026-08-21. **This document supersedes every "placeholder — resolve
with grep" marker in the three exam plans.** Everything below was read from the
source with file:line evidence.

Plans corrected here:
- `2026-08-21-dcgo-scripted-input-driver.md` (Unity)
- `2026-08-21-dcgo-exam-scenario-and-differ.md` (Rust)
- `2026-08-21-dcgo-exam-workflow.md` (workflow layer)

---

## A. Rust engine API — four plan guesses were wrong

| Plan said | Reality | Evidence |
|---|---|---|
| `game.apply_action(actor, id) -> Result` | **`game.decode_action(action_id: u16, player_id: PlayerId)` returning `()`.** No `Result`. Illegal/out-of-range ids are **silently ignored**. There is no `apply_action` and no `Game::step`. | `src/action/decode.rs:32` |
| `game.phase` | **`pub current_phase: GamePhase`** (pub field). Stringify with `.py_name().to_string()`. | `src/game/mod.rs:322` |
| `game.turn` | **`pub turn_count: u16`** (pub field). Turn player is the method `turn_player() -> PlayerId`. | `src/game/mod.rs:293`, `:1107` |
| `ReplayError::Setup(e)` | Variants are **`MissingInitialState`, `MalformedRecording(String)`, `UnknownCard(Vec<String>)`, `GameConstruction(String)`**. Use `GameConstruction`. | `src/runners/replay.rs:262-268` |

**Consequence of `decode_action` returning `()`:** the `ScenarioAdapter` lowering
loop **cannot** detect a bad action from a return value. This is fine — and is
exactly why lowering resolves against `build_action_mask` first — but the plan's
`.map_err(...)` on the apply call must be deleted, and the loop must instead
assert the mask bit before applying:

```rust
let mask = build_action_mask(&game, actor);
if mask[action_id as usize] != 1.0 {
    return Err(format!("step {i}: lowered action {action_id} is not in the mask"));
}
game.decode_action(action_id, actor);
```

Other confirmed signatures:

```rust
pub fn new_with_ordered_decks(
    deck_card_ids: &[Vec<String>],
    all_card_data: &std::collections::HashMap<String, CardData>,
    rules: Rules,
    seed: Option<u64>,
    first_player: PlayerId,
) -> Result<Self, String>                       // src/game/setup.rs:35-41

pub fn new(
    deck_card_ids: &[Vec<String>],
    all_card_data: &std::collections::HashMap<String, CardData>,
    rules: Rules,
    seed: Option<u64>,
) -> Result<Self, String>                       // src/game/setup.rs:22-27
```

Other Game pub fields the projection needs: `pub memory: i16`,
`pub memory_pair: (PlayerId, PlayerId)`, `pub game_over: bool`,
`pub winner: Option<PlayerId>` (`src/game/mod.rs:324,326,331,332`).

`StepSpec` construction — copy this verbatim shape (`VerificationReplayAdapter`,
the simplest of the three real examples, `src/runners/replay.rs`):

```rust
StepSpec {
    actor: action.seat,
    action_id: action.action_id,
    phase: String::new(),
    source: "verification_replay".to_string(),
    memory_after: None,
    dcgo_memory: None,
    turn: None,
    is_game_over: None,
    expected_digest: Some(*digest),
    selection: None,
    board_p0: None,
    board_p1: None,
}
```

**Open (recon blocker, resolve before relying on it):** the full `GamePhase`
variant list was read only as far as `SelectPermutation` (`src/enums.rs:108-138`);
re-read to the closing brace if the complete set matters. `GamePhase::py_name()`
was not located.

---

## B. Deck fixtures — `Game::new` does NOT validate deck legality

Recon read the complete body of `new_inner` (`src/game/setup.rs:51-295`) and
found **no deck-size / copy-count enforcement**. So a test deck does not need to
be tournament-legal for `Game::new` to accept it — but a scenario meant to mirror
DCGO **does**, because DCGO gates battles on `DeckData.IsValidDeckData()`
(50 main, ≤5 egg, per-card legality).

Use the **ST-1 list (54 cards: 4× `ST1-01` as the egg deck + 50 main)**, verified
against `data/cards.json` by reading it. Digi-eggs are split out by
`card_kind == 3`.

**No dcgo-replay or dcgo-harness test currently loads the real `data/cards.json`**,
so there is no in-crate precedent to copy. The nearest ones are
`code/digimon-engine/tests/bench_engine_throughput.rs:25` and
`code/tools/archetype-static-tests`. Copy repo-root resolution from those.

**`cargo test -p dcgo-harness` has two flaky daemon tests.** Recon did not check
whether they also flake on unmodified `main` (worktree `git status` was clean, so
nothing local causes them). **Attribute any daemon-test failure against main
before debugging your change** — CLAUDE.md rule 33's discipline applies.

---

## C. DCGO runtime accessors — for `StateDumper`

The plan's `StateDumper` used **`GetFieldPermanents()`. That is wrong.**

```csharp
public List<Permanent> GetBattleAreaPermanents()   // Player.cs:621  <-- USE THIS
public List<Permanent> GetFieldPermanents()        // Player.cs:669  <-- includes BREEDING
```

The recorder's own comment says why: *"GetBattleAreaPermanents, NOT
GetFieldPermanents: the latter walks every frame including the BREEDING one, so a
hatched egg or a digivolving stack showed up in the snapshot as though it were on
the battle field."* (`Recording/ActionEncoder.cs:577-584`). Using the wrong one
manufactures false divergences.

| Need | Accessor | Evidence |
|---|---|---|
| Printed card id | `CEntity_Base.CardID` (string). **Not** `CardIndex` (a per-game db index). | `CEntity_Base.cs:45`, `:10` |
| Card id off a zone card | `CardSource.CardID` | `CardSource.cs:3463` |
| Zones | `Player.HandCards`, `.TrashCards`, `.SecurityCards`, `.LibraryCards`, `.DigitamaLibraryCards` — all `List<CardSource>` **public fields** | `Player.cs:502-526` |
| Battle area | `player.GetBattleAreaPermanents()` | `Player.cs:621` |
| Permanent top card | `perm?.TopCard?.CardID` | `ActionEncoder.cs:573-590` |
| **Effective** DP | `Permanent.DP` (**property**, walks `IChangeDPEffect`). Proven to be the battle-comparison value by `CardController.CompareStats()`. Returns `-1` when `!HasDP`. | `Permanent.cs:499-506`; `CardController.cs:4762-4776` |
| Printed DP — do NOT use | `Permanent.BaseDP` | `Permanent.cs:193` |
| Suspended | `Permanent.IsSuspended` (public field). `OldIsSuspended` is a prior-value snapshot. | `Permanent.cs:1967-1970` |
| Digivolution sources | `Permanent.DigivolutionCards` | `CardController.cs:4766` |
| Memory (per-seat) | `Player.MemoryForPlayer` — negates `GameContext.Memory` for `PlayerID == 0` | `Player.cs:977-991` |
| Memory (raw) | `GameContext.Memory` — **stored positive-favors-PlayerID-1** | `GameContext.cs:27` |
| Turn number | **`GManager.instance.turnStateMachine.TurnCount`** — NOT on `GameContext` | `TurnStateMachine.cs:30-31` |

### Keywords have no collection — 22 separate bool properties

There is no `Permanent.Keywords` list. Each keyword is an independently-computed
bool property that already accounts for granted/temporary effects:
`HasBlocker` (`:2409`), `HasJamming` (`:2499`), `HasIceclad` (`:2553`),
`HasPierce` (`:2598`), `HasReboot` (`:2630`), `HasRaid` (`:2700`),
`HasRush` (`:2721`), `HasRetaliation` (`:2782`), `HasAscension` (`:2823`),
`HasGuard` (`:2847`), `HasEngage` (`:2868`), `HasFortitude` (`:2892`),
`HasBlitz` (`:2916`), `HasEvade` (`:2943`), `HasMindLink`, and more.

Each getter walks the whole field's effect lists, so dumping all of them **per
permanent per step** is O(22 × field × effects). **Dump keywords behind a flag,
default OFF**, and enable it only for scenarios whose clause is a keyword clause.
The projection must treat an absent keyword list as "not measured", never as
"no keywords" — otherwise every scenario run without the flag reports a false
keyword divergence.

**The existing recorder emits NO DP and NO suspended state** (verified by grep
across `GameRecorder.cs` and `ActionEncoder.cs` — zero hits). So for those two
fields there is no precedent to copy; `StateDumper` is the first consumer.

---

## D. The input driver — intercept at the RPCs, not the 13 gates

**This supersedes plan 1 Task 6 entirely.**

### The gate count, verified by reading

15 raw `DrivesLocalSeat` grep hits (excluding `HarnessAuto.cs`); **2 are comments**
(`CardController.cs:376` inside an XML doc block, `ICardEffect.cs:1233` inside a
`[Recording mod]` comment); **13 are live `if` statements**, of which 12 are real
decision gates and one is pacing-delay-only.

### Why the gates are the wrong seam

Every gate's AI branch funnels into a `[PunRPC]` that does exactly two things:

```csharp
GameRecorder.Instance?.LogSelectionRow(playerID, "<PROMPT>", phase, ...);
selectionPlayer.QueuePlayerSelection(new ValueSelection(x));   // or CardSelection / PermanentSelection
```

and every waiter is `yield return new WaitUntil(() => player.HasPlayerSelection());`.

Intercepting at the RPC instead of the gate is strictly better:

1. **The driver and the recorder hook the same place**, so what is recorded is
   exactly what is scriptable. A scenario authored from a recording can always
   match, by construction.
2. **It is uniform** — one shape, ~10 sites, instead of 13 heterogeneous gates
   with two different polarities and two different control-flow shapes
   (`yield break` vs `return`).
3. **It dissolves the `CardController.cs:700` blocker.** That gate has no
   AutoSelect of its own — it only sets flags and falls through to
   `selectCountEffect.Activate()`, whose answer surfaces downstream as the
   `SelectCountEffect` prompt. At the RPC layer it needs no hook at all.
4. The AI still computes a throwaway value first, which is **pure selection
   logic with no side effects** (e.g. `SelectPermanentEffect` random-samples up
   to 200 times against a predicate). Discarding it is safe.

### The interception points (14 `LogSelectionRow` sites across 9 files)

| RPC | Prompt string | File:line of the recorder hook |
|---|---|---|
| `SelectCardEffect.SetTargetCardAndIndicies` | `SelectCardEffect` | `872`, `881` |
| `SelectHandEffect.SetTargetHandCards` | `SelectHandEffect` | `799`, `808` |
| `SelectPermanentEffect.SetTargetFrames` | `SelectPermanentEffect` | `1066`, `1078` |
| `SelectAttackEffect.SetAttackTarget` | `SelectAttackEffect` | `570`, `577` |
| `SelectCountEffect.SetCount` | `SelectCountEffect` | `209` |
| `SelectDigiXrosClass.SetTargetDigiXrossIndex` | `SelectDigiXrosClass` | `1041` |
| `MultipleSkills.SetTargetSkill` | `MultipleSkills` | `437` |
| `OptionalSkill.SetUseOptional` | `OptionalSkill` | `141` |
| `UserSelectionManager.SetIntForPlayer` | `generic_int` | `32` |
| `UserSelectionManager.SetBoolForPlayer` | `generic_bool` | `62` |

**Prompt vocabulary is exactly these 10 strings.** Note `generic_int` /
`generic_bool` are the only two that are not the class name.

### Two more decision families that are NOT `LogSelectionRow`

A scripted line must cover these or it cannot get past turn 1:

- **Mulligan** — `LogMulligan(playerID, isRedraw)` at `TurnStateMachine.cs:702`,
  then `QueuePlayerSelection(new ValueSelection(isRedraw))`. Waited at `:572`.
  Row type `mulligan`.
- **Breeding** — `LogBreedingAction(...)` at `TurnStateMachine.cs:1077`, then
  `QueuePlayerSelection(new ValueSelection(doBreeding))`. Waited at `:989`.
  Row type `breeding_action`.
- **Main phase** — `QueueMainPhaseAction(Player player, MainPhaseAction action)`
  at `TurnStateMachine.cs:3430`, which calls `LogAction(...)` then RPCs
  `QueueMainPhaseAction_Internal`. Per-player queue is
  `Player.QueueMainPhaseAction` / `DequeueMainPhaseAction` / `HasMainPhaseAction`
  (`Player.cs:172-192`).

So the scripted-step vocabulary is **10 selection prompts + `mulligan` +
`breeding_action` + `main_phase` = 13 kinds.**

### Selection payload types

`QueuePlayerSelection` takes one of: `ValueSelection(int|bool)`,
`CardSelection(int[])`, `PermanentSelection(bool[] isTurnPlayer, int[] unitIndex)`.
Note `SelectCardEffect` enqueues **twice** (CardIDs then Indicies).

Encodings worth knowing:
- `SelectAttackEffect`: `-2` = decline, `-1` = attack the player/security, else a
  compact field slot via `ActionEncoder.ValidateFieldSlot`.
- `SelectDigiXrosClass`: `0`=Hand `1`=Field `2`=Trash `3`=TamerSources `4`=End.
- `SelectPermanentEffect`: `(null, null)` = cancel; empty arrays = zero-pick confirm.

### Open blockers on this seam

- `GameRecorder.LogSelectionRow` / `LogAction` / `LogActionResolution` /
  `LogMulligan` / `LogBreedingAction` parameter lists were **not** read. Read them
  before writing the driver.
- `ActionEncoder.ValidateFieldSlot(Player, int)` signature unverified.
- `SelectionElement<int>` / `<bool>` producers unread, so for `generic_int` /
  `generic_bool` we cannot recover what the values MEAN per prompt. Those rows
  carry no candidate list, which is a real limit on scripting them.
- A further local-seat gate may hide inside `TurnStateMachine.SetMainPhase()`
  (line 1211), unread. Worth a follow-up.
- Suspected index bug at `SelectDigiXrosClass.cs:543` — marked uncertain, not
  confirmed by a run.

---

## E. Environment — verified 2026-08-21

- Unity **2021.3.45f2** installed; batchmode probe exited **0**, 0 CS errors.
  The `[Licensing::Client] Error: Code 10` line is a signature warning, not a
  failure — entitlements resolve and compilation succeeds.
- Headless compile + EditMode tests are therefore **available**:
  `Unity.exe -quit -batchmode -nographics -projectPath <DCGO> -logFile <log>`
- DCGO art bundle present; `Library/` imported (prior successful compile).
- Harness player build exists: `D:/dcgo-build/spike/DCGO.exe` + `manifest.json`.
- Harness queue idle: 0 pending, 0 claimed, 78 done; heartbeat 2h stale.
- **A `DCGO` process (PID 17840) runs from `Documents\DCGO_Application` — that is
  the user's own game client, not the harness. Do not kill it.**
- Base DCGO repo: branch `add-recording-mod-r2` @ `8c4f98cb6`, matching the
  worktree gitlink.

### ⚠ The base DCGO repo has ~8349 pre-existing dirty files

Mostly re-serialized `.asset` files, mtime 2026-08-16 — they predate this work.

**Never run `git add -A`, `git add .`, or `git commit -a` in the base DCGO repo.**
Stage only explicit paths you created or edited. A sweeping add would commit
8349 unrelated asset changes.
