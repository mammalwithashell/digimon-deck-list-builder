## 1. Eligible Deck Pool

- [x] 1.1 Extract shared eligible-deck pool helpers from `MetaGauntlet.load()` without changing existing gauntlet defaults
- [x] 1.2 Add stable content-addressed deck IDs derived from canonical deck card counts
- [x] 1.3 Add implemented-card validation helpers that report missing card IDs for any deck source
- [x] 1.4 Add tests that unimplemented cards are rejected for gauntlet decks, generalist pools, and explicit deck inputs

## 2. Snapshot Support

- [x] 2.1 Define the frozen deck-pool snapshot JSON schema with archetypes, deck records, deck IDs, card IDs/counts, source metadata, and snapshot hash
- [x] 2.2 Implement snapshot writing at generalist run start
- [x] 2.3 Implement snapshot loading for later runs
- [x] 2.4 Add tests proving snapshot reuse is stable when live deck library order or contents change

## 3. Generalist Sampling

- [x] 3.1 Implement a wrapper or sampler that injects sampled `deck1` and `deck2` at episode reset
- [x] 3.2 Implement uniform-archetype-then-uniform-deck sampling for both player decks
- [x] 3.3 Add independent curriculum seed handling for deck-pair sampling
- [x] 3.4 Add tests that same snapshot plus same curriculum seed reproduces the same deck-pair sequence
- [x] 3.5 Add tests that different curriculum seeds produce different valid deck-pair sequences

## 4. CLI and Fine-Tuning

- [x] 4.1 Add pilot-training CLI flags for generalist mode, curriculum seed, eval seed, snapshot output, and snapshot reuse
- [x] 4.2 Validate explicit `--deck1`, `--deck1-json`, `--deck2`, and `--deck2-json` inputs against implemented-card IDs before training starts
- [x] 4.3 Add checkpoint-loading support for initializing archetype fine-tuning from a generalist base model
- [x] 4.4 Reject fine-tune checkpoints whose tensor profile, tensor layout hash, or action-space size is incompatible
- [x] 4.5 Add CLI tests covering generalist mode, snapshot reuse, explicit deck validation, and fine-tune compatibility checks

## 5. Metadata and Evaluation Provenance

- [x] 5.1 Extend training metadata with training mode, sampling policy, training seed, curriculum seed, eval seed, snapshot hash/path, eligible archetypes, and eligible deck count
- [x] 5.2 Record base checkpoint provenance when fine-tuning from a generalist model
- [x] 5.3 Ensure existing tensor profile, tensor version, tensor size, tensor layout hash, and action-space size remain present in saved metadata
- [x] 5.4 Add tests for generalist and fine-tune metadata sidecars

## 6. Documentation and Verification

- [x] 6.1 Add a Generalist Pilot Training section to `docs/TRAINING_RUNBOOK.md`
- [x] 6.2 Document recommended commands for pretraining, tensor-profile A/B comparison with snapshot reuse, and archetype fine-tuning
- [x] 6.3 Document seed semantics and the reproducibility limits of same-curriculum tensor-profile comparisons
- [x] 6.4 Run focused RL tests for gauntlet, pilot training config, snapshot sampling, metadata, and deck validation
- [x] 6.5 Run `openspec status --change generalist-pilot-pretraining` and confirm the change is apply-ready
