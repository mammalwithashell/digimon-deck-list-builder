# NOTES — Shotmon (BT21-054)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT21-054`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `inherited#0`.
DCGO script: `BT21/Black/BT21_054.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 4 clauses, 4 scenarios, 0 unreachable.** All four lower sim-only with
`tm_bt21_054_pool.json` (decks `tm-shotmon` — Shotmon x4 over the quiet ST-1
filler with Swipemon BT21-005 eggs — and `quiet-opponent`). Three files were
left uncommitted by the crashed stage (2026-09-18 00:06); they were audited
against `BT21_054.cs`, re-lowered unchanged, and `inherited#0` was added.

| Clause | Text | Scenario | Line |
|---|---|---|---|
| `BT21-054#effect#0` | `[Digivolve] Lv.2 w/[Three Musketeers] in text or w/[Appmon] trait: Cost 0` | `BT21-054-effect0.yaml` | breeding-digivolve onto the GREEN Swipemon BT21-005 (form Appmon): the printed Black Lv.2 circle fails on colour, the clause is the only route |
| `BT21-054#effect#1` | `[On Play] By trashing 1 [Appmon]/[Three Musketeers] trait card from any of your Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.` | `BT21-054-effect1.yaml` | Shotmon #2 is played; the cost is Swipemon under Shotmon #1 — a DIFFERENT Digimon ("any of your Digimon's") |
| `BT21-054#effect#2` | `+2000 DP` (the link box) | `BT21-054-effect2.yaml` | hand `<Link>` into a played Shotmon: host 1000 → 3000 |
| `BT21-054#inherited#0` | `<Link> [Appmon] trait: Cost 1 … [When Linking] Delete 1 of your opponent's Digimon with a play cost of 3 or less.` | `BT21-054-inherited0.yaml` | hand `<Link>` (2 → 1), then the mandatory delete with the play-cost gate visible: candidates `[ST1-02]`, Garudamon ST1-08 (cost 6) on the board but not offered |

## Adversarial pre-Unity review (from `BT21_054.cs`)

- **`[On Play]`** is `SetUpActivateClass(…, -1, true, …)` → `OptionalSkill`
  (shared with our `Replacement` gate: the body opens with a mandatory pick).
  `SelectTrashDigivolutionCards(isFromOnly1Permanent: false, canNoTrash:
  false)` asks `SelectPermanentEffect` (which Digimon — consumed even with one
  candidate) THEN `SelectCardEffect` (which card); our engine asks ONE
  union-material pick → the permanent row is `dcgo_only`, ours is answered
  `sim_only` through the resolver's single-accept path, and the card row rides
  the wire `dcgo_only`. This is the shape `BT21-074-effect2.yaml` CONFIRMED
  under the oracle (2026-09-18, 5/5).
- **`IDegeneration` count row.** `new IDegeneration(target, 1, activateClass)`
  carries no `DegenerationCountRuling`, so DCGO opens a `SelectCountEffect`
  over the single candidate `{1}` and the harness intercept consumes a scripted
  row for it (`NOTES-BT21-074.md`). `effect1` carries the `dcgo_only`
  `value: 1` row after the De-Digivolve target.
- **`<Link>`.** `CardEffectFactory.LinkEffect` is `isOptional: true` → DCGO
  confirms the declared link with an `OptionalSkill` BEFORE the host pick
  (measured on BT21-071 / BT21-074, first oracle run aborted without it). Both
  link files carry that `dcgo_only` accept row, then the shared host
  `SelectPermanentEffect` (single candidate, still asked).
- **`[When Linking]`** is `isOptional: false` → no `OptionalSkill`; ONE
  `SelectPermanentEffect` (`Mode.Destroy`, `canNoSelect: false`) and only when
  a play-cost-≤3 opponent Digimon exists. In `effect2` p1's board is EMPTY, so
  `CanActivateCondition` is false and nothing is asked on either side; in
  `inherited0` it is asked with the single candidate Biyomon.
- Linking from hand is not a play (`Permanent.AddLinkCard`, root `Hand`): the
  linking Shotmon's `[On Play]` does not fire. The host Shotmon's own
  `[On Play]` is silent in both link files (no own Digimon has a digivolution
  card → `CanActivateCondition` false; our `condition:` false).
- The from-FIELD origin of `<Link>` is not re-measured on this card; it rides
  the same `LinkEffect` / `FIELD_EFFECT_SLOT_FOR_LINK` path
  `BT21-071-inherited1.yaml` confirmed.

None of the lines digivolves into BT25-085.
