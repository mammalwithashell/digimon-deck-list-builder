# NOTES — ToyAgumon (BT25-064)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-064`):
**3 clauses** — `effect#0`, `effect#1`, `inherited#0`.
DCGO script: `BT25/Black/BT25_064.cs` exists — the card is **not** `unavailable`.
All three clauses have a scenario; none is `unreachable`.
Book: `qa/dcgo-exams/BT25/tm_bt25_064_pool.json` (decks `tm-toy-black`, `tm-quiet-red`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT25-064#effect#0` | `[Digivolve] Lv.2 w/[TS] trait: Cost 0` | `BT25-064-effect0.yaml` | lowers, asserts pass |
| `BT25-064#effect#1` | `[On Play] Reveal the top 3 cards of your deck. Add 1 Option card and 1 [TS] trait card among them to the hand. Return the rest to the bottom of the deck.` | `BT25-064-effect1.yaml` | lowers, asserts pass |
| `BT25-064#inherited#0` | `<Reboot>` (inherited) | `BT25-064-inherited0.yaml` | lowers, asserts pass |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

All three files were on disk, uncommitted; all lowered and asserted as
found, and the prompt sequences hold against `BT25_064.cs`:

- `effect#0`: `AddSelfDigivolutionRequirementStaticEffect(HasTSTraits, 0,
  false, card, null, level: 2)` with the default `cardColor: None`
  (`AddDigivolutionRequirement.cs` applies no colour check then). The base
  is a PURPLE Lv.2 [TS] egg (Tsunomon BT24-007), so the printed Black Lv.2
  circle fails and the clause is the only route: one cost, no
  `SelectCountEffect`. Breeding-area effects are inert, so no `[On Play]`.
- `effect#1`: `SetUpActivateClass(..., -1, false, ...)` — mandatory;
  `SimplifiedRevealDeckTopCardsAndSelect(revealCount 3, two conditions,
  mutualConditions: true, canNoSelect: false)` → `RevealLibrary.cs`
  loops the conditions: ONE `SelectCardEffect` "Select 1 Option card." then
  ONE "Select 1 [TS] trait card." (the `mutualConditions` relaxation only
  fires when the first pick was the sole first-condition card AND also
  satisfies the second — taking Der Blitz EX7-070 first rules it out, since
  Iron Slash BT25-100 also satisfies condition 1). No leading `generic_bool`
  (`canNoAction` is false on the simplified helper). The lone remainder goes
  to the deck bottom with no prompt (`ReturnRevealedCardsToLibraryBottom`,
  `Count == 1` → `AddLibraryBottomCards`); our engine parks a one-item
  `OrderedPermutation`, answered `sim_only` — the known
  `OrderedPermutation`-at-N=1 row of `docs/DCGO_EXAM.md`.
- `inherited#0`: `RebootSelfStaticEffect(isInheritedEffect: true)`;
  <Reboot> is mandatory (keyword-semantics 16-10), no prompt on either side.
  Witness: the host (Thundermon BT8-061, vanilla) reads unsuspended at P1's
  breeding decision after attacking on P0's turn.

## Second resume (2026-09-18): re-lowered; slot hygiene; one PREDICTED oracle diff

All three files re-lowered unchanged against the current harness. None
digivolves into BT25-085, resolves a `<De-Digivolve>`, or links. Slot hygiene
(`NOTES-BT25-083.md`): P0 never holds more than one permanent and P1 holds
none, so `field.0` is the same permanent on both engines.

**Predicted for `effect#1` — the `RevealBucket` add-timing quirk.**
`docs/DCGO_EXAM.md` ("Rows where one side looks rules-wrong") records that on
a ≥2-condition `SimplifiedRevealDeckTopCardsAndSelect` DCGO moves each
bucket's pick to the hand when THAT prompt closes, while ours collects every
pick and adds once (§15-15-10-1). Expect a one-field `p0.hand` divergence on
the 2nd pick row (DCGO already shows EX7-070 in hand) with no downstream
rows; triage it as the documented DCGO quirk (the sister ToyAgumon
EX7-008#effect#1 measured exactly this), not as an engine bug.
