## ADDED Requirements

### Requirement: One specialist per pod, fanned out across N pods
The Tier-1 launcher SHALL train exactly one deck's specialist per pod, dispatch all decks concurrently across N pods, and reuse the single-box driver's `build_specialist_argv` to construct each pod's training command rather than reimplementing argv assembly.

#### Scenario: Six decks fan out to six pods
- **WHEN** the operator launches a fan-out over six decks with a pod budget of six
- **THEN** six pods are provisioned, each runs one deck's `build_specialist_argv` command, all six train concurrently, and the launcher reports per-pod status

#### Scenario: More decks than pods
- **WHEN** there are more decks than the configured pod budget
- **THEN** the launcher queues the remaining decks and dispatches each to the next pod that frees up, never exceeding the budget

### Requirement: Fixed opponent pool — no cross-round barrier
The Tier-1 launcher SHALL train every specialist against a FIXED opponent set (the MLP generalist plus a frozen champion pool emitted by `champion_admin.py`) so the decks are mutually independent, and therefore MUST NOT impose any per-round barrier across pods.

#### Scenario: Decks never wait on each other
- **WHEN** one deck's pod finishes far earlier than another's
- **THEN** the fast pod is harvested and torn down immediately without waiting for any other pod, because no deck's opponent pool depends on another deck's output

#### Scenario: Frozen champion pool is identical across pods
- **WHEN** the fan-out is launched
- **THEN** every pod is given the same frozen champion-pool manifest, and no pod's pool is updated mid-run

### Requirement: Auto-harvest each pod's final artifacts before teardown
The Tier-1 launcher SHALL detect each pod's completion, download its `final.zip` and `.meta.json` to durable local storage, verify the download, and only then tear the pod down — so no work is lost despite the no-persistent-volume constraint.

#### Scenario: Completed pod is harvested then deleted
- **WHEN** a pod writes its `final.zip` (or a done-marker)
- **THEN** the launcher pulls the artifacts locally, verifies them, records the result, and deletes the pod

#### Scenario: Pod dies before completion
- **WHEN** a pod terminates or becomes unreachable before producing a verified `final.zip`
- **THEN** the launcher marks that deck as failed and MAY re-dispatch it to a fresh pod (decks are independent, so re-running one is safe)

### Requirement: One specialist per pod runs without per-worker resource caps
Because each pod hosts a single specialist on a full uncapped box, the Tier-1 path SHALL run with `DIGIMON_LEAGUE_CONCURRENCY=1` so the per-worker thread caps and the bounded opponent cache (which exist only for the in-pod-parallel case) are NOT applied, and MUST NOT pin OMP/MKL threads in the learner.

#### Scenario: No throttling on a single-specialist pod
- **WHEN** a Tier-1 pod trains its one specialist
- **THEN** the transient SubprocVecEnv thread cap is skipped, the opponent cache default stays large (no reload thrash), and learner OMP threads are left unpinned
