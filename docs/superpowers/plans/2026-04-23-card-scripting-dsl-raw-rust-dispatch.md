# Card Scripting DSL — raw_rust Dispatch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Ship the `raw_rust` escape hatch per spec §6 — a pluggable fn registry on the engine side that lets card scripts delegate to hand-written Rust for mechanics the DSL can't express. Wire step-level (`CompiledStep::RawRust`) and whole-clause (`CompiledDeclarativeClause::RawRust`) dispatch.

**Architecture:** New trait `RawRustRegistry` in `digimon-dsl/src/raw_rust_registry.rs` already ships (Phase 0). Engine side adds `digimon-engine/src/dsl_cards/raw_rust.rs` housing the concrete `EngineRawRustRegistry` with a `HashMap<String, Arc<dyn RawRustFn>>` of registered Rust fns. `DslCardEffect` grows a reference to the registry; `run_step` dispatches `CompiledStep::RawRust { fn_name }` via the registry; `effects()` dispatches `CompiledDeclarativeClause::RawRust { fn_name, triggers }` similarly.

**Tech Stack:** Rust 2021, `digimon-engine`, `digimon-dsl` leaf crate.

**Scope:**
- Two fn traits: `RawStepFn: Fn(&mut EffectContext)` and `RawDeclarativeFn: Fn(CardHandle) -> Vec<Effect>`.
- Engine-side registry struct + registration API.
- One concrete registered fn per trait as proof of concept:
  - Step-level: `ad1_025_on_play_process` — stub that does nothing (real AD1-025 Omnimon logic lives elsewhere).
  - Whole-clause: `bt10_111_arm_digixros_wildcard_for_turn` — stub that sets a turn-scoped flag (no-op in Phase 1).
- `DslCardEffect` accepts `Option<&EngineRawRustRegistry>` (backward-compatible: None means raw_rust steps/clauses no-op).
- Wire registry lookup into `register_dsl_cards` so all DSL cards share one engine-side registry.

**Non-goals:**
- Complete card-specific raw_rust implementations — those land per-archetype in the DSL card-migration skill (Phase 4).
- Step-level `consumes` / `binds` name resolution beyond a stub (Phase 4 when bindings carry actual values for all step types).

---

## File structure

- Create: `digimon-engine/src/dsl_cards/raw_rust.rs`
- Modify: `digimon-engine/src/dsl_cards/mod.rs` (registry plumbing + dispatch arms)
- Create: `digimon-engine/tests/dsl/raw_rust.rs`
- Modify: `digimon-engine/tests/dsl/main.rs`
- Modify: `digimon-engine/src/cards.rs` (pass registry through `register_dsl_cards` call)

---

## Task 1: Registry scaffold + two fn-trait aliases

**Files:** `raw_rust.rs`, `tests/dsl/raw_rust.rs`, `tests/dsl/main.rs`

- [ ] **Step 1: Failing test**

Create `digimon-engine/tests/dsl/raw_rust.rs`:

```rust
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;

#[test]
fn empty_registry_reports_no_fns() {
    let r = EngineRawRustRegistry::new();
    assert!(r.step_fn("anything").is_none());
    assert!(r.declarative_fn("anything").is_none());
}

#[test]
fn register_and_lookup_step_fn() {
    let mut r = EngineRawRustRegistry::new();
    r.register_step("noop_step", |_ctx| {});
    assert!(r.step_fn("noop_step").is_some());
    assert!(r.step_fn("missing").is_none());
}

#[test]
fn register_and_lookup_declarative_fn() {
    use digimon_engine::card_source::CardHandle;
    let mut r = EngineRawRustRegistry::new();
    r.register_declarative("noop_decl", |_card: CardHandle| Vec::new());
    assert!(r.declarative_fn("noop_decl").is_some());
}
```

Add `mod raw_rust;` to `tests/dsl/main.rs`.

- [ ] **Step 2: Run — FAIL**

- [ ] **Step 3: Implement**

Create `digimon-engine/src/dsl_cards/raw_rust.rs`:

```rust
//! Engine-side raw_rust dispatch registry. Holds two maps of Arc-wrapped
//! closures: step-level (`fn(&mut EffectContext)`) and whole-clause
//! declarative (`fn(CardHandle) -> Vec<Effect>`). Card scripts reference
//! entries by string name; unregistered names become no-ops.

use std::collections::HashMap;
use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::effect::Effect;
use crate::effect_context::EffectContext;

pub type RawStepFn = Arc<dyn Fn(&mut EffectContext) + Send + Sync + 'static>;
pub type RawDeclarativeFn = Arc<dyn Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static>;

#[derive(Default)]
pub struct EngineRawRustRegistry {
    steps: HashMap<String, RawStepFn>,
    declaratives: HashMap<String, RawDeclarativeFn>,
}

impl EngineRawRustRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register_step<F>(&mut self, name: &str, f: F)
    where
        F: Fn(&mut EffectContext) + Send + Sync + 'static,
    {
        self.steps.insert(name.to_string(), Arc::new(f));
    }

    pub fn register_declarative<F>(&mut self, name: &str, f: F)
    where
        F: Fn(CardHandle) -> Vec<Effect> + Send + Sync + 'static,
    {
        self.declaratives.insert(name.to_string(), Arc::new(f));
    }

    pub fn step_fn(&self, name: &str) -> Option<RawStepFn> {
        self.steps.get(name).cloned()
    }

    pub fn declarative_fn(&self, name: &str) -> Option<RawDeclarativeFn> {
        self.declaratives.get(name).cloned()
    }
}

impl std::fmt::Debug for EngineRawRustRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineRawRustRegistry")
            .field("steps", &self.steps.len())
            .field("declaratives", &self.declaratives.len())
            .finish()
    }
}
```

Add `pub mod raw_rust;` to `digimon-engine/src/dsl_cards/mod.rs`.

- [ ] **Step 4: Commit**

```
git add digimon-engine/src/dsl_cards/raw_rust.rs digimon-engine/src/dsl_cards/mod.rs digimon-engine/tests/dsl/raw_rust.rs digimon-engine/tests/dsl/main.rs
git commit -m "dsl: raw_rust dispatch registry scaffold (step + declarative fn maps)"
```

---

## Task 2: Plumb registry through `DslCardEffect` + `register_dsl_cards`

**Files:** `dsl_cards/mod.rs`, `cards.rs`, `tests/dsl/raw_rust.rs`

- [ ] **Step 1: Test**

Append:

```rust
use digimon_dsl::compiled::{CompiledCard, CompiledCardKind};
use digimon_engine::dsl_cards::DslCardEffect;
use std::sync::Arc;

#[test]
fn dsl_card_effect_accepts_raw_registry_and_stores_arc() {
    let mut reg = EngineRawRustRegistry::new();
    reg.register_step("noop", |_| {});
    let reg = Arc::new(reg);

    let compiled = CompiledCard {
        card: "F".into(), name: "F".into(), kind: CompiledCardKind::Digimon,
        level: None, color: vec![], cost: None, dp: None, traits: vec![],
        form: None, attribute: None, ace_overflow: None,
        identity: None, alt_paths: vec![], effects: vec![],
    };
    let dsl = DslCardEffect::with_raw_registry(Arc::new(compiled), reg.clone());
    // Sanity: lookup works via the adapter's accessor.
    assert!(dsl.raw_registry().and_then(|r| r.step_fn("noop")).is_some());
}
```

- [ ] **Step 2: Implement**

In `digimon-engine/src/dsl_cards/mod.rs`:

```rust
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;

pub struct DslCardEffect {
    compiled: Arc<CompiledCard>,
    raw: Option<Arc<EngineRawRustRegistry>>,
}

impl DslCardEffect {
    pub fn new(compiled: Arc<CompiledCard>) -> Self {
        Self { compiled, raw: None }
    }

    pub fn with_raw_registry(
        compiled: Arc<CompiledCard>,
        registry: Arc<EngineRawRustRegistry>,
    ) -> Self {
        Self { compiled, raw: Some(registry) }
    }

    pub fn compiled(&self) -> &CompiledCard { &self.compiled }

    pub fn raw_registry(&self) -> Option<&EngineRawRustRegistry> {
        self.raw.as_deref()
    }

    pub fn ace_overflow(&self) -> Option<i32> { self.compiled.ace_overflow }
}
```

Extend `register_dsl_cards` to accept an optional `Arc<EngineRawRustRegistry>` and pick `with_raw_registry` when present:

```rust
pub fn register_dsl_cards_with_raw(
    effect_registry: &mut CardEffectRegistry,
    dsl_registry: &DslCardRegistry,
    raw: Option<Arc<EngineRawRustRegistry>>,
) {
    for (card_id, compiled) in dsl_registry.iter() {
        let arc = Arc::new(compiled.clone());
        let dsl_effect: Arc<dyn CardEffect> = match &raw {
            Some(r) => Arc::new(DslCardEffect::with_raw_registry(arc, r.clone())),
            None => Arc::new(DslCardEffect::new(arc)),
        };
        effect_registry.insert(card_id, dsl_effect);
    }
}

// Keep the existing register_dsl_cards as a thin wrapper for back-compat.
pub fn register_dsl_cards(
    effect_registry: &mut CardEffectRegistry,
    dsl_registry: &DslCardRegistry,
) {
    register_dsl_cards_with_raw(effect_registry, dsl_registry, None);
}
```

Modify `digimon-engine/src/cards.rs::build_registry()` — call the `_with_raw` variant and pass a freshly-built `EngineRawRustRegistry` (with one sample fn registered to exercise the path):

```rust
#[cfg(feature = "dsl-yaml-loader")]
{
    use std::sync::Arc;
    let mut raw = crate::dsl_cards::raw_rust::EngineRawRustRegistry::new();
    // Sample registrations — cards reference these by name in YAML.
    raw.register_step("ad1_025_on_play_process", |_ctx| { /* real impl TBD */ });
    raw.register_declarative("bt10_111_arm_digixros_wildcard_for_turn", |_card| Vec::new());

    match crate::dsl_registry::from_embedded() {
        Ok(pack) => crate::dsl_cards::register_dsl_cards_with_raw(
            &mut registry, &pack, Some(Arc::new(raw),
        )),
        Err(e) => eprintln!("DSL embedded pack failed to load: {e}"),
    }
}
```

- [ ] **Step 3: Commit**

```
git commit -m "dsl: plumb EngineRawRustRegistry through DslCardEffect + build_registry"
```

---

## Task 3: Dispatch `CompiledStep::RawRust`

**Files:** `dsl_cards/step/mod.rs`, new `step/raw.rs`, tests

- [ ] **Step 1: Test**

Append to `tests/dsl/raw_rust.rs`:

```rust
#[test]
fn raw_rust_step_invokes_registered_fn() {
    use digimon_dsl::compiled::CompiledStep;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::dsl_cards::bindings::Bindings;
    use digimon_engine::effect_context::EffectContext;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc as StdArc;

    static CALLED: AtomicBool = AtomicBool::new(false);
    CALLED.store(false, Ordering::SeqCst);

    let mut reg = EngineRawRustRegistry::new();
    reg.register_step("marker", |_ctx| { CALLED.store(true, Ordering::SeqCst); });
    let reg = StdArc::new(reg);

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
    let mut bindings = Bindings::new();

    digimon_engine::dsl_cards::step::run_step_with_raw(
        &CompiledStep::RawRust {
            fn_name: "marker".into(),
            consumes: vec![],
            binds: vec![],
        },
        &mut ctx,
        &mut bindings,
        Some(&reg),
    );
    assert!(CALLED.load(Ordering::SeqCst));
}
```

- [ ] **Step 2: Implement**

Add a `run_step_with_raw` variant in `step/mod.rs`:

```rust
pub fn run_step_with_raw(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
    raw: Option<&crate::dsl_cards::raw_rust::EngineRawRustRegistry>,
) {
    if let CompiledStep::RawRust { fn_name, .. } = step {
        if let Some(r) = raw {
            if let Some(f) = r.step_fn(fn_name) {
                f(ctx);
            }
        }
        return;
    }
    run_step(step, ctx, bindings);
}
```

Update `lower_triggered.rs` to pass the adapter's `raw_registry()` into the closure — requires plumbing the `Option<Arc<EngineRawRustRegistry>>` into the `process` closure via `clone`. Sketch:

```rust
let raw = dsl.raw_registry().cloned(); // Arc<...> clone, cheap
builder = builder.process(move |ctx| {
    let mut bindings = Bindings::new();
    for step in process_steps.iter() {
        run_step_with_raw(step, ctx, &mut bindings, raw.as_deref());
    }
});
```

**Note:** `lower_triggered::lower` currently takes `&CompiledTriggeredClause` — grow its signature to accept the raw registry (`fn lower(card, clause, raw: Option<Arc<EngineRawRustRegistry>>)`) and propagate from `DslCardEffect::effects()`.

- [ ] **Step 3: Commit**

```
git commit -m "dsl: dispatch CompiledStep::RawRust through EngineRawRustRegistry"
```

---

## Task 4: Dispatch `CompiledDeclarativeClause::RawRust`

**Files:** `dsl_cards/mod.rs`, `lower_raw_rust.rs` (new), tests

- [ ] **Step 1: Test**

Append:

```rust
#[test]
fn raw_rust_declarative_clause_returns_fn_produced_effects() {
    use digimon_dsl::compiled::{CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming};
    use digimon_engine::effect::Effect;
    use digimon_engine::card_source::CardHandle;

    let mut reg = EngineRawRustRegistry::new();
    reg.register_declarative("emits_onplay", |card: CardHandle| {
        vec![Effect::on_play(card).name("raw").process(|_| {}).build()]
    });
    let reg = std::sync::Arc::new(reg);

    let compiled = CompiledCard {
        card: "F".into(), name: "F".into(), kind: CompiledCardKind::Digimon,
        level: None, color: vec![], cost: None, dp: None, traits: vec![],
        form: None, attribute: None, ace_overflow: None,
        identity: None, alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
            fn_name: "emits_onplay".into(),
            triggers: vec![CompiledTiming::OnPlay],
            scope: CompiledScope::FaceUp,
            summary: None, summary_key: None,
        })],
    };
    let dsl = DslCardEffect::with_raw_registry(Arc::new(compiled), reg);
    let effects = <DslCardEffect as digimon_engine::effect::CardEffect>::effects(&dsl, CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, digimon_engine::enums::EffectTiming::OnPlay);
}
```

- [ ] **Step 2: Implement dispatch arm**

In `DslCardEffect::effects()` match:

```rust
CompiledDeclarativeClause::RawRust { fn_name, .. } => {
    if let Some(r) = &self.raw {
        if let Some(f) = r.declarative_fn(fn_name) {
            out.extend(f(card));
        }
    }
}
```

- [ ] **Step 3: Commit**

```
git commit -m "dsl: dispatch CompiledDeclarativeClause::RawRust through registry"
```

---

## Task 5: Exit test — 3% budget logger

Per spec §6.5, emit a load-time log line with the raw_rust fn count / card count ratio so the migration tooling can track the budget.

Add to `build_registry()`:
```rust
let ratio = (raw_count as f32) / (card_count as f32);
if ratio > 0.03 {
    eprintln!("WARNING: raw_rust budget exceeded: {raw_count} fns for {card_count} cards ({:.1}%)", ratio * 100.0);
}
```

Ship with a simple test counting ratio behavior for a fake registry.

```
git commit -m "dsl: raw_rust 3% budget logger"
```

---

## Self-Review

**Spec coverage (§6):**
- Registry trait & implementation — Task 1
- Step-level raw_rust — Task 3
- Whole-clause raw_rust — Task 4
- 3% budget signal — Task 5

**Deferrals:** per-card raw_rust implementations land per-archetype in Phase 4 under the `batch-implement-cards-rust-dsl` skill (separate plan).

**Type consistency:** `EngineRawRustRegistry::{register_step, register_declarative, step_fn, declarative_fn}`, `DslCardEffect::{new, with_raw_registry, raw_registry}`, `register_dsl_cards_with_raw(registry, pack, Option<Arc<EngineRawRustRegistry>>)`.
