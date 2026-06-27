## ADDED Requirements

### Requirement: Configurable policy/value head width
The training entrypoint SHALL expose a `net_arch` configuration (CLI `--net-arch`, e.g. `256,256`) that sets the hidden-layer sizes of both the policy and value heads on top of the `CardEmbeddingExtractor`, and MUST default to the current SB3 behaviour (`[64,64]`) when unset so existing runs are byte-identical.

#### Scenario: Default preserves the league net
- **WHEN** training is launched without `--net-arch`
- **THEN** `TrainingConfig.net_arch` is `None` and the built model's policy/value heads are `512 → 64 → 64` (the historical league architecture)

#### Scenario: Width override takes effect
- **WHEN** training is launched with `--net-arch 256,256`
- **THEN** the built model's policy and value heads are `512 → 256 → 256`

#### Scenario: Invalid width is rejected
- **WHEN** `net_arch` is set to an empty list or contains a non-positive integer
- **THEN** `TrainingConfig` validation MUST raise `ValueError`

### Requirement: Extractor-only warm-start for cross-architecture transfer
The training entrypoint SHALL support `--init-extractor-from <checkpoint>` that loads ONLY the `CardEmbeddingExtractor` weights (embeddings + projection) into a freshly built model, leaving the policy/value heads randomly initialized, so a representation trained under one `net_arch` can seed a different `net_arch`.

#### Scenario: Extractor transfers, heads stay fresh
- **WHEN** a model is built with `--net-arch 256,256 --init-extractor-from <[64,64]-seed>`
- **THEN** the model's `features_extractor` tensors equal the seed's and the `[256,256]` heads are randomly initialized (not loaded from the seed)

#### Scenario: Mutually exclusive with full warm-start
- **WHEN** both `--init-extractor-from` and `--init-from` (or `--resume`) are supplied
- **THEN** configuration validation MUST raise `ValueError`

#### Scenario: Incompatible extractor fails loudly
- **WHEN** `--init-extractor-from` points at a checkpoint whose extractor has no shape-matching tensors for the current observation profile
- **THEN** the run MUST raise rather than silently train with a random extractor

### Requirement: Head-width comparison is judged by anchored evaluation
The head-width comparison protocol SHALL train each candidate width from an identical warm-started extractor against a non-trivial opponent (champion pool) and rank widths by anchored evaluation (vs greedy + frozen champions, seat-balanced), and MUST NOT use the in-run/greedy win rate as the verdict.

#### Scenario: Fair, headroom-preserving comparison
- **WHEN** comparing `[64,64]` vs `[256,256]`
- **THEN** both arms MUST start from the same extractor warm-start with fresh heads, train against a champion pool (not greedy alone), and be ranked by their anchored win rate vs the league2 champions

#### Scenario: Greedy is rejected as the sole judge
- **WHEN** the only available signal is win rate vs the greedy heuristic
- **THEN** the protocol MUST treat it as non-discriminating (it ceilings within 1–2 PPO updates) and require the anchored champion panel for the verdict

### Requirement: League driver passes through head width
The deck-specialist league driver SHALL accept `--net-arch` and forward it to each per-specialist training subprocess, so a chosen width can be used for a full league run.

#### Scenario: League forwards the width
- **WHEN** the league driver is invoked with `--net-arch 256,256`
- **THEN** every per-specialist `pilot_training` invocation it spawns includes the matching `--net-arch`/`net_arch` setting
