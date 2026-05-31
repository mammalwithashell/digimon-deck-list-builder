## ADDED Requirements

### Requirement: Model evaluation guide document
The repository SHALL contain `docs/MODEL_EVALUATION.md` documenting how to evaluate models across training modes, and `docs/INDEX.md` SHALL link to it.

#### Scenario: Doc exists and is indexed
- **WHEN** a contributor looks for evaluation guidance
- **THEN** `docs/MODEL_EVALUATION.md` exists and is listed in `docs/INDEX.md`

### Requirement: Per-mode metric taxonomy
The guide SHALL document, for each training mode (greedy, random, gauntlet, generalist, self-play, pool/FSP/PFSP), what the native in-run win rate actually measures and its failure mode — explicitly stating that self-play win rate is degenerate (≈50% + first-player edge) and that modes are not mutually comparable.

#### Scenario: Self-play degeneracy is documented
- **WHEN** a reader consults the taxonomy for self-play
- **THEN** the guide states the mirror win rate is pinned near 50% and is not a learning signal

### Requirement: Anchored reference frame and layered eval stack
The guide SHALL document the anchored reference-frame tiers (random / greedy / frozen champions / held-out scenarios) and the layered eval stack: L0 PPO diagnostics (value-loss, entropy, KL), L1 behavioral (game length, digivolves/game), L2 anchored win rate vs greedy and champions, L3 Elo ladder, L4 exploitability.

#### Scenario: The five layers are described
- **WHEN** a reader consults the eval stack section
- **THEN** layers L0 through L4 are each described with what they measure and their cost/cadence

### Requirement: Gated self-play documented as evaluation-as-control
The guide SHALL document AlphaGo-Zero-style gated self-play (frozen best-player anchor, ≥55% promotion) as a mode whose evaluation is built into the training control loop, yielding a monotone best-player Elo curve.

#### Scenario: Gating explained
- **WHEN** a reader consults the gated self-play section
- **THEN** it explains how promotion gating makes the best-player rating monotone and why that solves the self-play eval problem

### Requirement: Equilibrium-methods horizon section
The guide SHALL contain a "Robustness & equilibrium methods" horizon section covering Deep CFR, ReBeL, and Player of Games — what each requires (forkable model, infoset/public-belief structure, tractable belief space), what each offers (low exploitability, native hidden-info play, test-time search), and the explicit blocker that all of them depend on a cloneable/forkable engine, forward-referencing the `make-engine-cloneable` change.

#### Scenario: Horizon section names the clonability dependency
- **WHEN** a reader consults the equilibrium-methods section
- **THEN** it states that Deep CFR / ReBeL / Player of Games depend on a cloneable engine and references the `make-engine-cloneable` change

### Requirement: CLAUDE.md evaluation working rule
`CLAUDE.md` SHALL reference `docs/MODEL_EVALUATION.md` and SHALL contain a Working Rule stating that in-run win rate is not a cross-mode learning signal and is degenerate under self-play, that improvement claims must come from the anchored benchmark (greedy + frozen champions, seat-balanced), and that exploitability is the robustness signal.

#### Scenario: Working rule present
- **WHEN** a contributor reads CLAUDE.md's working rules
- **THEN** a rule forbids claiming model improvement from the mirror/self-play eval and directs them to the anchored benchmark and exploitability
