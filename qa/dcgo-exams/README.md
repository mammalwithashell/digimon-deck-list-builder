# DCGO exam scenarios

Scripted scenarios for the **card-clause exam**: hand-authored legal lines that
make DCGO answer a *targeted* question about one clause of one card, instead of
only reporting whatever its AI happened to do.

- **Design (authoritative):** `docs/superpowers/specs/2026-08-21-dcgo-scripted-scenario-exam-design.md`
- **Verified API facts / plan corrections:** `docs/superpowers/plans/2026-08-21-dcgo-exam-recon-facts.md`
- **Harness operating manual:** `docs/DCGO_HARNESS.md`
- **Recording format:** `docs/DCGO_RECORDING_SCHEMA.md`
- **Verdict store:** `qa/qa-reports/dcgo_exam_verdicts.json`

## Layout

```
qa/dcgo-exams/<SET>/<CARD-ID>.yaml
```

One file per clause (a card with several clauses gets several files). The set
directory mirrors the card id's set code, e.g. `qa/dcgo-exams/EX12/EX12-035.yaml`.

## Format

```yaml
card: EX12-035
clause: EX12-035#effect#0      # a clause_coverage Clause.id: {card_id}#{zone}#{idx}
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-035, BT16-082], rest: <deck-name> }
  p1: { stack: [...],                        rest: <deck-name> }
steps:
  - actor: 0
    do:     { hatch: {} }
    expect: { prompt: main_phase }
  - actor: 0
    do:     { play: {card: EX12-035, from: hand} }
    expect: { prompt: main_phase }
  - actor: 0
    do:     { select: {targets: [opp.field.0]} }
    expect: { prompt: select_permanent, count: 1 }
assert:
  - at: 3
    that: { opp.field.0.dp: 3000, p0.memory: -2 }
```

### `clause` is not free text

It is a `clause_coverage.models.Clause.id`, formatted `{card_id}#{zone}#{idx}`.
The exam **rejects** a scenario naming a clause that `clause_coverage.extract`
does not produce for that card — otherwise a typo silently creates an invisible
sixth verdict class: a scenario that passes while covering nothing in the
denominator.

Clause `idx` is positional within a zone, so the verdict store records the
clause `label` and a hash of its `text` alongside the id. If a card's text
changes, the stored verdict is invalidated back to `unmeasured` rather than
reporting a stale `confirmed`.

### `stack` is a prefix, and applies to the initial shuffle only

You name the first N cards in draw order; the remainder is seeded-shuffled from
a named deck. Requiring all 50 would make every file unauthorable.

Initial-shuffle-only is a *correctness* requirement, not an optimization: if the
stack applied to every shuffle, a card reading "shuffle your deck" would
silently re-impose the opening order and the exam would confidently answer a
question about a game that cannot occur. Mid-game shuffles fall through to
seeded `GameRandom` — still deterministic, but honest.

`Game::new` does not validate deck legality, but a scenario meant to mirror DCGO
still needs a tournament-legal list, because DCGO gates battles on
`DeckData.IsValidDeckData()` (50 main, ≤5 egg).

### Both seats are fully scripted

If DCGO's AI plays the opponent, our engine cannot reproduce its choices and
every game diverges for reasons that are not findings.

### `expect` is asserted *before* the step is answered

A driver that answers whatever it is asked will, on one ordering mismatch,
desynchronize the whole remainder of the line while every step still looks
successful. So each step carries the prompt it expects and the driver asserts
first. A mismatch aborts the job and reports itself — **that is a finding, not
an error**: "our engine expected a choice here and DCGO never asked" is exactly
the divergence class that never surfaces as an illegal action.

The scripted-step vocabulary is 13 kinds: the 10 selection prompts
(`SelectCardEffect`, `SelectHandEffect`, `SelectPermanentEffect`,
`SelectAttackEffect`, `SelectCountEffect`, `SelectDigiXrosClass`,
`MultipleSkills`, `OptionalSkill`, `generic_int`, `generic_bool`) plus
`mulligan`, `breeding_action`, and `main_phase`.

### `do` is symbolic, lowered to an action id

Both engines share the 2192-slot action space (`ActionSpace.cs` is codegen'd
from `space.rs` behind a CI drift gate, rule 27). Raw integers would be
unwritable by hand and would rot silently on renumbering, so steps are symbolic
and a lowering pass resolves each against our engine's live action mask, failing
loudly on illegal or ambiguous intent — in milliseconds, before any Unity time
is spent.

### `assert` is backfilled, not hand-guessed

You author the line; the oracle records what happened; on a `confirmed` verdict
`exam backfill` writes that confirmed DCGO state into the `assert:` block. That
is what makes these blocks load-bearing rather than decorative — they are the
guard that survives into CI after the oracle has gone home.

Backfill refuses to run on a diverged line (that would bake DCGO's disagreement
in as our expected value), preserves hand-authored assertions it did not
generate, is idempotent, and never asserts over security *contents* — security
is a count in the projection precisely because its contents are hidden
information.

## Running

```bash
# sim-only: our engine alone, checks the backfilled assert: blocks.
# Milliseconds, no Unity. This is what CI runs (.github/workflows/dcgo-exam-sim.yml).
cargo run -p dcgo-harness -- exam --scenario qa/dcgo-exams/ --sim-only --cards-json data/cards.json

# the oracle pass: deliberate, local, ~40s of Unity per scenario
cargo run -p dcgo-harness -- exam --card EX12-035 --cards-json data/cards.json

# scenario-suite regression replay against the oracle
cargo run -p dcgo-harness -- exam --suite
```

Always run sim-only first: every scenario must lower and run before any Unity
time is spent.

## The five verdict classes

| Verdict | Meaning |
|---|---|
| `confirmed` | A scenario exercised this clause and both engines agreed for the whole line |
| `diverged` | Both engines ran it and disagreed |
| `unreachable` | A scenario exists but the line could not legally reach the clause |
| `unavailable` | DCGO's pool has no script for this card, so no oracle exists |
| `unmeasured` | No scenario authored yet |

**Always print the full denominator.** A card reads as
`EX12-035: 8 clauses — 5 confirmed, 1 diverged, 2 unmeasured`, never
`EX12-035: passed`.

Three honesty constraints:

- **`unavailable` is per card, not per set.** DCGO's `Assets/Scripts/CardEffect/`
  spans AD1, BT1–BT26, EX1–EX12, ST1–ST24, LM, P and RB1, so "newer than DCGO"
  is the wrong test — a set directory can exist while an individual card has no
  script (`<SET>/<COLOR>/<CARD_ID>.cs`, underscored filename). `unavailable`
  must read as "not verified", never as "passed".
- **`diverged` does not mean we are wrong.** Source priority puts
  `general_rule.pdf` above DCGO. A divergence is ranked and diagnosed; the fix
  stays a decision, not an automation.
- **25 cards route through `SetIsBackgroundProcess(true)`** and bypass the
  `effect_activation` recorder hook entirely (rule 27). Their clauses are
  structurally unmeasurable by activation matching, so they get `unreachable`
  carrying that specific reason — not a silent pass.

## Note for CI

`.github/workflows/dcgo-exam-sim.yml` gates PRs on this directory, but it runs
`--sim-only` and checks out **no submodules**. It asserts our engine still
agrees with what the oracle previously confirmed; it **cannot** find a new
divergence, because there is no oracle in that job. See the workflow's header
comment for the four reasons DCGO itself cannot gate PRs on GitHub-hosted
runners.
