//! Offline conversion diagnostics only; never constructs a game or runtime handle.
//! The actual CLI requires the reviewed 99-row profile; tests use small fixtures.

use digimon_dsl::{compiled::*, CardPack};
use digimon_engine::{card_data::CardData, card_registry::CardRegistry};
use digimon_engine::enums::Keyword as DiagnosticKeyword;
#[path = "../src/card_store/diagnostic_wire.rs"]
mod diagnostic_wire;
use diagnostic_wire::*;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_DESCRIPTORS: usize = 16384;
const MAX_ROLES: usize = 32768;
const MAX_FILE: u64 = 512 * 1024 * 1024;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024;
const MAX_REQUEST: u64 = 16 * 1024 * 1024;
const MAX_OUTPUT: u64 = 128 * 1024 * 1024;
type Result<T> = std::result::Result<T, String>;
const PACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cards.pack"));
const FAMILIES: &[&str] = &[
    "traits",
    "numeric_sentinels",
    "kind_colors",
    "registry_f32",
    "printed_keywords",
    "raw_digixros_aliases",
    "dual_face_lookup",
    "dsl_enrichment",
    "dsl_dna_restrictions",
    "tokens",
];

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn value<T: serde::Serialize>(v: T) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

fn presence(row: &Value, k: &str) -> Value {
    match row.get(k) {
        None => json!({"presence":"absent"}),
        Some(v) => {
            json!({"presence":if v.is_null(){"null"}else if v==""||v==&json!([])||v==&json!({}){"empty"}else{"value"},"value":v})
        }
    }
}
fn defaults(row: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for (field, native, default) in [
        ("level", "level", Value::Null),
        ("dp", "dp", Value::Null),
        ("play_cost", "play_cost", json!(0)),
    ] {
        if row[field].is_null() || row[field] == json!(-1) {
            out.push(json!({"field":field,"input":presence(row,field),"native_field":native,"default":default,"reason":"OPTIONAL_OR_MINUS_ONE_SENTINEL"}));
        }
    }
    for (field, native, default) in [
        ("link_effect_description_eng", "link_text", json!("")),
        ("dna_costs", "dna_costs", json!([])),
        ("ace_overflow", "ace_overflow", Value::Null),
        ("dual", "dual", Value::Null),
        ("digixros_aliases", "digixros_aliases", json!([])),
    ] {
        if row.get(field).is_none() {
            out.push(json!({"field":field,"input":presence(row,field),"native_field":native,"default":default,"reason":"ABSENT_RAW_OPTIONAL_FIELD"}));
        }
    }
    out
}
fn lookup(c: &CardData) -> Value {
    json!({"is_digimon_card_for_search":c.is_digimon_card_for_search(),"is_option_card_for_search":c.is_option_card_for_search(),
        "digimon_level":c.digimon_level(),"digimon_dp":c.digimon_dp(),"digimon_colors":c.digimon_colors(),
        "option_colors":c.option_colors(),"option_use_cost":c.option_use_cost(),"digivolution_costs":c.digivolution_costs(),
        "text_for_search_all_faces":c.text_for_search_all_faces()})
}
fn pointer(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}
fn token_occurrences(ir: &Value) -> Result<Vec<Value>> {
    fn walk(v: &Value, path: &str, out: &mut Vec<Value>) -> Result<()> {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    let p = format!("{path}/{}", pointer(k));
                    if k == "PlayToken" {
                        keys(
                            x,
                            &["controller", "token_name", "bind_as"],
                            &["controller", "token_name", "bind_as"],
                            "PlayToken",
                        )?;
                        let name = x["token_name"].as_str().ok_or("token_name string")?;
                        let reg = digimon_engine::token_registry::build_registry();
                        out.push(json!({"pointer":p,"token_name":name,"registered":reg.get(name).is_some(),"template_source_proven":false,"payload":x}));
                    }
                    walk(x, &p, out)?;
                }
            }
            Value::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    walk(x, &format!("{path}/{i}"), out)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(ir, "", &mut out)?;
    Ok(out)
}
fn spans(row: &Value) -> Vec<Value> {
    let mut result = Vec::new();
    for field in [
        "effect_description_eng",
        "inherited_effect_description_eng",
        "security_effect_description_eng",
    ] {
        let text = row[field].as_str().unwrap_or("");
        for (open, close) in [('＜', '＞'), ('<', '>')] {
            let mut at = 0;
            while let Some(start) = text[at..].find(open) {
                let begin = at + start;
                let content = begin + open.len_utf8();
                let Some(end) = text[content..].find(close) else {
                    break;
                };
                let end = content + end;
                result.push(json!({"field":field,"byte_start":begin,"byte_end":end+close.len_utf8(),"text":&text[content..end],"status":"UNCLASSIFIED_TEXT_SPAN_NOT_RULE_PROOF"}));
                at = end + close.len_utf8();
            }
        }
    }
    result
}
fn paths(card: &CompiledCard, original: &CardData) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut seen = original.dna_costs.clone();
    for (ordinal, path) in card.alt_paths.iter().enumerate() {
        let mut one = card.clone();
        one.alt_paths = vec![path.clone()];
        let p = CardPack::new("one-path-probe", vec![one]);
        let reg = digimon_dsl::CardRegistry::from_pack_bytes(&p.to_bytes()?)?;
        let mut base = original.clone();
        base.dna_costs.clear();
        let mut map = HashMap::from([(card.card.clone(), base)]);
        digimon_engine::dsl_bridge::enrich_card_data_with_dsl_alt_paths(&mut map, &reg);
        let emitted = &map[&card.card].dna_costs;
        let status = if path.kind != CompiledAltPathKind::DnaDigivolve {
            "NOT_DNA"
        } else if emitted.is_empty() {
            "OMITTED_UNSUPPORTED"
        } else {
            "EMITTED"
        };
        let duplicate = !emitted.is_empty() && emitted.iter().all(|x| seen.contains(x));
        for x in emitted {
            if !seen.contains(x) {
                seen.push(x.clone());
            }
        }
        let mut limitations = Vec::new();
        if path
            .materials
            .iter()
            .any(|m| m.filter.name_is.is_some() && m.filter.name_contains.is_none())
        {
            limitations.push("NAME_IS_REPRESENTED_AS_SUBSTRING");
        }
        if path
            .materials
            .iter()
            .any(|m| m.filter.name_is.is_some() && m.filter.name_contains.is_some())
        {
            limitations.push("NAME_CONTAINS_TAKES_PRECEDENCE_OVER_NAME_IS");
        }
        if path.materials.iter().any(|m| m.filter.color_only.is_some()) {
            limitations.push("COLOR_ONLY_PREDICATE_REDUCED_TO_COLOR_LIST");
        }
        if path
            .materials
            .iter()
            .any(|m| m.filter.color_only.is_some() && m.filter.color_is.is_some())
        {
            limitations.push("COLOR_ONLY_TAKES_PRECEDENCE_OVER_COLOR_IS");
        }
        if path.condition.is_some()
            || path.from.is_some()
            || !path.extra_cost.is_empty()
            || path.ignore_requirements
            || path.direction != CompiledAltPathDirection::From
            || path.stacks_unsuspended
            || path.source_treated_as.is_some()
            || !path.on_burst_turn_end.is_empty()
            || path.marker
            || !path.extra_material_zones.is_empty()
            || path.materials.iter().any(|m| {
                m.repeat.is_some()
                    || m.distinct_by.is_some()
                    || !m.zones.is_empty()
                    || m.cost_delta.is_some()
                    || m.stack_under
            })
        {
            limitations.push("PATH_CONTEXT_NOT_REPRESENTED_BY_DNA_COST");
        }
        out.push(json!({"ordinal":ordinal,"compiled_path":path,"status":status,"emitted":emitted,
            "full_pass_disposition":if duplicate{"DEDUPLICATED"}else if emitted.is_empty(){"NOT_APPENDED"}else{"APPENDED"},
            "limitations":limitations,"material_semantics_proven":false}));
    }
    Ok(out)
}
#[cfg(test)]
fn diagnose(
    rows: &Value,
    global: &Value,
    scope: &BTreeSet<String>,
    pack: &CardPack,
) -> Result<Value> {
    diagnose_bytes(rows.to_string().as_bytes(), global, scope, pack)
}
fn diagnose_bytes(
    record_bytes: &[u8],
    global: &Value,
    scope: &BTreeSet<String>,
    pack: &CardPack,
) -> Result<Value> {
    let parsed = strict_json(record_bytes)?;
    let rows = &parsed;
    let assignments = registry_wire(global)?;
    let input = object(rows, "records")?;
    require(
        input.keys().cloned().collect::<BTreeSet<_>>() == *scope,
        "selected membership differs",
    )?;
    for (id, row) in input {
        wire_row(id, row, assignments)?;
    }
    let raw =
        CardData::load_from_str(std::str::from_utf8(record_bytes).map_err(|e| e.to_string())?)?;
    require(raw.len() == scope.len(), "native membership differs")?;
    let index = CardRegistry::from_cards(&raw);
    let production_raw = digimon_engine::cards::raw_rust::build_registry();
    let bytes = pack.to_bytes()?;
    checked_pack(&bytes, &production_raw)?;
    let registry =
        digimon_dsl::CardRegistry::from_pack_bytes_with_raw_registry(&bytes, &production_raw)?;
    let mut enriched = raw.clone();
    let inserted =
        digimon_engine::dsl_bridge::enrich_card_data_with_dsl_alt_paths(&mut enriched, &registry);
    let mut cards = BTreeMap::new();
    let mut missing = Vec::new();
    for id in scope {
        let a = &raw[id];
        let b = &enriched[id];
        require(
            index.get_index(id) == a.index
                && index.get_id(a.index) == Some(id.as_str())
                && index.get_norm_id(id).to_bits() == a.norm_id.to_bits(),
            "native registry round trip",
        )?;
        let av = value(a)?;
        let bv = value(b)?;
        let mut changes = Vec::new();
        let allkeys: BTreeSet<_> = object(&av, "native")?
            .keys()
            .chain(object(&bv, "enriched")?.keys())
            .collect();
        for k in allkeys {
            if av.get(k) != bv.get(k) {
                require(
                    [
                        "ace_overflow",
                        "digixros_aliases",
                        "also_treated_as",
                        "dna_costs",
                    ]
                    .contains(&k.as_str()),
                    format!("unexpected enrichment mutation: {k}"),
                )?;
                changes.push(json!({"field":k,"before":presence(&av,k),"after":presence(&bv,k),"source_agreement_proven":false}));
            }
        }
        let row = &rows[id];
        let fields: BTreeMap<_, _> = RAW
            .iter()
            .chain(COMPAT)
            .chain(META)
            .copied()
            .chain(["card_index"])
            .map(|k| (k, presence(row, k)))
            .collect();
        let compat: BTreeMap<_, _> = COMPAT.iter().map(|k| (*k, presence(row, k))).collect();
        let compiled = registry.lookup(id);
        if compiled.is_none() {
            missing.push(id.clone());
        }
        let keyword = digimon_engine::card_data::parse_printed_keywords;
        let (main, inh, sec) = (
            a.effect_text.as_str(),
            a.inherited_text.as_str(),
            a.security_text.as_str(),
        );
        cards.insert(id,json!({"presence":fields,"raw_native":av,"enriched_native":bv,"raw_lookup":lookup(a),"enriched_lookup":lookup(b),
            "registry":{"index":index.get_index(id),"norm_id_bits":index.get_norm_id(id).to_bits()},
            "keyword_parsing":{"main":keyword(main,"",""),"inherited":keyword("",inh,""),"security":keyword("","",sec),"combined":keyword(main,inh,sec)},
            "keyword_text_spans":spans(row),"keyword_parser_excluded_fields":["link_effect_description_eng"],"default_conversions":defaults(row),"enrichment_changes":changes,"compiled":value(compiled)?,
            "alt_path_diagnostics":compiled.map(|c|paths(c,a)).transpose()?.unwrap_or_default(),
            "token_references":compiled.map(|c|value(c).and_then(|v|token_occurrences(&v))).transpose()?.unwrap_or_default(),
            "ignored_compatibility":compat,"compatibility_semantics_proven":false,"native_semantics_proven":false}));
    }
    let tokenreg = digimon_engine::token_registry::build_registry();
    let mut tokens:Vec<_>=tokenreg.iter().map(|t|json!({"name":t.name,"card_id":t.card_id,"native":t.to_card_data(),"outside_e":!scope.contains(&t.card_id),"template_source_proven":false,"dispatch_admitted":false})).collect();
    tokens.sort_by(|a, b| a["card_id"].as_str().cmp(&b["card_id"].as_str()));
    let membership: BTreeSet<_> = pack.cards.iter().map(|c| c.card.clone()).collect();
    let outside: Vec<_> = membership.difference(scope).cloned().collect();
    Ok(
        json!({"kind":"native-conversion-diagnostic-v1","status":"DIAGNOSTIC_ONLY_NO_ADMISSION","cards":cards,"full_registry":global,
        "selected_registry_count":index.count(),"pack_manifest":pack.manifest,"pack_membership":membership,"outside_e_pack":outside,"missing_selected_pack":missing,
        "tokens":tokens,"dna_inserted":inserted,"derived_families":FAMILIES,"outside_native_lookup":"NOT_CONSTRUCTED; selected registry returns padding for absent IDs",
        "admission":{"source":false,"runtime":false,"oracle":false,"training":false},"native_admission":false,"projection_epoch":null,
        "full_clause_count":null,"full_gating_interaction_count":null,"unresolved":["SOURCE_NATIVE_SEMANTIC_AGREEMENT_UNPROVEN","COMPATIBILITY_CONSUMER_SEMANTICS_UNPROVEN",
        "TOKEN_TEMPLATE_CAPABILITY_UNISSUED","RAW_FUNCTION_BEHAVIOR_OPAQUE","CACHE_AND_DISPATCH_NOT_TESTED","NO_RUNTIME_HANDLE","TASK_2_3_INCOMPLETE"]}),
    )
}

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
struct PhysicalId {
    volume: u64,
    file: u64,
}

#[cfg(windows)]
fn physical_id(file: &fs::File) -> Result<(PhysicalId, u64)> {
    use std::os::windows::io::AsRawHandle;
    // Stable Win32 BY_HANDLE_FILE_INFORMATION, read through an opened handle.
    // No new crate, process, global setting, privileges or writable handle.
    #[repr(C)]
    struct Info {
        attributes: u32,
        creation: [u32; 2],
        access: [u32; 2],
        write: [u32; 2],
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(handle: *mut std::ffi::c_void, info: *mut Info) -> i32;
    }
    let mut info = std::mem::MaybeUninit::<Info>::uninit();
    // SAFETY: the handle is borrowed from a live File and the output buffer has
    // exactly the documented C layout; success initializes every field.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let info = unsafe { info.assume_init() };
    require(info.attributes & 0x400 == 0, "reparse handle")?;
    Ok((
        PhysicalId {
            volume: u64::from(info.volume),
            file: (u64::from(info.index_high) << 32) | u64::from(info.index_low),
        },
        u64::from(info.links),
    ))
}
#[cfg(unix)]
fn physical_id(file: &fs::File) -> Result<(PhysicalId, u64)> {
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata().map_err(|e| e.to_string())?;
    Ok((
        PhysicalId {
            volume: m.dev(),
            file: m.ino(),
        },
        m.nlink(),
    ))
}
fn reparse(m: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
fn path_key(p: &Path) -> Result<String> {
    let s = p.to_str().ok_or("non-UTF8 path")?.replace('\\', "/");
    #[cfg(windows)]
    {
        Ok(s.strip_prefix("//?/").unwrap_or(&s).to_ascii_lowercase())
    }
    #[cfg(not(windows))]
    {
        Ok(s)
    }
}
fn lexical_path(p: &Path) -> Result<()> {
    require(p.is_absolute(), "absolute path required")?;
    let s = p.to_str().ok_or("non-UTF8 path")?.replace('\\', "/");
    require(s.len() <= 32768, "path length limit")?;
    #[cfg(windows)]
    let body = {
        let s = s.strip_prefix("//?/").unwrap_or(&s);
        require(
            s.len() >= 3 && s.as_bytes()[0].is_ascii_alphabetic() && &s[1..3] == ":/",
            "local absolute drive path required",
        )?;
        &s[3..]
    };
    #[cfg(not(windows))]
    let body = s.strip_prefix('/').ok_or("absolute path")?;
    require(
        !body.is_empty()
            && body.split('/').all(|x| {
                !x.is_empty()
                    && x != "."
                    && x != ".."
                    && !x.ends_with(['.', ' '])
                    && !x.chars().any(|c| c == '\0' || "<>:\"|?*".contains(c))
            }),
        "noncanonical or traversing path",
    )?;
    #[cfg(windows)]
    for part in body.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        require(
            !["CON", "PRN", "AUX", "NUL", "CLOCK$"].contains(&stem.as_str())
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9')),
            "reserved device path",
        )?;
    }
    Ok(())
}
// Check every physical component with lstat semantics. In missing-tail mode,
// the first missing component is allowed, but no existing reparse ancestor is.
fn checked_path(p: &Path, missing_tail: bool) -> Result<PathBuf> {
    lexical_path(p)?;
    let mut ancestors: Vec<_> = p.ancestors().collect();
    ancestors.reverse();
    for a in ancestors {
        match fs::symlink_metadata(a) {
            Ok(m) => {
                require(!reparse(&m), format!("reparse path: {}", a.display()))?;
                if a != p {
                    require(m.is_dir(), "non-directory ancestor")?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && missing_tail => {
                let parent = a.parent().ok_or("missing root")?;
                let canonical = parent.canonicalize().map_err(|e| e.to_string())?;
                require(
                    path_key(parent)? == path_key(&canonical)?,
                    "noncanonical existing ancestor",
                )?;
                return Ok(p.to_path_buf());
            }
            Err(e) => return Err(format!("path {}: {e}", a.display())),
        }
    }
    let canonical = p.canonicalize().map_err(|e| e.to_string())?;
    require(path_key(p)? == path_key(&canonical)?, "physical path alias")?;
    Ok(canonical)
}
fn stop_absent(p: &Path) -> Result<()> {
    checked_path(p, true)?;
    match fs::symlink_metadata(p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err("STOP present".into()),
        Err(e) => Err(format!("STOP inspection: {e}")),
    }
}
fn open_physical(p: &Path) -> Result<fs::File> {
    checked_path(p, false)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000 | 0x02000000);
    }
    let file = options.open(p).map_err(|e| e.to_string())?;
    require(
        !reparse(&file.metadata().map_err(|e| e.to_string())?),
        "reparse opened file",
    )?;
    Ok(file)
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    id: PhysicalId,
    len: u64,
    modified: std::time::SystemTime,
}
fn stamp(file: &fs::File) -> Result<Stamp> {
    let m = file.metadata().map_err(|e| e.to_string())?;
    let (id, links) = physical_id(file)?;
    require(
        m.is_file() && links == 1,
        "regular non-hardlinked file required",
    )?;
    Ok(Stamp {
        id,
        len: m.len(),
        modified: m.modified().map_err(|e| e.to_string())?,
    })
}
#[derive(Clone)]
struct BoundFile {
    descriptor: Descriptor,
    original: Stamp,
}
impl BoundFile {
    fn bind(d: &Descriptor) -> Result<Self> {
        require(d.bytes <= MAX_FILE, "per-file byte limit")?;
        require(
            d.sha256.len() == 64 && d.sha256.bytes().all(|x| x.is_ascii_hexdigit()),
            "SHA256 descriptor",
        )?;
        let f = open_physical(Path::new(&d.path))?;
        let original = stamp(&f)?;
        require(original.len == d.bytes, "declared file length differs")?;
        Ok(Self {
            descriptor: d.clone(),
            original,
        })
    }
    fn fast(&self) -> Result<()> {
        require(
            stamp(&open_physical(Path::new(&self.descriptor.path))?)? == self.original,
            "physical file identity/metadata changed",
        )
    }
    fn check(&self, body_limit: Option<u64>) -> Result<Vec<u8>> {
        if let Some(limit) = body_limit {
            require(self.descriptor.bytes <= limit, "body byte limit")?;
        }
        let mut f = open_physical(Path::new(&self.descriptor.path))?;
        require(
            stamp(&f)? == self.original,
            "physical file replaced before read",
        )?;
        let mut digest = Sha256::new();
        let mut total = 0u64;
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 65536];
        loop {
            let n = f.read(&mut chunk).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            total = total.checked_add(n as u64).ok_or("read size overflow")?;
            require(
                total <= self.descriptor.bytes && total <= MAX_FILE,
                "file grew past bound",
            )?;
            digest.update(&chunk[..n]);
            if body_limit.is_some() {
                bytes.extend_from_slice(&chunk[..n]);
            }
        }
        require(
            total == self.descriptor.bytes
                && format!("{:x}", digest.finalize()) == self.descriptor.sha256,
            "byte pin differs",
        )?;
        require(stamp(&f)? == self.original, "file changed during read")?;
        self.fast()?;
        Ok(bytes)
    }
}
impl Descriptor {
    fn read(&self) -> Result<Vec<u8>> {
        BoundFile::bind(self)?.check(Some(MAX_JSON))
    }
    fn from_path(p: &Path) -> Result<Self> {
        let mut f = open_physical(p)?;
        let original = stamp(&f)?;
        require(original.len <= MAX_FILE, "per-file byte limit")?;
        let mut digest = Sha256::new();
        let mut n = 0u64;
        let mut chunk = [0u8; 65536];
        loop {
            let got = f.read(&mut chunk).map_err(|e| e.to_string())?;
            if got == 0 {
                break;
            }
            n += got as u64;
            require(n <= original.len, "file grew during hash")?;
            digest.update(&chunk[..got]);
        }
        require(
            n == original.len && stamp(&f)? == original,
            "file changed during hash",
        )?;
        let d = Self {
            path: p.to_string_lossy().into(),
            bytes: n,
            sha256: format!("{:x}", digest.finalize()),
        };
        require(
            BoundFile::bind(&d)?.original == original,
            "physical file changed after hash",
        )?;
        Ok(d)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema_version: u32,
    kind: String,
    stop_path: String,
    records: Descriptor,
    source_report: Descriptor,
    registry: Descriptor,
    closure_manifest: Descriptor,
    comparison_runtime: Descriptor,
    generation_completion: Descriptor,
    generation_natural_exit: Descriptor,
    independent_source_review: Descriptor,
    build_receipt: Descriptor,
    executable: Descriptor,
    pack: Descriptor,
    dependencies: Vec<Descriptor>,
    yaml_inputs: Vec<Descriptor>,
}
impl Request {
    fn inputs(&self) -> Vec<&Descriptor> {
        let mut all = vec![
            &self.records,
            &self.source_report,
            &self.registry,
            &self.closure_manifest,
            &self.comparison_runtime,
            &self.generation_completion,
            &self.generation_natural_exit,
            &self.independent_source_review,
            &self.build_receipt,
            &self.executable,
            &self.pack,
        ];
        all.extend(&self.dependencies);
        all.extend(&self.yaml_inputs);
        all
    }
    fn roles(&self) -> Vec<(String, &Descriptor)> {
        let names = [
            "records",
            "source_report",
            "registry",
            "closure_manifest",
            "comparison_runtime",
            "generation_completion",
            "generation_natural_exit",
            "independent_source_review",
            "build_receipt",
            "executable",
            "pack",
        ];
        self.inputs()
            .into_iter()
            .enumerate()
            .map(|(i, d)| {
                let role = if i < names.len() {
                    names[i].to_owned()
                } else if i < names.len() + self.dependencies.len() {
                    format!("dependencies[{}]", i - names.len())
                } else {
                    format!("yaml_inputs[{}]", i - names.len() - self.dependencies.len())
                };
                (role, d)
            })
            .collect()
    }
}

fn bounded_plan(roles: &[(String, &Descriptor)]) -> Result<BTreeMap<String, Descriptor>> {
    require(roles.len() <= MAX_ROLES, "descriptor role count limit")?;
    let mut files: BTreeMap<String, Descriptor> = BTreeMap::new();
    let mut total = 0u64;
    for (_, d) in roles {
        lexical_path(Path::new(&d.path))?;
        require(d.bytes <= MAX_FILE, "per-file byte limit")?;
        let key = path_key(Path::new(&d.path))?;
        if let Some(prior) = files.get(&key) {
            require(
                prior.bytes == d.bytes && prior.sha256 == d.sha256,
                "conflicting repeated role",
            )?;
        } else {
            total = total
                .checked_add(d.bytes)
                .ok_or("descriptor size overflow")?;
            require(total <= MAX_TOTAL, "total input byte limit")?;
            files.insert(key, (*d).clone());
            require(
                files.len() <= MAX_DESCRIPTORS,
                "physical descriptor count limit",
            )?;
        }
    }
    Ok(files)
}
struct InputBindings {
    files: BTreeMap<String, BoundFile>,
    roles: Vec<(String, String, Descriptor)>,
}
impl InputBindings {
    fn bind(request: &Request, request_file: &BoundFile, out: &Path) -> Result<Self> {
        require(
            request.dependencies.len() <= MAX_DESCRIPTORS
                && request.yaml_inputs.len() <= MAX_DESCRIPTORS,
            "descriptor vector limit",
        )?;
        let roles = request.roles();
        let descriptors = bounded_plan(&roles)?;
        checked_path(out, true)?;
        let out_key = path_key(out)?;
        let inside = |child: &str, parent: &str| {
            child == parent || child.starts_with(&(parent.to_owned() + "/"))
        };
        let request_parent = Path::new(&request_file.descriptor.path)
            .parent()
            .ok_or("request parent")?;
        require(
            !inside(&out_key, &path_key(request_parent)?),
            "output under request authority",
        )?;
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or("native root")?;
        require(
            !inside(&out_key, &path_key(repo)?),
            "output under native source authority",
        )?;
        let target = Path::new(&request.executable.path)
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or("build target root")?;
        require(
            !inside(&out_key, &path_key(target)?),
            "output under build target authority",
        )?;
        let mut files = BTreeMap::new();
        let mut physical = BTreeMap::new();
        for (key, d) in descriptors {
            let parent = Path::new(&d.path).parent().ok_or("input parent")?;
            require(
                !inside(&out_key, &path_key(parent)?) && !inside(&key, &out_key),
                "output/input authority ancestry",
            )?;
            let bound = BoundFile::bind(&d)?;
            require(
                bound.original.id != request_file.original.id,
                "request/input physical alias",
            )?;
            require(
                physical.insert(bound.original.id, key.clone()).is_none(),
                "input physical alias",
            )?;
            files.insert(key, bound);
        }
        let roles = roles
            .into_iter()
            .map(|(role, d)| Ok((role, path_key(Path::new(&d.path))?, d.clone())))
            .collect::<Result<_>>()?;
        Ok(Self { files, roles })
    }
    fn verify(&self) -> Result<()> {
        for file in self.files.values() {
            file.check(None)?;
        }
        Ok(())
    }
    fn report(&self) -> Value {
        json!(self.roles.iter().map(|(role,key,d)|json!({"role":role,"descriptor":d,"physical_identity":self.files[key].original.id})).collect::<Vec<_>>())
    }
}
// These checks bind the request to the actual build's complete source readback,
// including all native YAML and used cached crates. They do not grant approval.
fn descriptor_set(items: &[Descriptor]) -> Result<BTreeMap<String, Descriptor>> {
    let mut out = BTreeMap::new();
    for d in items {
        require(
            Path::new(&d.path).is_absolute()
                && d.sha256.len() == 64
                && d.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid dependency descriptor",
        )?;
        let normalized = d.path.replace('\\', "/").to_ascii_lowercase();
        require(
            out.insert(normalized, d.clone()).is_none(),
            "duplicate dependency path",
        )?;
    }
    Ok(out)
}
fn build_binding(
    build: &Value,
    dependencies: &[Descriptor],
    yaml: &[Descriptor],
    executable: &Descriptor,
    pack: &Descriptor,
) -> Result<()> {
    require(
        build["actual_natural_exit"] == json!(0) && build["mode"] == "build",
        "successful actual build required",
    )?;
    require(
        build["compiler_artifacts"].as_array().is_some_and(|a| {
            a.iter().any(|x| {
                x["target"]["name"] == "native_conversion_diagnostic"
                    && x["profile"]["test"] == false
                    && x["executable"] == json!(executable)
            })
        }),
        "build executable absent or test binary",
    )?;
    require(
        build["build_scripts"]
            .as_array()
            .is_some_and(|a| a.iter().any(|x| x["pack"] == json!(pack))),
        "build pack absent",
    )?;
    let sources: Vec<Descriptor> =
        serde_json::from_value(build["source_readset"].clone()).map_err(|e| e.to_string())?;
    let expected_yaml: Vec<Descriptor> =
        serde_json::from_value(build["yaml_inputs"].clone()).map_err(|e| e.to_string())?;
    require(
        !sources.is_empty() && !expected_yaml.is_empty(),
        "build source/YAML readset required",
    )?;
    let sources = descriptor_set(&sources)?;
    let expected_yaml = descriptor_set(&expected_yaml)?;
    require(
        descriptor_set(dependencies)? == sources,
        "complete build source readset differs",
    )?;
    require(
        descriptor_set(yaml)? == expected_yaml,
        "complete build YAML readset differs",
    )?;
    require(
        expected_yaml.iter().all(|(p, d)| sources.get(p) == Some(d)),
        "YAML not in build source readset",
    )
}

fn read_yaml_in_build_order(
    inputs: &[Descriptor],
    root: &Path,
) -> Result<Vec<(Descriptor, digimon_dsl::CardSpec)>> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut ordered = Vec::new();
    for d in inputs {
        let path = Path::new(&d.path)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| "YAML outside compiled package cards root")?;
        require(
            relative.components().count() >= 2
                && path.extension().is_some_and(|x| x == "yaml" || x == "yml"),
            "YAML outside build direct-subdirectory inventory",
        )?;
        ordered.push((path, d));
    }
    // build.rs sorts direct subdirectories; load_dir_ok sorts the files below
    // each one. Full path ordering yields the same concatenated file order.
    ordered.sort_by(|a, b| a.0.cmp(&b.0));
    ordered
        .into_iter()
        .map(|(_, d)| {
            let spec = serde_yml::from_slice(&d.read()?).map_err(|e| e.to_string())?;
            Ok((d.clone(), spec))
        })
        .collect()
}

fn yaml_pack_binding(
    declarations: &[(Descriptor, digimon_dsl::CardSpec)],
    pack: &CardPack,
) -> Result<Value> {
    let specs: Vec<_> = declarations.iter().map(|(_, s)| s.clone()).collect();
    // Use the actual build API. Its ordered insert makes the final declaration
    // win; every earlier declaration remains in the diagnostic inventory.
    let rebuilt = digimon_dsl::CardRegistry::from_specs(pack.manifest.pack_id.clone(), &specs)
        .map_err(|e| format!("YAML compile errors: {e:?}"))?;
    require(
        rebuilt.len() == pack.cards.len()
            && rebuilt.manifest == pack.manifest
            && pack
                .cards
                .iter()
                .all(|c| rebuilt.lookup(&c.card) == Some(c)),
        "current YAML winning IR differs from embedded pack",
    )?;
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, (_, s)) in declarations.iter().enumerate() {
        groups.entry(s.card.clone()).or_default().push(i);
    }
    let mut inventory = Vec::new();
    let mut shadowed = Vec::new();
    for (ordinal, (source, spec)) in declarations.iter().enumerate() {
        let indices = &groups[&spec.card];
        let winner = *indices.last().ok_or("empty YAML group")?;
        let compiled = if ordinal == winner {
            rebuilt
                .lookup(&spec.card)
                .ok_or("compiled YAML missing")?
                .clone()
        } else {
            digimon_dsl::compile::compile(spec)
                .map_err(|e| format!("shadowed YAML compile errors: {e:?}"))?
        };
        let ir = value(&compiled)?;
        let encoded = serde_json::to_vec(&ir).map_err(|e| e.to_string())?;
        inventory.push(json!({"ordinal":ordinal,"card_id":spec.card,"source":source,"is_build_winner":ordinal==winner,
            "winner_ordinal":winner,"compiled_ir_sha256":sha(&encoded),"source_authority":"UNPROVEN"}));
        if ordinal != winner {
            shadowed.push(json!({"card_id":spec.card,"source":source,"winner_source":declarations[winner].0,
                "compiled":ir,"same_ir_as_winner":rebuilt.lookup(&spec.card)==Some(&compiled),
                "required_raw_rust_fns":digimon_dsl::pack::required_raw_rust_fns_for_cards(&[compiled]),"source_authority":"UNPROVEN"}));
        }
    }
    let winners: BTreeMap<_, _> = rebuilt.iter().collect();
    Ok(
        json!({"declaration_count":declarations.len(),"unique_pack_card_count":rebuilt.len(),"build_order_inventory":inventory,
        "shadowed_declarations":shadowed,"full_winning_ir_equal_to_embedded":true,
        "winning_ir_sha256":sha(&serde_json::to_vec(&value(winners)?).map_err(|e|e.to_string())?),
        "duplicate_source_authority":"UNPROVEN","build_behavior":"SORTED_PATHS_LAST_DECLARATION_WINS_EXISTING_API",
        "runtime_admission":false}),
    )
}
fn source_binding(
    rows: &Value,
    report: &Value,
    global: &Value,
    comparison: &Value,
    records_sha: &str,
) -> Result<BTreeSet<String>> {
    require(
        report["generated_cards_sha256"].as_str() == Some(records_sha),
        "report record byte hash differs",
    )?;
    require(
        &report["registry"] == global,
        "report/global registry differs",
    )?;
    let ids = |v: &Value| -> Result<BTreeSet<String>> {
        let a = v.as_array().ok_or("scope array")?;
        let mut s = BTreeSet::new();
        for x in a {
            let id = x.as_str().ok_or("scope string")?;
            require(s.insert(id.into()), "duplicate scope member")?;
        }
        Ok(s)
    };
    let e = ids(&report["closure"]["E"])?;
    let q = ids(&report["closure"]["Q"])?;
    let scope = e.union(&q).cloned().collect::<BTreeSet<_>>();
    require(
        object(rows, "rows")?
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            == scope,
        "source selected membership",
    )?;
    let old = object(comparison, "original comparison")?;
    let assignments = registry_wire(global)?;
    require(
        old.keys().eq(assignments.keys()),
        "complete original comparison membership differs",
    )?;
    let original = object(
        &report["closure"]["original_runtime_indices"],
        "original indices",
    )?;
    require(
        original.keys().eq(assignments.keys()),
        "complete original index membership differs",
    )?;
    for (id, n) in assignments {
        require(
            &old[id]["index"] == n && &original[id]["index"] == n,
            "original index changed",
        )?;
    }
    require(
        object(&report["cards"], "report cards")?
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            == scope,
        "report card membership",
    )?;
    for id in &scope {
        let projection = object(&report["cards"][id]["normal_projection"], "projection")?;
        require(
            object(&rows[id], "row")?
                .keys()
                .all(|k| projection.contains_key(k)),
            "projection omits supplied field",
        )?;
        for (k, witness) in projection {
            let actual = presence(&rows[id], k);
            require(
                actual["presence"] == witness["presence"]
                    && actual.get("value") == witness.get("value"),
                format!("projection value/presence differs: {id}.{k}"),
            )?;
        }
    }
    Ok(scope)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    require(bytes.len() as u64 <= MAX_OUTPUT, "output byte limit")?;
    checked_path(path, true)?;
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("exclusive {}: {e}", path.display()))?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())
}
fn encode(v: &Value) -> Result<Vec<u8>> {
    encode_limited(v, MAX_OUTPUT)
}
fn encode_limited(v: &Value, limit: u64) -> Result<Vec<u8>> {
    struct Limited {
        bytes: Vec<u8>,
        limit: u64,
    }
    impl Write for Limited {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if (self.bytes.len() as u64)
                .checked_add(b.len() as u64)
                .is_none_or(|n| n > self.limit)
            {
                return Err(std::io::Error::other("serialized output byte limit"));
            }
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Limited {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer_pretty(&mut out, v).map_err(|e| e.to_string())?;
    out.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(out.bytes)
}
fn guarded_inputs(cheap: impl Fn() -> Result<()>, read_all: impl Fn() -> Result<()>) -> Result<()> {
    cheap()?;
    read_all()?;
    cheap()
}
struct OutputOwner {
    root: PathBuf,
    id: PhysicalId,
    members: BTreeMap<String, BoundFile>,
}
impl OutputOwner {
    fn root_id(&self) -> Result<PhysicalId> {
        let f = open_physical(&self.root)?;
        require(
            f.metadata().map_err(|e| e.to_string())?.is_dir(),
            "output root not directory",
        )?;
        Ok(physical_id(&f)?.0)
    }
    fn fast(&self) -> Result<()> {
        require(self.root_id()? == self.id, "output root identity changed")?;
        let actual = fs::read_dir(&self.root)
            .map_err(|e| e.to_string())?
            .map(|e| {
                e.map_err(|e| e.to_string()).and_then(|e| {
                    e.file_name()
                        .into_string()
                        .map_err(|_| "non-UTF8 output member".into())
                })
            })
            .collect::<Result<BTreeSet<_>>>()?;
        require(
            actual == self.members.keys().cloned().collect(),
            "output phase membership differs",
        )?;
        for b in self.members.values() {
            b.fast()?;
        }
        stop_absent(&self.root.join("STOP"))
    }
    fn full(&self) -> Result<()> {
        self.fast()?;
        for b in self.members.values() {
            b.check(None)?;
        }
        self.fast()
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<()> {
        self.fast()?;
        let path = self.root.join(name);
        write_new(&path, bytes)?;
        let d = Descriptor {
            path: path.to_string_lossy().into(),
            bytes: bytes.len() as u64,
            sha256: sha(bytes),
        };
        let bound = BoundFile::bind(&d)?;
        bound.check(None)?;
        self.members.insert(name.into(), bound);
        self.fast()
    }
}
#[cfg(test)]
fn publish_diagnostic(
    out: &Path,
    expected: &str,
    guard: impl Fn() -> Result<()>,
    produce: impl FnOnce() -> Result<Value>,
) -> Result<()> {
    publish_with_controls(out, expected, guard, || Ok(()), produce)
}
fn publish_with_controls(
    out: &Path,
    expected: &str,
    guard: impl Fn() -> Result<()>,
    terminal: impl Fn() -> Result<()>,
    produce: impl FnOnce() -> Result<Value>,
) -> Result<()> {
    checked_path(out, true)?;
    guard()?;
    terminal()?;
    fs::create_dir(out).map_err(|e| format!("fresh output required: {e}"))?;
    let id = physical_id(&open_physical(out)?)?.0;
    let mut owned = OutputOwner {
        root: out.to_path_buf(),
        id,
        members: BTreeMap::new(),
    };
    let owner = encode(
        &json!({"pid":std::process::id(),"request_sha256":expected,"kind":"native-diagnostic-owner-v2"}),
    )?;
    owned.save("OWNER.json", &owner)?;
    terminal()?;
    owned.fast()?;
    owned.save("START.json", &owner)?;
    terminal()?;
    owned.fast()?;
    let boundary = |state: &OutputOwner| -> Result<()> {
        guard()?;
        state.full()?;
        terminal()?;
        state.fast()?;
        terminal()
    };
    let result = (|| -> Result<()> {
        boundary(&owned)?; // All controlled input/owned reads finish before conversion.
        let bytes = encode(&produce()?)?;
        let completed = encode(
            &json!({"kind":"native-diagnostic-completed-v2","diagnostic_sha256":sha(&bytes),"request_sha256":expected,
            "worker_exit_requires_external_receipt":true,"admission":{"source":false,"runtime":false,"oracle":false,"training":false},"native_admission":false,"projection_epoch":null}),
        )?;
        boundary(&owned)?;
        owned.save("DIAGNOSTIC.json", &bytes)?;
        terminal()?;
        owned.fast()?;
        terminal()?;
        boundary(&owned)?;
        owned.save("COMPLETED.json", &completed)?;
        // Completion bytes are not process success. Retain them as an orphan
        // and return failure if these post-durable-write controls refuse.
        guard()?;
        owned.full()?;
        terminal()?;
        owned.fast()?;
        terminal()
    })();
    if let Err(ref error) = result {
        if !owned.members.contains_key("COMPLETED.json") && boundary(&owned).is_ok() {
            let bytes = encode(
                &json!({"error":error,"admission":{"source":false,"runtime":false,"oracle":false,"training":false},"native_admission":false,"projection_epoch":null}),
            )?;
            if owned.save("FAILED.json", &bytes).is_ok() {
                let _ = owned
                    .full()
                    .and_then(|_| terminal())
                    .and_then(|_| owned.fast())
                    .and_then(|_| terminal());
            }
        }
    }
    result
}
fn execute(request_path: &Path, expected: &str, out: &Path) -> Result<()> {
    require(
        request_path.is_absolute() && out.is_absolute(),
        "absolute request/output required",
    )?;
    let request_meta = stamp(&open_physical(request_path)?)?;
    require(request_meta.len <= MAX_REQUEST, "request byte limit")?;
    let request_file = BoundFile::bind(&Descriptor {
        path: request_path.to_string_lossy().into(),
        bytes: request_meta.len,
        sha256: expected.into(),
    })?;
    let request_bytes = request_file.check(Some(MAX_REQUEST))?;
    let request: Request =
        serde_json::from_value(strict_json(&request_bytes)?).map_err(|e| e.to_string())?;
    require(
        request.schema_version == 1 && request.kind == "native-conversion-diagnostic-request-v1",
        "request profile",
    )?;
    require(
        Path::new(&request.stop_path).is_absolute(),
        "absolute STOP required",
    )?;
    let cheap = || -> Result<()> {
        request_file.check(None)?;
        request_file.fast()?;
        stop_absent(Path::new(&request.stop_path))?;
        stop_absent(&out.join("STOP"))
    };
    let terminal = || -> Result<()> {
        request_file.fast()?;
        stop_absent(Path::new(&request.stop_path))?;
        stop_absent(&out.join("STOP"))
    };
    cheap()?;
    let bindings = InputBindings::bind(&request, &request_file, out)?;
    let guard = || guarded_inputs(cheap, || bindings.verify());
    guard()?;
    require(
        Descriptor::from_path(&std::env::current_exe().map_err(|e| e.to_string())?)?.sha256
            == request.executable.sha256,
        "running executable differs",
    )?;
    require(
        request.pack.sha256 == sha(PACK) && request.pack.bytes == PACK.len() as u64,
        "embedded pack pin differs",
    )?;
    let rows = strict_json(&request.records.read()?)?;
    let report = strict_json(&request.source_report.read()?)?;
    let global = strict_json(&request.registry.read()?)?;
    let comparison = strict_json(&request.comparison_runtime.read()?)?;
    let scope = source_binding(
        &rows,
        &report,
        &global,
        &comparison,
        &request.records.sha256,
    )?;
    require(
        scope.len() == 99 && registry_wire(&global)?.len() == 4483,
        "actual CLI requires complete99/4483 profile",
    )?;
    require(
        report["closure"]["manifest_sha256"].as_str() == Some(&request.closure_manifest.sha256),
        "closure identity differs",
    )?;
    let completion = strict_json(&request.generation_completion.read()?)?;
    require(
        completion["files"]["diagnostic-cards.json"]["sha256"].as_str()
            == Some(&request.records.sha256)
            && completion["files"]["focused-source-report.json"]["sha256"].as_str()
                == Some(&request.source_report.sha256),
        "generation completion differs",
    )?;
    let exit = strict_json(&request.generation_natural_exit.read()?)?;
    require(
        exit["actual_natural_exit"] == json!(0)
            && exit["completion"]["sha256"].as_str() == Some(&request.generation_completion.sha256),
        "generation natural exit binding differs",
    )?;
    let build = strict_json(&request.build_receipt.read()?)?;
    build_binding(
        &build,
        &request.dependencies,
        &request.yaml_inputs,
        &request.executable,
        &request.pack,
    )?;
    let raw = digimon_engine::cards::raw_rust::build_registry();
    let pack = checked_pack(PACK, &raw)?;
    let declarations = read_yaml_in_build_order(
        &request.yaml_inputs,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("cards"),
    )?;
    let yaml_binding = yaml_pack_binding(&declarations, &pack)?;
    let production = digimon_engine::dsl_registry::from_embedded_with_raw_registry(&raw)?;
    require(
        production.len() == pack.cards.len()
            && production.manifest == pack.manifest
            && pack
                .cards
                .iter()
                .all(|c| production.lookup(&c.card) == Some(c)),
        "production embedded registry differs",
    )?;
    // Read immutable source bytes now, before the final guarded conversion
    // boundary; the closure performs no unguarded input-file read.
    let record_bytes = request.records.read()?;
    publish_with_controls(out, expected, guard, terminal, || {
        let mut diagnostic = diagnose_bytes(&record_bytes, &global, &scope, &pack)?;
        diagnostic["request_sha256"] = json!(expected);
        diagnostic["actual_embedded_pack_sha256"] = json!(sha(PACK));
        diagnostic["source_report_sha256"] = json!(request.source_report.sha256);
        diagnostic["retained_closure"] = report["closure"].clone();
        diagnostic["retained_raw_transport_obligations"] =
            report["raw_transport_obligations"].clone();
        diagnostic["input_descriptors"] = value(request.inputs())?;
        diagnostic["physical_role_bindings"] = bindings.report();
        diagnostic["resource_limits"] = json!({"unique_descriptors":MAX_DESCRIPTORS,"descriptor_roles":MAX_ROLES,"per_file_bytes":MAX_FILE,"total_input_bytes":MAX_TOTAL,"request_bytes":MAX_REQUEST,"json_body_bytes":MAX_JSON,"json_nodes":MAX_JSON_NODES,"output_bytes":MAX_OUTPUT});
        diagnostic["yaml_pack_binding"] = yaml_binding;
        diagnostic["history_limits"] = json!({"deck_seat_records":318,"mirrored_executions_proven":false,"original_missing_raw_obligations":9009,"seven_qa_additions_unapplied":true,"task_2_3_complete":false});
        Ok(diagnostic)
    })
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = if args.len() == 7
        && args[1] == "--request"
        && args[3] == "--request-sha256"
        && args[5] == "--output"
    {
        std::thread::Builder::new()
            .stack_size(268435456)
            .spawn(move || execute(Path::new(&args[2]), &args[4], Path::new(&args[6])))
            .map_err(|e| e.to_string())
            .and_then(|h| h.join().map_err(|_| "diagnostic panicked".to_string()))
            .and_then(|r| r)
    } else {
        Err("usage: native_conversion_diagnostic --request ABSOLUTE --request-sha256 SHA256 --output NEW_ABSOLUTE_DIRECTORY".into())
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1)
    }
}

#[cfg(test)]
#[path = "native_conversion_diagnostic/controls.rs"]
mod controls;
