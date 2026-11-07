## ADDED Requirements

### Requirement: Tamer-first baselines are explicit policy variants
The evaluator SHALL identify greedy and development Tamer-first variants separately from their original policies. Each variant MUST prioritize mask-legal direct Main-phase digivolutions onto the deciding player's Tamer, preserve fallback behavior when none exist, retain phase/selection legality and repeated-state guards, and record eligible routes and applied preference. Existing training and frozen runs MUST NOT change implicitly.

#### Scenario: Tamer evolution passes memory
- **WHEN** the mask exposes a direct Tamer evolution that passes memory
- **THEN** the Tamer-first variant still chooses an eligible Tamer evolution, labels its version and leaves the engine to execute its actual costs and effects

#### Scenario: A pending effect reuses Main action IDs
- **WHEN** an action ID is presented as a pending selection rather than a direct Main-phase digivolution
- **THEN** the variant uses its base policy's selection behavior and does not misclassify the action as a direct Tamer evolution

### Requirement: Search exploration has opportunity and execution evidence
New instrumented runs SHALL preserve complete-plan pre/post-noise priors, root visits/Q, selection mode/probability, selected plan and executed prefix. Direct Tamer, raising, battle and DNA evolution routes MUST remain distinct. Reports MUST count opportunities once per root, label missing historical probabilities and effect-sequence opportunities unmeasured, and never infer execution from the most-visited action. Instrumentation MUST NOT alter RNG, model calls, actions, observations, rewards or training targets.

#### Scenario: Sampling chooses a less-visited hard play
- **WHEN** a legal Tamer evolution receives more visits but the training sampler chooses a hard play
- **THEN** telemetry preserves both alternatives and their probability/visit mass, identifies the selected hard play and records it as executed only after its complete route is dispatched

### Requirement: Closed actors use only entitled information
The evaluator SHALL use independent per-seat knowledge and a frozen training-only prior through the production belief APIs. It MUST exclude actual hidden opponent composition, oracle list/archetype labels, held-out candidate decks and future evidence from closed actor inputs. It MUST fail explicitly on unsupported states rather than reverting to exact information.

#### Scenario: Actual hidden composition changes
- **WHEN** two states share entitled information and fixed RNG but have different actual hidden opponent cards
- **THEN** the closed actor's evidence, observation, sampled worlds, plans and selected action remain invariant and the source games remain unchanged

#### Scenario: A held-out list is played
- **WHEN** the game driver supplies a source-held-out deck to an opponent
- **THEN** that list is absent from the actor's prior candidates and any population fallback is recorded without reading the true hidden composition

### Requirement: Mode comparisons share one frozen runtime
The evaluator SHALL freeze source, binaries, dependencies, data, models, metadata, prior, search settings, seeds and per-actor modes. Matched open and closed comparisons MUST use the same runtime and model hashes. Existing model contract mismatches MUST block execution.

#### Scenario: Historical open results use another engine
- **WHEN** the report includes the existing older-engine campaign
- **THEN** it labels those results historical and computes information-regime differences only from new same-runtime paired controls

### Requirement: Scored execution requires adapter qualification
The campaign SHALL require full-budget preflight coverage of all 21 training lists and all four models, continuation and information-invariance checks, and Windows/Linux exact reference parity before scored execution. Diagnostic games MUST be excluded from scored denominators.

#### Scenario: A preflight fails
- **WHEN** a required preflight rejects a result or reaches its cap
- **THEN** the affected scored stage remains blocked and the original request, seeds, model, decks, trace and error are preserved without a retry or substitution

### Requirement: Stage populations and budgets remain explicit
The campaign SHALL plan six closed pairings and six open controls of 520 games each, four same-checkpoint open-versus-closed panels of 520 games each, and a separate six-pair held-out extension of 480 games each. It MUST verify four-game blocks, seat/opener balance, archetype exposure and list coverage. Training, held-out, supplemental and pet populations MUST remain separate.

#### Scenario: Held-out coverage is reported
- **WHEN** a held-out panel is summarized
- **THEN** it identifies 23 source-held-out lists across 12 archetypes, verifies 120 blocks and 40 candidate appearances per archetype, and labels Light Hybrid unseen-list strength unmeasured

### Requirement: Evidence validation precedes scoring and reporting
The evaluator SHALL verify every request/output hash and completed-game semantic before reporting a complete score. It MUST report W/L/D, block intervals, paired mode differences, seat splits, per-archetype uncertainty, decision timing and belief support/fallback statistics. It MUST keep evolution routes distinct and label unmeasured opportunities. Shared seeds and role reports MUST NOT be counted as independent samples.

#### Scenario: One game is rejected
- **WHEN** a panel has an unsupported, crashed, truncated, timed-out or rejected result
- **THEN** it remains incomplete, its queue stops, the evidence is preserved, and no draw replacement or completed panel score is emitted

#### Scenario: An open/closed score difference is estimated
- **WHEN** matched panels complete on aligned blocks
- **THEN** the report uses paired block differences and declares exploratory multiple comparisons rather than deriving a promotion decision

### Requirement: New work preserves active workloads
The campaign SHALL use isolated artifacts and check live CPU, memory, disk and exact process roots on both hosts before launch. It MUST respect other workloads, retain the running campaign's immutable inputs, and freeze feasible worker deadlines before scored execution. This plan MUST NOT itself launch, train, promote or restart anything.

#### Scenario: Shared host capacity is occupied
- **WHEN** a planned host has insufficient idle resources
- **THEN** new jobs remain queued without stopping or changing the existing workload, and ETA is withheld until a representative pilot is available
