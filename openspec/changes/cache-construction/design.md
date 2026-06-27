## Context

`Game::new` ([game/setup.rs]) rebuilds the immutable card store on every game:
`all_card_data.clone()` → `enrich_card_data_with_dsl_alt_paths` → `card_data_store:
Vec<CardData>` (per-card clone) + `card_id_index` + global token absorption. Profile:
~14 ms/game, 61% of the per-step training cost. The result is a pure function of
`all_card_data` (the global token registry is deck-independent), and the binding
passes a stable `&'static` card DB, so it is identical for every game in a process.

## Goals / Non-Goals

**Goals**
- Collapse `Game::new`'s card-store cost from a clone+enrich to an `Arc` clone;
  target a material construction speedup on `bench_engine_throughput.rs`.
- Byte-identical store + no behavioral change; `Game: Send` preserved.

**Non-Goals**
- No DSL/card-script/tensor/action change. No change to how cards are *authored* or
  enriched — only *when* (once, cached) and how *shared* (`Arc`).

## Decisions

1. **`CardStore` newtype, `Deref`-transparent.** `pub struct CardStore(Arc<Vec<CardData>>)`
   with `Deref<Target=Vec<CardData>>` + `Clone` (= `Arc` clone). `Game.card_data:
   CardStore`. This keeps the ~3126 `.card_data` accesses (indexing, `.iter()`,
   `.len()`, `.get()`, `&card_data` as `&[CardData]`/`&Vec<CardData>`) working via
   `Deref`/`Deref` coercion; the compiler flags the few breakers (explicit `Vec`
   bindings, code expecting `.clone()` to deep-copy). `card_id_index` →
   `Arc<HashMap<String, usize>>`. `alt_path_registry` (cfg `dsl-yaml-loader`) → `Arc`.

2. **Process-global memo keyed by a content fingerprint of `all_card_data`.** A
   single-or-small-entry `Mutex<…>` cache holds `Arc<SharedCardStore>`. The
   fingerprint is an **order-independent** content hash (len + wrapping-sum of
   per-card `hash(card_id, key fields)`) — O(cards) (~300–500 µs) but ~30× cheaper
   than the 14 ms it elides, and robust (different DB/overrides → different
   fingerprint → rebuild). Tests with small synthetic DBs simply miss and rebuild
   (correct; they don't need the speedup).

3. **`Game::new` flow:** fingerprint `all_card_data` → cache hit ⇒ `Arc`-clone the
   `SharedCardStore`; miss ⇒ build once (the current clone+enrich+token path),
   insert, clone. Wrap the shared `Arc<Vec<CardData>>` in `CardStore` for the field.

4. **Immutability is the invariant.** `Game.card_data` is never mutated
   post-construction (tokens are absorbed at build time; the only `.card_data.insert`
   is `DebugRunner`'s separate `HashMap`). So sharing one `Arc<Vec<CardData>>` across
   concurrent games (each its own `Game`, single-threaded under the GIL) is sound.

5. **Reference build stays.** Keep the per-game build path (the cache miss branch IS
   it) so the cache is verifiable against it; a debug assertion can compare the
   shared store's fingerprint to a fresh build's on miss.

## Risks / Trade-offs

- **Fingerprint incompleteness → wrong shared store.** Mitigated by hashing card
  *content* (not just ids) + the per-game build remaining the miss path + the
  behavioral suites (a wrong store would diverge games).
- **`.card_data` ripple** across 4 crates (~3126 sites). Mitigated by the `Deref`
  newtype; the field change is atomic (no partial compile), so it lands in one
  sweep guided by the compiler, then full re-verification.
- **`Send`.** Cache value is `Arc<…>` (Send+Sync) and the global cache is a
  `Mutex` (Send+Sync); `Game` stays `Send`. Verify `digimon-engine-py` compiles.
- **Memory.** One shared store per distinct DB fingerprint (effectively one in a
  training process), freed with the process. Strictly less than today (was one
  full copy per live `Game`).
