# DCGO Scripted Input Driver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Teach the DCGO mod to play a scripted line — a fixed deck order plus an explicit action list — and to dump per-step game state, so DCGO can answer targeted questions instead of only reporting what its AI happened to do.

**Architecture:** Three new C# classes in the existing `Digimon.Harness` namespace. `DeckStacker` reorders the two harness deck-construction short-circuits. `ScriptedLine` holds the action cursor and the prompt-assertion logic as *pure, unit-testable* code. `InputDriver` is the thin glue consulted at the 13 `HarnessAuto.DrivesLocalSeat` gate sites. `StateDumper` writes a sidecar JSONL keyed by the recorder's step index. `HarnessJob` gains two optional fields so phase-1 jobs keep parsing unchanged.

**Tech Stack:** C# / Unity 2021.3.45f2, `JsonUtility` for job parsing, NUnit EditMode tests via a `Tests~/Editor` asmdef.

## Global Constraints

- **All DCGO edits happen in the BASE repo, never in a worktree** (CLAUDE.md rule 29). Resolve it once: `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`. Every path below is relative to `$BASE_DCGO`. **Never run `git submodule update --init DCGO` in a worktree** — that clones a multi-GB Unity checkout per worktree.
- Unity Editor version **2021.3.45f2 exactly** (`docs/DCGO_BUILD.md`).
- Job JSON field names are **snake_case on the C# fields themselves**. `JsonUtility` has no name-mapping attribute, so the C# field wears the wire name. See `HarnessJob.cs`'s existing fields.
- `JsonUtility` **silently ignores unknown fields**. This is load-bearing forward-compatibility and must not be replaced with a strict parser: a phase-1 client must tolerate phase-2 jobs.
- New `.cs` files need Unity to import them. **No `.meta` file next to a `.cs` file means Unity has not seen it, and a "clean" Console is reporting the *previous* compile.** Click into the Editor to force a rescan.
- **Stop Play before requeueing.** A running harness claims jobs the instant they appear.
- The recorder's step index is owned by `GameRecorder._stepIndex`. `StateDumper` must **read** it, never maintain a parallel counter — two counters drift and the differ silently misaligns.
- Do not commit the DCGO Assets art bundle. Verify `git -C "$BASE_DCGO" status` before committing.

## File Structure

| File | Responsibility |
|---|---|
| `Assets/Scripts/Script/Harness/DeckStacker.cs` (create) | Reorder a shuffled deck so a named prefix comes first. Pure static, no Unity types beyond the card entity. |
| `Assets/Scripts/Script/Harness/ScriptedLine.cs` (create) | The action cursor + prompt-assertion. Pure, no `MonoBehaviour`, no statics that survive a domain reload. |
| `Assets/Scripts/Script/Harness/InputDriver.cs` (create) | Glue: owns the active `ScriptedLine`, answers the 13 gate sites, aborts the job on mismatch. |
| `Assets/Scripts/Script/Harness/StateDumper.cs` (create) | Sidecar JSONL of normalized state, keyed by `GameRecorder`'s step index. |
| `Assets/Scripts/Script/Harness/HarnessJob.cs` (modify) | Add `deck_order` + `inputs`. |
| `Assets/Scripts/Script/Harness/JobWatcher.cs` (modify, `ApplyJob` ~line 242, `ClearOverrides` ~line 402) | Install and release the stacker/driver/dumper. |
| `Assets/Scripts/Script/CardObjectController.cs` (modify, lines 141 + 235) | Apply the stack at the two harness deck short-circuits. |
| The 13 gate sites (modify) | Consult `InputDriver` before falling through to `AutoSelect()`. |
| `Assets/Scripts/Script/Harness/Tests~/Editor/` (create) | NUnit EditMode tests + asmdef, mirroring `Recording/Tests~/Editor/`. |

**Why the pure/glue split:** DCGO has no way to unit-test a `MonoBehaviour` mid-game. The only existing C# test suite is `Recording/Tests~/Editor/ActionEncoderTests.cs`, a pure-logic NUnit suite. So all decision logic lives in `DeckStacker` and `ScriptedLine`, which are testable with no Unity runtime, and the glue that cannot be tested is kept trivial enough to review by eye.

> **`Tests~` is hidden from Unity on purpose.** A trailing `~` makes Unity skip the folder entirely, so these tests do not ship in a player build. To *run* them, temporarily rename `Tests~` to `Tests`, run the Editor's Test Runner (Window → General → Test Runner → EditMode), then rename it back before committing. Confirm the rename is reverted in `git status`.

---

### Task 1: `DeckStacker` — reorder a shuffled deck by a named prefix

**Files:**
- Create: `Assets/Scripts/Script/Harness/DeckStacker.cs`
- Create: `Assets/Scripts/Script/Harness/Tests~/Editor/Digimon.Harness.Tests.Editor.asmdef`
- Test: `Assets/Scripts/Script/Harness/Tests~/Editor/DeckStackerTests.cs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: `Digimon.Harness.DeckStacker.Apply(List<CEntity_Base> shuffled, string[] stack, out string error) -> List<CEntity_Base>`. Returns a NEW list; returns `shuffled` unchanged and sets `error` to `null` when `stack` is null/empty. Sets `error` to a non-null message and returns `null` when the stack names a card the deck does not contain, or names one more copies than the deck holds.

- [ ] **Step 1: Write the failing test**

Create `Assets/Scripts/Script/Harness/Tests~/Editor/Digimon.Harness.Tests.Editor.asmdef`:

```json
{
    "name": "Digimon.Harness.Tests.Editor",
    "rootNamespace": "Digimon.Harness.Tests",
    "references": [],
    "includePlatforms": ["Editor"],
    "excludePlatforms": [],
    "allowUnsafeCode": false,
    "overrideReferences": true,
    "precompiledReferences": ["nunit.framework.dll"],
    "autoReferenced": false,
    "defineConstraints": ["UNITY_INCLUDE_TESTS"],
    "versionDefines": [],
    "noEngineReferences": false
}
```

Create `Assets/Scripts/Script/Harness/Tests~/Editor/DeckStackerTests.cs`:

```csharp
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using Digimon.Harness;

namespace Digimon.Harness.Tests
{
    public class DeckStackerTests
    {
        // The stacker only needs an id off each entry, so the tests drive it
        // through a tiny stand-in rather than constructing real CEntity_Base
        // instances (which need Unity asset loading).
        private static List<string> Ids(List<string> deck) => deck;

        [Test]
        public void NullStack_ReturnsInputUnchanged()
        {
            var deck = new List<string> { "A", "B", "C" };
            var result = DeckStacker.ApplyIds(deck, null, out string error);
            Assert.IsNull(error);
            CollectionAssert.AreEqual(new[] { "A", "B", "C" }, result);
        }

        [Test]
        public void EmptyStack_ReturnsInputUnchanged()
        {
            var deck = new List<string> { "A", "B", "C" };
            var result = DeckStacker.ApplyIds(deck, new string[0], out string error);
            Assert.IsNull(error);
            CollectionAssert.AreEqual(new[] { "A", "B", "C" }, result);
        }

        [Test]
        public void PrefixMovesToFront_InStackOrder()
        {
            var deck = new List<string> { "A", "B", "C", "D" };
            var result = DeckStacker.ApplyIds(deck, new[] { "C", "A" }, out string error);
            Assert.IsNull(error);
            CollectionAssert.AreEqual(new[] { "C", "A", "B", "D" }, result);
        }

        [Test]
        public void RemainderKeepsShuffledOrder()
        {
            var deck = new List<string> { "A", "B", "C", "D", "E" };
            var result = DeckStacker.ApplyIds(deck, new[] { "D" }, out string error);
            Assert.IsNull(error);
            CollectionAssert.AreEqual(new[] { "D", "A", "B", "C", "E" }, result);
        }

        [Test]
        public void Duplicates_ConsumeOneCopyEach()
        {
            var deck = new List<string> { "A", "A", "A", "B" };
            var result = DeckStacker.ApplyIds(deck, new[] { "A", "A" }, out string error);
            Assert.IsNull(error);
            CollectionAssert.AreEqual(new[] { "A", "A", "A", "B" }, result);
            Assert.AreEqual(3, result.Count(x => x == "A"));
        }

        [Test]
        public void CardNotInDeck_IsAnError()
        {
            var deck = new List<string> { "A", "B" };
            var result = DeckStacker.ApplyIds(deck, new[] { "Z" }, out string error);
            Assert.IsNull(result);
            StringAssert.Contains("Z", error);
        }

        [Test]
        public void MoreCopiesThanDeckHolds_IsAnError()
        {
            var deck = new List<string> { "A", "B" };
            var result = DeckStacker.ApplyIds(deck, new[] { "A", "A" }, out string error);
            Assert.IsNull(result);
            StringAssert.Contains("A", error);
        }

        [Test]
        public void DeckLengthIsPreserved()
        {
            var deck = new List<string> { "A", "B", "C", "D", "E" };
            var result = DeckStacker.ApplyIds(deck, new[] { "E", "B" }, out string error);
            Assert.IsNull(error);
            Assert.AreEqual(5, result.Count);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Rename `Tests~` to `Tests`, then in the Unity Editor: Window → General → Test Runner → EditMode → Run All.

Expected: every `DeckStackerTests` case FAILS to compile with `The name 'DeckStacker' does not exist in the current context`.

- [ ] **Step 3: Write the implementation**

Create `Assets/Scripts/Script/Harness/DeckStacker.cs`:

```csharp
using System.Collections.Generic;

namespace Digimon.Harness
{
    /// <summary>
    /// Reorders a freshly-shuffled deck so a named prefix of card IDs sits on
    /// top, in the order named. Everything else keeps its shuffled order.
    /// </summary>
    /// <remarks>
    /// Applied ONLY at the two harness deck-construction short-circuits in
    /// <c>CardObjectController</c> (<c>DeckRecipie</c> and
    /// <c>DigitamaDeckRecipie</c>), never inside
    /// <c>RandomUtility.ShuffledDeckCards</c>.
    ///
    /// That placement is what makes "initial shuffle only" structural. Search
    /// and shuffle effects also route through <c>ShuffledDeckCards</c>; a
    /// stacker living there would silently re-impose the opening order when a
    /// card says "shuffle your deck", and the exam would confidently answer a
    /// question about a game that cannot occur. Mid-game
    /// <c>CardObjectController.Shuffle(Player)</c> never passes through a
    /// harness short-circuit, so there is nothing to exclude and no latch to
    /// get wrong.
    ///
    /// The short-circuits also already resolve the seat
    /// (<c>player == MasterPlayer</c>), which <c>ShuffledDeckCards</c> cannot —
    /// it takes no player argument, so a stacker there would have to guess the
    /// seat from call order.
    /// </remarks>
    public static class DeckStacker
    {
        /// <summary>
        /// Reorder <paramref name="shuffled"/> so <paramref name="stack"/>'s
        /// card IDs lead, in order. Returns a new list; returns the input
        /// unchanged when the stack is null or empty.
        /// Returns null and sets <paramref name="error"/> when the stack names
        /// a card the deck does not hold in sufficient quantity.
        /// </summary>
        public static List<CEntity_Base> Apply(
            List<CEntity_Base> shuffled, string[] stack, out string error)
        {
            error = null;
            if (shuffled == null) { error = "deck is null"; return null; }
            if (stack == null || stack.Length == 0) return shuffled;

            List<CEntity_Base> remainder = new List<CEntity_Base>(shuffled);
            List<CEntity_Base> front = new List<CEntity_Base>(stack.Length);

            foreach (string wanted in stack)
            {
                int at = -1;
                for (int i = 0; i < remainder.Count; i++)
                {
                    if (CardIdOf(remainder[i]) == wanted) { at = i; break; }
                }
                if (at < 0)
                {
                    // Deliberately loud. A stack that silently drops a card
                    // produces a game that looks fine and answers the wrong
                    // question -- the exact failure this whole harness exists
                    // to avoid.
                    error = "stack names '" + wanted +
                            "', which the deck does not contain (or not enough copies of)";
                    return null;
                }
                front.Add(remainder[at]);
                remainder.RemoveAt(at);
            }

            front.AddRange(remainder);
            return front;
        }

        /// <summary>
        /// String-keyed twin of <see cref="Apply"/>, used by the unit tests so
        /// the ordering rules can be exercised without Unity asset loading.
        /// Both delegate to the same algorithm shape; keep them in step.
        /// </summary>
        public static List<string> ApplyIds(List<string> shuffled, string[] stack, out string error)
        {
            error = null;
            if (shuffled == null) { error = "deck is null"; return null; }
            if (stack == null || stack.Length == 0) return shuffled;

            List<string> remainder = new List<string>(shuffled);
            List<string> front = new List<string>(stack.Length);

            foreach (string wanted in stack)
            {
                int at = remainder.IndexOf(wanted);
                if (at < 0)
                {
                    error = "stack names '" + wanted +
                            "', which the deck does not contain (or not enough copies of)";
                    return null;
                }
                front.Add(remainder[at]);
                remainder.RemoveAt(at);
            }

            front.AddRange(remainder);
            return front;
        }

        /// <summary>Card ID as the job spec writes it (e.g. "EX12-035").</summary>
        private static string CardIdOf(CEntity_Base entity)
        {
            return entity == null ? null : entity.CardID;
        }
    }
}
```

> **Verify `CEntity_Base.CardID` is the right member before running.** Confirm with:
> ```bash
> grep -n "CardID\|public string card" "$BASE_DCGO/Assets/Scripts/Script/CEntity_Base.cs" | head
> ```
> If the printed-ID member has a different name, fix `CardIdOf` only — the rest of the class is id-agnostic.

- [ ] **Step 4: Run tests to verify they pass**

Test Runner → EditMode → Run All.
Expected: 8 `DeckStackerTests` cases PASS.

- [ ] **Step 5: Rename `Tests` back to `Tests~`, then commit**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git -C "$BASE_DCGO" status --short
git -C "$BASE_DCGO" add Assets/Scripts/Script/Harness/DeckStacker.cs \
    Assets/Scripts/Script/Harness/DeckStacker.cs.meta \
    "Assets/Scripts/Script/Harness/Tests~/"
git -C "$BASE_DCGO" commit -m "harness: DeckStacker reorders a deck by a named prefix"
```

Confirm `git status` shows no `Tests/` directory (only `Tests~/`) before committing.

---

### Task 2: Job schema — `deck_order` and `inputs`

**Files:**
- Modify: `Assets/Scripts/Script/Harness/HarnessJob.cs`
- Test: `Assets/Scripts/Script/Harness/Tests~/Editor/HarnessJobTests.cs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `HarnessJob.deck_order` (`HarnessJobDeckOrder` with `string[] p0`, `string[] p1`); `HarnessJob.inputs` (`HarnessJobStep[]`, each with `int actor`, `int action_id`, `string expect_prompt`, `int expect_count`, `string[] expect_candidates`). `HarnessJob.IsScripted` (bool). Tasks 3–5 consume all of these.

- [ ] **Step 1: Write the failing test**

Create `Assets/Scripts/Script/Harness/Tests~/Editor/HarnessJobTests.cs`:

```csharp
using NUnit.Framework;
using Digimon.Harness;

namespace Digimon.Harness.Tests
{
    public class HarnessJobTests
    {
        private const string Phase1Job = @"{
            ""job_id"": ""vol-00042"",
            ""policy"": ""ai"",
            ""decks"": { ""p0"": [""EX12-035""], ""p1"": [""BT16-082""] },
            ""first_player"": 0,
            ""seed"": 424242,
            ""limits"": { ""max_turns"": 40, ""timeout_seconds"": 180 }
        }";

        private const string ScriptedJob = @"{
            ""job_id"": ""exam-EX12-035-0"",
            ""policy"": ""scripted"",
            ""decks"": { ""p0"": [""EX12-035""], ""p1"": [""BT16-082""] },
            ""deck_order"": { ""p0"": [""ST1-02"", ""EX12-035""], ""p1"": [] },
            ""inputs"": [
                { ""actor"": 0, ""action_id"": 12, ""expect_prompt"": ""main_phase"" },
                { ""actor"": 0, ""action_id"": 1150, ""expect_prompt"": ""select_permanent"", ""expect_count"": 1 }
            ],
            ""first_player"": 0,
            ""seed"": 424242,
            ""limits"": { ""max_turns"": 40, ""timeout_seconds"": 180 }
        }";

        [Test]
        public void Phase1Job_StillParses()
        {
            HarnessJob job = HarnessJob.Parse(Phase1Job);
            Assert.IsNotNull(job);
            Assert.AreEqual("vol-00042", job.job_id);
            Assert.AreEqual("ai", job.policy);
        }

        [Test]
        public void Phase1Job_IsNotScripted()
        {
            HarnessJob job = HarnessJob.Parse(Phase1Job);
            Assert.IsFalse(job.IsScripted);
        }

        [Test]
        public void Phase1Job_HasNoDeckOrderOrInputs()
        {
            HarnessJob job = HarnessJob.Parse(Phase1Job);
            // Absent arrays must normalize to empty, never null: every consumer
            // would otherwise need its own null guard, and one missing guard is
            // a NullReferenceException mid-game.
            Assert.IsNotNull(job.inputs);
            Assert.AreEqual(0, job.inputs.Length);
            Assert.IsNotNull(job.deck_order);
            Assert.AreEqual(0, job.deck_order.p0.Length);
        }

        [Test]
        public void ScriptedJob_IsScripted()
        {
            HarnessJob job = HarnessJob.Parse(ScriptedJob);
            Assert.IsNotNull(job);
            Assert.IsTrue(job.IsScripted);
        }

        [Test]
        public void ScriptedJob_ParsesDeckOrder()
        {
            HarnessJob job = HarnessJob.Parse(ScriptedJob);
            CollectionAssert.AreEqual(new[] { "ST1-02", "EX12-035" }, job.deck_order.p0);
            Assert.AreEqual(0, job.deck_order.p1.Length);
        }

        [Test]
        public void ScriptedJob_ParsesInputs()
        {
            HarnessJob job = HarnessJob.Parse(ScriptedJob);
            Assert.AreEqual(2, job.inputs.Length);
            Assert.AreEqual(0, job.inputs[0].actor);
            Assert.AreEqual(12, job.inputs[0].action_id);
            Assert.AreEqual("main_phase", job.inputs[0].expect_prompt);
            Assert.AreEqual("select_permanent", job.inputs[1].expect_prompt);
            Assert.AreEqual(1, job.inputs[1].expect_count);
        }

        [Test]
        public void ScriptedPolicyWithNoInputs_IsRejected()
        {
            // A scripted job with an empty line would start a game nobody
            // drives and hang until the timeout -- indistinguishable from a
            // hung Unity. Reject it at parse time instead.
            string bad = ScriptedJob.Replace(
                @"""inputs"": [
                { ""actor"": 0, ""action_id"": 12, ""expect_prompt"": ""main_phase"" },
                { ""actor"": 0, ""action_id"": 1150, ""expect_prompt"": ""select_permanent"", ""expect_count"": 1 }
            ],", @"""inputs"": [],");
            Assert.IsNull(HarnessJob.Parse(bad));
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Rename `Tests~` → `Tests`, Test Runner → EditMode → Run All.
Expected: compile error `'HarnessJob' does not contain a definition for 'IsScripted'`.

- [ ] **Step 3: Write the implementation**

Replace the body of `Assets/Scripts/Script/Harness/HarnessJob.cs` with:

```csharp
using System;
using UnityEngine;

namespace Digimon.Harness
{
    /// <summary>
    /// One unattended game, as written by the `dcgo-harness submit` CLI.
    /// </summary>
    /// <remarks>
    /// Parsed with Unity's <see cref="JsonUtility"/>, which handles nested
    /// [Serializable] classes and arrays and silently ignores unknown fields —
    /// exactly the forward-compatibility we want.
    ///
    /// Field names are snake_case to match the JSON verbatim; JsonUtility has no
    /// name-mapping attribute, so the C# fields wear the wire names.
    /// </remarks>
    [Serializable]
    public class HarnessJob
    {
        public string job_id;
        public string policy;
        public HarnessJobDecks decks;
        public int first_player;
        public long seed;
        public HarnessJobLimits limits;

        /// <summary>Fixed prefix of the initial draw order, per seat. May be empty.</summary>
        public HarnessJobDeckOrder deck_order;

        /// <summary>The scripted line. Empty for `policy: "ai"` jobs.</summary>
        public HarnessJobStep[] inputs;

        /// <summary>
        /// True when this job carries a line for <see cref="InputDriver"/> to
        /// play. Keyed off the policy string rather than off `inputs.Length`
        /// so a malformed scripted job fails loudly instead of silently
        /// degrading into an AI game that reports a plausible-looking result.
        /// </summary>
        public bool IsScripted => policy == "scripted";

        /// <summary>Parse a job file. Returns null when the text is unusable.</summary>
        public static HarnessJob Parse(string json)
        {
            if (string.IsNullOrEmpty(json)) return null;
            try
            {
                HarnessJob job = JsonUtility.FromJson<HarnessJob>(json);
                if (job == null || string.IsNullOrEmpty(job.job_id)) return null;
                if (job.decks == null || job.decks.p0 == null || job.decks.p1 == null) return null;
                if (job.limits == null) job.limits = new HarnessJobLimits();

                // Normalize absent optional collections to empty rather than
                // null. Every consumer would otherwise need its own null guard,
                // and one missing guard is a NullReferenceException mid-game.
                if (job.deck_order == null) job.deck_order = new HarnessJobDeckOrder();
                if (job.deck_order.p0 == null) job.deck_order.p0 = new string[0];
                if (job.deck_order.p1 == null) job.deck_order.p1 = new string[0];
                if (job.inputs == null) job.inputs = new HarnessJobStep[0];

                // A scripted job with no line would start a game nobody drives
                // and hang until the timeout -- indistinguishable from a hung
                // Unity, which is the failure mode the heartbeat exists to make
                // legible. Reject it here instead.
                if (job.IsScripted && job.inputs.Length == 0)
                {
                    Debug.LogError("[Harness] scripted job " + job.job_id + " carries no inputs");
                    return null;
                }

                return job;
            }
            catch (Exception e)
            {
                Debug.LogError("[Harness] job parse failed: " + e.Message);
                return null;
            }
        }
    }

    [Serializable]
    public class HarnessJobDecks
    {
        public string[] p0;
        public string[] p1;
    }

    [Serializable]
    public class HarnessJobDeckOrder
    {
        public string[] p0 = new string[0];
        public string[] p1 = new string[0];
    }

    /// <summary>
    /// One scripted decision: the action to feed, plus the prompt the author
    /// expects DCGO to be asking at that moment.
    /// </summary>
    /// <remarks>
    /// `expect_prompt` is asserted BEFORE the action is fed. A driver that
    /// answers whatever it is asked will, on a single ordering mismatch,
    /// desynchronize the entire remainder of the line while every step still
    /// looks successful.
    /// </remarks>
    [Serializable]
    public class HarnessJobStep
    {
        public int actor;
        public int action_id;
        /// <summary>Expected prompt kind. Empty means "do not assert".</summary>
        public string expect_prompt;
        /// <summary>Expected number of picks. -1 (default) means "do not assert".</summary>
        public int expect_count = -1;
        /// <summary>Expected candidate card IDs, order-insensitive. Empty means "do not assert".</summary>
        public string[] expect_candidates = new string[0];
    }

    [Serializable]
    public class HarnessJobLimits
    {
        public int max_turns = 40;
        public int timeout_seconds = 180;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Test Runner → EditMode → Run All.
Expected: 7 `HarnessJobTests` cases PASS, 8 `DeckStackerTests` still PASS.

- [ ] **Step 5: Rename `Tests` back to `Tests~`, then commit**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git -C "$BASE_DCGO" add Assets/Scripts/Script/Harness/HarnessJob.cs "Assets/Scripts/Script/Harness/Tests~/"
git -C "$BASE_DCGO" commit -m "harness: job carries deck_order + scripted inputs"
```

---

### Task 3: Wire `DeckStacker` into the two deck short-circuits

**Files:**
- Modify: `Assets/Scripts/Script/CardObjectController.cs:141` (`DeckRecipie`) and `:235` (`DigitamaDeckRecipie`)
- Modify: `Assets/Scripts/Script/Harness/JobWatcher.cs` (`ApplyJob` ~242, `ClearOverrides` ~402)

**Interfaces:**
- Consumes: `DeckStacker.Apply` (Task 1); `HarnessJob.deck_order` (Task 2).
- Produces: `CardObjectController.HarnessDeckOrderP0` / `HarnessDeckOrderP1` (`static string[]`, default `null`), released by `ClearOverrides`.

- [ ] **Step 1: Add the override fields**

In `Assets/Scripts/Script/CardObjectController.cs`, directly below the existing lines 19-20:

```csharp
    public static DeckData HarnessDeckOverrideP0 = null;
    public static DeckData HarnessDeckOverrideP1 = null;
    // [Harness mod - phase 2] Fixed prefix of the initial draw order per seat.
    // Applied ONLY in the two short-circuits below, which is what keeps
    // mid-game "shuffle your deck" effects honest -- see DeckStacker's remarks.
    public static string[] HarnessDeckOrderP0 = null;
    public static string[] HarnessDeckOrderP1 = null;
```

- [ ] **Step 2: Apply the stack in `DeckRecipie`**

Replace the harness short-circuit at line 141:

```csharp
            if (HarnessDeckOverrideP0 != null && HarnessDeckOverrideP1 != null)
            {
                DeckData chosen = (player == MasterPlayer) ? HarnessDeckOverrideP0 : HarnessDeckOverrideP1;
                return RandomUtility.ShuffledDeckCards(chosen.DeckCards());
            }
```

with:

```csharp
            if (HarnessDeckOverrideP0 != null && HarnessDeckOverrideP1 != null)
            {
                bool isP0 = (player == MasterPlayer);
                DeckData chosen = isP0 ? HarnessDeckOverrideP0 : HarnessDeckOverrideP1;
                List<CEntity_Base> shuffled = RandomUtility.ShuffledDeckCards(chosen.DeckCards());

                string[] order = isP0 ? HarnessDeckOrderP0 : HarnessDeckOrderP1;
                List<CEntity_Base> stacked =
                    Digimon.Harness.DeckStacker.Apply(shuffled, order, out string stackError);
                if (stackError != null)
                {
                    // Fail the job rather than play a game whose opening hand is
                    // not the one the scenario asked for. A silently-unstacked
                    // deck answers a different question and reads as a pass.
                    Debug.LogError("[Harness] main deck stack failed: " + stackError);
                    if (Digimon.Harness.JobWatcher.Instance != null)
                    {
                        Digimon.Harness.JobWatcher.Instance.AbortCurrentJob(
                            "main deck stack failed: " + stackError);
                    }
                    return shuffled;
                }
                return stacked;
            }
```

- [ ] **Step 3: Apply the stack in `DigitamaDeckRecipie`**

Replace the harness short-circuit at line 235 with the same shape, differing only in the deck accessor and the log text:

```csharp
            if (HarnessDeckOverrideP0 != null && HarnessDeckOverrideP1 != null)
            {
                bool isP0 = (player == MasterPlayer);
                DeckData chosen = isP0 ? HarnessDeckOverrideP0 : HarnessDeckOverrideP1;
                List<CEntity_Base> shuffled = RandomUtility.ShuffledDeckCards(chosen.DigitamaDeckCards());

                // The egg deck draws from the SAME per-seat order array. A
                // scenario naming only main-deck cards leaves the egg order
                // untouched, because DeckStacker.Apply errors on a card the
                // deck does not hold -- so egg stacking is opt-in by naming an
                // egg card, and mis-naming is loud rather than silent.
                string[] order = isP0 ? HarnessDeckOrderP0 : HarnessDeckOrderP1;
                List<CEntity_Base> stacked =
                    Digimon.Harness.DeckStacker.Apply(shuffled, order, out string stackError);
                if (stackError != null)
                {
                    Debug.Log("[Harness] egg deck not stacked (" + stackError +
                              "); using shuffled order");
                    return shuffled;
                }
                return stacked;
            }
```

> **Note the deliberate asymmetry.** A stack error on the MAIN deck aborts the job; on the EGG deck it logs and continues. The same `deck_order` array is offered to both decks, so a main-deck-only stack *necessarily* fails to resolve against the egg deck — treating that as fatal would make every ordinary scenario abort. Task 8's acceptance run must confirm an egg-only stack still applies.

- [ ] **Step 4: Install and release the arrays in `JobWatcher`**

In `ApplyJob`, immediately after the two `HarnessDeckOverride` assignments (~line 248):

```csharp
            CardObjectController.HarnessDeckOrderP0 = job.deck_order.p0;
            CardObjectController.HarnessDeckOrderP1 = job.deck_order.p1;
```

In `ClearOverrides` (~line 402), alongside the existing deck-override clears:

```csharp
            CardObjectController.HarnessDeckOrderP0 = null;
            CardObjectController.HarnessDeckOrderP1 = null;
```

- [ ] **Step 5: Add `AbortCurrentJob` to `JobWatcher`**

Next to the existing `ClearCurrentJob` (~line 351):

```csharp
        /// <summary>
        /// Abandon the running job and file it as failed. Used when the harness
        /// discovers mid-game that it cannot honor the job's contract -- a deck
        /// stack that will not resolve, or a scripted prompt mismatch.
        /// </summary>
        /// <remarks>
        /// This is deliberately distinct from the turn-cap path, which files a
        /// usable `partial`. A job aborted here produced a game that answers a
        /// DIFFERENT question than the one asked, so its recording must never be
        /// triaged as evidence.
        /// </remarks>
        public void AbortCurrentJob(string reason)
        {
            if (CurrentJob == null) return;
            Debug.LogError("[Harness] aborting job " + CurrentJob.job_id + ": " + reason);
            _abortReason = reason;
            _abortRequested = true;
        }

        private static bool _abortRequested;
        private static string _abortReason;
```

Then in `PollLoop`, immediately after the `TouchHeartbeat()` call, add the drain:

```csharp
                if (_abortRequested && CurrentJob != null)
                {
                    _abortRequested = false;
                    string reason = _abortReason;
                    _abortReason = null;
                    // Route through the same failure filing the rest of the
                    // watcher uses, on the main thread, rather than from
                    // whatever coroutine noticed the problem.
                    FailRunningJob(reason);
                }
```

Add `FailRunningJob` next to `Fail`, filing a `JobResult` with `outcome: "failed"` and `message: reason`, then calling `ClearCurrentJob()` and `ClearOverrides()`. Model it on the existing `Fail(string claimedPath, string message)` body, which already moves the claimed file into `HarnessConfig.FailedDir`.

- [ ] **Step 6: Verify compilation**

In the Unity Editor, confirm the Console shows no compile errors and that `.meta` files now exist beside every new `.cs`:

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
ls "$BASE_DCGO/Assets/Scripts/Script/Harness/"*.meta
```

Expected: a `.meta` for `DeckStacker.cs`. **A missing `.meta` means Unity has not imported the file and the Console is reporting the previous compile.**

- [ ] **Step 7: Commit**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git -C "$BASE_DCGO" add Assets/Scripts/Script/CardObjectController.cs Assets/Scripts/Script/Harness/JobWatcher.cs
git -C "$BASE_DCGO" commit -m "harness: apply deck_order at the two deck short-circuits"
```

---

### Task 4: `ScriptedLine` — the cursor and prompt assertion

**Files:**
- Create: `Assets/Scripts/Script/Harness/ScriptedLine.cs`
- Test: `Assets/Scripts/Script/Harness/Tests~/Editor/ScriptedLineTests.cs`

**Interfaces:**
- Consumes: `HarnessJobStep` (Task 2).
- Produces:
  - `class PromptContext { public string Kind; public int Count; public string[] Candidates; }`
  - `class ScriptedLine`
    - `ScriptedLine(HarnessJobStep[] steps)`
    - `bool IsExhausted { get; }`
    - `int Cursor { get; }`
    - `bool TryTake(int actor, PromptContext ctx, out int actionId, out string mismatch)` — advances the cursor on success; on failure leaves it and sets `mismatch`.

- [ ] **Step 1: Write the failing test**

Create `Assets/Scripts/Script/Harness/Tests~/Editor/ScriptedLineTests.cs`:

```csharp
using NUnit.Framework;
using Digimon.Harness;

namespace Digimon.Harness.Tests
{
    public class ScriptedLineTests
    {
        private static HarnessJobStep Step(int actor, int id, string prompt,
                                           int count = -1, string[] candidates = null)
        {
            return new HarnessJobStep
            {
                actor = actor,
                action_id = id,
                expect_prompt = prompt,
                expect_count = count,
                expect_candidates = candidates ?? new string[0],
            };
        }

        private static PromptContext Ctx(string kind, int count = -1, string[] candidates = null)
        {
            return new PromptContext { Kind = kind, Count = count, Candidates = candidates ?? new string[0] };
        }

        [Test]
        public void MatchingStep_YieldsActionAndAdvances()
        {
            var line = new ScriptedLine(new[] { Step(0, 12, "main_phase"), Step(0, 13, "main_phase") });
            Assert.IsTrue(line.TryTake(0, Ctx("main_phase"), out int id, out string mismatch));
            Assert.AreEqual(12, id);
            Assert.IsNull(mismatch);
            Assert.AreEqual(1, line.Cursor);
        }

        [Test]
        public void WrongActor_IsAMismatchAndDoesNotAdvance()
        {
            var line = new ScriptedLine(new[] { Step(0, 12, "main_phase") });
            Assert.IsFalse(line.TryTake(1, Ctx("main_phase"), out int id, out string mismatch));
            StringAssert.Contains("actor", mismatch);
            Assert.AreEqual(0, line.Cursor);
        }

        [Test]
        public void WrongPromptKind_IsAMismatchAndDoesNotAdvance()
        {
            var line = new ScriptedLine(new[] { Step(0, 12, "main_phase") });
            Assert.IsFalse(line.TryTake(0, Ctx("select_permanent"), out int id, out string mismatch));
            StringAssert.Contains("main_phase", mismatch);
            StringAssert.Contains("select_permanent", mismatch);
            Assert.AreEqual(0, line.Cursor);
        }

        [Test]
        public void EmptyExpectPrompt_SkipsTheKindAssertion()
        {
            var line = new ScriptedLine(new[] { Step(0, 12, "") });
            Assert.IsTrue(line.TryTake(0, Ctx("anything_at_all"), out int id, out string mismatch));
            Assert.AreEqual(12, id);
        }

        [Test]
        public void CountMismatch_IsAMismatch()
        {
            var line = new ScriptedLine(new[] { Step(0, 12, "select_permanent", count: 2) });
            Assert.IsFalse(line.TryTake(0, Ctx("select_permanent", count: 1), out int id, out string mismatch));
            StringAssert.Contains("count", mismatch);
        }

        [Test]
        public void NegativeExpectCount_SkipsTheCountAssertion()
        {
            var line = new ScriptedLine(new[] { Step(0, 12, "select_permanent", count: -1) });
            Assert.IsTrue(line.TryTake(0, Ctx("select_permanent", count: 3), out int id, out string mismatch));
        }

        [Test]
        public void CandidatesCompareAsAMultiset_NotByOrder()
        {
            var line = new ScriptedLine(new[] {
                Step(0, 12, "select_permanent", candidates: new[] { "A", "B" }) });
            Assert.IsTrue(line.TryTake(0, Ctx("select_permanent", candidates: new[] { "B", "A" }),
                                       out int id, out string mismatch));
        }

        [Test]
        public void CandidateSetMismatch_IsAMismatch()
        {
            var line = new ScriptedLine(new[] {
                Step(0, 12, "select_permanent", candidates: new[] { "A", "B" }) });
            Assert.IsFalse(line.TryTake(0, Ctx("select_permanent", candidates: new[] { "A", "C" }),
                                        out int id, out string mismatch));
            StringAssert.Contains("candidates", mismatch);
        }

        [Test]
        public void ExhaustedLine_IsAMismatchNotASilentPass()
        {
            // DCGO asking one more question than the line answers means the two
            // engines disagree about how many decisions this position has --
            // exactly the divergence class that never shows up as an illegal
            // action. It must never fall through to AutoSelect.
            var line = new ScriptedLine(new[] { Step(0, 12, "main_phase") });
            line.TryTake(0, Ctx("main_phase"), out _, out _);
            Assert.IsTrue(line.IsExhausted);
            Assert.IsFalse(line.TryTake(0, Ctx("main_phase"), out int id, out string mismatch));
            StringAssert.Contains("exhausted", mismatch);
        }

        [Test]
        public void EmptyLine_IsExhaustedImmediately()
        {
            var line = new ScriptedLine(new HarnessJobStep[0]);
            Assert.IsTrue(line.IsExhausted);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Rename `Tests~` → `Tests`, Test Runner → EditMode → Run All.
Expected: compile error `The name 'ScriptedLine' does not exist in the current context`.

- [ ] **Step 3: Write the implementation**

Create `Assets/Scripts/Script/Harness/ScriptedLine.cs`:

```csharp
using System;
using System.Collections.Generic;

namespace Digimon.Harness
{
    /// <summary>What DCGO is asking, at the moment it asks.</summary>
    public class PromptContext
    {
        /// <summary>Prompt kind, matching the recorder's `selection.prompt` vocabulary.</summary>
        public string Kind;
        /// <summary>Number of picks required, or -1 when the site does not know.</summary>
        public int Count = -1;
        /// <summary>Selectable card IDs, unordered.</summary>
        public string[] Candidates = new string[0];
    }

    /// <summary>
    /// The scripted action cursor, plus the assertion that DCGO is asking the
    /// question the author expected.
    /// </summary>
    /// <remarks>
    /// Pure by design: no MonoBehaviour, no Unity types, no statics. That is
    /// what makes the whole decision surface unit-testable, since DCGO has no
    /// way to test a MonoBehaviour mid-game.
    ///
    /// The assertion is the point of the class. A driver that answers whatever
    /// it is asked will, on a single ordering mismatch, desynchronize the
    /// entire remainder of the line while every step still looks successful --
    /// and report a confident wrong answer.
    /// </remarks>
    public class ScriptedLine
    {
        private readonly HarnessJobStep[] _steps;
        private int _cursor;

        public ScriptedLine(HarnessJobStep[] steps)
        {
            _steps = steps ?? new HarnessJobStep[0];
        }

        public int Cursor => _cursor;
        public bool IsExhausted => _cursor >= _steps.Length;

        /// <summary>
        /// Take the next scripted action if it matches what is being asked.
        /// Advances the cursor only on success.
        /// </summary>
        public bool TryTake(int actor, PromptContext ctx, out int actionId, out string mismatch)
        {
            actionId = -1;
            mismatch = null;

            if (IsExhausted)
            {
                // NOT a silent fall-through to AutoSelect. DCGO asking a
                // question the line has no answer for means the two engines
                // disagree about how many decisions this position contains --
                // which is a finding, not an error.
                mismatch = "line exhausted after " + _steps.Length +
                           " steps, but DCGO asked actor " + actor +
                           " a '" + (ctx == null ? "<null>" : ctx.Kind) + "' prompt";
                return false;
            }

            HarnessJobStep step = _steps[_cursor];

            if (step.actor != actor)
            {
                mismatch = "step " + _cursor + " expected actor " + step.actor +
                           " but DCGO asked actor " + actor;
                return false;
            }

            string kind = ctx == null ? null : ctx.Kind;

            if (!string.IsNullOrEmpty(step.expect_prompt) && step.expect_prompt != kind)
            {
                mismatch = "step " + _cursor + " expected prompt '" + step.expect_prompt +
                           "' but DCGO asked '" + kind + "'";
                return false;
            }

            if (step.expect_count >= 0 && ctx != null && ctx.Count >= 0 &&
                step.expect_count != ctx.Count)
            {
                mismatch = "step " + _cursor + " expected count " + step.expect_count +
                           " but DCGO asked for count " + ctx.Count;
                return false;
            }

            if (step.expect_candidates != null && step.expect_candidates.Length > 0)
            {
                string[] actual = (ctx == null || ctx.Candidates == null)
                    ? new string[0] : ctx.Candidates;
                if (!SameMultiset(step.expect_candidates, actual))
                {
                    mismatch = "step " + _cursor + " expected candidates [" +
                               string.Join(",", step.expect_candidates) +
                               "] but DCGO offered [" + string.Join(",", actual) + "]";
                    return false;
                }
            }

            actionId = step.action_id;
            _cursor++;
            return true;
        }

        /// <summary>
        /// Order-insensitive, duplicate-sensitive comparison. Candidate order is
        /// a DCGO presentation detail, but the NUMBER of copies offered is
        /// semantics -- so this is a multiset compare, not a set compare.
        /// </summary>
        private static bool SameMultiset(string[] a, string[] b)
        {
            if (a.Length != b.Length) return false;
            Dictionary<string, int> counts = new Dictionary<string, int>();
            foreach (string s in a)
            {
                counts.TryGetValue(s ?? "", out int n);
                counts[s ?? ""] = n + 1;
            }
            foreach (string s in b)
            {
                string k = s ?? "";
                if (!counts.TryGetValue(k, out int n) || n == 0) return false;
                counts[k] = n - 1;
            }
            foreach (KeyValuePair<string, int> kv in counts)
            {
                if (kv.Value != 0) return false;
            }
            return true;
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Test Runner → EditMode → Run All.
Expected: 10 `ScriptedLineTests` cases PASS; Tasks 1–2 suites still PASS.

- [ ] **Step 5: Rename `Tests` back to `Tests~`, then commit**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git -C "$BASE_DCGO" add Assets/Scripts/Script/Harness/ScriptedLine.cs "Assets/Scripts/Script/Harness/Tests~/"
git -C "$BASE_DCGO" commit -m "harness: ScriptedLine cursor asserts the prompt before answering"
```

---

### Task 5: `InputDriver` — the gate-site glue

**Files:**
- Create: `Assets/Scripts/Script/Harness/InputDriver.cs`
- Modify: `Assets/Scripts/Script/Harness/JobWatcher.cs` (`ApplyJob`, `ClearOverrides`)

**Interfaces:**
- Consumes: `ScriptedLine`, `PromptContext` (Task 4); `HarnessJob.IsScripted`, `HarnessJob.inputs` (Task 2); `JobWatcher.AbortCurrentJob` (Task 3).
- Produces:
  - `InputDriver.IsActive` (bool) — true only while a scripted job is running.
  - `InputDriver.TryAnswer(int actor, PromptContext ctx, out int actionId)` — true when the driver supplies the answer. On a mismatch it aborts the job and returns false.
  - `InputDriver.Install(HarnessJob job)` / `InputDriver.Release()`.

- [ ] **Step 1: Write the implementation**

There is no unit test for this task: every branch depends on `JobWatcher.Instance` and Unity runtime state. The testable decision logic all lives in `ScriptedLine` (Task 4), which is why this class is kept this thin. Its acceptance is Task 8's end-to-end run.

Create `Assets/Scripts/Script/Harness/InputDriver.cs`:

```csharp
using UnityEngine;

namespace Digimon.Harness
{
    /// <summary>
    /// Feeds a scripted line into DCGO at the decision points that would
    /// otherwise route to <c>AutoSelect()</c>.
    /// </summary>
    /// <remarks>
    /// Consulted from the 13 live <see cref="HarnessAuto.DrivesLocalSeat"/> gate
    /// sites plus <c>TurnStateMachine.QueueMainPhaseAction</c>. Those gates
    /// currently choose between "show UI and wait" and "let the AI decide"; with
    /// a scripted job active there is a third answer, and it wins.
    ///
    /// Deliberately thin. Every decision this class could get wrong lives in
    /// <see cref="ScriptedLine"/>, which is unit-tested; DCGO has no way to test
    /// a MonoBehaviour mid-game, so anything untestable is kept small enough to
    /// review by eye.
    /// </remarks>
    public static class InputDriver
    {
        private static ScriptedLine _line;

        /// <summary>True while a scripted job is driving both seats.</summary>
        public static bool IsActive => _line != null;

        public static void Install(HarnessJob job)
        {
            if (job == null || !job.IsScripted) { _line = null; return; }
            _line = new ScriptedLine(job.inputs);
            Debug.Log("[Harness] scripted line installed: " + job.inputs.Length + " steps");
        }

        public static void Release()
        {
            _line = null;
        }

        /// <summary>
        /// Supply the next scripted action for <paramref name="actor"/>, if this
        /// is the question the line expects. Aborts the job on a mismatch.
        /// </summary>
        public static bool TryAnswer(int actor, PromptContext ctx, out int actionId)
        {
            actionId = -1;
            if (_line == null) return false;

            if (_line.TryTake(actor, ctx, out actionId, out string mismatch)) return true;

            // A prompt mismatch is a FINDING, not an error: "our engine expected
            // a choice here and DCGO never asked" (or asked a different one) is
            // exactly the divergence class that never surfaces as an illegal
            // action. Abort loudly rather than answering blind -- answering
            // would desync the whole remainder while every later step still
            // looked successful.
            if (JobWatcher.Instance != null)
            {
                JobWatcher.Instance.AbortCurrentJob("prompt mismatch: " + mismatch);
            }
            else
            {
                Debug.LogError("[Harness] prompt mismatch with no JobWatcher: " + mismatch);
            }
            _line = null;
            return false;
        }

        /// <summary>
        /// True when the line ran to its end. Checked at game end so a job whose
        /// line stopped early is not filed as a clean completion.
        /// </summary>
        public static bool IsExhausted => _line == null || _line.IsExhausted;

        /// <summary>How many scripted steps have been consumed.</summary>
        public static int Cursor => _line == null ? 0 : _line.Cursor;
    }
}
```

- [ ] **Step 2: Install and release it in `JobWatcher`**

In `ApplyJob`, immediately after the `HarnessDeckOrder` assignments added in Task 3:

```csharp
            InputDriver.Install(job);
```

In `ClearOverrides`, alongside the other releases:

```csharp
            InputDriver.Release();
```

- [ ] **Step 3: Verify compilation**

Unity Console shows no errors; `InputDriver.cs.meta` exists.

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
ls "$BASE_DCGO/Assets/Scripts/Script/Harness/InputDriver.cs.meta"
```

- [ ] **Step 4: Commit**

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git -C "$BASE_DCGO" add Assets/Scripts/Script/Harness/InputDriver.cs Assets/Scripts/Script/Harness/JobWatcher.cs
git -C "$BASE_DCGO" commit -m "harness: InputDriver answers scripted prompts, aborts on mismatch"
```

---

### Task 6: Consult `InputDriver` at the 13 gate sites

**Files (all under `Assets/Scripts/Script/`):**

| File | Line |
|---|---|
| `CardController.cs` | 700 |
| `MultipleSkills.cs` | 193, 274 |
| `OptionalSkill.cs` | 63 |
| `SelectAttackEffect.cs` | 234 |
| `SelectCardEffect.cs` | 383 |
| `SelectCountEffect.cs` | 131 |
| `SelectDigiXrosClass.cs` | 484, 569 |
| `SelectHandEffect.cs` | 189 |
| `SelectPermanentEffect.cs` | 298 |
| `UserSelectionManager.cs` | 128, 187 |

Plus `TurnStateMachine.QueueMainPhaseAction`.

**Interfaces:**
- Consumes: `InputDriver.IsActive`, `InputDriver.TryAnswer`, `PromptContext` (Tasks 4–5).
- Produces: no new API. Every site must construct a `PromptContext` whose `Kind` matches the `prompt` string the *recorder* already emits at that same site via `LogSelectionRow`, so a recorded corpus and a scripted line speak one vocabulary.

- [ ] **Step 1: Re-derive the gate list — do not trust this table**

CLAUDE.md rule 27 exists because a stale hook map caused two separate false diagnoses. The same discipline applies here.

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
grep -rn "DrivesLocalSeat" "$BASE_DCGO/Assets/Scripts/Script/" --include=*.cs | grep -v "Harness/HarnessAuto.cs"
```

Expected: **15 hits.** Two of them (`CardController.cs:376`, `ICardEffect.cs:1233`) are **comments referencing** the property, not gates. **Read each hit** — do not count with `wc -l`. The live gate count is **13**.

Note the two inverted-polarity sites, `CardController.cs:700` and `SelectDigiXrosClass.cs:569`, which read `(!card.Owner.isYou || DrivesLocalSeat)` — they route *toward* the AI path rather than gating away from it. The interception must go before the `AutoSelect()` call in both shapes, not mechanically before the `if`.

- [ ] **Step 2: Record the prompt vocabulary**

For each of the 13 sites, find the `LogSelectionRow` call in the same method and note its `prompt` argument.

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
grep -rn "LogSelectionRow" "$BASE_DCGO/Assets/Scripts/Script/" --include=*.cs -A 2 | grep -i "prompt\|\"" | head -40
```

Write the mapping into `docs/DCGO_RECORDING_SCHEMA.md`'s selection-row prompt table if any site's prompt is not already listed there. **The `expect_prompt` vocabulary and the recorder's `prompt` vocabulary must be the same strings** — otherwise a scenario authored from a recording can never match.

- [ ] **Step 3: Apply the interception at one site first**

Start with `SelectPermanentEffect.cs:298`, the simplest non-inverted shape. Immediately *before* the existing gate:

```csharp
            // [Harness mod - phase 2] A scripted line answers before either the
            // UI path or AutoSelect gets a say. TryAnswer aborts the job itself
            // on a prompt mismatch, so a false return here means "no scripted
            // job" -- never "the script declined".
            if (InputDriver.IsActive)
            {
                PromptContext __ctx = new PromptContext
                {
                    Kind = "select_permanent",
                    Count = _selectNum,
                    Candidates = CandidateCardIds(),
                };
                if (InputDriver.TryAnswer(_selectPlayer.PlayerID, __ctx, out int __scripted))
                {
                    ApplyScriptedSelection(__scripted);
                    yield break;
                }
                yield break;
            }
            if (_selectPlayer.isYou && !Digimon.Harness.HarnessAuto.DrivesLocalSeat)
```

> `_selectNum`, `CandidateCardIds()`, `ApplyScriptedSelection(...)` and the exact
> control-flow keyword (`yield break` vs `return`) are **placeholders for the real
> member names at this site** — read the method and substitute. The shape that
> must hold at every site: build the context, call `TryAnswer`, apply on true,
> and on false **stop** rather than falling through to `AutoSelect()`.

- [ ] **Step 4: Verify the single site end-to-end before doing the other 12**

Build a one-step scripted job that reaches a `select_permanent` prompt and run it. Confirm the Player log shows `[Harness] scripted line installed: 1 steps` and no `prompt mismatch`.

**Do not proceed to the remaining sites until this one works.** The `PromptContext` shape is the risk this task exists to retire; discovering at site 13 that it cannot carry what a site knows means redoing all 13.

- [ ] **Step 5: Apply the same shape to the remaining 12 sites**

Commit per file, so a bisect can attribute a regression to one site:

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git -C "$BASE_DCGO" add Assets/Scripts/Script/SelectHandEffect.cs
git -C "$BASE_DCGO" commit -m "harness: InputDriver interception at SelectHandEffect"
```

- [ ] **Step 6: Intercept `TurnStateMachine.QueueMainPhaseAction`**

Main-phase decisions do not pass a `DrivesLocalSeat` gate. Intercept where the AI chooses its action, with `Kind = "main_phase"`, `Count = -1`, `Candidates = new string[0]`.

- [ ] **Step 7: Record the final gate map in the schema doc**

Add a "Scripted input driver" section to `docs/DCGO_RECORDING_SCHEMA.md` listing all 14 interception points (13 gates + `QueueMainPhaseAction`) with file and line, and the rule-27 warning that the count must be re-derived by reading, not grepping.

```bash
cd "$(git rev-parse --show-toplevel)"
git add docs/DCGO_RECORDING_SCHEMA.md
git commit -m "docs: record the scripted input driver interception map"
```

---

### Task 7: `StateDumper` — the per-step sidecar

**Files:**
- Create: `Assets/Scripts/Script/Harness/StateDumper.cs`
- Modify: `Assets/Scripts/Script/Recording/GameRecorder.cs` (expose the step index)
- Modify: `Assets/Scripts/Script/Harness/JobWatcher.cs` (open/close the sidecar)
- Modify: `docs/DCGO_RECORDING_SCHEMA.md` (document the sidecar)

**Interfaces:**
- Consumes: `GameRecorder.Instance`, `GameRecorder.CurrentRecordingPath`.
- Produces: `StateDumper.Dump(int step)` writing one JSONL row; sidecar path is the recording path with `.jsonl` replaced by `.state.jsonl`.
- Produces: `GameRecorder.CurrentStepIndex` (int, read-only) — the existing private `_stepIndex` exposed.

- [ ] **Step 1: Expose the recorder's step index**

In `Assets/Scripts/Script/Recording/GameRecorder.cs`, beside `CurrentRecordingPath` (~line 85):

```csharp
        /// <summary>
        /// The step counter the next decision row will use. Exposed so the
        /// harness's StateDumper can key its sidecar to the SAME index the
        /// recording uses.
        /// </summary>
        /// <remarks>
        /// Read-only on purpose. A second counter maintained in parallel would
        /// drift the moment either side added a row type that does or does not
        /// increment -- and a drifted sidecar makes the differ compare step N of
        /// one game against step N+1 of the other, reporting divergences that
        /// are pure bookkeeping.
        /// </remarks>
        public int CurrentStepIndex => _stepIndex;
```

- [ ] **Step 2: Write the dumper**

Create `Assets/Scripts/Script/Harness/StateDumper.cs`:

```csharp
using System.Collections.Generic;
using System.IO;
using System.Text;
using UnityEngine;

namespace Digimon.Harness
{
    /// <summary>
    /// Writes one normalized state row per decision boundary, keyed by the
    /// recorder's step index so the sidecar aligns with the recording.
    /// </summary>
    /// <remarks>
    /// State dumping is not optional. Without it a probe reports what was
    /// LEGAL, never what HAPPENED -- and what happened is the question card
    /// authoring asks.
    ///
    /// The projection is deliberately narrow and matches the Rust differ's
    /// expectations exactly: normalize representation, never semantics.
    /// Effective DP is representation (the two engines track modifiers
    /// differently, so a modifier-list diff is pure noise). Whether a Digimon
    /// is suspended is semantics, and must be dumped.
    ///
    /// Security is a COUNT, never contents: the contents are hidden
    /// information, and dumping them would let a differ "confirm" a line whose
    /// legality depended on knowing them.
    /// </remarks>
    public static class StateDumper
    {
        private static StreamWriter _writer;

        public static void Open(string recordingPath)
        {
            Close();
            if (string.IsNullOrEmpty(recordingPath)) return;
            try
            {
                string sidecar = recordingPath.EndsWith(".jsonl")
                    ? recordingPath.Substring(0, recordingPath.Length - ".jsonl".Length) + ".state.jsonl"
                    : recordingPath + ".state.jsonl";
                _writer = new StreamWriter(sidecar, append: false, new UTF8Encoding(false));
                // Unbuffered, matching GameRecorder: a crashed run must leave
                // everything it observed on disk, since the crash is often the
                // finding.
                _writer.AutoFlush = true;
                Debug.Log("[Harness] state sidecar: " + sidecar);
            }
            catch (System.Exception e)
            {
                Debug.LogError("[Harness] could not open state sidecar: " + e.Message);
                _writer = null;
            }
        }

        public static void Close()
        {
            if (_writer == null) return;
            try { _writer.Flush(); _writer.Dispose(); }
            catch (System.Exception) { }
            _writer = null;
        }

        /// <summary>Write the current game state under the recorder's step index.</summary>
        public static void Dump(GameContext ctx)
        {
            if (_writer == null || ctx == null) return;
            if (GameRecorder.Instance == null) return;

            StringBuilder sb = new StringBuilder(512);
            sb.Append('{');
            sb.Append("\"step\":").Append(GameRecorder.Instance.CurrentStepIndex).Append(',');
            sb.Append("\"turn\":").Append(TurnOf(ctx)).Append(',');
            sb.Append("\"phase\":\"").Append(Escape(PhaseOf(ctx))).Append("\",");
            sb.Append("\"memory\":").Append(MemoryOf(ctx)).Append(',');
            AppendPlayer(sb, "p0", PlayerOf(ctx, 0));
            sb.Append(',');
            AppendPlayer(sb, "p1", PlayerOf(ctx, 1));
            sb.Append('}');

            _writer.WriteLine(sb.ToString());
        }

        private static void AppendPlayer(StringBuilder sb, string key, Player p)
        {
            sb.Append('"').Append(key).Append("\":{");
            sb.Append("\"security\":").Append(p == null ? 0 : p.SecurityCards.Count).Append(',');
            AppendCardIds(sb, "hand", p == null ? null : p.HandCards);
            sb.Append(',');
            AppendCardIds(sb, "trash", p == null ? null : p.TrashCards);
            sb.Append(',');
            sb.Append("\"field\":[");
            if (p != null)
            {
                List<Permanent> field = p.GetFieldPermanents();
                for (int i = 0; i < field.Count; i++)
                {
                    if (i > 0) sb.Append(',');
                    AppendPermanent(sb, field[i]);
                }
            }
            sb.Append("]}");
        }

        private static void AppendPermanent(StringBuilder sb, Permanent perm)
        {
            sb.Append('{');
            sb.Append("\"card_id\":\"").Append(Escape(TopCardIdOf(perm))).Append("\",");
            sb.Append("\"dp\":").Append(EffectiveDpOf(perm)).Append(',');
            sb.Append("\"suspended\":").Append(IsSuspended(perm) ? "true" : "false").Append(',');
            AppendCardIds(sb, "sources", SourceCardsOf(perm));
            sb.Append(',');
            AppendStrings(sb, "keywords", ActiveKeywordsOf(perm));
            sb.Append('}');
        }

        private static void AppendCardIds(StringBuilder sb, string key, List<CardSource> cards)
        {
            sb.Append('"').Append(key).Append("\":[");
            if (cards != null)
            {
                for (int i = 0; i < cards.Count; i++)
                {
                    if (i > 0) sb.Append(',');
                    sb.Append('"').Append(Escape(CardIdOf(cards[i]))).Append('"');
                }
            }
            sb.Append(']');
        }

        private static void AppendStrings(StringBuilder sb, string key, List<string> values)
        {
            sb.Append('"').Append(key).Append("\":[");
            if (values != null)
            {
                for (int i = 0; i < values.Count; i++)
                {
                    if (i > 0) sb.Append(',');
                    sb.Append('"').Append(Escape(values[i])).Append('"');
                }
            }
            sb.Append(']');
        }

        private static string Escape(string s)
        {
            if (string.IsNullOrEmpty(s)) return "";
            return s.Replace("\\", "\\\\").Replace("\"", "\\\"");
        }
    }
}
```

> **The `TurnOf` / `PhaseOf` / `MemoryOf` / `PlayerOf` / `TopCardIdOf` / `EffectiveDpOf` / `IsSuspended` / `SourceCardsOf` / `ActiveKeywordsOf` / `CardIdOf` accessors must be filled in against the real DCGO types.** They are named separately, rather than inlined, precisely so each can be resolved and reviewed one at a time. Find them with:
> ```bash
> BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
> grep -n "public .*DP\|IsSuspend\|GetFieldPermanents\|HandCards\|TrashCards\|SecurityCards" \
>   "$BASE_DCGO/Assets/Scripts/Script/Player.cs" "$BASE_DCGO/Assets/Scripts/Script/Permanent.cs"
> ```
> `EffectiveDpOf` must return DP **after** modifiers — the printed value would make every buffed Digimon read as a divergence.

- [ ] **Step 3: Open and close the sidecar with the job**

In `JobWatcher.ApplyJob`, after `InputDriver.Install(job)`, the recording path is not yet known (the recorder opens at `LogGameStart`). So open the sidecar from `GameRecorder.LogGameStart` instead, right after `_writer` is assigned (~line 197):

```csharp
            // [Harness mod - phase 2] Keep the state sidecar's lifetime tied to
            // the recording's, so the two always describe the same game.
            if (Digimon.Harness.JobWatcher.Instance != null &&
                Digimon.Harness.JobWatcher.Instance.CurrentJob != null)
            {
                Digimon.Harness.StateDumper.Open(_currentRecordingPath);
            }
```

And in `LogGameEnd` (~line 292), before the writer is closed:

```csharp
            Digimon.Harness.StateDumper.Close();
```

- [ ] **Step 4: Call `Dump` at each decision boundary**

Add `StateDumper.Dump(<the GameContext at hand>)` immediately *before* each `GameRecorder.Instance.LogSelectionRow(...)` and `LogAction(...)` call site — before, so the dumped state is the state the decision was made *in*, not after it resolved.

- [ ] **Step 5: Verify a sidecar appears and aligns**

Run one `policy: "ai"` job (the dumper works for AI jobs too, which is what makes it verifiable before `InputDriver` is trusted):

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"
dcgo-harness --root "$ROOT" submit --count 1 --decks qa/dcgo-harness/vb_pool.json --seed 1
dcgo-harness --root "$ROOT" enable
```

Press Play. When it drains:

```bash
CORPUS="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings"
REC=$(ls -t "$CORPUS"/*.jsonl | grep -v state | head -1)
echo "recording steps: $(grep -c '"type":"action"' "$REC")"
echo "sidecar rows:    $(wc -l < "${REC%.jsonl}.state.jsonl")"
# Every sidecar step must exist in the recording:
python - <<'PY'
import json,sys,glob,os
rec=sorted((p for p in glob.glob(os.environ.get("CORPUS","")+"/*.jsonl") if ".state." not in p), key=os.path.getmtime)[-1]
steps={json.loads(l).get("step") for l in open(rec) if l.strip()}
side={json.loads(l)["step"] for l in open(rec[:-6]+".state.jsonl") if l.strip()}
missing=side-steps
print("sidecar steps not in recording:", sorted(missing)[:10])
assert not missing, "sidecar/recording step index drift"
print("OK: aligned")
PY
```

Expected: `OK: aligned`. **A drift here means the two counters disagree and every later diff is bookkeeping noise — fix it before proceeding.**

- [ ] **Step 6: Document and commit**

Add a "State sidecar: `<recording>.state.jsonl`" section to `docs/DCGO_RECORDING_SCHEMA.md` with the row shape and the normalize-representation-not-semantics rule.

```bash
BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
git -C "$BASE_DCGO" add Assets/Scripts/Script/Harness/StateDumper.cs \
    Assets/Scripts/Script/Harness/StateDumper.cs.meta \
    Assets/Scripts/Script/Recording/GameRecorder.cs Assets/Scripts/Script/Harness/JobWatcher.cs
git -C "$BASE_DCGO" commit -m "harness: per-step state sidecar keyed to the recorder step index"
cd "$(git rev-parse --show-toplevel)"
git add docs/DCGO_RECORDING_SCHEMA.md && git commit -m "docs: state sidecar format"
```

---

### Task 8: Acceptance — a golden scripted job, twice

**Files:**
- Create: `qa/dcgo-harness/golden-scripted-job.json`
- Modify: `docs/DCGO_HARNESS.md`
- Modify: the repo's `DCGO` gitlink

**Interfaces:**
- Consumes: everything above.
- Produces: a committed golden job + its recording/sidecar pair, used as the differ's CI fixture in the plan-2 work.

- [ ] **Step 1: Hand-write the golden job**

Create `qa/dcgo-harness/golden-scripted-job.json` with `policy: "scripted"`, a two-card `deck_order` prefix per seat, and a line long enough to reach one selection prompt. Action IDs come from `code/digimon-engine/src/action/space.rs`; the plan-2 lowering tool does not exist yet, so this one is hand-derived and that is expected.

- [ ] **Step 2: Run it and confirm the line completes**

```bash
ROOT="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness"
cp qa/dcgo-harness/golden-scripted-job.json "$ROOT/jobs/"
dcgo-harness --root "$ROOT" enable
```

Press Play. Expected in the Player log: `[Harness] scripted line installed: N steps`, **no** `prompt mismatch`, and the job filed in `done/` with `outcome: "completed"`.

- [ ] **Step 3: Confirm the egg-only stack case**

Author a second job whose `deck_order` names **only** an egg card. Expected: the main deck logs `[Harness] main deck stack failed` and **aborts**, per Task 3's asymmetry — so instead assert the inverse: a job naming only *main-deck* cards must log `[Harness] egg deck not stacked` and still complete.

> If this reveals the asymmetry is wrong in practice — e.g. most scenarios need
> both decks stacked — split `deck_order` into `deck_order.p0.main` / `.egg`
> rather than sharing one array. **Report this before changing it**; it alters the
> job schema Task 2 fixed and the plan-2 lowering depends on.

- [ ] **Step 4: The determinism check — Editor vs player**

Run the same golden job at the same seed in the Editor and in the built player.

```bash
diff <(cat editor-run.jsonl) <(cat player-run.jsonl) && echo "RECORDINGS IDENTICAL"
diff <(cat editor-run.state.jsonl) <(cat player-run.state.jsonl) && echo "SIDECARS IDENTICAL"
```

Expected: both print IDENTICAL.

**This is the acceptance gate, not "it builds".** A player that launches but diverges from the Editor is worse than no player, because it makes the oracle disagree with itself, and everything downstream assumes the two are the same program.

- [ ] **Step 5: Commit the golden fixture and bump the gitlink**

```bash
CORPUS="C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_recordings"
REC=$(ls -t "$CORPUS"/*.jsonl | grep -v state | head -1)
mkdir -p qa/dcgo-harness/golden
cp "$REC" qa/dcgo-harness/golden/scripted.jsonl
cp "${REC%.jsonl}.state.jsonl" qa/dcgo-harness/golden/scripted.state.jsonl
git add qa/dcgo-harness/golden-scripted-job.json qa/dcgo-harness/golden/ docs/DCGO_HARNESS.md
git commit -m "qa: golden scripted job + recording/sidecar fixture"
git add DCGO
git commit -m "Bump DCGO gitlink: scripted input driver, deck stacker, state sidecar"
```

> The corpus is derived data and is NOT committed. This golden pair is the
> documented exception: it is a minimized regression fixture, which is exactly
> what the phase-1 rule permits.

- [ ] **Step 6: Update `docs/DCGO_HARNESS.md`**

Add a "Scripted jobs" section covering `policy: "scripted"`, `deck_order` (prefix, initial-shuffle-only, and *why*), `inputs` with the prompt assertion, the state sidecar, and the abort-on-mismatch behavior. Move `job.first_player` out of "Known gaps" only if this work actually honored it — **it does not**, so leave that gap listed.

---

## Self-Review

**Spec coverage.** This plan implements the spec's Unity half: `DeckStacker` (Tasks 1, 3), `policy: "scripted"` + `deck_order` + `inputs` (Task 2), `InputDriver` with prompt assertion (Tasks 4–6), `StateDumper` (Task 7), and the Editor-vs-player determinism gate (Task 8). Corrections 1 and 2 are both implemented as written. Not covered here, by design: the scenario YAML, lowering, `ScenarioAdapter`, differ, verdict store, CI, and MCP — those are plans 2 and 3.

**Known gap this plan does not close.** `job.first_player` is still not honored by DCGO. It was already listed as a phase-1 known gap and this work does not touch seat assignment. A scripted line whose `actor` values assume a particular seat will therefore fail its prompt assertion roughly half the time — **the plan-2 lowering must not assume `first_player` works**, and the scenario runner should either derive the seat from the recording's `my_player_id` or submit with an explicit retry on seat mismatch. Flagged rather than fixed because fixing it means changing DCGO's own seat roll, which is out of this plan's scope.

**Placeholder scan.** Three sites carry explicitly-marked substitution points rather than final code — Task 6 Step 3's member names, Task 7 Step 2's DCGO type accessors, and Task 8 Step 1's action IDs. Each is called out inline with the exact `grep` that resolves it. These are unavoidable: the DCGO types involved are from an AssetRipper-derived checkout whose member names cannot be confirmed without opening the files, and inventing plausible names would be worse than naming the lookup.

**Type consistency.** `PromptContext` fields (`Kind`, `Count`, `Candidates`) are used identically in Tasks 4, 5, and 6. `HarnessJobStep` fields (`actor`, `action_id`, `expect_prompt`, `expect_count`, `expect_candidates`) match between Task 2's parser, Task 4's tests, and Task 4's consumer. `DeckStacker.Apply` / `ApplyIds` signatures match between Task 1 and Task 3. `JobWatcher.AbortCurrentJob(string)` is defined in Task 3 and consumed in Task 5.
