# meta-scoped-gauntlet — delta spec

## ADDED Requirements

### Requirement: Win-rate-based multiplicative Threat Index
The MetaGauntlet SHALL compute each archetype's Threat Index as `TI = meta_share × exp(c · (wr_adj − 0.5))`, where `wr_adj` is the DigiLab win rate shrunk toward 0.5 by empirical-Bayes shrinkage (`wr_adj = (wins + k·0.5) / (games + k)`), and SHALL NOT use raw conversion rate as a weighting input.

#### Scenario: Popular even-win-rate deck outweighs fringe high-variance deck
- **WHEN** archetype A has meta_share 0.12 and wr 0.505 over 1,300 games, and archetype B has meta_share 0.004 and wr 0.66 over 8 games
- **THEN** A's Threat Index exceeds B's, because B's shrunk win rate is pulled near 0.5 while A retains its share advantage

#### Scenario: Win rate tilts weight between equally popular decks
- **WHEN** two archetypes have equal meta_share and one has wr_adj 0.60 over 500 games while the other has wr_adj 0.50
- **THEN** the higher-win-rate archetype receives strictly greater Threat Index

### Requirement: Shrinkage-gated sleeper rule
The sleeper floor SHALL trigger only on the shrunk win rate (`wr_adj > sleeper_threshold`), and the legacy `confidence_min_appearances` gate SHALL be retired.

#### Scenario: Small-sample fluke does not pin the floor
- **WHEN** an archetype shows wr 0.70 over 6 games (wr_adj ≈ 0.54 at k=25) and sleeper_threshold is 0.55
- **THEN** the sleeper floor is not applied

### Requirement: Format-windowed statistics
The MetaGauntlet SHALL support a format window (date range) that scopes DigiLab-derived shares and win rates to that window before weight computation, and SHALL fail loudly when the scoped query is unavailable unless an explicit optional-fallback flag is set.

#### Scenario: Scoped weights reflect the window, not all history
- **WHEN** a run sets a window covering only the current format and an archetype's all-history share differs from its in-window share
- **THEN** sampling weights are computed from the in-window share

#### Scenario: DigiLab outage without fallback flag aborts
- **WHEN** the scoped query fails and the optional-fallback flag is not set
- **THEN** gauntlet loading raises an error instead of silently using all-history weights

### Requirement: Deck-pool deduplication
Decklists SHALL be deduplicated by content-addressed deck identity (`stable_deck_id`) across sources and alias-merged archetype entries before sampling weights are computed.

#### Scenario: Same list from two sources is weighted once
- **WHEN** an identical 50-card list is ingested under two sources (or two aliases of one archetype)
- **THEN** it appears once in the sampling pool

### Requirement: Weighted within-archetype deck routing
Within an archetype, deck sampling probability SHALL be weighted by source preference and event-date recency rather than split evenly.

#### Scenario: Preferred-source recent list is sampled more often
- **WHEN** an archetype has a recent digimonmeta list and an old low-preference list
- **THEN** the recent digimonmeta list has strictly greater sampling probability

### Requirement: Adaptive sampling from live agent performance
The gauntlet SHALL support recomputing sampling weights each evaluation window as `w ∝ TI × (1 − wr_agent)^τ`, where `wr_agent` is the shrunk per-archetype agent win rate from training telemetry, with a per-archetype weight floor and τ=0 disabling adaptation.

#### Scenario: Mastered matchup yields weight to weak matchup
- **WHEN** the agent's shrunk win rate is 0.85 vs archetype A and 0.35 vs archetype B, with equal TI and τ=1
- **THEN** B's recomputed sampling weight exceeds A's, and A's weight does not fall below the configured floor

### Requirement: Hash-verified gauntlet snapshot
The MetaGauntlet SHALL write a hash-verified snapshot (window, scoped stats, TI parameters, deduplicated deck records with weights, pilot bindings) at run start and SHALL restore from it on resume, refusing a snapshot whose content hash does not match.

#### Scenario: Library rebuild mid-run does not change the pool on resume
- **WHEN** `deck_library.json` is rebuilt while a run is suspended and the run resumes from its snapshot
- **THEN** the restored pool and weights are identical to the original run's

### Requirement: Bounty bonus retired in favor of sampling emphasis
Once adaptive sampling is enabled, the GauntletWrapper SHALL NOT add opponent-dependent terminal reward bonuses; terminal rewards remain pure game/match outcomes.

#### Scenario: Beating a high-threat opponent yields the standard terminal reward
- **WHEN** the agent wins against an opponent whose Threat Index exceeds any threshold
- **THEN** the terminal reward equals the standard win reward with no bounty term
