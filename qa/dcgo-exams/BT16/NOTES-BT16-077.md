# NOTES — Dinobeemon (BT16-077)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT16-077`):
**5 clauses** — `effect#0` (`[DNA Digivolve] Purple Lv.4 + red Lv.4: Cost 0 …`),
`effect#1` (`<Raid>`), `effect#2` (`<Partition (purple Lv.4 & red Lv.4)>`),
`effect#3` ([When Digivolving] "If DNA digivolving, you may play … [Free] …
from your trash … Then, 1 of your Digimon may gain <Rush> for the turn and
attack a player"), `inherited#0` (inherited `<Partition …>`).
DCGO script: `BT16/Purple/BT16_077.cs` exists — the card is **not** `unavailable`.

**Status: 5 clauses — 4 scenarios, 1 `unreachable` (`effect#0`).** Book:
`qa/dcgo-exams/BT16/tm_bt16_077_pool.json` (decks `tm-purple-line`,
`tm-quiet-red` for effect1/effect3; `tm-partition`, `tm-quiet-gaia` — added on
the resumed audit — for effect2/inherited0).

| Clause | Scenario | Sim-only | Expected at the oracle |
|---|---|---|---|
| `effect#0` | — | — | **unreachable** — no DNA verb on either wire |
| `effect#1` | `BT16-077-effect1.yaml` (REPAIRED) | lowers, asserts pass | clean |
| `effect#2` | `BT16-077-effect2.yaml` (NEW) | lowers, asserts pass | clean (our extra source pick is `sim_only`) |
| `effect#3` | `BT16-077-effect3.yaml` (REPAIRED; second sentence only) | lowers, asserts pass | **predicted divergence — card YAML** |
| `inherited#0` | `BT16-077-inherited0.yaml` (NEW) | lowers, asserts pass | clean (same) |

## `effect#0` — `unreachable`: the exam cannot declare a DNA digivolution

- `code/tools/dcgo-harness/src/exam/scenario.rs` fixes the `do:` verbs to
  `hatch, pass, move, play, digivolve, attack, main, link, select`
  (`STEP_VERBS`); there is no DNA verb and `exam::lower::matches_intent` has
  no `DnaDigivolve` arm, so our engine's DNA action bits cannot be named.
- DCGO's side refuses the range outright:
  `Assets/Scripts/Script/Harness/InputDriver.cs` ~277, "Anything else -- DNA
  digivolve, … -- returns null … and the job aborts".

DCGO does script the condition (`AddJogressConditionClass`, purple Lv.4 + red
Lv.4, cost 0), so the oracle exists; the wire cannot reach it. Tooling gap
`G-TOOLING-EXAM-NO-DNA-VERB` (`docs/RUST_ENGINE_GAPS.md`). This also makes the
FIRST sentence of `effect#3` ("If DNA digivolving, you may play 1 level 5 or
lower [Free] Digimon from your trash") unreachable — see below. Never a silent
skip, never `confirmed`.

## `effect#2` / `inherited#0` — `<Partition>` reached WITHOUT a DNA verb

Partition needs "each of the specified digivolution cards" — a purple Lv.4 AND
a red Lv.4 (`PartitionCondition(4, Purple)`, `(4, Red)`) — and an ordinary
digivolution line holds exactly one Lv.4, so the crashed draft (and this
audit's first pass) had both clauses down as DNA-only. They are not: Scopemon
BT21-071's [On Play] places "1 card with the [Appmon] or [Three Musketeers]
trait … as 1 of YOUR DIGIMON's bottom digivolution card" — any own Digimon —
and Effecmon BT22-009 is a RED Lv.4 with the [Appmon] trait. So:

1. Dinobeemon digivolves onto a hard-played vanilla Youkomon ST6-07 (purple
   Lv.4) through its ordinary purple circle;
2. Scopemon is played and tucks Effecmon under Dinobeemon (`effect2`), or
   under the vanilla Phoenixmon ST1-10 that was digivolved on top of it
   first (`inherited0`, so only the INHERITED copy can answer);
3. P1's Gaia Force ST1-16 (an opponent's effect: not "your effects", not a
   battle) deletes the stack; P1's red Tai Kamiya pays its colour.

Every slot-addressed step happens while that seat has ONE Digimon; the
two-Digimon picks (tuck host, Gaia Force target) are identity picks.

Prompt shape (`KeyWordEffects/Partition.cs` + `CardEffectFactory/…/Partition.cs`):
`WhenRemoveField`, `SetUpActivateClass(…, -1, TRUE, …)` → ONE `OptionalSkill`;
`PartitionClass.Partition` opens a `SelectCardEffect` per condition ONLY when
that condition has more than one matching digivolution card — here one each,
so none; both cards are played free, Effecmon's [On Play] delete finds no
target (P1 fields a Tamer only) and asks nothing.

### ENGINE FINDING — our Partition does not enforce its slots

`G-ENGINE-PARTITION-SLOT-ENFORCEMENT-DEFERRED` (`docs/RUST_ENGINE_GAPS.md`).
`lower_partition.rs` documents `_sources` as deferred; measured here it is a
rules-visible gap, not a cosmetic one. After the `Replacement` accept our
engine parks a generic `CountCappedMultiSelect { min: 1, max: 2 }` "select 2
cards to play" over ALL the digivolution cards. Two negative probes (scratch
copies, not committed):

- `inherited0` with the pick answered `[BT16-077, ST6-07]`: our engine PLAYS
  DINOBEEMON (a Lv.5 that matches neither slot) and trashes Effecmon. "1 each
  of the specified cards" (general_rule 16-28: all-or-nothing across the
  specified set) admits no such play; DCGO's slot lists cannot
  contain it.
- `effect2` with the pick answered `[ST6-07]` alone: our engine plays ONE card
  and trashes the other. "1 each" is not "up to"; DCGO auto-takes both.

The committed lines pick the two legal cards, `sim_only` (DCGO asks nothing at
one candidate per slot), so they are expected CLEAN — the finding lives here,
not in a verdict. The fix is per-slot candidate lists (one pick per slot, a
pick parked only when a slot has >1 candidate — which also removes the
sim-only row).

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

`effect1` / `effect3` were on disk, uncommitted, with `assert: []`.

### Both files would have aborted at step 4: wrong `expect:` on the breeding digivolves

The T3 (P0) and T4 (P1) breeding digivolves carried `expect: { prompt:
breeding_action }`. A Lv.2 in an occupied breeding area can neither hatch nor
move, so neither engine asks a breeding decision that turn — the turn opens on
`main_phase` (the confirmed `P-180-effect1.yaml` convention). Our sim-side
check is loose there; DCGO asserts `expect:` strictly. Fixed to `main_phase`
in both files.

### `effect#3` — a missing `<Raid>` row (removed by changing the board)

The draft promoted P1's Agumon on T6. The effect attack ("1 of your Digimon may
gain <Rush> and attack a player") is still an attack declaration, so
Dinobeemon's own `<Raid>` (`RaidSelfEffect`, `OnAllyAttack`) would have opened
an `OptionalSkill` on the unsuspended Agumon — a row the draft did not have.
P1 now keeps Agumon in the breeding area all line (a declined `move`), so the
declaration stacks nothing.

### Asserts

Both files now pin the shared pre-clause board; `effect1` also pins the
post-switch board (Agumon deleted, security untouched at 5).

## CARD-YAML FINDING — the whole [When Digivolving] is gated on `dna_origin` (`effect#3`)

`code/digimon-engine/cards/bt16/BT16-077.yaml` puts `condition: { dna_origin:
true }` on the WHOLE clause. The printed text scopes "If DNA digivolving" to
the first sentence only; the official Bandai Q&A on the card
(`data/card_bundles/BT16-077.md`) says so in terms — "Even if this Digimon
didn't DNA digivolve, this card's [When Digivolving] effect can give 1 of your
Digimon <Rush> and that Digimon can attack" — and `BT16_077.cs` gates only the
trash play on `IsJogress(hashtable)`, running the Rush/attack half
unconditionally. So on an ordinary digivolution our engine opens nothing while
DCGO asks `OptionalSkill` (`isOptional: TRUE`) → `SelectPermanentEffect`
(`canNoSelect: true`) → `SelectAttackEffect` (`SetCanNotSelectNotAttack`,
players only). `BT16-077-effect3.yaml` scripts DCGO's three rows (they answer
no live prompt on our side and ride the wire) and is PREDICTED to read
`diverged` right after the digivolve (DCGO: Dinobeemon suspended, P1 security
4; ours: unsuspended, 5). The fix is to move the `dna_origin` gate onto the
trash-play steps only. Not applied here (exam stage). The same clause's DCGO
decline is crossed by `effect1` (`OptionalSkill` "no", wire-only row).

Also noted: the YAML's `alt_paths` author only the `purple Lv.4 / 4` circle;
the official DB and `data/card_overrides.json` print `Purple Lv.4 / 4` AND
`Red Lv.4 / 4`. Both lines here digivolve from a purple Lv.4, so the red
circle is un-probed.

## Prompt shapes (re-derived from the C#)

- `<Raid>`: `OptionalSkill` + `SelectPermanentEffect` (`canNoSelect: true`);
  ours one declinable pick → the fold (`expect: OptionalSkill` on the row).
- [When Digivolving]: `SetUpActivateClass(CanActivateCondition, …, -1, TRUE,
  …)` — the `OptionalSkill` is asked on EVERY digivolution into Dinobeemon
  (`CanActivateCondition` is only `IsExistOnBattleArea`).
- Both Dinobeemon circles cost 4, so a purple Lv.4 base opens no
  `SelectCountEffect`.
- Slot hygiene: one Digimon per seat in effect1/effect3; see above for
  effect2/inherited0.
