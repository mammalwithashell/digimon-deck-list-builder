# NOTES — Imperialdramon: Fighter Mode (BT20-020)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT20-020`):
**5 clauses** — `effect#0` (`[Digivolve] [Imperialdramon: Dragon Mode]: Cost
2`), `effect#1` (`<Raid>`), `effect#2` (`<Piercing>`), `effect#3` ([When
Digivolving] can't-play-by-effects gate + conditional security trash),
`effect#4` ([All Turns] [Once Per Turn] delete on opposing security removal).
DCGO script: `BT20/Red/BT20_020.cs` exists — the card is **not** `unavailable`.

**Status: 5 clauses, 5 scenarios, 0 unreachable.** Book:
`qa/dcgo-exams/BT20/tm_bt20_020_pool.json` (decks `tm-imperialdramon`,
`quiet-opponent`).

| Clause | Scenario | Sim-only | Expected at the oracle |
|---|---|---|---|
| `effect#0` | `BT20-020-effect0.yaml` | lowers, asserts pass | clean |
| `effect#1` | `BT20-020-effect1.yaml` | lowers, asserts pass | clean |
| `effect#2` | `BT20-020-effect2.yaml` (RE-AUTHORED) | lowers, asserts pass | clean |
| `effect#3` | `BT20-020-effect3.yaml` (EXTENDED) | lowers, asserts pass | **predicted divergence — OUR ENGINE** (below) |
| `effect#4` | `BT20-020-effect4.yaml` | lowers, asserts pass | clean |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

All five were on disk, uncommitted; all five lowered as found and the prompt
sequences hold against `BT20_020.cs` / `BT20_076.cs`. None digivolves into
BT25-085 and none crosses a `<De-Digivolve>`. Two were reworked for
measurement quality:

### `effect#2` — isolated from `<Raid>`

The draft reached the Digimon battle THROUGH the `<Raid>` switch, which made
it a byte-for-byte twin of `effect1`: one `<Raid>` finding would have taken
both clauses down, and the two verdicts would never have been independent. The
new line has P1's Biyomon attack first (it survives a 2000 security Agumon and
stays SUSPENDED), then Fighter Mode attacks it directly. P1 has no unsuspended
Digimon, so `CanActivateRaid` is false and `<Piercing>` is the only keyword in
play. `field.0` addresses P1's ONLY Digimon (slot-safe).

### `effect#3` — the first sentence now has a witness

The draft measured only the security trash and said so ("not separately
witnessed"). P1's second security card is now Tai Kamiya ST1-12 ("[Security]
Play this card without paying the cost" — a Tamer played by P1's own effect),
and Fighter Mode attacks on the digivolve turn: the gate must stop the play.
The book's `quiet-opponent` deck swaps ST1-13 ×4 for ST1-12 ×4 for this (the
book is this card's own; no other scenario uses it).

## ENGINE FINDING — a security-flipped "play this card" ignores the can't-play-by-effect gate (`effect#3`)

Logged as `G-ENGINE-PLAY-PENDING-SECURITY-IGNORES-CANNOT-PLAY-BY-EFFECT` in
`docs/RUST_ENGINE_GAPS.md`. Measured sim-side while authoring: with
`CannotPlayTamerByEffect` installed on P1 until the end of P1's turn, the
checked Tai Kamiya is PLAYED on our side (`p1.field` = [ST1-12], `p1.trash` =
[ST1-02]).

- The modifier is enforced only on the from-hand and from-trash `ByEffect`
  paths (`code/digimon-engine/src/game_actions/mod.rs` ~440,
  `game_actions/play.rs` ~140).
- A `[Security] Play this card` resolves through
  `CompiledStep::PlayFromSecurity` → `ctx.play_pending_security()`
  (`dsl_cards/step/play_digivolve.rs` ~805), which marks the parked
  `pending_security` card as played without consulting
  `CannotPlayTamerByEffect` / `CannotPlayDigimonByEffect`. The non-security
  `play_from_security*` helpers (`effect_context/action/play.rs` ~584-690)
  should be checked for the same hole.
- DCGO: `CanNotPutFieldClass` (cardCondition Digimon / Tamer / DigiEgg,
  cardEffectCondition "effect source owned by the opponent") makes
  `CanPlayAsNewPermanent` false, so `PlaySelfTamerSecurityEffect`'s
  `CanActivateCondition` fails (`CardEffectFactory.cs` 165-169) and the card
  is trashed after the check.
- Impact is every "opponent can't play … by effects" gate against every
  security-play card (Tamers' `[Security] Play this card`, security Digimon
  plays), not just this pair.

The `effect3` asserts pin the shared facts (security 3, suspended attacker)
and deliberately NOT where Tai went; the oracle run is expected to report
`p1.field` / `p1.trash` at step 11. Not fixed here (exam stage: triage and
report).

## Un-probed: does a target-less trigger burn the [Once Per Turn]?

`effect#4` triggers on EVERY opposing security removal, including ones with no
legal target (the `effect0/1/2/3` lines all cross one on an empty board). DCGO
does not ACTIVATE then (`CanActivateCondition` needs a target), so its
per-turn count is untouched; our clause resolves an empty `if:` body, and
whether that consumes `once_per_turn` is not visible on any line here — it
would need a Digimon to appear on the opposing board between two removals in
one turn, which the gate in `effect#3` itself forbids for effect plays. Flagged
for a `cards_behavioral` probe rather than an exam line.

## Prompt shapes (re-derived from the C#)

- `effect#0`: `AddSelfDigivolutionRequirementStaticEffect(EqualsCardName
  "Imperialdramon: Dragon Mode", 2, ignoreDigivolutionRequirement: false)`. The
  base is a hard-played Lv.6 Dragon Mode BT20-076, which neither printed Lv.5
  circle admits: one route, no `SelectCountEffect`.
- Dragon Mode's [On Play] delete is `SetUpActivateClass(…, -1, FALSE, …)` with
  the pick inside `if (HasMatchConditionPermanent(…))` — silent on an empty
  opposing board; its "if DNA digivolving" rider is off for a hard play. Its
  `[Hand] [Counter] <Blast DNA Digivolve>` never has materials here.
- `effect#3`: `SetUpActivateClass(…, -1, FALSE, …)`, `IDestroySecurity` with no
  pick — no prompt.
- `effect#4`: `OnLoseSecurity`, `SetUpActivateClass(…, 1, FALSE, …)`, ONE
  `SelectPermanentEffect` (`Mode.Destroy`, `canNoSelect: false`) over opposing
  Digimon with DP ≤ this Digimon's — asked (and the scripted row consumed) with
  a single candidate.
- `<Raid>`: the documented fold. `<Piercing>`: mandatory, no prompt.
- Slot hygiene: one Digimon per seat wherever a slot is addressed.
