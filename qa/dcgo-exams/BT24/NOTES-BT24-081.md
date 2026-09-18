# NOTES — Titamon + SkullBaluchimon (BT24-081)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT24-081`):
**6 clauses** — `effect#0` (`<Rush>`), `effect#1` (`<Piercing>`), `effect#2`
(`<Execute>`), `effect#3` ([On Play] [When Digivolving] [When Attacking] "By
trashing 1 card in your hand, delete all … lowest level"), `effect#4`
([On Deletion] "You may play 1 [Titamon] or 1 level 5 or lower … [Titan] …
from your trash"), `effect#5` (`Assembly -6: [Titamon]×[SkullBaluchimon]`).
DCGO script: `BT24/Purple/BT24_081.cs` exists — the card is **not**
`unavailable`. No stored verdict yet (all six `unmeasured` going in).

**Status: 6 clauses, 6 scenarios, 0 unreachable.** All six lower sim-only with
`--decks qa/dcgo-exams/BT24/tm_bt24_081_pool.json` (decks `tm-titan`,
`quiet-opponent`).

| Clause | Scenario | Line (T9 is the clause turn) |
|---|---|---|
| `effect#5` | `BT24-081-effect5.yaml` | THE SPINE. Titamon BT1-080 and SkullBaluchimon BT10-080 die in security battles (T5/T7), P1 donates 12 memory (T8), the Assembly play lands at 14 − 6 = 8 (9 → 1) with both materials stacked under. |
| `effect#0` | `BT24-081-effect0.yaml` | Spine, then the freshly played Lv.7 attacks the player the same turn. |
| `effect#1` | `BT24-081-effect1.yaml` | Spine, [On Play] deletes Biyomon, then the attack goes at the SUSPENDED Phoenixmon; the won battle is followed by a security check (3 → 2). |
| `effect#2` | `BT24-081-effect2.yaml` | Spine, pass at 1, spend the end-of-turn window on the player; `<Execute>`'s self-delete then fires [On Deletion]. |
| `effect#3` | `BT24-081-effect3.yaml` | T8 donation changed to two Lv.3s (Biyomon + Monodramon): the play lands from memory 1 (→ −7) and BOTH Lv.3s are deleted while the Lv.6 stays. |
| `effect#4` | `BT24-081-effect4.yaml` | Spine with Starlight Explosion: the `<Rush>` attack runs into a 19000 security Phoenixmon, the carrier dies, Titamon BT1-080 is revived from the trash. |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

`effect4` / `effect5` were on disk, uncommitted (the dispatch counted zero —
they are untracked). Both lowered as found and both prompt sequences hold
against `BT24_081.cs` + `SelectAssemblyClass.cs`; only stale comments were
fixed in `effect4` (its T8 donation is Starlight Explosion, not Biyomon).
`effect0`–`effect3` are new, built on the same spine.

## Prompt shapes (re-derived from the C#)

- **Assembly** (`SelectAssemblyClass.Select`): no "use Assembly?" yes/no; ONE
  `SelectCardEffect` per `AssemblyConditionElement` (root Trash, `canNoSelect:
  () => true`, `maxCount = ElementCount`), in recipe order — [Titamon], then
  [SkullBaluchimon]. Ours: one `CountCappedMultiSelect` per element (element 0
  declinable = "use Assembly?", element 1 exact). Two shared `cards:` rows. The
  one-step `materials:` form is exact only for ONE-element recipes
  (`EX12-076-effect0.yaml`), so it is not used here.
- **[On Play] / [When Digivolving] / [When Attacking]**: three ActivateClasses,
  each `SetUpActivateClass(SharedCanActivateCondition, …, -1, FALSE, …)` → no
  `OptionalSkill`; one `SelectHandEffect` (`maxCount 1`, `canNoSelect: true`,
  `Mode.Discard`), then `DestroyPermanentsClass` over every `IsMinLevel`
  opposing Digimon (no pick). Ours is one declinable cost pick. 1:1, accepted
  with `cards:` and declined with `decline: true`.
- **[On Deletion]**: `SetUpActivateClass(…, -1, FALSE, …)` → no
  `OptionalSkill`; ONE `SelectCardEffect` (root Trash, `canNoSelect: () =>
  true`). Ours: `optional: true` over a mandatory `select_trash`, so the
  lowering installs an outer accept gate first — answered `sim_only` (zero
  wire rows), then the shared pick. Two sim prompts for DCGO's one; outcome-
  neutral, the same family as the `Replacement` row in `docs/DCGO_EXAM.md`.
- **`<Execute>`** (`ExecuteSelfEffect`, `isOptional: TRUE`): `OptionalSkill`
  then `SelectAttackEffect` — the documented end-of-turn fold (`attack:` /
  `pass:` with `expect: OptionalSkill`). `CanActivateExecute` needs `CanAttack`,
  so a carrier that already attacked (effect0/effect1) opens no gate on either
  side.
- **`<Rush>` / `<Piercing>`**: static / mandatory, no prompt.

## Slot hygiene (the campaign's centre-out rule)

P1 fields up to three Digimon on T8 but never addresses a slot afterwards. P0
only ever has one Digimon. `effect1` is the one file that targets a P1 slot:
the [On Play] deletion of Biyomon (DCGO frame 3) is what makes `field.0` the
suspended Phoenixmon on BOTH engines (DCGO compact order [4, 5]; ours
[first, third]). A mis-address would name the UNSUSPENDED Phoenixmon, which
DCGO refuses loudly.

## Not measured

- The [When Digivolving] and [When Attacking] arms of `effect#3` are only
  crossed as DECLINED prompts (effect0/1/2/4); the accepted arm is [On Play].
  A [When Digivolving] line needs a purple/green Lv.6 under the Lv.7 (cost 4),
  i.e. a second six-turn ladder; not authored.
- `effect#4`'s "[Titan] trait Lv.5 or lower" branch: the revive target is
  Titamon BT1-080 by name. BT10-080 (no [Titan] trait) is correctly not a
  candidate on our side; DCGO's candidate set is asserted by the oracle run
  only through the pick succeeding.
