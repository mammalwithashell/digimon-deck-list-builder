## ADDED Requirements

### Requirement: Ordered returns preserve deck legality and entitled egg order
Ordered deck returns MUST route Digi-Egg cards to their owner's Digi-Egg deck in the chosen order, preserve the main-deck total, and emit a main-deck addition only if a main-deck card was actually added. Hypothetical worlds MUST preserve legitimately remembered Digi-Egg deck positions for either seat.

#### Scenario: An ordered return includes an egg above a retained base
- **WHEN** a combined field stack returns an ordered group containing a Digi-Egg to the deck
- **THEN** the egg enters the Digi-Egg deck, the other cards enter the main deck, and their respective relative orders follow the player's choices
- **AND** sampled worlds conserve the complete deck and keep the returned egg at its entitled position

### Requirement: Link continuations use choosing-seat and public identities
Closed worlds MUST support link-host continuation data only when the source identity and cached hand slot are entitled to the choosing seat, and cached host candidates refer to public field objects. Unsupported ownership or reference forms MUST fail explicitly.

#### Scenario: Link prompts survive a composition-changing hypothesis
- **WHEN** an Option link, from-hand Option link or Digimon link is waiting for its host choice
- **THEN** changing the actual concealed opponent cards leaves the policy observation, sampled world, legal root choices and resolved hypothetical continuation unchanged for the same evidence and seed

### Requirement: Unused event references require a compiler proof
Hypothetical queued effects MAY omit concealed event-card references only when a conservative analysis proves that the entire clause, its gates and implicit timing behavior do not read those identities. Unknown predicates, steps, binding kinds, granted closures and raw Rust effects MUST retain strict reference auditing. The source game MUST remain unchanged.

#### Scenario: A queued observer outlives anonymous hand membership
- **WHEN** an identity-independent security-removal observer remains queued after subsequent hand actions erase the removed card's known membership
- **THEN** the hypothetical context omits only proven-unused concealed reference fields
- **AND** changing the actual concealed card or original opponent deck produces the same sampled world and continuation behavior for the same public evidence and seed

#### Scenario: A clause reads the event card
- **WHEN** a gate, nested selection filter, formula or binding reads event-card identity, or a clause shape has no proof
- **THEN** the search retains strict auditing and rejects an unresolved concealed reference instead of deleting it or reading the actual hidden card

### Requirement: Public hand departures preserve independent knowledge
The knowledge ledger MUST preserve remembered hand identities distinct from the publicly departing card's identity. Revealing a matching card MUST NOT reveal whether it is the remembered physical copy or a different copy, nor increase a historical copy lower bound on that basis alone. Anonymous hand memberships MUST be invalidated conservatively when a departure makes their location ambiguous.

#### Scenario: A different card is publicly discarded
- **WHEN** a player has a remembered hand card and publicly discards another card of a different identity
- **THEN** subsequent knowledge refresh and hypothetical worlds retain the independently remembered hand card

#### Scenario: The announced card matches a remembered identity
- **WHEN** a player announces a card matching a remembered hand identity
- **THEN** the ledger treats the declaration equally whether the engine selected that remembered instance or another concealed copy

### Requirement: Beliefs depend only on entitled information
Closed-decklist beliefs and search outputs SHALL depend only on the viewer's own information, public facts, entitled remembered knowledge, frozen prior configuration, and seeds. The provider MUST NOT read true opponent composition or hidden assignments.

#### Scenario: Different hidden compositions are indistinguishable
- **WHEN** two legal source games have identical viewer knowledge and root action masks but different concealed opponent compositions
- **THEN** closed-mode beliefs, sampled worlds, and search outputs are identical under fixed seeds and evaluator, including after resume

### Requirement: Legal hypotheses account for public copies
Hypotheses SHALL obey the pinned format's deck construction, restrictions, combination and card-specific copy rules and current public accounting across all zones. Historical sightings MUST NOT double-count recycled instances.

#### Scenario: All copies are accounted for
- **WHEN** a hypothesis contains four copies of an ID and two are in trash and two are public elsewhere
- **THEN** it assigns zero copies of that ID to hidden zones until a legal zone move changes accounting

#### Scenario: Restriction and recovery
- **WHEN** a format limits a card to one copy and its public instance is returned from trash to a hidden hand
- **THEN** every hypothesis contains at most that one copy and viewer-entitled identity knowledge follows the move without inventing another copy

### Requirement: Candidate uncertainty includes unfamiliar legal lists
The provider SHALL maintain a normalized candidate mixture and configurable positive unseen-list mass, condition on evidence, and use a constrained fallback from the frozen legal population. Exhaustion SHALL be explicit and SHALL NOT consult true hidden information or relax legality.

#### Scenario: Evidence eliminates every candidate
- **WHEN** public facts contradict every known candidate list
- **THEN** the provider samples consistent legal fallback completions or reports bounded exhaustion with diagnostics

#### Scenario: Compatible candidates remain
- **WHEN** one candidate remains but the prior config reserves unseen-list mass
- **THEN** the provider retains that mass rather than asserting that the opponent must use the remaining list

### Requirement: Entitled memory has lifecycle semantics
Viewer knowledge SHALL retain legitimate reveals and peeks, distinguish private and public facts, invalidate knowledge when warranted by hidden randomization, and survive snapshot/resume without hidden-truth reconstruction.

#### Scenario: Peek followed by shuffle
- **WHEN** a viewer knows a deck's top card and that deck is shuffled
- **THEN** the prior no longer pins that top position and the opponent does not acquire the private peek

#### Scenario: Concealed security return has a known destination
- **WHEN** an unrevealed security card is returned to the deck bottom without a reveal requirement
- **THEN** memory retains its publicly determined destination without recording its real identity, and continuations referencing that physical move resolve against a sampled identity

#### Scenario: Own unknown assignments are interchangeable
- **WHEN** the owner's unseen deck/security joint multiset and entitled facts are unchanged but actual concealed identities are reassigned between those zones
- **THEN** the same sampling seed produces the same hypothetical world, including anonymous movement references, without retaining true native identity aliases

### Requirement: Sampled worlds represent the hypothesis
Closed-mode worlds SHALL be constructed from sampled compositions and entitled facts, with coherent unique instances, counts, metadata, action masks, and continuations. Invariants SHALL compare to the hypothesis, not the actual hidden deck. Unsupported continuation states SHALL fail explicitly.

#### Scenario: Hypothetical card differs from the true deck
- **WHEN** a consistent sampled list includes a legal card absent from the actual opponent list
- **THEN** the materialized world can contain and resolve that card without borrowing hidden instances or enforcing the true list's composition
