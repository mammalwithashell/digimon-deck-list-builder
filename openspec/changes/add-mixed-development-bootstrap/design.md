## Context

Verified against main `054a06b12` and the current training branch. Bootstrap
`collect_bootstrap` calls Rust `game.greedy_action()` for both players. Its
400-game corpus trains four policy-imitation epochs and eight critic-fitting
epochs before export. Critic targets are terminal outcomes; search then supplies
new visit distributions rather than continued greedy labels.

Rust greedy currently orders Main choices as setup play, keep-turn evolution,
attack, then other play. Setup includes any Tamer or broadly detected resource
card, without distinguishing an immediately useful draw from text on another
timing. Evolution cost estimation considers ordinary printed circles. Many
optional pending choices fall through to PASS. A new teacher must address those
differences rather than merely claim to prioritize evolution already present.

## Goals / Non-Goals

**Goals:** provide diverse, legal examples of developing stacks and replenishing
useful hand resources; preserve memory awareness, combat, private information,
bounded execution, and measurable route coverage.

**Non-Goals:** modifying the running cloud job; changing training rewards,
search budgets, tensors or action semantics; promising a stronger policy from
higher evolution counts alone; replacing self-play with heuristic opponents.

## Decisions

### 1. A separate, versioned Rust teacher

Implement `development_v1` under `code/digimon-engine/src/policies/`, with a
`Policy` wrapper and an explicit PyO3 selector for bootstrap collection.
Preserve `greedy_action()` and its defaults. Each seat owns teacher state,
reset at game boundaries. Always use the actual deciding seat, including
opponent-turn selections. Keep implementation in Rust so native and Python
callers share behavior.

Refactor only reusable read-only action descriptions and cost quotes. Do not
rewrite card effects in the heuristic. Native masks decide legality; internal
teacher features do not extend observations or actions. A missing exact cost or
effect-benefit description is explicit unknown, never a claimed free action or
guaranteed draw. Reuse engine cost queries where possible; add narrowly scoped
read-only queries where missing, with parity tests against actual payment.

### 2. Development preferences with combat and memory constraints

Use lexicographic safety priorities first: finish an immediate winning line
when supported by visible information, avoid a demonstrated immediate public
loss when a legal defense exists, and complete required selections. Do not call
an attack guaranteed lethal merely because security is empty; consider visible
blockers and protection, and do not peek at hidden responses.

For remaining Main candidates use this initial, versioned scoring rubric:

| Feature | Initial weight / bound |
| --- | --- |
| Useful level progression on an existing stack | +4 per level, capped at 2 |
| Completing a visible next step in an evolution line | +2, once per candidate |
| Supported net hand gain after played/discarded cards | +2 per card, bounded -3..3 |
| Card replacement/filtering without net gain | +0.5, at most once |
| Memory paid | -0.75 per memory |
| Memory handed to opponent | additional -1.5 per memory |
| Additional friendly bodies consumed by a route | -2 per body |

Use existing combat evaluation as a comparable fallback, and use pinned fixture
expectations to calibrate the initial score scales before large data collection.
Unknown benefits receive no positive hand/evolution bonus. Unknown costs are
logged and use conservative existing-policy fallback rather than fabricated
quotes. Stable tie breaks favor lower known cost then stable action ordering.
Record the selected feature contributions for diagnosis.

These weights are starting hypotheses, not tuned strength claims. Evolution
that passes a little memory can outrank an expensive hard play; evolution that
gives away excessive memory must not win simply because it increases level.
DNA considers both materials and the resulting board, not just the resulting
level. Same-level routes need supported utility instead of a level bonus.
Separate raising, battle, activated/effect, and DNA routes in diagnostics.

Normal evolution commonly spends one hand card and draws one: score that as
replacement, not +1 net card advantage. Playing a draw-one card has the same
accounting. Draw two/discard two is filtering, not +2 cards. Search/retrieval
benefits depend on the current eligible operation and visible selections;
unknown deck outcomes get no hidden-information-derived bonus. Broad text
matching such as `resource_flow` can identify candidates for inspection but
cannot establish immediate net gain. Diminish hand-growth preference when it
only accumulates redundant cards; prioritize missing visible evolution pieces
and sufficient playable low-level bodies before duplicate top-end cards.

### 3. Pending choices are part of the teacher

Follow each effect through its player-visible decisions: optional activation,
payment, card selection, material selection, and completion. Prefer retaining
the sole visible evolution bridge when discarding; select useful eligible
retrieval/search cards; evaluate DNA material tradeoffs. Preserve each real
decision as its own training row. No hidden auto-resolution or first-card stub.

Page navigation uses the owned-choice interface. Inspect each permitted page
at most once per unchanged candidate set, remember the best physical candidate,
then rebind its action on the correct current page. Inspection-only rows cannot
be committed. Refresh invalidated menus through the engine's legal action.
Navigation is not hand gain or evolution progress. Detect cycles using visible
semantic state and prompt identity, excluding monotonically increasing page
generation/decision counters that could conceal a cycle. Prefer a legal
progressing alternative or allowed decline; a mandatory menu with no proven
progress must produce a diagnostic failure, not concede or bypass legality.

Teacher preferences inspect only own private cards, public board/trash/counts,
and explicitly revealed cards. Do not clone the true game and look at drawn
cards, opponent hands, hidden security, or future deck order to score actions.

### 4. Mix teachers by seat and game

Default mixture: 50% greedy and 50% development player-game appearances.
Within each four-game block use G/G, D/D, G/D, and D/G, with opening seats
balanced and teacher assignments fixed for each player throughout its game.
Stratify by archetype, use all distinct lists, and emit actual per-archetype,
per-seat, per-teacher counts. Stable game IDs, a dedicated assignment RNG,
deck hashes, and independent game seeds make the schedule reproducible across
worker counts. Teacher assignment must not consume gameplay RNG.

For this 13-archetype pool propose 416 bootstrap games (832 player-game
appearances): 64 appearances per archetype, 32 per teacher. This is a small
bootstrap, not enough evidence to certify archetype strength. For other pool
sizes scale to a multiple of four that provides the same coverage, and report
any integer-balance limitations. Choose matchups/list variants without using
outcomes. Do not let longer development games dominate optimization: sample
teacher/archetype strata with equal configured mass, then games and decision
rows, while reporting actual sampled rows. Use the same sampler in controls.

Alternatives: switching teachers every decision breaks coherent setup lines;
greedy-only is the control; making all demonstrations development-only risks
hoarding, over-evolving, and neglecting attacks. Keep both pure controls in the
comparison instead of assuming a 50/50 mixture is best.

### 5. Training and comparison

Demonstration targets remain legal one-hot teacher actions. Train the same
architecture from fresh random weights; fit the critic on terminal outcomes
with whole-game held-out separation. Never use the heuristic's score as a
value target or an added reward. Exclude every row of capped/crashed games
from both policy and value fitting; preserve their recordings and fail the
qualification run before fitting rather than silently reducing coverage.

Run greedy-only, development-only and 50/50 controls with the same 416-game
deck/seed schedule, initialization seed, fitting recipe, and equal row/update
budget. Report decision-generation cost separately. First compare the three
seed policies; then compare one matched 1,100-game search update per seed with
320 simulations / four worlds / batch four and eight workers.

Keep a frozen greedy-only bootstrap as a common model anchor across arms; do
not give each arm a different opponent when comparing strength. Use the same
5,200-game, seat-balanced evaluation cohort (200 games/archetype/anchor, against
greedy and the common frozen anchor), plus tactical fixtures. Log paired win
rate differences with confidence intervals, taking opening-seat pairs as
correlated units. Pet same-list results remain separate from held-out meta.

Track legal opportunities and choices for each evolution route, DNA only in
decks containing DNA-capable cards, successful and net hand additions by
draw/search/retrieval, filtering, memory passed, attacks, lethal misses, game
length, and truncations. Audit event coverage first; add only missing route/
resource telemetry, without changing tensor/action contracts or rewards.

Qualification gates: zero illegal/hidden-information-dependent choices; all
targeted tactical/progress tests pass; zero unexplained caps/crashes in the
qualification corpus; minimum declared teacher/archetype coverage. Prefer a
mixed seed only if it improves intended opportunity-normalized behaviors and
has no tactical regression. For playing strength use a predeclared 5-percentage-
point aggregate noninferiority margin against greedy-only: require the paired
95% interval's lower bound above -5 points, report per-archetype uncertainty,
and extend evaluation rather than label an inconclusive result as a win.
No automatic model promotion or live deployment.

## Risks / Trade-offs

- Hand hoarding or evolution for its own sake -> cost/body penalties, net-card
  accounting, lethal fixtures, and outcome-based evaluation.
- Another rules implementation inside a bot -> use native legal masks and
  typed current-action metadata; unknown effects receive conservative handling.
- Hidden-information leakage through one-ply simulation -> no true-state
  rollout scoring; hidden-zone permutation tests must preserve choices.
- Misleading mix percentages -> report game and row weights separately, with
  stratified optimization and no claims that 50% games means 50% raw rows.
- Heuristic loops/paging -> bounded browsing, progress signatures, whole-game
  quarantine, and reproducible recordings.
- High card-use counts without stronger play -> tactical checks and common-
  anchor paired evaluations before selecting a future seed.

## Migration Plan

Implement and qualify locally first. Add an explicit teacher-mixture option;
the default remains greedy-only. Freeze a separate corpus/seed/output folder
for every arm. Build a new image only after tests and comparison readiness.
The currently running schema-2 experiment is an untouched baseline; adoption
belongs to a subsequent experiment. Rollback selects the existing greedy
teacher and preserves all comparison artifacts.

## Open Questions

Exact feature weights require the fixture/ablation checks above. Typed benefit
and effective-cost coverage must be inventoried before coding; unsupported
mechanics must be visible in a coverage report, not silently called supported.
The proposed 50/50 ratio is the first comparison, not an established optimum.
