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

#[cfg(feature = "dsl-yaml-loader")]
use crate::enums::Keyword as DiagnosticKeyword;
#[cfg(feature = "dsl-yaml-loader")]
mod diagnostic_wire;
#[cfg(feature = "dsl-yaml-loader")]
mod scoped_diagnostic;
#[cfg(feature = "dsl-yaml-loader")]
pub use scoped_diagnostic::ScopedDiagnosticStore;

/// Pre-parsed printed keyword slices for one `CardData` row, avoiding repeated
/// English text parsing on runtime `has_keyword` / declarative queries.
#[derive(Debug, Clone)]
pub struct ParsedCardKeywords {
    pub face: Vec<crate::enums::Keyword>,
    pub inherited: Vec<crate::enums::Keyword>,
    pub link: Vec<crate::enums::Keyword>,
}

impl ParsedCardKeywords {
    pub(crate) fn from_card_data(cd: &CardData) -> Self {
        Self {
            face: crate::game::face_keywords(cd),
            inherited: crate::game::inherited_keywords(cd),
            link: crate::game::link_keywords(cd),
        }
    }
}

/// Two-layer copy-on-write registry of compiled DSL alternate digivolution
/// paths keyed by result card id.
///
/// The immutable embedded-pack map (~2,000 entries) lives in `base` behind a
/// single process-global `Arc`, while per-game test overrides (`DebugRunner`)
/// land in the tiny `overrides` map. This makes `DebugRunner::start()` O(1)
/// instead of cloning ~2,000 `Vec<CompiledAltPath>` trees on every test.
#[cfg(feature = "dsl-yaml-loader")]
#[derive(Debug, Default, Clone)]
pub(crate) struct AltPathRegistry {
    base: Option<Arc<HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>>>>,
    overrides: Arc<HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>>>,
}

#[cfg(feature = "dsl-yaml-loader")]
impl AltPathRegistry {
    pub(crate) fn from_base(
        base: Arc<HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>>>,
    ) -> Self {
        Self {
            base: Some(base),
            overrides: Arc::default(),
        }
    }

    #[inline]
    pub(crate) fn get(&self, card_id: &str) -> Option<&Vec<digimon_dsl::compiled::CompiledAltPath>> {
        if !self.overrides.is_empty() {
            if let Some(paths) = self.overrides.get(card_id) {
                return Some(paths);
            }
        }
        self.base.as_deref().and_then(|base| base.get(card_id))
    }

    pub(crate) fn extend_overrides<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, Vec<digimon_dsl::compiled::CompiledAltPath>)>,
    {
        let mut iter = iter.into_iter();
        let Some((first_k, first_v)) = iter.next() else {
            return;
        };
        let map = Arc::make_mut(&mut self.overrides);
        map.insert(first_k, first_v);
        map.extend(iter);
    }
}

/// The immutable, enriched card store derived from `all_card_data`. Memoized and
/// shared; reused only for identical original metadata inputs.
pub struct SharedCardStore {
    /// Enriched flat store: the DB's cards (in `all_card_data` iteration order)
    /// followed by the absorbed global token rows. Indexed by `data_index`.
    pub data: Arc<Vec<CardData>>,
    /// Pre-parsed printed keywords parallel to `data`.
    pub keywords: Arc<Vec<ParsedCardKeywords>>,
    /// `card_id -> data_index` for DB cards and absorbed global token rows.
    pub index: Arc<HashMap<String, usize>>,
    /// Compiled DSL alt-paths keyed by card_id (only the DSL-loader build has it).
    #[cfg(feature = "dsl-yaml-loader")]
    pub(crate) alt_paths: AltPathRegistry,
}

#[derive(Clone)]
pub(crate) struct CachedCardKeywords {
    /// Retains an `Arc` to the exact `Vec<CardData>` these keywords were parsed
    /// from. Because this keeps the original `Arc`'s strong count ≥ 2, ANY
    /// mutation via `Arc::make_mut(&mut card_store.0)` or `DerefMut` forks a
    /// new allocation, making `Arc::ptr_eq(&self.0, &kw.origin)` return false.
    origin: Arc<Vec<CardData>>,
    entries: Arc<Vec<ParsedCardKeywords>>,
}

/// `Deref`-transparent handle to the shared card-data vec. `Game.card_data` is a
/// `CardStore` so the workspace's ~3126 `.card_data` reads (indexing, `.iter()`,
/// `.len()`, `&card_data` as `&[CardData]`/`&Vec<CardData>`) keep working through
/// `Deref`, while a clone is an `Arc` refcount bump rather than a ~4000-card copy.
#[derive(Clone)]
pub struct CardStore(pub Arc<Vec<CardData>>, pub(crate) Option<CachedCardKeywords>);

impl CardStore {
    pub(crate) fn from_shared(shared: &SharedCardStore) -> Self {
        Self(
            shared.data.clone(),
            Some(CachedCardKeywords {
                origin: shared.data.clone(),
                entries: shared.keywords.clone(),
            }),
        )
    }

    #[inline]
    fn cached_entry(&self, data_index: usize) -> Option<&ParsedCardKeywords> {
        let kw = self.1.as_ref()?;
        if Arc::ptr_eq(&self.0, &kw.origin) {
            kw.entries.get(data_index)
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn face_keywords(
        &self,
        data_index: usize,
    ) -> std::borrow::Cow<'_, [crate::enums::Keyword]> {
        if let Some(entry) = self.cached_entry(data_index) {
            std::borrow::Cow::Borrowed(&entry.face)
        } else {
            std::borrow::Cow::Owned(crate::game::face_keywords(&self.0[data_index]))
        }
    }

    #[inline]
    pub(crate) fn inherited_keywords(
        &self,
        data_index: usize,
    ) -> std::borrow::Cow<'_, [crate::enums::Keyword]> {
        if let Some(entry) = self.cached_entry(data_index) {
            std::borrow::Cow::Borrowed(&entry.inherited)
        } else {
            std::borrow::Cow::Owned(crate::game::inherited_keywords(&self.0[data_index]))
        }
    }

    #[inline]
    pub(crate) fn link_keywords(
        &self,
        data_index: usize,
    ) -> std::borrow::Cow<'_, [crate::enums::Keyword]> {
        if let Some(entry) = self.cached_entry(data_index) {
            std::borrow::Cow::Borrowed(&entry.link)
        } else {
            std::borrow::Cow::Owned(crate::game::link_keywords(&self.0[data_index]))
        }
    }
}

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
        self.1 = None;
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

struct CachedStore {
    // Compare the original input, not the DSL-enriched output. Deriving
    // CardData equality keeps new metadata fields part of cache identity.
    input: HashMap<String, CardData>,
    store: Arc<SharedCardStore>,
}

type StoreCache = HashMap<u64, Vec<CachedStore>>;

fn cache() -> &'static Mutex<StoreCache> {
    static C: OnceLock<Mutex<StoreCache>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cheap order-independent bucket selector, deliberately not an identity proof.
/// Different inputs can share this fingerprint. Every hit also compares the
/// complete original metadata before any cached store can be returned.
fn fingerprint(all_card_data: &HashMap<String, CardData>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut sum: u64 = all_card_data.len() as u64;
    for (id, d) in all_card_data {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        // Common fixed-size variations select different buckets cheaply.
        id.hash(&mut h);
        d.card_kind.hash(&mut h);
        d.level.hash(&mut h);
        d.dp.hash(&mut h);
        d.play_cost.hash(&mut h);
        d.index.hash(&mut h);
        d.colors.hash(&mut h);
        d.traits.hash(&mut h);
        d.keywords.hash(&mut h);
        d.ace_overflow.hash(&mut h);
        // Every returned field is content-keyed: same-length text, identity
        // aliases, routes and dual faces can change legal predicates or costs.
        d.card_id.hash(&mut h);
        d.norm_id.to_bits().hash(&mut h);
        d.card_name.hash(&mut h);
        d.effect_text.hash(&mut h);
        d.inherited_text.hash(&mut h);
        d.security_text.hash(&mut h);
        d.link_text.hash(&mut h);
        d.effect_class_name.hash(&mut h);
        d.evo_costs.len().hash(&mut h);
        // Equal-length route updates still change legal hosts and payment.
        // Hash the small printed circles by value, preserving route order.
        for cost in &d.evo_costs {
            cost.card_color.hash(&mut h);
            cost.level.hash(&mut h);
            cost.memory_cost.hash(&mut h);
        }
        // `dna_costs` are hashed by discriminating CONTENT, not just length.
        // The effect-initiated DNA verbs (G-ENGINE-DNA-PRINTED-COST /
        // -RECIPE-ENFORCEMENT) read `dna_costs[].memory_cost` and each
        // requirement's level + colors at resolution time, so two synthetic DBs
        // that differ in a DNA cost's amount or recipe select separate buckets.
        // `dna_costs` is tiny (≤2 entries), so folding the fields is cheap.
        // (`DnaCost`/`DnaRequirement` don't derive `Hash`, so project the fields.)
        d.dna_costs.len().hash(&mut h);
        for cost in &d.dna_costs {
            cost.memory_cost.hash(&mut h);
            for req in [&cost.requirement1, &cost.requirement2] {
                req.level.hash(&mut h);
                req.card_colors.len().hash(&mut h);
                for c in &req.card_colors {
                    (*c as u8).hash(&mut h);
                }
                req.name_is.hash(&mut h);
                req.name_contains.hash(&mut h);
                req.text_contains.hash(&mut h);
            }
        }
        d.digixros_aliases.hash(&mut h);
        d.also_treated_as.hash(&mut h);
        d.dual.is_some().hash(&mut h);
        if let Some(dual) = &d.dual {
            let face = &dual.digimon;
            face.level.hash(&mut h);
            face.dp.hash(&mut h);
            face.colors.hash(&mut h);
            face.traits.hash(&mut h);
            face.evo_costs.len().hash(&mut h);
            for cost in &face.evo_costs {
                cost.card_color.hash(&mut h);
                cost.level.hash(&mut h);
                cost.memory_cost.hash(&mut h);
            }
            face.effect_text.hash(&mut h);
            face.inherited_text.hash(&mut h);
            face.keywords.hash(&mut h);
            let face = &dual.option;
            face.use_cost.hash(&mut h);
            face.colors.hash(&mut h);
            face.effect_text.hash(&mut h);
            face.security_text.hash(&mut h);
            face.keywords.hash(&mut h);
        }
        sum = sum.wrapping_add(h.finish());
    }
    sum
}

fn cached_store(
    cache: &StoreCache,
    fp: u64,
    input: &HashMap<String, CardData>,
) -> Option<Arc<SharedCardStore>> {
    cache
        .get(&fp)?
        .iter()
        .find(|entry| {
            entry.input.len() == input.len()
                && entry.input.iter().all(|(id, original)| {
                    input.get(id).is_some_and(|current| {
                        original == current
                // Preserve the float's representation too (notably -0.0).
                && original.norm_id.to_bits() == current.norm_id.to_bits()
                    })
                })
        })
        .map(|entry| Arc::clone(&entry.store))
}

/// Memoized accessor: returns the shared enriched store for `all_card_data`,
/// reusing only an exact original metadata match within a fingerprint bucket.
pub fn shared_card_store(all_card_data: &HashMap<String, CardData>) -> Arc<SharedCardStore> {
    let fp = fingerprint(all_card_data);
    if let Some(existing) = cached_store(&cache().lock().unwrap(), fp, all_card_data) {
        return existing;
    }
    let built = Arc::new(build_shared_card_store(all_card_data));
    let mut cache = cache().lock().unwrap();
    // Another caller may have populated this exact input while we built it.
    if let Some(existing) = cached_store(&cache, fp, all_card_data) {
        return existing;
    }
    if cache.values().map(Vec::len).sum::<usize>() >= CACHE_SOFT_CAP {
        evict_unused(&mut cache);
    }
    cache.entry(fp).or_default().push(CachedStore {
        input: all_card_data.clone(),
        store: built.clone(),
    });
    built
}

/// Past this many distinct DB inputs, an insert first drops every entry no
/// live `Game` still holds. Production keys one stable DB, so it never gets here;
/// test suites key one store per distinct card set, and without eviction the map
/// grew ~2 MB per test until a full `cards_behavioral` run was OOM-killed.
const CACHE_SOFT_CAP: usize = 16;

/// Drop entries whose only owner is the cache. The build is deterministic
/// (sorted `data_index`), so a later miss rebuilds an identical store — eviction
/// is behavior-neutral; games already holding an `Arc` keep theirs.
fn evict_unused(cache: &mut StoreCache) {
    cache.retain(|_, entries| {
        entries.retain(|entry| Arc::strong_count(&entry.store) > 1);
        !entries.is_empty()
    });
}

/// Process-cached embedded DSL alt-path map. Independent of `all_card_data`, so
/// building it once avoids cloning ~2,000 `Vec<CompiledAltPath>` entries on
/// every `shared_card_store` cache miss in test suites.
#[cfg(feature = "dsl-yaml-loader")]
fn embedded_alt_paths() -> AltPathRegistry {
    static BASE: OnceLock<
        Option<Arc<HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>>>>,
    > = OnceLock::new();
    let base = BASE.get_or_init(|| {
        let dsl_registry = crate::dsl_registry::from_embedded_cached().ok()?;
        let map: HashMap<String, Vec<digimon_dsl::compiled::CompiledAltPath>> =
            if let Some(enrichments) = dsl_registry.enrichments() {
                enrichments
                    .iter()
                    .filter(|e| !e.alt_paths.is_empty())
                    .map(|e| (e.card_id.clone(), e.alt_paths.clone()))
                    .collect()
            } else {
                dsl_registry
                    .iter()
                    .filter_map(|(card_id, compiled)| {
                        if compiled.alt_paths.is_empty() {
                            None
                        } else {
                            Some((card_id.clone(), compiled.alt_paths.clone()))
                        }
                    })
                    .collect()
            };
        Some(Arc::new(map))
    });
    match base {
        Some(arc) => AltPathRegistry::from_base(Arc::clone(arc)),
        None => AltPathRegistry::default(),
    }
}

/// Process-cached sorted global token rows `(card_id, CardData, ParsedCardKeywords)`.
fn cached_token_rows() -> &'static [(String, CardData, ParsedCardKeywords)] {
    static ROWS: OnceLock<Vec<(String, CardData, ParsedCardKeywords)>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let token_registry = crate::token_registry::build_registry_cached();
        let mut token_defs: Vec<_> = token_registry.iter().collect();
        token_defs.sort_by(|a, b| a.card_id.cmp(&b.card_id));
        token_defs
            .into_iter()
            .map(|def| {
                let cd = def.to_card_data();
                let kw = ParsedCardKeywords::from_card_data(&cd);
                (def.card_id.clone(), cd, kw)
            })
            .collect()
    })
}

/// The reference build — the exact clone+enrich+index+token-absorb sequence
/// previously inline in `Game::new`. The cache-miss path; also the oracle baseline.
pub fn build_shared_card_store(all_card_data: &HashMap<String, CardData>) -> SharedCardStore {
    #[allow(unused_mut)]
    let mut effective_card_data = all_card_data.clone();

    #[cfg(feature = "dsl-yaml-loader")]
    if let Ok(dsl_registry) = crate::dsl_registry::from_embedded_cached() {
        crate::dsl_bridge::enrich_card_data_with_dsl_alt_paths(
            &mut effective_card_data,
            dsl_registry,
        );
    }

    // Assign `data_index` in a DETERMINISTIC order (sorted by card_id), NOT
    // `HashMap` iteration order. The store is memoized and shared across games, so
    // the assignment must not depend on a particular `HashMap` instance's seed —
    // otherwise a cache hit would return a different assignment than a caller that
    // independently re-derives one (e.g. `DebugRunner`, which sorts identically).
    // `data_index` is internal (the obs tensor keys on the stable registry index),
    // so this is behavior-neutral.
    let token_rows = cached_token_rows();
    let total_len = effective_card_data.len() + token_rows.len();
    let mut sorted_entries: Vec<(String, CardData)> = effective_card_data.into_iter().collect();
    sorted_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut data: Vec<CardData> = Vec::with_capacity(total_len);
    let mut keywords: Vec<ParsedCardKeywords> = Vec::with_capacity(total_len);
    let mut index: HashMap<String, usize> = HashMap::with_capacity(total_len);
    for (card_id, cd) in sorted_entries {
        let idx = data.len();
        keywords.push(ParsedCardKeywords::from_card_data(&cd));
        index.insert(card_id, idx);
        data.push(cd);
    }

    // Absorb the global (deck-independent) token rows. They also need index
    // entries so `card_data_by_id` and printed-keyword auto-effect synthesis
    // can resolve token card IDs.
    for (card_id, cd, kw) in token_rows {
        let idx = data.len();
        index.insert(card_id.clone(), idx);
        data.push(cd.clone());
        keywords.push(kw.clone());
    }

    SharedCardStore {
        data: Arc::new(data),
        keywords: Arc::new(keywords),
        index: Arc::new(index),
        #[cfg(feature = "dsl-yaml-loader")]
        alt_paths: embedded_alt_paths(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_store() -> Arc<SharedCardStore> {
        Arc::new(SharedCardStore {
            data: Arc::new(Vec::new()),
            keywords: Arc::new(Vec::new()),
            index: Arc::new(HashMap::new()),
            #[cfg(feature = "dsl-yaml-loader")]
            alt_paths: AltPathRegistry::default(),
        })
    }

    #[test]
    fn evict_unused_keeps_only_stores_a_game_still_holds() {
        let held = empty_store();
        let mut cache = StoreCache::new();
        cache.insert(
            1,
            vec![CachedStore {
                input: HashMap::new(),
                store: held.clone(),
            }],
        );
        for fp in 2..=20 {
            cache.insert(
                fp,
                vec![CachedStore {
                    input: HashMap::new(),
                    store: empty_store(),
                }],
            );
        }
        // Collisions share a bucket, but each original input is evicted alone.
        cache.get_mut(&1).unwrap().push(CachedStore {
            input: HashMap::new(),
            store: empty_store(),
        });

        evict_unused(&mut cache);

        assert_eq!(cache.len(), 1, "only the held store survives");
        assert_eq!(cache[&1].len(), 1);
        assert!(Arc::ptr_eq(&cache[&1][0].store, &held));
    }

    #[test]
    fn evicted_fingerprint_rebuilds_an_identical_store() {
        let mut db = HashMap::new();
        db.insert(
            "EVICT-TEST-1".to_string(),
            crate::debug_runner::make_test_card("EVICT-TEST-1", "Evict Test"),
        );
        let first = shared_card_store(&db);
        let ids: Vec<String> = first.data.iter().map(|c| c.card_id.clone()).collect();
        let index = (*first.index).clone();
        drop(first);
        // Force the fingerprint out (other tests may share the global map, so
        // evict through the same path an over-cap insert uses).
        evict_unused(&mut cache().lock().unwrap());

        let rebuilt = shared_card_store(&db);
        let rebuilt_ids: Vec<String> = rebuilt.data.iter().map(|c| c.card_id.clone()).collect();
        assert_eq!(rebuilt_ids, ids);
        assert_eq!(*rebuilt.index, index);
    }

    #[test]
    fn card_store_keyword_cache_invalidates_on_direct_arc_make_mut_and_deref_mut() {
        let mut c = crate::debug_runner::make_test_card("KW-TEST-1", "Kw Test");
        c.effect_text = "＜Blocker＞".to_string();
        let mut db = HashMap::new();
        db.insert(c.card_id.clone(), c);
        let shared = shared_card_store(&db);
        let idx = shared.index["KW-TEST-1"];

        let mut via_arc = CardStore::from_shared(&shared);
        assert!(via_arc.face_keywords(idx).contains(&crate::enums::Keyword::Blocker));
        assert!(!via_arc.face_keywords(idx).contains(&crate::enums::Keyword::Rush));

        // Direct mutation of `.0` via `Arc::make_mut` must fork the Arc and invalidate the cache.
        Arc::make_mut(&mut via_arc.0)[idx].effect_text = "＜Rush＞".to_string();
        assert!(via_arc.face_keywords(idx).contains(&crate::enums::Keyword::Rush));
        assert!(!via_arc.face_keywords(idx).contains(&crate::enums::Keyword::Blocker));

        // Mutation via `DerefMut` must also invalidate the cache without mutating the shared store.
        let mut via_deref = CardStore::from_shared(&shared);
        via_deref[idx].inherited_text = "＜Jamming＞".to_string();
        assert!(via_deref.inherited_keywords(idx).contains(&crate::enums::Keyword::Jamming));

        // Original shared store stays untouched.
        let fresh = CardStore::from_shared(&shared);
        assert!(fresh.face_keywords(idx).contains(&crate::enums::Keyword::Blocker));
        assert!(!fresh.inherited_keywords(idx).contains(&crate::enums::Keyword::Jamming));
    }

    #[cfg(feature = "dsl-yaml-loader")]
    #[test]
    fn alt_path_registry_overlay_does_not_clone_base() {
        let base_reg = embedded_alt_paths();
        let mut copy = base_reg.clone();
        // Empty extend must leave both base and overrides Arc pointers shared.
        copy.extend_overrides(std::iter::empty());
        assert!(Arc::ptr_eq(&base_reg.overrides, &copy.overrides));
        if let (Some(a), Some(b)) = (&base_reg.base, &copy.base) {
            assert!(Arc::ptr_eq(a, b));
        }
        // Non-empty override shadows without cloning base.
        copy.extend_overrides([("SYNTHETIC-ALT-CARD".to_string(), Vec::new())]);
        if let (Some(a), Some(b)) = (&base_reg.base, &copy.base) {
            assert!(Arc::ptr_eq(a, b));
        }
        assert!(copy.get("SYNTHETIC-ALT-CARD").is_some());
        assert!(base_reg.get("SYNTHETIC-ALT-CARD").is_none());
    }
}
