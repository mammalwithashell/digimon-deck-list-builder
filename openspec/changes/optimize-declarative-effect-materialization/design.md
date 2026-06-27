## Context

`Game::decode_action` calls `tick_declarative_effects()` 2-3× per action
(`decode.rs:46` pre-action, `:49` post-selection, `:86` post-action). Each call:

```
clear_materialized_declaratives()                 // wipe all declarative modifiers
for each player, each battle-area permanent + each inherited stack source
        + breeding + each face-up security card:
    sources.push(card_id.to_string(), handle, ...)   // String heap alloc per card
for (card_id, ...) in sources:
    for effect in effects_for_card(card_id):
        if effect.declarative && materializes:
            run condition closure; run process closure   // re-install the modifier
```

So every action throws away and rebuilds the whole board's static-effect state ~2×.
Measured: 73-92% of engine time, 1.4-3.5 ms/step, O(board). Most actions don't
change which declarative sources exist, so most of this rebuild is wasted.

## Goals / Non-Goals

**Goals:**
- Cut engine steps/sec time spent in `tick_declarative_effects` substantially
  (target ≥2× engine steps/sec on `bench_engine_throughput.rs`, stretch ≥5×).
- **Byte-identical behavior:** the materialized modifier state, every game outcome,
  and every behavioral/card/archetype/parity test result are unchanged.
- A correctness oracle that proves the optimized path equals the always-rebuild
  baseline across the test corpus.

**Non-Goals:**
- No DSL vocabulary, card-script, tensor, or action-space change.
- Harness levers (batched inference, leaner tensor), multi-threading, per-game
  registry construction caching — out of scope.
- A full dependency-tracking effect system — start conservative; that is a later
  option if the conservative dirty-flag leaves wins on the table.

## Decisions

1. **Land the obviously-correct cheap wins first (no behavior risk):**
   - **Remove the per-card `String` alloc.** The `sources` Vec collects owned
     `String`s precisely to break a borrow (it's built from `&self.players`, then
     iterated while `process` closures take `&mut self`). Replace the `String` with
     a **`Copy` interned card id / registry index** (or the existing card-registry
     integer index) so the Vec holds `Copy` ids — no heap alloc, same borrow-break.
   - **Collapse the 2-3× tick to 1× per action.** Verify the pre-action tick
     (`decode.rs:46`) is redundant with the post-action tick for normal actions
     (it is needed only if an action's own resolution reads materialized state
     mid-resolution); keep the minimum set that preserves identical behavior under
     the oracle.

2. **Dirty-flag the declarative state (the big win).** Add a
   `declaratives_dirty` flag (on `Game` or `ModifierRegistry`).
   `tick_declarative_effects` returns early when not dirty (state already current);
   otherwise it does the full clear+rebuild and clears the flag. Game mutations
   that can change the declarative-relevant inputs **set** the flag.

3. **Conservative-but-correct invalidation, proven by an oracle.** Enumerating
   *exactly* which mutations matter is error-prone (conditions can read dynamic
   state — turn/phase, memory, suspended, DP, counts). So:
   - Set the flag at a **curated, deliberately broad** set of chokepoints: any
     battle-area / stack / breeding / face-up-security change, plus turn/phase
     transitions and the dynamic inputs declarative conditions read (memory,
     suspend, DP, counts). Broad invalidation still skips the many no-op /
     pure-selection / unchanged-board actions that dominate.
   - **Correctness oracle:** under `cfg(debug_assertions)` (or a test feature),
     each tick runs BOTH paths — the dirty-flag fast path AND a fresh full rebuild
     — and `debug_assert!`s the resulting materialized modifier sets are identical.
     Run the full behavioral/card/archetype/parity suites in this mode in CI. A
     missed invalidation site fails a test loudly; release builds run only the fast
     path. This makes the conservative approach *safe to ship by construction of
     the test gate*.

4. **Keep the always-rebuild path as the reference.** It stays in the code as the
   oracle's baseline and a fallback (a single config/env can force it), so we can
   A/B correctness and perf and bisect any divergence.

5. **Measure on the committed benchmark.** Re-run `bench_engine_throughput.rs`
   (greedy + random, per-phase breakdown) before/after each lever; record the
   engine-step steps/sec delta. Ship only if the behavioral suites are green under
   the oracle.

## Risks / Trade-offs

- **Missed invalidation → stale modifier (correctness bug).** Primary risk.
  Mitigation: the broad conservative chokepoint set + the per-tick differential
  oracle run across the whole corpus; the always-rebuild reference stays available.
- **Dynamic-condition declaratives.** A declarative whose condition reads dynamic
  state (turn/memory/counts) must re-materialize when that state changes. Handled
  by invalidating on those inputs (conservative) — and, if profiling shows those
  invalidations dominate, by later classifying static- vs dynamic-condition
  declaratives so only the dynamic ones re-eval on those triggers.
- **Smaller-than-hoped win.** If real games change the board most actions, the
  dirty-flag skips less. The benchmark quantifies this per lever; the cheap wins
  (String alloc, 1× tick) bank value regardless.
- **Borrow-checker friction** removing the `String` — interned `Copy` ids resolve
  it without reintroducing allocation.
- **Alignment upside:** incremental, allocation-light, byte-identical materialization
  is exactly what the cloneable-engine / DSL data-VM roadmap needs for cheap
  tree-search clones — so this is a down-payment, not a detour.
