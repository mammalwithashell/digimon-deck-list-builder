//! Shared, memoized card store (the `cache-construction` change).
//!
//! `Game::new` used to rebuild the immutable card store on every game —
//! `all_card_data.clone()` + DSL alt-path enrichment + a per-card `Vec` clone +
//! the `card_id_index` + global token absorption (~14 ms/game, 61% of the
//! per-step training cost). That work is a **pure function of `all_card_data`**
//! (the token registry is deck-independent, and the binding passes a stable
//! `&'static` card DB), so it is memoized here and shared across games via `Arc`:
//! `Game::new` becomes an `Arc` clone + deck deal.
//!
//! `card_data` is never mutated after construction (tokens are absorbed at build
//! time), so one `Arc<Vec<CardData>>` is safely shared across concurrent `Game`s.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::card_data::CardData;

/// The immutable, enriched card store derived from `all_card_data`. Memoized and
/// shared; built once per distinct DB fingerprint.
pub struct SharedCardStore {
    /// Enriched flat store: the DB's cards (in `all_card_data` iteration order)
    /// followed by the absorbed global token rows. Indexed by `data_index`.
    pub data: Arc<Vec<CardData>>,
    /// `card_id -> data_index` for the DB cards (NOT tokens — matching the prior
    /// `data_index_map`, which token absorption deliberately did not extend).
    pub index: Arc<HashMap<String, usize>>,
    /// Compiled DSL alt-paths keyed by card_id (only the DSL-loader build has it).
    #[cfg(feature = "dsl-yaml-loader")]
    pub alt_paths: Arc<HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>>>,
}

/// `Deref`-transparent handle to the shared card-data vec. `Game.card_data` is a
/// `CardStore` so the workspace's ~3126 `.card_data` reads (indexing, `.iter()`,
/// `.len()`, `&card_data` as `&[CardData]`/`&Vec<CardData>`) keep working through
/// `Deref`, while a clone is an `Arc` refcount bump rather than a ~4000-card copy.
#[derive(Clone)]
pub struct CardStore(pub Arc<Vec<CardData>>);

impl std::ops::Deref for CardStore {
    type Target = Vec<CardData>;
    fn deref(&self) -> &Vec<CardData> {
        &self.0
    }
}

impl std::ops::DerefMut for CardStore {
    /// Mutating the store **copy-on-writes** off the shared `Arc` (via
    /// `Arc::make_mut`) so a per-game edit (test card injection, `force_base_dp`,
    /// DebugRunner-staged cards) gives this game a private store and never
    /// touches the shared cache. Production play never mutates `card_data`, so it
    /// never forks — it keeps the shared `Arc`.
    fn deref_mut(&mut self) -> &mut Vec<CardData> {
        std::sync::Arc::make_mut(&mut self.0)
    }
}

impl std::fmt::Debug for CardStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardStore")
            .field("cards", &self.0.len())
            .finish()
    }
}

fn cache() -> &'static Mutex<HashMap<u64, Arc<SharedCardStore>>> {
    static C: OnceLock<Mutex<HashMap<u64, Arc<SharedCardStore>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Order-independent content fingerprint of `all_card_data`. A wrapping-sum of
/// per-card hashes is commutative (so `HashMap` iteration order is irrelevant) and
/// hashes card *content* (not just ids), so an override change re-keys. O(cards)
/// (~300–500 µs) — ~30× cheaper than the ~14 ms build it elides.
fn fingerprint(all_card_data: &HashMap<String, CardData>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut sum: u64 = all_card_data.len() as u64;
    for (id, d) in all_card_data {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        // Hash the FULL discriminating content (values, not lengths). Two card
        // sets that differ in ANY behavioral field must produce different
        // fingerprints — e.g. test cards built by `make_digimon_dp("WEAK", c, dp)`
        // share id/name/text but differ in `dp`, so `dp` (and every other field)
        // MUST be hashed, or a cache hit would serve the wrong card's stats.
        // String/Vec hashing reads bytes in place (no allocation); only the few
        // non-`Hash` structural fields go through one `Debug` per card.
        id.hash(&mut h);
        d.card_name.hash(&mut h);
        d.card_kind.hash(&mut h);
        d.level.hash(&mut h);
        d.dp.hash(&mut h);
        d.play_cost.hash(&mut h);
        d.index.hash(&mut h);
        d.colors.hash(&mut h);
        d.traits.hash(&mut h);
        d.keywords.hash(&mut h);
        d.effect_text.hash(&mut h);
        d.inherited_text.hash(&mut h);
        d.security_text.hash(&mut h);
        d.effect_class_name.hash(&mut h);
        d.ace_overflow.hash(&mut h);
        d.digixros_aliases.hash(&mut h);
        d.also_treated_as.hash(&mut h);
        format!("{:?}", (&d.evo_costs, &d.dna_costs, &d.dual)).hash(&mut h);
        sum = sum.wrapping_add(h.finish());
    }
    sum
}

/// Memoized accessor: returns the shared enriched store for `all_card_data`,
/// building (and caching) it on the first call for a given DB fingerprint.
pub fn shared_card_store(all_card_data: &HashMap<String, CardData>) -> Arc<SharedCardStore> {
    let fp = fingerprint(all_card_data);
    if let Some(existing) = cache().lock().unwrap().get(&fp).cloned() {
        return existing;
    }
    let built = Arc::new(build_shared_card_store(all_card_data));
    cache().lock().unwrap().insert(fp, built.clone());
    built
}

/// The reference build — the exact clone+enrich+index+token-absorb sequence
/// previously inline in `Game::new`. The cache-miss path; also the oracle baseline.
pub fn build_shared_card_store(all_card_data: &HashMap<String, CardData>) -> SharedCardStore {
    #[allow(unused_mut)]
    let mut effective_card_data = all_card_data.clone();

    #[cfg(feature = "dsl-yaml-loader")]
    let mut alt_path_registry: HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>> =
        HashMap::new();
    #[cfg(feature = "dsl-yaml-loader")]
    if let Ok(dsl_registry) = crate::dsl_registry::from_embedded_cached() {
        crate::dsl_bridge::enrich_card_data_with_dsl_alt_paths(&mut effective_card_data, dsl_registry);
        alt_path_registry = dsl_registry
            .iter()
            .filter_map(|(card_id, compiled)| {
                if compiled.alt_paths.is_empty() {
                    None
                } else {
                    Some((card_id.clone(), compiled.alt_paths.clone()))
                }
            })
            .collect();
    }

    // Assign `data_index` in a DETERMINISTIC order (sorted by card_id), NOT
    // `HashMap` iteration order. The store is memoized and shared across games, so
    // the assignment must not depend on a particular `HashMap` instance's seed —
    // otherwise a cache hit would return a different assignment than a caller that
    // independently re-derives one (e.g. `DebugRunner`, which sorts identically).
    // `data_index` is internal (the obs tensor keys on the stable registry index),
    // so this is behavior-neutral.
    let mut sorted_ids: Vec<&String> = effective_card_data.keys().collect();
    sorted_ids.sort();
    let mut data: Vec<CardData> = Vec::with_capacity(effective_card_data.len());
    let mut index: HashMap<String, usize> = HashMap::with_capacity(effective_card_data.len());
    for card_id in sorted_ids {
        let idx = data.len();
        index.insert(card_id.clone(), idx);
        data.push(effective_card_data[card_id].clone());
    }

    // Absorb the global (deck-independent) token rows — appended to `data` only,
    // NOT to `index` (matching the prior in-`Game::new` behavior). See
    // `EffectContext::play_token` for how token rows are resolved.
    let token_registry = crate::token_registry::build_registry();
    for def in token_registry.iter() {
        data.push(def.to_card_data());
    }

    SharedCardStore {
        data: Arc::new(data),
        index: Arc::new(index),
        #[cfg(feature = "dsl-yaml-loader")]
        alt_paths: Arc::new(alt_path_registry),
    }
}
