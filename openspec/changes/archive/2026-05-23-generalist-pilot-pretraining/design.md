## Context

Pilot training currently supports fixed player decks, explicit opponent decks, self-play, and gauntlet opponent sampling. The gauntlet path can now filter to fully implemented Rust DSL archetypes and Rust-registered card IDs, but that sampling only controls `deck2`; `deck1` remains fixed unless a caller wires `DeckPoolWrapper` programmatically.

The desired model is a reusable generalist base pilot: one policy pretrained across multiple fully implemented archetypes so later runs can fine-tune it into a specific archetype pilot. The same mechanism must also support fair A/B comparisons between tensor profiles by holding the game curriculum constant while changing the observation profile.

## Goals / Non-Goals

**Goals:**

- Provide a first-class CLI mode for generalist pilot pretraining.
- Sample both player and opponent decks from the eligible Rust DSL deck pool.
- Use uniform archetype sampling followed by uniform deck sampling within the selected archetype.
- Make deck-pair sampling reproducible through a curriculum seed that is independent from model/training seed.
- Freeze the eligible deck pool at run start with stable content-addressed deck IDs.
- Allow a later run to reuse a frozen deck-pool snapshot for tensor-profile A/B testing.
- Fail fast when any training deck source contains unimplemented cards.
- Record the curriculum, pool, seed, and tensor contract in model metadata.

**Non-Goals:**

- Training a strong self-play league or PFSP system for the generalist in this change.
- Changing observation tensor layouts, action space size, or action-mask semantics.
- Declaring partially implemented archetypes eligible for training.
- Guaranteeing bit-identical model weights across tensor profiles; the guarantee is an identical deck curriculum when seeds and snapshots match.

## Decisions

### Use a shared eligible deck pool abstraction

Create a small shared abstraction around the existing gauntlet deck loading logic so both `deck2` gauntlet sampling and generalist `deck1` sampling consume the same implementation-safe pool. The pool should expose archetype names, deck records, stable deck IDs, source metadata, and card IDs.

Alternative considered: add a separate generalist-only loader. That would duplicate eligibility logic and increase the chance that gauntlet and generalist training disagree about which decks are safe.

### Sample uniform archetype, then uniform deck

For generalist mode, sampling should choose an archetype uniformly, then choose one deck uniformly from that archetype. This prevents high-volume archetypes in `deck_library.json` from dominating pretraining.

Alternative considered: uniform decklist sampling. That is useful for broad random exposure but makes the training distribution depend heavily on scraper/library volume rather than intended gameplay coverage.

### Use content-addressed deck IDs and frozen snapshots

Each deck record should have a stable ID derived from canonical card counts, including main deck and Digi-Egg cards. A frozen snapshot file should contain the eligible records and a snapshot hash. Reusing that snapshot plus the same curriculum seed should reproduce the same deck-pair sequence even if `data/deck_library.json` changes later.

Alternative considered: store deck indexes. Indexes are fragile because deck library rebuilds can reorder, deduplicate, insert, or remove decklists.

### Separate training seed from curriculum seed

The existing training seed should continue to seed Python, NumPy, Torch, SB3, and env reset behavior. A new curriculum seed should drive only deck-pair sampling. Evaluation should also accept a separate eval seed so tensor-profile comparisons can use a fixed evaluation schedule.

Alternative considered: one global seed for everything. That is simpler but makes it harder to isolate whether a change affected model initialization, gameplay RNG, or deck curriculum.

### Validate explicit decks before training starts

The generalist pool must be implementation-safe, and explicit `--deck1` / `--deck2` inputs should be held to the same standard. The CLI should reject unimplemented card IDs before constructing training environments and include the missing IDs in the error.

Alternative considered: let the engine fail if it hits an unsupported card. That produces late failures and can silently pollute training with unsupported behavior if crashes are counted as draws.

### Treat fine-tuning as checkpoint initialization plus fixed deck curriculum

Fine-tuning should load a generalist checkpoint and then run the existing fixed-deck plus gauntlet flow. The model metadata should preserve both the base checkpoint reference and the fine-tune tensor contract.

Alternative considered: create a separate fine-tuning subsystem. The current SB3 checkpoint loading path should be enough unless implementation finds shape-compatibility details that require a narrower helper.

## Risks / Trade-offs

- Sparse eligible archetypes -> The first generalist model is only general across currently implemented archetypes. Mitigation: metadata records the eligible pool and count so the model's scope is explicit.
- Curriculum reproducibility drift -> Live deck libraries change over time. Mitigation: snapshot files and content-addressed deck IDs are required for A/B comparisons.
- Overfitting to greedy opponent behavior -> Initial generalist mode may still learn greedy-opponent exploits. Mitigation: keep the initial change focused, then add saved-agent or league opponents in a later proposal.
- More noisy learning -> Randomizing `deck1` increases policy variance. Mitigation: recommend longer pretraining and evaluate whether fine-tuning reaches target strength faster than from-scratch runs.
- Snapshot size growth -> Frozen pools may be larger as more archetypes become eligible. Mitigation: store card IDs/counts as compact JSON records and hash canonical content.

## Migration Plan

1. Add the shared eligible deck pool and validation helpers behind the existing gauntlet behavior.
2. Add generalist deck sampling wrappers and CLI flags without changing default training behavior.
3. Extend metadata and runbook documentation.
4. Add tests for deterministic sampling, snapshot reuse, implemented-card validation, and CLI wiring.
5. Roll back by avoiding `--generalist`; existing fixed-deck and gauntlet commands remain the default path.

## Open Questions

- Should the first implementation write a full per-episode curriculum manifest by default, or only when an audit flag is provided?
- Should `deck1_archetype` and `deck2_archetype` be allowed to match in the same episode, or should mirror matches be optional?
- Should generalist evaluation use fixed sampled deck pairs from the same snapshot, or a separate frozen evaluation snapshot?
