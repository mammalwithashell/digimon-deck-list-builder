#![allow(unused_variables)]
//! Typed occurrence inventory only. No source authority, registry resolution or admission.
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
#[path = "reference_formulas.rs"]
mod formulas;
#[path = "reference_predicates.rs"]
mod predicates;
#[path = "reference_steps.rs"]
mod steps;
#[derive(Debug, Serialize)]
pub struct Node {
    pub path: String,
    pub node_type: String,
    pub role: String,
    pub value: Value,
}
#[derive(Debug, Serialize)]
pub struct Pass {
    pub traversal_complete: bool,
    pub nodes: Vec<Node>,
    pub blockers: BTreeSet<String>,
}
pub struct Collector {
    pub nodes: Vec<Node>,
    pub blockers: BTreeSet<String>,
    pub complete: bool,
}
impl Collector {
    fn new() -> Self {
        Self {
            nodes: vec![],
            blockers: BTreeSet::new(),
            complete: true,
        }
    }
    fn node(&mut self, path: &str, typ: &str, role: &str, value: Value) {
        if role == "TOKEN_CONSTRUCTION"
            || role.starts_with("HOOK_")
            || role.starts_with("ENGINE_")
            || role == "UNCLASSIFIED_STRING"
        {
            self.blockers.insert(format!("UNRESOLVED_{role}"));
        }
        self.nodes.push(Node {
            path: path.into(),
            node_type: typ.into(),
            role: role.into(),
            value,
        });
    }
    fn unknown(&mut self, path: &str, code: &str, detail: String) {
        self.complete = false;
        self.blockers.insert(code.into());
        self.node(path, "unresolved", code, json!(detail));
    }
    fn finish(self) -> Pass {
        Pass {
            traversal_complete: self.complete,
            nodes: self.nodes,
            blockers: self.blockers,
        }
    }
}
pub trait Walk {
    fn walk(&self, c: &mut Collector, path: &str, role: &str);
}
impl Walk for String {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "String", role, json!(self));
    }
}
macro_rules! scalar {($($t:ty),*)=>{$(impl Walk for $t {fn walk(&self,c:&mut Collector,path:&str,_role:&str){c.node(path,stringify!($t),"SCALAR_METADATA",json!(self));}})*};}
scalar!(bool, u8, u16, u32, u64, usize, i8, i16, i32, i64, f32, f64);
impl<T: Walk> Walk for Option<T> {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        match self {
            Some(v) => v.walk(c, path, role),
            None => c.node(path, "Option", "ABSENT", Value::Null),
        }
    }
}
impl<T: Walk> Walk for Vec<T> {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "Vec", "ORDERED_CONTAINER", json!(self.len()));
        for (i, v) in self.iter().enumerate() {
            v.walk(c, &format!("{path}[{i}]"), role);
        }
    }
}
impl<T: Walk> Walk for Box<T> {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        self.as_ref().walk(c, path, role);
    }
}
impl<A: Walk, B: Walk> Walk for (A, B) {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        self.0.walk(c, &format!("{path}.0"), role);
        self.1.walk(c, &format!("{path}.1"), role);
    }
}
impl<A: Walk, B: Walk, C: Walk> Walk for (A, B, C) {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        self.0.walk(c, &format!("{path}.0"), role);
        self.1.walk(c, &format!("{path}.1"), role);
        self.2.walk(c, &format!("{path}.2"), role);
    }
}
impl<T: Walk> Walk for indexmap::IndexMap<String, T> {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "IndexMap", "ORDERED_CONTAINER", json!(self.len()));
        for (i, (k, v)) in self.iter().enumerate() {
            k.walk(c, &format!("{path}[{i}].key"), "UNCLASSIFIED_STRING");
            v.walk(c, &format!("{path}[{i}].value"), role);
        }
    }
}
impl Walk for serde_yml::Value {
    fn walk(&self, c: &mut Collector, path: &str, _role: &str) {
        c.unknown(path, "OPAQUE_AUTHORED_VALUE", format!("{self:?}"));
    }
}
pub fn visit(
    spec: &digimon_dsl::CardSpec,
    compiled: &digimon_dsl::compiled::CompiledCard,
) -> (Pass, Pass) {
    let mut a = Collector::new();
    spec.walk(&mut a, "authored", "UNCLASSIFIED_STRING");
    let mut b = Collector::new();
    compiled.walk(&mut b, "compiled", "UNCLASSIFIED_STRING");
    (a.finish(), b.finish())
}
// Explicit typed patterns: all fields and variants, no rest/wildcard patterns.
// Roles follow typed field semantics, never string values. ENGINE_* names remain
// unresolved until a separate engine witness proves lookup and behavior. Identity
// categories and binding slots do not assert card construction or source authority.
impl Walk for digimon_dsl::spec::CardSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::spec::CardSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::spec::CardSpec {
            card,
            name,
            kind,
            level,
            color,
            cost,
            dp,
            traits,
            form,
            attribute,
            ace_overflow,
            identity,
            digixros_aliases,
            also_treated_as,
            dual,
            use_requirement,
            alt_paths,
            effects,
            spec_version,
            deck_rule,
        } = self;
        card.walk(c, &format!("{}.card", path), "OWNER_IDENTITY");
        name.walk(c, &format!("{}.name", path), "OWNER_IDENTITY");
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        color.walk(c, &format!("{}.color", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        traits.walk(c, &format!("{}.traits", path), "MATCH_OR_DEFINE_TRAIT");
        form.walk(c, &format!("{}.form", path), "CARD_FORM_DEFINITION");
        attribute.walk(c, &format!("{}.attribute", path), "CARD_ATTRIBUTE_DEFINITION");
        ace_overflow.walk(c, &format!("{}.ace_overflow", path), "UNCLASSIFIED_STRING");
        identity.walk(c, &format!("{}.identity", path), "UNCLASSIFIED_STRING");
        digixros_aliases.walk(
            c,
            &format!("{}.digixros_aliases", path),
            "DIGIXROS_NAME_DEFINITION",
        );
        also_treated_as.walk(
            c,
            &format!("{}.also_treated_as", path),
            "STATIC_NAME_DEFINITION",
        );
        dual.walk(c, &format!("{}.dual", path), "UNCLASSIFIED_STRING");
        use_requirement.walk(
            c,
            &format!("{}.use_requirement", path),
            "UNCLASSIFIED_STRING",
        );
        alt_paths.walk(c, &format!("{}.alt_paths", path), "UNCLASSIFIED_STRING");
        effects.walk(c, &format!("{}.effects", path), "UNCLASSIFIED_STRING");
        spec_version.walk(c, &format!("{}.spec_version", path), "UNCLASSIFIED_STRING");
            deck_rule.walk(c, &format!("{}.deck_rule", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::spec::AttributeSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::spec::AttributeSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::spec::AttributeSpec::One(f0) => {
                c.node(
                    path,
                    "digimon_dsl::spec::AttributeSpec",
                    "VARIANT",
                    serde_json::json!("One"),
                );
                f0.walk(c, &format!("{}@One.0", path), role);
            }
            digimon_dsl::spec::AttributeSpec::Many(f0) => {
                c.node(
                    path,
                    "digimon_dsl::spec::AttributeSpec",
                    "VARIANT",
                    serde_json::json!("Many"),
                );
                f0.walk(c, &format!("{}@Many.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::spec::CardKind {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::spec::CardKind",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::spec::CardKind::Digimon => c.node(
                path,
                "digimon_dsl::spec::CardKind",
                "ENUM_SCALAR",
                serde_json::json!("Digimon"),
            ),
            digimon_dsl::spec::CardKind::Tamer => c.node(
                path,
                "digimon_dsl::spec::CardKind",
                "ENUM_SCALAR",
                serde_json::json!("Tamer"),
            ),
            digimon_dsl::spec::CardKind::Option => c.node(
                path,
                "digimon_dsl::spec::CardKind",
                "ENUM_SCALAR",
                serde_json::json!("Option"),
            ),
            digimon_dsl::spec::CardKind::DigiEgg => c.node(
                path,
                "digimon_dsl::spec::CardKind",
                "ENUM_SCALAR",
                serde_json::json!("DigiEgg"),
            ),
            digimon_dsl::spec::CardKind::Token => c.node(
                path,
                "digimon_dsl::spec::CardKind",
                "ENUM_SCALAR",
                serde_json::json!("Token"),
            ),
            digimon_dsl::spec::CardKind::Dual => c.node(
                path,
                "digimon_dsl::spec::CardKind",
                "ENUM_SCALAR",
                serde_json::json!("Dual"),
            ),
        }
    }
}
impl Walk for digimon_dsl::spec::DualSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::spec::DualSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::spec::DualSpec { digimon, option } = self;
        digimon.walk(c, &format!("{}.digimon", path), "UNCLASSIFIED_STRING");
        option.walk(c, &format!("{}.option", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::spec::DualDigimonSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::spec::DualDigimonSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::spec::DualDigimonSpec {
            level,
            dp,
            colors,
            traits,
            effect_text,
            inherited_text,
            effects,
        } = self;
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        colors.walk(c, &format!("{}.colors", path), "UNCLASSIFIED_STRING");
        traits.walk(c, &format!("{}.traits", path), "MATCH_OR_DEFINE_TRAIT");
        effect_text.walk(
            c,
            &format!("{}.effect_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        inherited_text.walk(
            c,
            &format!("{}.inherited_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        effects.walk(c, &format!("{}.effects", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::spec::DualOptionSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::spec::DualOptionSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::spec::DualOptionSpec {
            use_cost,
            colors,
            effect_text,
            security_text,
            keywords,
            use_requirement,
            effects,
            arts_digivolve,
        } = self;
        use_cost.walk(c, &format!("{}.use_cost", path), "UNCLASSIFIED_STRING");
        colors.walk(c, &format!("{}.colors", path), "UNCLASSIFIED_STRING");
        effect_text.walk(
            c,
            &format!("{}.effect_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        security_text.walk(
            c,
            &format!("{}.security_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        keywords.walk(c, &format!("{}.keywords", path), "ENGINE_KEYWORD_NAME");
        use_requirement.walk(
            c,
            &format!("{}.use_requirement", path),
            "UNCLASSIFIED_STRING",
        );
        effects.walk(c, &format!("{}.effects", path), "UNCLASSIFIED_STRING");
        arts_digivolve.walk(
            c,
            &format!("{}.arts_digivolve", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::spec::ColorSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::spec::ColorSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::spec::ColorSpec::Red => c.node(
                path,
                "digimon_dsl::spec::ColorSpec",
                "ENUM_SCALAR",
                serde_json::json!("Red"),
            ),
            digimon_dsl::spec::ColorSpec::Blue => c.node(
                path,
                "digimon_dsl::spec::ColorSpec",
                "ENUM_SCALAR",
                serde_json::json!("Blue"),
            ),
            digimon_dsl::spec::ColorSpec::Yellow => c.node(
                path,
                "digimon_dsl::spec::ColorSpec",
                "ENUM_SCALAR",
                serde_json::json!("Yellow"),
            ),
            digimon_dsl::spec::ColorSpec::Green => c.node(
                path,
                "digimon_dsl::spec::ColorSpec",
                "ENUM_SCALAR",
                serde_json::json!("Green"),
            ),
            digimon_dsl::spec::ColorSpec::Black => c.node(
                path,
                "digimon_dsl::spec::ColorSpec",
                "ENUM_SCALAR",
                serde_json::json!("Black"),
            ),
            digimon_dsl::spec::ColorSpec::Purple => c.node(
                path,
                "digimon_dsl::spec::ColorSpec",
                "ENUM_SCALAR",
                serde_json::json!("Purple"),
            ),
            digimon_dsl::spec::ColorSpec::White => c.node(
                path,
                "digimon_dsl::spec::ColorSpec",
                "ENUM_SCALAR",
                serde_json::json!("White"),
            ),
        }
    }
}
impl Walk for digimon_dsl::identity::IdentitySpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::identity::IdentitySpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::identity::IdentitySpec {
            name_aliases,
            source_name_aliases,
        } = self;
        name_aliases.walk(c, &format!("{}.name_aliases", path), "UNCLASSIFIED_STRING");
        source_name_aliases.walk(
            c,
            &format!("{}.source_name_aliases", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::identity::NameAliasSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::identity::NameAliasSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::identity::NameAliasSpec { treat_as, when } = self;
        treat_as.walk(
            c,
            &format!("{}.treat_as", path),
            "CONDITIONAL_NAME_DEFINITION",
        );
        when.walk(c, &format!("{}.when", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::identity::SourceNameAliasSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::identity::SourceNameAliasSpec",
            "DYNAMIC_SOURCE_NAME_DEFINITION",
            serde_json::Value::Null,
        );
        let digimon_dsl::identity::SourceNameAliasSpec { level_lte } = self;
        level_lte.walk(c, &format!("{}.level_lte", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::identity::AliasCondition {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::identity::AliasCondition",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::identity::AliasCondition {
            zone,
            has_inherited,
        } = self;
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        has_inherited.walk(c, &format!("{}.has_inherited", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::identity::InheritedFilter {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::identity::InheritedFilter",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::identity::InheritedFilter {
            card_number_is,
            name_is,
        } = self;
        card_number_is.walk(c, &format!("{}.card_number_is", path), "MATCH_CARD_ID");
        name_is.walk(c, &format!("{}.name_is", path), "MATCH_NAME_EXACT");
    }
}
impl Walk for digimon_dsl::alt_path::AltPathSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::AltPathSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::alt_path::AltPathSpec {
            kind,
            from,
            materials,
            cost,
            stacks_unsuspended,
            ignore_requirements,
            source_treated_as,
            extra_cost,
            on_burst_turn_end,
            marker,
            condition,
            direction,
            extra_material_zones,
        } = self;
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        materials.walk(c, &format!("{}.materials", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        stacks_unsuspended.walk(
            c,
            &format!("{}.stacks_unsuspended", path),
            "UNCLASSIFIED_STRING",
        );
        ignore_requirements.walk(
            c,
            &format!("{}.ignore_requirements", path),
            "UNCLASSIFIED_STRING",
        );
        source_treated_as.walk(
            c,
            &format!("{}.source_treated_as", path),
            "CONDITIONAL_NAME_DEFINITION",
        );
        extra_cost.walk(c, &format!("{}.extra_cost", path), "UNCLASSIFIED_STRING");
        on_burst_turn_end.walk(
            c,
            &format!("{}.on_burst_turn_end", path),
            "UNCLASSIFIED_STRING",
        );
        marker.walk(c, &format!("{}.marker", path), "UNCLASSIFIED_STRING");
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
        direction.walk(c, &format!("{}.direction", path), "UNCLASSIFIED_STRING");
        extra_material_zones.walk(
            c,
            &format!("{}.extra_material_zones", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::alt_path::ConditionalMaterialZoneSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::ConditionalMaterialZoneSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::alt_path::ConditionalMaterialZoneSpec {
            zone,
            while_condition,
        } = self;
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        while_condition.walk(
            c,
            &format!("{}.while_condition", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::alt_path::AltPathDirection {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::AltPathDirection",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::alt_path::AltPathDirection::From => c.node(
                path,
                "digimon_dsl::alt_path::AltPathDirection",
                "ENUM_SCALAR",
                serde_json::json!("From"),
            ),
            digimon_dsl::alt_path::AltPathDirection::Into => c.node(
                path,
                "digimon_dsl::alt_path::AltPathDirection",
                "ENUM_SCALAR",
                serde_json::json!("Into"),
            ),
        }
    }
}
impl Walk for digimon_dsl::alt_path::AltPathKind {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::AltPathKind",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::alt_path::AltPathKind::Digivolve => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("Digivolve"),
            ),
            digimon_dsl::alt_path::AltPathKind::DnaDigivolve => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("DnaDigivolve"),
            ),
            digimon_dsl::alt_path::AltPathKind::BlastDnaDigivolve => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("BlastDnaDigivolve"),
            ),
            digimon_dsl::alt_path::AltPathKind::DigiXros => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("DigiXros"),
            ),
            digimon_dsl::alt_path::AltPathKind::BurstDigivolve => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("BurstDigivolve"),
            ),
            digimon_dsl::alt_path::AltPathKind::AppFusion => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("AppFusion"),
            ),
            digimon_dsl::alt_path::AltPathKind::Assembly => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("Assembly"),
            ),
            digimon_dsl::alt_path::AltPathKind::ActivatedDigivolve => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("ActivatedDigivolve"),
            ),
            digimon_dsl::alt_path::AltPathKind::CastTimeAssembly => c.node(
                path,
                "digimon_dsl::alt_path::AltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("CastTimeAssembly"),
            ),
        }
    }
}
impl Walk for digimon_dsl::alt_path::CostSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::CostSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::alt_path::CostSpec::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::alt_path::CostSpec",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::alt_path::CostSpec::Formula(f0) => {
                c.node(
                    path,
                    "digimon_dsl::alt_path::CostSpec",
                    "VARIANT",
                    serde_json::json!("Formula"),
                );
                f0.walk(c, &format!("{}@Formula.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::alt_path::FormulaCost {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::FormulaCost",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::alt_path::FormulaCost { formula } = self;
        formula.walk(c, &format!("{}.formula", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::alt_path::MaterialSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::MaterialSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::alt_path::MaterialSpec {
            filter,
            inline_filter,
            repeat,
            distinct_by,
            zones,
            cost_delta,
            stack_under,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        inline_filter.walk(c, &format!("{}.inline_filter", path), "UNCLASSIFIED_STRING");
        repeat.walk(c, &format!("{}.repeat", path), "UNCLASSIFIED_STRING");
        distinct_by.walk(c, &format!("{}.distinct_by", path), "UNCLASSIFIED_STRING");
        zones.walk(c, &format!("{}.zones", path), "UNCLASSIFIED_STRING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
        stack_under.walk(c, &format!("{}.stack_under", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::alt_path::RepeatSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::RepeatSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::alt_path::RepeatSpec::Keyword(f0) => {
                c.node(
                    path,
                    "digimon_dsl::alt_path::RepeatSpec",
                    "VARIANT",
                    serde_json::json!("Keyword"),
                );
                f0.walk(c, &format!("{}@Keyword.0", path), role);
            }
            digimon_dsl::alt_path::RepeatSpec::Range { min, max } => {
                c.node(
                    path,
                    "digimon_dsl::alt_path::RepeatSpec",
                    "VARIANT",
                    serde_json::json!("Range"),
                );
                min.walk(
                    c,
                    &format!("{}.min", &format!("{}@Range", path)),
                    "UNCLASSIFIED_STRING",
                );
                max.walk(
                    c,
                    &format!("{}.max", &format!("{}@Range", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::alt_path::RepeatKeyword {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::RepeatKeyword",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::alt_path::RepeatKeyword::Unbounded => c.node(
                path,
                "digimon_dsl::alt_path::RepeatKeyword",
                "ENUM_SCALAR",
                serde_json::json!("Unbounded"),
            ),
        }
    }
}
impl Walk for digimon_dsl::alt_path::DistinctBy {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::alt_path::DistinctBy",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::alt_path::DistinctBy::CardNumber => c.node(
                path,
                "digimon_dsl::alt_path::DistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("CardNumber"),
            ),
            digimon_dsl::alt_path::DistinctBy::Level => c.node(
                path,
                "digimon_dsl::alt_path::DistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("Level"),
            ),
            digimon_dsl::alt_path::DistinctBy::Name => c.node(
                path,
                "digimon_dsl::alt_path::DistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("Name"),
            ),
            digimon_dsl::alt_path::DistinctBy::Color => c.node(
                path,
                "digimon_dsl::alt_path::DistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("Color"),
            ),
        }
    }
}
impl Walk for digimon_dsl::common::PlayerRef {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::common::PlayerRef",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::common::PlayerRef::You => c.node(
                path,
                "digimon_dsl::common::PlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("You"),
            ),
            digimon_dsl::common::PlayerRef::Opponent => c.node(
                path,
                "digimon_dsl::common::PlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("Opponent"),
            ),
            digimon_dsl::common::PlayerRef::Any => c.node(
                path,
                "digimon_dsl::common::PlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("Any"),
            ),
            digimon_dsl::common::PlayerRef::Active => c.node(
                path,
                "digimon_dsl::common::PlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("Active"),
            ),
        }
    }
}
impl Walk for digimon_dsl::clause::ClauseSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ClauseSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::ClauseSpec::Triggered(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::ClauseSpec",
                    "VARIANT",
                    serde_json::json!("Triggered"),
                );
                f0.walk(c, &format!("{}@Triggered.0", path), role);
            }
            digimon_dsl::clause::ClauseSpec::Declarative(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::ClauseSpec",
                    "VARIANT",
                    serde_json::json!("Declarative"),
                );
                f0.walk(c, &format!("{}@Declarative.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::clause::TriggeredClause {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::TriggeredClause",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::TriggeredClause {
            when,
            scope,
            summary,
            summary_key,
            active_when,
            condition,
            timing_conditions,
            optional,
            outer_prompt,
            once_per_turn,
            max_per_turn,
            process,
            trigger_if,
        } = self;
        when.walk(c, &format!("{}.when", path), "UNCLASSIFIED_STRING");
        scope.walk(c, &format!("{}.scope", path), "UNCLASSIFIED_STRING");
        summary.walk(
            c,
            &format!("{}.summary", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        summary_key.walk(
            c,
            &format!("{}.summary_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        active_when.walk(c, &format!("{}.active_when", path), "UNCLASSIFIED_STRING");
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
        timing_conditions.walk(
            c,
            &format!("{}.timing_conditions", path),
            "UNCLASSIFIED_STRING",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        outer_prompt.walk(c, &format!("{}.outer_prompt", path), "UNCLASSIFIED_STRING");
        once_per_turn.walk(c, &format!("{}.once_per_turn", path), "UNCLASSIFIED_STRING");
        max_per_turn.walk(c, &format!("{}.max_per_turn", path), "UNCLASSIFIED_STRING");
        process.walk(c, &format!("{}.process", path), "UNCLASSIFIED_STRING");
            trigger_if.walk(c, &format!("{}.trigger_if", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::clause::TimingCondition {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::TimingCondition",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::TimingCondition { when, condition } = self;
        when.walk(c, &format!("{}.when", path), "UNCLASSIFIED_STRING");
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::TimingSet {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::TimingSet",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::TimingSet::Single(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TimingSet",
                    "VARIANT",
                    serde_json::json!("Single"),
                );
                f0.walk(c, &format!("{}@Single.0", path), role);
            }
            digimon_dsl::clause::TimingSet::Multi(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TimingSet",
                    "VARIANT",
                    serde_json::json!("Multi"),
                );
                f0.walk(c, &format!("{}@Multi.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::clause::Timing {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::Timing",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::Timing::OnPlay => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnPlay"),
            ),
            digimon_dsl::clause::Timing::WhenDigivolving => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("WhenDigivolving"),
            ),
            digimon_dsl::clause::Timing::WhenAttacking => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("WhenAttacking"),
            ),
            digimon_dsl::clause::Timing::EndOfAttack => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("EndOfAttack"),
            ),
            digimon_dsl::clause::Timing::EndOfBattle => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("EndOfBattle"),
            ),
            digimon_dsl::clause::Timing::OnAttack => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAttack"),
            ),
            digimon_dsl::clause::Timing::OnBlock => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnBlock"),
            ),
            digimon_dsl::clause::Timing::OnAllyAttack => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAllyAttack"),
            ),
            digimon_dsl::clause::Timing::OnOpponentAttack => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnOpponentAttack"),
            ),
            digimon_dsl::clause::Timing::OnDeletion => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDeletion"),
            ),
            digimon_dsl::clause::Timing::OnAnyDeletion => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAnyDeletion"),
            ),
            digimon_dsl::clause::Timing::OnAllyWonBattle => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAllyWonBattle"),
            ),
            digimon_dsl::clause::Timing::OnDiscardHand => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDiscardHand"),
            ),
            digimon_dsl::clause::Timing::OnAddToDeck => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAddToDeck"),
            ),
            digimon_dsl::clause::Timing::OnEnterFieldAnyone => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnEnterFieldAnyone"),
            ),
            digimon_dsl::clause::Timing::OnAnyDigimonPlayed => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAnyDigimonPlayed"),
            ),
            digimon_dsl::clause::Timing::OnAllyPlayed => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAllyPlayed"),
            ),
            digimon_dsl::clause::Timing::OnLeaveField => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnLeaveField"),
            ),
            digimon_dsl::clause::Timing::OnSuspend => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnSuspend"),
            ),
            digimon_dsl::clause::Timing::OnUnsuspend => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnUnsuspend"),
            ),
            digimon_dsl::clause::Timing::OnAddToHand => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAddToHand"),
            ),
            digimon_dsl::clause::Timing::OnHatch => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnHatch"),
            ),
            digimon_dsl::clause::Timing::OnMove => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnMove"),
            ),
            digimon_dsl::clause::Timing::OnDigivolve => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDigivolve"),
            ),
            digimon_dsl::clause::Timing::OnDnaDigivolve => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDnaDigivolve"),
            ),
            digimon_dsl::clause::Timing::OnDigixros => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDigixros"),
            ),
            digimon_dsl::clause::Timing::OnOpponentSecurityRemoved => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnOpponentSecurityRemoved"),
            ),
            digimon_dsl::clause::Timing::OnOwnSecurityRemoved => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnOwnSecurityRemoved"),
            ),
            digimon_dsl::clause::Timing::OnDigivolutionCardTrashed => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDigivolutionCardTrashed"),
            ),
            digimon_dsl::clause::Timing::OnAddDigivolutionCards => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAddDigivolutionCards"),
            ),
            digimon_dsl::clause::Timing::OnSourceReturnedToDeckBottom => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnSourceReturnedToDeckBottom"),
            ),
            digimon_dsl::clause::Timing::OnSecurityCheck => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnSecurityCheck"),
            ),
            digimon_dsl::clause::Timing::OnCheckFaceUpSecurity => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnCheckFaceUpSecurity"),
            ),
            digimon_dsl::clause::Timing::OnLoseSecurity => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnLoseSecurity"),
            ),
            digimon_dsl::clause::Timing::OnDiscardSecurity => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDiscardSecurity"),
            ),
            digimon_dsl::clause::Timing::OnDiscardDeck => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnDiscardDeck"),
            ),
            digimon_dsl::clause::Timing::OnSecurity => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnSecurity"),
            ),
            digimon_dsl::clause::Timing::OnOptionPlaced => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnOptionPlaced"),
            ),
            digimon_dsl::clause::Timing::OnOptionTrashed => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnOptionTrashed"),
            ),
            digimon_dsl::clause::Timing::OnUseOption => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnUseOption"),
            ),
            digimon_dsl::clause::Timing::OnPlaceSecurity => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnPlaceSecurity"),
            ),
            digimon_dsl::clause::Timing::OnAddedToSecurity => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAddedToSecurity"),
            ),
            digimon_dsl::clause::Timing::Main => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("Main"),
            ),
            digimon_dsl::clause::Timing::StartOfYourTurn => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("StartOfYourTurn"),
            ),
            digimon_dsl::clause::Timing::StartOfOpponentsTurn => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("StartOfOpponentsTurn"),
            ),
            digimon_dsl::clause::Timing::StartOfYourMainPhase => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("StartOfYourMainPhase"),
            ),
            digimon_dsl::clause::Timing::EndOfYourTurn => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("EndOfYourTurn"),
            ),
            digimon_dsl::clause::Timing::EndOfOpponentsTurn => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("EndOfOpponentsTurn"),
            ),
            digimon_dsl::clause::Timing::EndOfYourNextTurn => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("EndOfYourNextTurn"),
            ),
            digimon_dsl::clause::Timing::EndOfOpponentsNextTurn => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("EndOfOpponentsNextTurn"),
            ),
            digimon_dsl::clause::Timing::UntilNextUnsuspend => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("UntilNextUnsuspend"),
            ),
            digimon_dsl::clause::Timing::OnAttackTargetChange => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAttackTargetChange"),
            ),
            digimon_dsl::clause::Timing::MainFromHand => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("MainFromHand"),
            ),
            digimon_dsl::clause::Timing::MainOnField => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("MainOnField"),
            ),
            digimon_dsl::clause::Timing::MainFromTrash => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("MainFromTrash"),
            ),
            digimon_dsl::clause::Timing::Counter => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("Counter"),
            ),
            digimon_dsl::clause::Timing::BeforePayCost => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("BeforePayCost"),
            ),
            digimon_dsl::clause::Timing::BeforePayCostObserve => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("BeforePayCostObserve"),
            ),
            digimon_dsl::clause::Timing::Delayed => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("Delayed"),
            ),
            digimon_dsl::clause::Timing::WhenLinked => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("WhenLinked"),
            ),
            digimon_dsl::clause::Timing::WhenCardLinkedToThis => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("WhenCardLinkedToThis"),
            ),
            digimon_dsl::clause::Timing::WhenWouldLinkToThis => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("WhenWouldLinkToThis"),
            ),
            digimon_dsl::clause::Timing::OnAnyLink => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnAnyLink"),
            ),
            digimon_dsl::clause::Timing::OnLinkCardTrashed => c.node(
                path,
                "digimon_dsl::clause::Timing",
                "ENUM_SCALAR",
                serde_json::json!("OnLinkCardTrashed"),
            ),
                    digimon_dsl::clause::Timing::OnTopCardTrashed => c.node(path, "digimon_dsl::clause::Timing", "ENUM_SCALAR", serde_json::json!("OnTopCardTrashed")),
            digimon_dsl::clause::Timing::StartOfOpponentsMainPhase => c.node(path, "digimon_dsl::clause::Timing", "ENUM_SCALAR", serde_json::json!("StartOfOpponentsMainPhase")),
}
    }
}
impl Walk for digimon_dsl::clause::ClauseScope {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ClauseScope",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::ClauseScope::FaceUp => c.node(
                path,
                "digimon_dsl::clause::ClauseScope",
                "ENUM_SCALAR",
                serde_json::json!("FaceUp"),
            ),
            digimon_dsl::clause::ClauseScope::Inherited => c.node(
                path,
                "digimon_dsl::clause::ClauseScope",
                "ENUM_SCALAR",
                serde_json::json!("Inherited"),
            ),
            digimon_dsl::clause::ClauseScope::Both => c.node(
                path,
                "digimon_dsl::clause::ClauseScope",
                "ENUM_SCALAR",
                serde_json::json!("Both"),
            ),
            digimon_dsl::clause::ClauseScope::Linked => c.node(
                path,
                "digimon_dsl::clause::ClauseScope",
                "ENUM_SCALAR",
                serde_json::json!("Linked"),
            ),
            digimon_dsl::clause::ClauseScope::Security => c.node(
                path,
                "digimon_dsl::clause::ClauseScope",
                "ENUM_SCALAR",
                serde_json::json!("Security"),
            ),
            digimon_dsl::clause::ClauseScope::Trash => c.node(
                path,
                "digimon_dsl::clause::ClauseScope",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
        }
    }
}
impl Walk for digimon_dsl::clause::DeclarativeClause {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::DeclarativeClause",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::DeclarativeClause {
            kind,
            scope,
            active_when,
            summary,
            summary_key,
            body,
        } = self;
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        scope.walk(c, &format!("{}.scope", path), "UNCLASSIFIED_STRING");
        active_when.walk(c, &format!("{}.active_when", path), "UNCLASSIFIED_STRING");
        summary.walk(
            c,
            &format!("{}.summary", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        summary_key.walk(
            c,
            &format!("{}.summary_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        c.node(
            &format!("{}.body", path),
            "native_typed_body",
            "ORIGINAL_VALUE_RETAINED",
            serde_json::to_value(body).unwrap(),
        );
        match self.typed_body() {
            Ok(v) => v.walk(c, &format!("{}.body", path), role),
            Err(e) => c.unknown(path, "INVALID_TYPED_BODY", e.to_string()),
        };
    }
}
impl Walk for digimon_dsl::clause::DeclarativeKind {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::DeclarativeKind",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::DeclarativeKind::Aura => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("Aura"),
            ),
            digimon_dsl::clause::DeclarativeKind::CostReduction => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("CostReduction"),
            ),
            digimon_dsl::clause::DeclarativeKind::Replacement => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("Replacement"),
            ),
            digimon_dsl::clause::DeclarativeKind::Partition => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("Partition"),
            ),
            digimon_dsl::clause::DeclarativeKind::AceOverflow => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("AceOverflow"),
            ),
            digimon_dsl::clause::DeclarativeKind::GrantKeyword => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("GrantKeyword"),
            ),
            digimon_dsl::clause::DeclarativeKind::Delay => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("Delay"),
            ),
            digimon_dsl::clause::DeclarativeKind::LinkRequirement => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("LinkRequirement"),
            ),
            digimon_dsl::clause::DeclarativeKind::LinkCondition => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("LinkCondition"),
            ),
            digimon_dsl::clause::DeclarativeKind::FloodGate => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("FloodGate"),
            ),
            digimon_dsl::clause::DeclarativeKind::AltPathRegistration => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("AltPathRegistration"),
            ),
            digimon_dsl::clause::DeclarativeKind::RawRust => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("RawRust"),
            ),
            digimon_dsl::clause::DeclarativeKind::Succession => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("Succession"),
            ),
            digimon_dsl::clause::DeclarativeKind::UseCostIncrease => c.node(
                path,
                "digimon_dsl::clause::DeclarativeKind",
                "ENUM_SCALAR",
                serde_json::json!("UseCostIncrease"),
            ),
        }
    }
}
impl Walk for digimon_dsl::clause::TypedDeclarativeBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::TypedDeclarativeBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::TypedDeclarativeBody::Aura(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("Aura"),
                );
                f0.walk(c, &format!("{}@Aura.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::CostReduction(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("CostReduction"),
                );
                f0.walk(c, &format!("{}@CostReduction.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::Replacement(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("Replacement"),
                );
                f0.walk(c, &format!("{}@Replacement.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::Partition(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("Partition"),
                );
                f0.walk(c, &format!("{}@Partition.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::AceOverflow(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("AceOverflow"),
                );
                f0.walk(c, &format!("{}@AceOverflow.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::GrantKeyword(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("GrantKeyword"),
                );
                f0.walk(c, &format!("{}@GrantKeyword.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::Delay(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("Delay"),
                );
                f0.walk(c, &format!("{}@Delay.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::LinkRequirement(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("LinkRequirement"),
                );
                f0.walk(c, &format!("{}@LinkRequirement.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::LinkCondition(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("LinkCondition"),
                );
                f0.walk(c, &format!("{}@LinkCondition.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::FloodGate(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("FloodGate"),
                );
                f0.walk(c, &format!("{}@FloodGate.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::AltPathRegistration(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("AltPathRegistration"),
                );
                f0.walk(c, &format!("{}@AltPathRegistration.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::RawRust(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("RawRust"),
                );
                f0.walk(c, &format!("{}@RawRust.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::Succession(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("Succession"),
                );
                f0.walk(c, &format!("{}@Succession.0", path), role);
            }
            digimon_dsl::clause::TypedDeclarativeBody::UseCostIncrease(f0) => {
                c.node(
                    path,
                    "digimon_dsl::clause::TypedDeclarativeBody",
                    "VARIANT",
                    serde_json::json!("UseCostIncrease"),
                );
                f0.walk(c, &format!("{}@UseCostIncrease.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::clause::AuraBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::AuraBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::AuraBody {
            target,
            target_player,
            dp_modifier,
            security_attack,
            grant_keyword,
            grant_traits,
            modifier,
            modifier_value,
            modifier_name,
            synth_identity,
            while_condition,
            applies_to_opponent_security_dp,
            applies_to_own_security_dp,
            effect_immunity,
            modifier_from,
            dna_material_identity,
            source_card_trash_protection,
        } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        target_player.walk(c, &format!("{}.target_player", path), "UNCLASSIFIED_STRING");
        dp_modifier.walk(c, &format!("{}.dp_modifier", path), "UNCLASSIFIED_STRING");
        security_attack.walk(
            c,
            &format!("{}.security_attack", path),
            "UNCLASSIFIED_STRING",
        );
        grant_keyword.walk(c, &format!("{}.grant_keyword", path), "UNCLASSIFIED_STRING");
        grant_traits.walk(
            c,
            &format!("{}.grant_traits", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        modifier.walk(c, &format!("{}.modifier", path), "ENGINE_MODIFIER_NAME");
        modifier_value.walk(
            c,
            &format!("{}.modifier_value", path),
            "UNCLASSIFIED_STRING",
        );
        modifier_name.walk(c, &format!("{}.modifier_name", path), "UNCLASSIFIED_STRING");
        synth_identity.walk(
            c,
            &format!("{}.synth_identity", path),
            "UNCLASSIFIED_STRING",
        );
        while_condition.walk(
            c,
            &format!("{}.while_condition", path),
            "UNCLASSIFIED_STRING",
        );
        applies_to_opponent_security_dp.walk(
            c,
            &format!("{}.applies_to_opponent_security_dp", path),
            "UNCLASSIFIED_STRING",
        );
        applies_to_own_security_dp.walk(
            c,
            &format!("{}.applies_to_own_security_dp", path),
            "UNCLASSIFIED_STRING",
        );
        effect_immunity.walk(
            c,
            &format!("{}.effect_immunity", path),
            "UNCLASSIFIED_STRING",
        );
        modifier_from.walk(c, &format!("{}.modifier_from", path), "UNCLASSIFIED_STRING");
        dna_material_identity.walk(
            c,
            &format!("{}.dna_material_identity", path),
            "UNCLASSIFIED_STRING",
        );
            source_card_trash_protection.walk(c, &format!("{}.source_card_trash_protection", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::clause::DnaMaterialIdentitySpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::DnaMaterialIdentitySpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::DnaMaterialIdentitySpec {
            for_result,
            level,
            name,
            result_scope,
        } = self;
        for_result.walk(c, &format!("{}.for_result", path), "MATCH_DNA_RESULT_NAME");
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        name.walk(c, &format!("{}.name", path), "CONDITIONAL_NAME_DEFINITION");
            result_scope.walk(c, &format!("{}.result_scope", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::clause::AuraEffectImmunity {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::AuraEffectImmunity",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::AuraEffectImmunity {
            source_kind,
            source_controller,
        } = self;
        source_kind.walk(c, &format!("{}.source_kind", path), "UNCLASSIFIED_STRING");
        source_controller.walk(
            c,
            &format!("{}.source_controller", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::clause::GrantKeywordValue {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::GrantKeywordValue",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::GrantKeywordValue { keyword, value } = self;
        keyword.walk(c, &format!("{}.keyword", path), "ENGINE_KEYWORD_NAME");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::CostReductionBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::CostReductionBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::CostReductionBody {
            reduction_timing,
            when_playing_this,
            when_digivolving_this,
            when_any_ally_played,
            when_any_ally_digivolves_into,
            condition,
            optional,
            once_per_turn,
            amount,
            pay_cost,
            unlocks,
            set_cost,
        } = self;
        reduction_timing.walk(
            c,
            &format!("{}.reduction_timing", path),
            "ENGINE_TIMING_NAME",
        );
        when_playing_this.walk(
            c,
            &format!("{}.when_playing_this", path),
            "UNCLASSIFIED_STRING",
        );
        when_digivolving_this.walk(c, &format!("{}.when_digivolving_this", path), "UNCLASSIFIED_STRING");
        when_any_ally_played.walk(
            c,
            &format!("{}.when_any_ally_played", path),
            "UNCLASSIFIED_STRING",
        );
        when_any_ally_digivolves_into.walk(
            c,
            &format!("{}.when_any_ally_digivolves_into", path),
            "UNCLASSIFIED_STRING",
        );
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        once_per_turn.walk(c, &format!("{}.once_per_turn", path), "UNCLASSIFIED_STRING");
        amount.walk(c, &format!("{}.amount", path), "UNCLASSIFIED_STRING");
        pay_cost.walk(c, &format!("{}.pay_cost", path), "UNCLASSIFIED_STRING");
        unlocks.walk(c, &format!("{}.unlocks", path), "UNCLASSIFIED_STRING");
            set_cost.walk(c, &format!("{}.set_cost", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::clause::ReplacementBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReplacementBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::ReplacementBody {
            trigger,
            timing,
            optional,
            once_per_turn,
            cost,
            choose,
            outcome,
            then_steps,
            process,
            prevent_all_matching,
        } = self;
        trigger.walk(c, &format!("{}.trigger", path), "ENGINE_TIMING_NAME");
        timing.walk(c, &format!("{}.timing", path), "ENGINE_TIMING_NAME");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        once_per_turn.walk(c, &format!("{}.once_per_turn", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        choose.walk(c, &format!("{}.choose", path), "UNCLASSIFIED_STRING");
        outcome.walk(c, &format!("{}.outcome", path), "UNCLASSIFIED_STRING");
        then_steps.walk(c, &format!("{}.then_steps", path), "UNCLASSIFIED_STRING");
        process.walk(c, &format!("{}.process", path), "UNCLASSIFIED_STRING");
            prevent_all_matching.walk(c, &format!("{}.prevent_all_matching", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::clause::ReplacementCostBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReplacementCostBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::ReplacementCostBody {
            delay_self,
            trash_own_link_card,
            link_card_filter,
            place_link_card_as_bottom_digivolution,
            return_own_sources_to_deck,
            delete_self,
            trash_option_from_own_digivolution_cards,
        } = self;
        delay_self.walk(c, &format!("{}.delay_self", path), "UNCLASSIFIED_STRING");
        trash_own_link_card.walk(
            c,
            &format!("{}.trash_own_link_card", path),
            "UNCLASSIFIED_STRING",
        );
        link_card_filter.walk(
            c,
            &format!("{}.link_card_filter", path),
            "UNCLASSIFIED_STRING",
        );
        place_link_card_as_bottom_digivolution.walk(
            c,
            &format!("{}.place_link_card_as_bottom_digivolution", path),
            "UNCLASSIFIED_STRING",
        );
        return_own_sources_to_deck.walk(
            c,
            &format!("{}.return_own_sources_to_deck", path),
            "UNCLASSIFIED_STRING",
        );
        delete_self.walk(c, &format!("{}.delete_self", path), "UNCLASSIFIED_STRING");
        trash_option_from_own_digivolution_cards.walk(
            c,
            &format!("{}.trash_option_from_own_digivolution_cards", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::clause::ReturnOwnSourcesToDeckCost {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReturnOwnSourcesToDeckCost",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::ReturnOwnSourcesToDeckCost {
            filter,
            count,
            position,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        count.walk(c, &format!("{}.count", path), "UNCLASSIFIED_STRING");
        position.walk(c, &format!("{}.position", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::ReplacementChooseBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReplacementChooseBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::ReplacementChooseBody {
            from_zone,
            card_filter,
            min,
            max,
        } = self;
        from_zone.walk(c, &format!("{}.from_zone", path), "UNCLASSIFIED_STRING");
        card_filter.walk(c, &format!("{}.card_filter", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::ReplacementChooseFrom {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReplacementChooseFrom",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::ReplacementChooseFrom::Hand => c.node(
                path,
                "digimon_dsl::clause::ReplacementChooseFrom",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
        }
    }
}
impl Walk for digimon_dsl::clause::ReplacementOutcome {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReplacementOutcome",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::clause::ReplacementOutcome::Prevent => c.node(
                path,
                "digimon_dsl::clause::ReplacementOutcome",
                "ENUM_SCALAR",
                serde_json::json!("Prevent"),
            ),
        }
    }
}
impl Walk for digimon_dsl::clause::ReplacementThenStep {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReplacementThenStep",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::ReplacementThenStep {
            digivolve_without_cost,
        } = self;
        digivolve_without_cost.walk(
            c,
            &format!("{}.digivolve_without_cost", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::clause::ReplacementDigivolveWithoutCostBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::ReplacementDigivolveWithoutCostBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::ReplacementDigivolveWithoutCostBody { target, card } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        card.walk(c, &format!("{}.card", path), "EXISTING_INSTANCE_BINDING");
    }
}
impl Walk for digimon_dsl::clause::PartitionBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::PartitionBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::PartitionBody {
            sources,
            exclude_cause,
            process,
        } = self;
        sources.walk(c, &format!("{}.sources", path), "UNCLASSIFIED_STRING");
        exclude_cause.walk(c, &format!("{}.exclude_cause", path), "UNCLASSIFIED_STRING");
        process.walk(c, &format!("{}.process", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::AceOverflowBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::AceOverflowBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::AceOverflowBody { value } = self;
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::GrantKeywordBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::GrantKeywordBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::GrantKeywordBody {
            keyword,
            value,
            overclock_cost_filter,
        } = self;
        keyword.walk(c, &format!("{}.keyword", path), "ENGINE_KEYWORD_NAME");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
        overclock_cost_filter.walk(
            c,
            &format!("{}.overclock_cost_filter", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::clause::DelayBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::DelayBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::DelayBody {
            trigger,
            active_when,
            process,
        } = self;
        trigger.walk(c, &format!("{}.trigger", path), "ENGINE_TIMING_NAME");
        active_when.walk(c, &format!("{}.active_when", path), "UNCLASSIFIED_STRING");
        process.walk(c, &format!("{}.process", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::LinkRequirementBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::LinkRequirementBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::LinkRequirementBody { cost, filter } = self;
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::FloodGateBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::FloodGateBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::FloodGateBody {
            modifier,
            target,
            target_player,
            expiry,
        } = self;
        modifier.walk(c, &format!("{}.modifier", path), "ENGINE_MODIFIER_NAME");
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        target_player.walk(c, &format!("{}.target_player", path), "UNCLASSIFIED_STRING");
        expiry.walk(c, &format!("{}.expiry", path), "ENGINE_EXPIRY_NAME");
    }
}
impl Walk for digimon_dsl::clause::SuccessionBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::SuccessionBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::SuccessionBody { filter } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::UseCostIncreaseBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::UseCostIncreaseBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::UseCostIncreaseBody { amount } = self;
        amount.walk(c, &format!("{}.amount", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::AltPathRegistrationBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::AltPathRegistrationBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::AltPathRegistrationBody {
            trigger,
            registers,
            applies_to,
        } = self;
        trigger.walk(c, &format!("{}.trigger", path), "ENGINE_TIMING_NAME");
        c.node(
            &format!("{}.registers", path),
            "native_alt_path",
            "ORIGINAL_VALUE_RETAINED",
            serde_json::to_value(registers).unwrap(),
        );
        let v = serde_yml::Value::Mapping(
            registers
                .iter()
                .map(|(k, v)| (serde_yml::Value::String(k.clone()), v.clone()))
                .collect(),
        );
        match serde_yml::from_value::<digimon_dsl::alt_path::AltPathSpec>(v) {
            Ok(v) => v.walk(c, &format!("{}.registers", path), role),
            Err(e) => c.unknown(path, "INVALID_TYPED_ALT_PATH", e.to_string()),
        };
        applies_to.walk(c, &format!("{}.applies_to", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::clause::RawRustClauseBody {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::clause::RawRustClauseBody",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::clause::RawRustClauseBody { fn_name, triggers } = self;
        fn_name.walk(c, &format!("{}.fn_name", path), "HOOK_DECLARATIVE");
        triggers.walk(c, &format!("{}.triggers", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledCard {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledCard",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledCard {
            card,
            name,
            kind,
            level,
            color,
            cost,
            dp,
            traits,
            form,
            attribute,
            ace_overflow,
            identity,
            digixros_aliases,
            also_treated_as,
            dual,
            use_requirement,
            alt_paths,
            effects,
            deck_rule,
        } = self;
        card.walk(c, &format!("{}.card", path), "OWNER_IDENTITY");
        name.walk(c, &format!("{}.name", path), "OWNER_IDENTITY");
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        color.walk(c, &format!("{}.color", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        traits.walk(c, &format!("{}.traits", path), "MATCH_OR_DEFINE_TRAIT");
        form.walk(c, &format!("{}.form", path), "CARD_FORM_DEFINITION");
        attribute.walk(c, &format!("{}.attribute", path), "CARD_ATTRIBUTE_DEFINITION");
        ace_overflow.walk(c, &format!("{}.ace_overflow", path), "UNCLASSIFIED_STRING");
        identity.walk(c, &format!("{}.identity", path), "UNCLASSIFIED_STRING");
        digixros_aliases.walk(
            c,
            &format!("{}.digixros_aliases", path),
            "DIGIXROS_NAME_DEFINITION",
        );
        also_treated_as.walk(
            c,
            &format!("{}.also_treated_as", path),
            "STATIC_NAME_DEFINITION",
        );
        dual.walk(c, &format!("{}.dual", path), "UNCLASSIFIED_STRING");
        use_requirement.walk(
            c,
            &format!("{}.use_requirement", path),
            "UNCLASSIFIED_STRING",
        );
        alt_paths.walk(c, &format!("{}.alt_paths", path), "UNCLASSIFIED_STRING");
        effects.walk(c, &format!("{}.effects", path), "UNCLASSIFIED_STRING");
            deck_rule.walk(c, &format!("{}.deck_rule", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::compiled::CompiledCardKind {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledCardKind",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledCardKind::Digimon => c.node(
                path,
                "digimon_dsl::compiled::CompiledCardKind",
                "ENUM_SCALAR",
                serde_json::json!("Digimon"),
            ),
            digimon_dsl::compiled::CompiledCardKind::Tamer => c.node(
                path,
                "digimon_dsl::compiled::CompiledCardKind",
                "ENUM_SCALAR",
                serde_json::json!("Tamer"),
            ),
            digimon_dsl::compiled::CompiledCardKind::Option => c.node(
                path,
                "digimon_dsl::compiled::CompiledCardKind",
                "ENUM_SCALAR",
                serde_json::json!("Option"),
            ),
            digimon_dsl::compiled::CompiledCardKind::DigiEgg => c.node(
                path,
                "digimon_dsl::compiled::CompiledCardKind",
                "ENUM_SCALAR",
                serde_json::json!("DigiEgg"),
            ),
            digimon_dsl::compiled::CompiledCardKind::Token => c.node(
                path,
                "digimon_dsl::compiled::CompiledCardKind",
                "ENUM_SCALAR",
                serde_json::json!("Token"),
            ),
            digimon_dsl::compiled::CompiledCardKind::Dual => c.node(
                path,
                "digimon_dsl::compiled::CompiledCardKind",
                "ENUM_SCALAR",
                serde_json::json!("Dual"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledColor {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledColor",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledColor::Red => c.node(
                path,
                "digimon_dsl::compiled::CompiledColor",
                "ENUM_SCALAR",
                serde_json::json!("Red"),
            ),
            digimon_dsl::compiled::CompiledColor::Blue => c.node(
                path,
                "digimon_dsl::compiled::CompiledColor",
                "ENUM_SCALAR",
                serde_json::json!("Blue"),
            ),
            digimon_dsl::compiled::CompiledColor::Yellow => c.node(
                path,
                "digimon_dsl::compiled::CompiledColor",
                "ENUM_SCALAR",
                serde_json::json!("Yellow"),
            ),
            digimon_dsl::compiled::CompiledColor::Green => c.node(
                path,
                "digimon_dsl::compiled::CompiledColor",
                "ENUM_SCALAR",
                serde_json::json!("Green"),
            ),
            digimon_dsl::compiled::CompiledColor::Black => c.node(
                path,
                "digimon_dsl::compiled::CompiledColor",
                "ENUM_SCALAR",
                serde_json::json!("Black"),
            ),
            digimon_dsl::compiled::CompiledColor::Purple => c.node(
                path,
                "digimon_dsl::compiled::CompiledColor",
                "ENUM_SCALAR",
                serde_json::json!("Purple"),
            ),
            digimon_dsl::compiled::CompiledColor::White => c.node(
                path,
                "digimon_dsl::compiled::CompiledColor",
                "ENUM_SCALAR",
                serde_json::json!("White"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledDual {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDual",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledDual { digimon, option } = self;
        digimon.walk(c, &format!("{}.digimon", path), "UNCLASSIFIED_STRING");
        option.walk(c, &format!("{}.option", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledDualDigimon {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDualDigimon",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledDualDigimon {
            level,
            dp,
            colors,
            traits,
            effect_text,
            inherited_text,
            effects,
        } = self;
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        colors.walk(c, &format!("{}.colors", path), "UNCLASSIFIED_STRING");
        traits.walk(c, &format!("{}.traits", path), "MATCH_OR_DEFINE_TRAIT");
        effect_text.walk(
            c,
            &format!("{}.effect_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        inherited_text.walk(
            c,
            &format!("{}.inherited_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        effects.walk(c, &format!("{}.effects", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledDualOption {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDualOption",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledDualOption {
            use_cost,
            colors,
            effect_text,
            security_text,
            keywords,
            use_requirement,
            effects,
        } = self;
        use_cost.walk(c, &format!("{}.use_cost", path), "UNCLASSIFIED_STRING");
        colors.walk(c, &format!("{}.colors", path), "UNCLASSIFIED_STRING");
        effect_text.walk(
            c,
            &format!("{}.effect_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        security_text.walk(
            c,
            &format!("{}.security_text", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        keywords.walk(c, &format!("{}.keywords", path), "ENGINE_KEYWORD_NAME");
        use_requirement.walk(
            c,
            &format!("{}.use_requirement", path),
            "UNCLASSIFIED_STRING",
        );
        effects.walk(c, &format!("{}.effects", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledIdentity {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledIdentity",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledIdentity {
            name_aliases,
            source_name_aliases,
        } = self;
        name_aliases.walk(c, &format!("{}.name_aliases", path), "UNCLASSIFIED_STRING");
        source_name_aliases.walk(
            c,
            &format!("{}.source_name_aliases", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::compiled::CompiledNameAlias {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledNameAlias",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledNameAlias {
            treat_as,
            zone,
            has_inherited_card_number,
            has_inherited_name,
        } = self;
        treat_as.walk(
            c,
            &format!("{}.treat_as", path),
            "CONDITIONAL_NAME_DEFINITION",
        );
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        has_inherited_card_number.walk(
            c,
            &format!("{}.has_inherited_card_number", path),
            "MATCH_CARD_ID",
        );
        has_inherited_name.walk(
            c,
            &format!("{}.has_inherited_name", path),
            "MATCH_NAME_EXACT",
        );
    }
}
impl Walk for digimon_dsl::compiled::CompiledSourceNameAlias {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledSourceNameAlias",
            "DYNAMIC_SOURCE_NAME_DEFINITION",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledSourceNameAlias { level_lte } = self;
        level_lte.walk(c, &format!("{}.level_lte", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledZone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledZone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledZone::Hand => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::compiled::CompiledZone::Deck => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("Deck"),
            ),
            digimon_dsl::compiled::CompiledZone::Trash => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
            digimon_dsl::compiled::CompiledZone::BattleArea => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("BattleArea"),
            ),
            digimon_dsl::compiled::CompiledZone::Security => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("Security"),
            ),
            digimon_dsl::compiled::CompiledZone::Breeding => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("Breeding"),
            ),
            digimon_dsl::compiled::CompiledZone::Reveal => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("Reveal"),
            ),
            digimon_dsl::compiled::CompiledZone::DigiEggDeck => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("DigiEggDeck"),
            ),
            digimon_dsl::compiled::CompiledZone::Material => c.node(
                path,
                "digimon_dsl::compiled::CompiledZone",
                "ENUM_SCALAR",
                serde_json::json!("Material"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledLinkSourceZone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledLinkSourceZone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledLinkSourceZone::Hand => c.node(
                path,
                "digimon_dsl::compiled::CompiledLinkSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::compiled::CompiledLinkSourceZone::Trash => c.node(
                path,
                "digimon_dsl::compiled::CompiledLinkSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
            digimon_dsl::compiled::CompiledLinkSourceZone::SelfSources => c.node(
                path,
                "digimon_dsl::compiled::CompiledLinkSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("SelfSources"),
            ),
            digimon_dsl::compiled::CompiledLinkSourceZone::OwnDigimonSources => c.node(
                path,
                "digimon_dsl::compiled::CompiledLinkSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("OwnDigimonSources"),
            ),
            digimon_dsl::compiled::CompiledLinkSourceZone::SelfOption => c.node(
                path,
                "digimon_dsl::compiled::CompiledLinkSourceZone",
                "ENUM_SCALAR",
                serde_json::json!("SelfOption"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledLinkTo {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledLinkTo",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledLinkTo::SelfPermanent => c.node(
                path,
                "digimon_dsl::compiled::CompiledLinkTo",
                "ENUM_SCALAR",
                serde_json::json!("SelfPermanent"),
            ),
            digimon_dsl::compiled::CompiledLinkTo::OwnDigimon => c.node(
                path,
                "digimon_dsl::compiled::CompiledLinkTo",
                "ENUM_SCALAR",
                serde_json::json!("OwnDigimon"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledLinkCount {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledLinkCount",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledLinkCount::Exactly(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledLinkCount",
                    "VARIANT",
                    serde_json::json!("Exactly"),
                );
                f0.walk(c, &format!("{}@Exactly.0", path), role);
            }
            digimon_dsl::compiled::CompiledLinkCount::UpTo(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledLinkCount",
                    "VARIANT",
                    serde_json::json!("UpTo"),
                );
                f0.walk(c, &format!("{}@UpTo.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledAltPath {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAltPath",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledAltPath {
            kind,
            from,
            materials,
            cost,
            stacks_unsuspended,
            ignore_requirements,
            source_treated_as,
            extra_cost,
            on_burst_turn_end,
            marker,
            condition,
            direction,
            extra_material_zones,
        } = self;
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        from.walk(c, &format!("{}.from", path), "EXISTING_INSTANCE_BINDING");
        materials.walk(c, &format!("{}.materials", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
        stacks_unsuspended.walk(
            c,
            &format!("{}.stacks_unsuspended", path),
            "UNCLASSIFIED_STRING",
        );
        ignore_requirements.walk(
            c,
            &format!("{}.ignore_requirements", path),
            "UNCLASSIFIED_STRING",
        );
        source_treated_as.walk(
            c,
            &format!("{}.source_treated_as", path),
            "CONDITIONAL_NAME_DEFINITION",
        );
        extra_cost.walk(c, &format!("{}.extra_cost", path), "UNCLASSIFIED_STRING");
        on_burst_turn_end.walk(
            c,
            &format!("{}.on_burst_turn_end", path),
            "UNCLASSIFIED_STRING",
        );
        marker.walk(c, &format!("{}.marker", path), "UNCLASSIFIED_STRING");
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
        direction.walk(c, &format!("{}.direction", path), "UNCLASSIFIED_STRING");
        extra_material_zones.walk(
            c,
            &format!("{}.extra_material_zones", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::compiled::CompiledConditionalMaterialZone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledConditionalMaterialZone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledConditionalMaterialZone { zone, condition } = self;
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledAltPathDirection {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAltPathDirection",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledAltPathDirection::From => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathDirection",
                "ENUM_SCALAR",
                serde_json::json!("From"),
            ),
            digimon_dsl::compiled::CompiledAltPathDirection::Into => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathDirection",
                "ENUM_SCALAR",
                serde_json::json!("Into"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledAltPathKind {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAltPathKind",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledAltPathKind::Digivolve => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("Digivolve"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::DnaDigivolve => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("DnaDigivolve"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::BlastDnaDigivolve => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("BlastDnaDigivolve"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::DigiXros => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("DigiXros"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::BurstDigivolve => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("BurstDigivolve"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::AppFusion => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("AppFusion"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::Assembly => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("Assembly"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::ActivatedDigivolve => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("ActivatedDigivolve"),
            ),
            digimon_dsl::compiled::CompiledAltPathKind::CastTimeAssembly => c.node(
                path,
                "digimon_dsl::compiled::CompiledAltPathKind",
                "ENUM_SCALAR",
                serde_json::json!("CastTimeAssembly"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledMaterial {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledMaterial",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledMaterial {
            filter,
            repeat,
            distinct_by,
            zones,
            cost_delta,
            stack_under,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        repeat.walk(c, &format!("{}.repeat", path), "UNCLASSIFIED_STRING");
        distinct_by.walk(c, &format!("{}.distinct_by", path), "UNCLASSIFIED_STRING");
        zones.walk(c, &format!("{}.zones", path), "UNCLASSIFIED_STRING");
        cost_delta.walk(c, &format!("{}.cost_delta", path), "UNCLASSIFIED_STRING");
        stack_under.walk(c, &format!("{}.stack_under", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledRepeat {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledRepeat",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledRepeat::Unbounded => c.node(
                path,
                "digimon_dsl::compiled::CompiledRepeat",
                "ENUM_SCALAR",
                serde_json::json!("Unbounded"),
            ),
            digimon_dsl::compiled::CompiledRepeat::Range { min, max } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledRepeat",
                    "VARIANT",
                    serde_json::json!("Range"),
                );
                min.walk(
                    c,
                    &format!("{}.min", &format!("{}@Range", path)),
                    "UNCLASSIFIED_STRING",
                );
                max.walk(
                    c,
                    &format!("{}.max", &format!("{}@Range", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledSameBy {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledSameBy",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledSameBy::Level => c.node(
                path,
                "digimon_dsl::compiled::CompiledSameBy",
                "ENUM_SCALAR",
                serde_json::json!("Level"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledDistinctBy {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDistinctBy",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledDistinctBy::CardNumber => c.node(
                path,
                "digimon_dsl::compiled::CompiledDistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("CardNumber"),
            ),
            digimon_dsl::compiled::CompiledDistinctBy::Level => c.node(
                path,
                "digimon_dsl::compiled::CompiledDistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("Level"),
            ),
            digimon_dsl::compiled::CompiledDistinctBy::Name => c.node(
                path,
                "digimon_dsl::compiled::CompiledDistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("Name"),
            ),
            digimon_dsl::compiled::CompiledDistinctBy::Color => c.node(
                path,
                "digimon_dsl::compiled::CompiledDistinctBy",
                "ENUM_SCALAR",
                serde_json::json!("Color"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledCost {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledCost",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledCost::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledCost",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::compiled::CompiledCost::Formula(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledCost",
                    "VARIANT",
                    serde_json::json!("Formula"),
                );
                f0.walk(c, &format!("{}@Formula.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledPermanentProperty {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledPermanentProperty",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledPermanentProperty::Level => c.node(
                path,
                "digimon_dsl::compiled::CompiledPermanentProperty",
                "ENUM_SCALAR",
                serde_json::json!("Level"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledDpConstraint {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDpConstraint",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledDpConstraint::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDpConstraint",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::compiled::CompiledDpConstraint::Formula(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDpConstraint",
                    "VARIANT",
                    serde_json::json!("Formula"),
                );
                f0.walk(c, &format!("{}@Formula.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledReplacementCause {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledReplacementCause",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledReplacementCause::Battle => c.node(
                path,
                "digimon_dsl::compiled::CompiledReplacementCause",
                "ENUM_SCALAR",
                serde_json::json!("Battle"),
            ),
            digimon_dsl::compiled::CompiledReplacementCause::OwnEffect => c.node(
                path,
                "digimon_dsl::compiled::CompiledReplacementCause",
                "ENUM_SCALAR",
                serde_json::json!("OwnEffect"),
            ),
            digimon_dsl::compiled::CompiledReplacementCause::OpponentEffect => c.node(
                path,
                "digimon_dsl::compiled::CompiledReplacementCause",
                "ENUM_SCALAR",
                serde_json::json!("OpponentEffect"),
            ),
            digimon_dsl::compiled::CompiledReplacementCause::SecurityCheck => c.node(
                path,
                "digimon_dsl::compiled::CompiledReplacementCause",
                "ENUM_SCALAR",
                serde_json::json!("SecurityCheck"),
            ),
            digimon_dsl::compiled::CompiledReplacementCause::Cost => c.node(
                path,
                "digimon_dsl::compiled::CompiledReplacementCause",
                "ENUM_SCALAR",
                serde_json::json!("Cost"),
            ),
            digimon_dsl::compiled::CompiledReplacementCause::Overclock => c.node(
                path,
                "digimon_dsl::compiled::CompiledReplacementCause",
                "ENUM_SCALAR",
                serde_json::json!("Overclock"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledEventCause {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledEventCause",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledEventCause::BattleDeletion => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("BattleDeletion"),
            ),
            digimon_dsl::compiled::CompiledEventCause::EffectDeletion => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("EffectDeletion"),
            ),
            digimon_dsl::compiled::CompiledEventCause::OwnEffect => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("OwnEffect"),
            ),
            digimon_dsl::compiled::CompiledEventCause::OpponentEffect => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("OpponentEffect"),
            ),
            digimon_dsl::compiled::CompiledEventCause::Overclock => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("Overclock"),
            ),
            digimon_dsl::compiled::CompiledEventCause::Return => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("Return"),
            ),
            digimon_dsl::compiled::CompiledEventCause::DeckBottom => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("DeckBottom"),
            ),
            digimon_dsl::compiled::CompiledEventCause::SecurityPlacement => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("SecurityPlacement"),
            ),
            digimon_dsl::compiled::CompiledEventCause::SecurityRemoval => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("SecurityRemoval"),
            ),
            digimon_dsl::compiled::CompiledEventCause::Cost => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("Cost"),
            ),
            digimon_dsl::compiled::CompiledEventCause::Rule => c.node(
                path,
                "digimon_dsl::compiled::CompiledEventCause",
                "ENUM_SCALAR",
                serde_json::json!("Rule"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledBindingCompare {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledBindingCompare",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledBindingCompare::Binding(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingCompare",
                    "VARIANT",
                    serde_json::json!("Binding"),
                );
                f0.walk(
                    c,
                    &format!("{}@Binding.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledBindingCompare::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingCompare",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledBindingOwnerPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledBindingOwnerPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledBindingOwnerPredicate { binding, of } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledBindingCardKindPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledBindingCardKindPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledBindingCardKindPredicate { binding, kind } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledBindingCardColorPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledBindingCardColorPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledBindingCardColorPredicate { binding, color_is } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        color_is.walk(c, &format!("{}.color_is", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledPlayCostBindingPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledPlayCostBindingPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledPlayCostBindingPredicate {
            binding,
            offset,
            op,
        } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        offset.walk(c, &format!("{}.offset", path), "UNCLASSIFIED_STRING");
        op.walk(c, &format!("{}.op", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledDistinctNamedCount {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDistinctNamedCount",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledDistinctNamedCount { of, filter, n } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        n.walk(c, &format!("{}.n", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledCountAggregate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledCountAggregate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledCountAggregate { filter, n } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        n.walk(c, &format!("{}.n", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledComparatorOp {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledComparatorOp",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledComparatorOp::Eq => c.node(
                path,
                "digimon_dsl::compiled::CompiledComparatorOp",
                "ENUM_SCALAR",
                serde_json::json!("Eq"),
            ),
            digimon_dsl::compiled::CompiledComparatorOp::Gte => c.node(
                path,
                "digimon_dsl::compiled::CompiledComparatorOp",
                "ENUM_SCALAR",
                serde_json::json!("Gte"),
            ),
            digimon_dsl::compiled::CompiledComparatorOp::Lte => c.node(
                path,
                "digimon_dsl::compiled::CompiledComparatorOp",
                "ENUM_SCALAR",
                serde_json::json!("Lte"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledSelfSourceCountPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledSelfSourceCountPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledSelfSourceCountPredicate { filter, op, value } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        op.walk(c, &format!("{}.op", path), "UNCLASSIFIED_STRING");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledExistential {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledExistential",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledExistential { of, predicate } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        predicate.walk(c, &format!("{}.predicate", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledDigivolveCandidate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDigivolveCandidate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledDigivolveCandidate { of, zones, filter, cost } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        zones.walk(c, &format!("{}.zones", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledFaceUpSecurityNamed {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledFaceUpSecurityNamed",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledFaceUpSecurityNamed {
            of,
            card_number_is,
            name_is,
            color_is,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card_number_is.walk(c, &format!("{}.card_number_is", path), "MATCH_CARD_ID");
        name_is.walk(c, &format!("{}.name_is", path), "MATCH_NAME_EXACT");
        color_is.walk(c, &format!("{}.color_is", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledPlayerRef {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledPlayerRef",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledPlayerRef::You => c.node(
                path,
                "digimon_dsl::compiled::CompiledPlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("You"),
            ),
            digimon_dsl::compiled::CompiledPlayerRef::Opponent => c.node(
                path,
                "digimon_dsl::compiled::CompiledPlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("Opponent"),
            ),
            digimon_dsl::compiled::CompiledPlayerRef::Any => c.node(
                path,
                "digimon_dsl::compiled::CompiledPlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("Any"),
            ),
            digimon_dsl::compiled::CompiledPlayerRef::Active => c.node(
                path,
                "digimon_dsl::compiled::CompiledPlayerRef",
                "ENUM_SCALAR",
                serde_json::json!("Active"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledSourceStackScope {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledSourceStackScope",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledSourceStackScope::SelfCarrier => c.node(
                path,
                "digimon_dsl::compiled::CompiledSourceStackScope",
                "ENUM_SCALAR",
                serde_json::json!("SelfCarrier"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledCountBound {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledCountBound",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledCountBound::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledCountBound",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::compiled::CompiledCountBound::Formula(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledCountBound",
                    "VARIANT",
                    serde_json::json!("Formula"),
                );
                f0.walk(c, &format!("{}@Formula.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledAggregateSelector {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAggregateSelector",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledAggregateSelector::LowestDp => c.node(
                path,
                "digimon_dsl::compiled::CompiledAggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestDp"),
            ),
            digimon_dsl::compiled::CompiledAggregateSelector::HighestDp => c.node(
                path,
                "digimon_dsl::compiled::CompiledAggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestDp"),
            ),
            digimon_dsl::compiled::CompiledAggregateSelector::LowestLevel => c.node(
                path,
                "digimon_dsl::compiled::CompiledAggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestLevel"),
            ),
            digimon_dsl::compiled::CompiledAggregateSelector::HighestLevel => c.node(
                path,
                "digimon_dsl::compiled::CompiledAggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestLevel"),
            ),
            digimon_dsl::compiled::CompiledAggregateSelector::FewestMaterials => c.node(
                path,
                "digimon_dsl::compiled::CompiledAggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("FewestMaterials"),
            ),
            digimon_dsl::compiled::CompiledAggregateSelector::LowestPlayCost => c.node(
                path,
                "digimon_dsl::compiled::CompiledAggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestPlayCost"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledModifierValue {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledModifierValue",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledModifierValue::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierValue",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::compiled::CompiledModifierValue::Formula(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierValue",
                    "VARIANT",
                    serde_json::json!("Formula"),
                );
                f0.walk(c, &format!("{}@Formula.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledModifierPayload {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledModifierPayload",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledModifierPayload::Name(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierPayload",
                    "VARIANT",
                    serde_json::json!("Name"),
                );
                f0.walk(c, &format!("{}@Name.0", path), role);
            }
            digimon_dsl::compiled::CompiledModifierPayload::Colors(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierPayload",
                    "VARIANT",
                    serde_json::json!("Colors"),
                );
                f0.walk(c, &format!("{}@Colors.0", path), role);
            }
            digimon_dsl::compiled::CompiledModifierPayload::Dp(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierPayload",
                    "VARIANT",
                    serde_json::json!("Dp"),
                );
                f0.walk(c, &format!("{}@Dp.0", path), role);
            }
            digimon_dsl::compiled::CompiledModifierPayload::Traits { add, replace } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierPayload",
                    "VARIANT",
                    serde_json::json!("Traits"),
                );
                add.walk(
                    c,
                    &format!("{}.add", &format!("{}@Traits", path)),
                    "UNCLASSIFIED_STRING",
                );
                replace.walk(
                    c,
                    &format!("{}.replace", &format!("{}@Traits", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledSynthIdentity {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledSynthIdentity",
            "EXISTING_INSTANCE_SYNTH_IDENTITY",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledSynthIdentity {
            kind,
            level,
            colors,
            traits,
            dp,
        } = self;
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        colors.walk(c, &format!("{}.colors", path), "UNCLASSIFIED_STRING");
        traits.walk(c, &format!("{}.traits", path), "MATCH_OR_DEFINE_TRAIT");
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledClause {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledClause",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledClause::Triggered(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledClause",
                    "VARIANT",
                    serde_json::json!("Triggered"),
                );
                f0.walk(c, &format!("{}@Triggered.0", path), role);
            }
            digimon_dsl::compiled::CompiledClause::Declarative(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledClause",
                    "VARIANT",
                    serde_json::json!("Declarative"),
                );
                f0.walk(c, &format!("{}@Declarative.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledTriggeredClause {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledTriggeredClause",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledTriggeredClause {
            when,
            scope,
            active_when,
            condition,
            timing_conditions,
            optional,
            outer_prompt,
            once_per_turn,
            max_per_turn,
            process,
            summary,
            summary_key,
            trigger_if,
        } = self;
        when.walk(c, &format!("{}.when", path), "UNCLASSIFIED_STRING");
        scope.walk(c, &format!("{}.scope", path), "UNCLASSIFIED_STRING");
        active_when.walk(c, &format!("{}.active_when", path), "UNCLASSIFIED_STRING");
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
        timing_conditions.walk(
            c,
            &format!("{}.timing_conditions", path),
            "UNCLASSIFIED_STRING",
        );
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        outer_prompt.walk(c, &format!("{}.outer_prompt", path), "UNCLASSIFIED_STRING");
        once_per_turn.walk(c, &format!("{}.once_per_turn", path), "UNCLASSIFIED_STRING");
        max_per_turn.walk(c, &format!("{}.max_per_turn", path), "UNCLASSIFIED_STRING");
        process.walk(c, &format!("{}.process", path), "UNCLASSIFIED_STRING");
        summary.walk(
            c,
            &format!("{}.summary", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
        summary_key.walk(
            c,
            &format!("{}.summary_key", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
            trigger_if.walk(c, &format!("{}.trigger_if", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::compiled::CompiledTimingCondition {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledTimingCondition",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledTimingCondition { when, condition } = self;
        when.walk(c, &format!("{}.when", path), "UNCLASSIFIED_STRING");
        condition.walk(c, &format!("{}.condition", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledDeclarativeClause {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDeclarativeClause",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                target,
                target_player,
                dp_modifier,
                dp_modifier_fn,
                security_attack,
                security_attack_fn,
                grant_keyword,
                grant_traits,
                modifier,
                modifier_value,
                modifier_name,
                synth_identity,
                while_condition,
                applies_to_opponent_security_dp,
                applies_to_own_security_dp,
                effect_immunity,
                modifier_from,
                dna_material_identity,
                summary,
                summary_key,
            source_card_trash_protection,
        } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("Aura"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                target.walk(
                    c,
                    &format!("{}.target", &format!("{}@Aura", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
                target_player.walk(
                    c,
                    &format!("{}.target_player", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                dp_modifier.walk(
                    c,
                    &format!("{}.dp_modifier", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                dp_modifier_fn.walk(
                    c,
                    &format!("{}.dp_modifier_fn", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                security_attack.walk(
                    c,
                    &format!("{}.security_attack", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                security_attack_fn.walk(
                    c,
                    &format!("{}.security_attack_fn", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                grant_keyword.walk(
                    c,
                    &format!("{}.grant_keyword", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                grant_traits.walk(
                    c,
                    &format!("{}.grant_traits", &format!("{}@Aura", path)),
                    "MATCH_OR_DEFINE_TRAIT",
                );
                modifier.walk(
                    c,
                    &format!("{}.modifier", &format!("{}@Aura", path)),
                    "ENGINE_MODIFIER_NAME",
                );
                modifier_value.walk(
                    c,
                    &format!("{}.modifier_value", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                modifier_name.walk(
                    c,
                    &format!("{}.modifier_name", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                synth_identity.walk(
                    c,
                    &format!("{}.synth_identity", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                while_condition.walk(
                    c,
                    &format!("{}.while_condition", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                applies_to_opponent_security_dp.walk(
                    c,
                    &format!(
                        "{}.applies_to_opponent_security_dp",
                        &format!("{}@Aura", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
                applies_to_own_security_dp.walk(
                    c,
                    &format!("{}.applies_to_own_security_dp", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                effect_immunity.walk(
                    c,
                    &format!("{}.effect_immunity", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                modifier_from.walk(
                    c,
                    &format!("{}.modifier_from", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                dna_material_identity.walk(
                    c,
                    &format!("{}.dna_material_identity", &format!("{}@Aura", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@Aura", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@Aura", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            source_card_trash_protection.walk(c, &format!("{}@Aura.source_card_trash_protection", path), "UNCLASSIFIED_STRING");
}
            digimon_dsl::compiled::CompiledDeclarativeClause::CostReduction {
                scope,
                active_when,
                reduction_timing,
                when_playing_this,
            when_digivolving_this,
                when_any_ally_played,
                when_any_ally_digivolves_into,
                condition,
                optional,
                once_per_turn,
                amount,
                amount_fn,
                pay_cost,
                summary,
                summary_key,
            set_cost,
        } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("CostReduction"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                reduction_timing.walk(
                    c,
                    &format!("{}.reduction_timing", &format!("{}@CostReduction", path)),
                    "ENGINE_TIMING_NAME",
                );
                when_playing_this.walk(
                    c,
                    &format!("{}.when_playing_this", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                when_digivolving_this.walk(c, &format!("{}.when_digivolving_this", &format!("{}@CostReduction", path)), "UNCLASSIFIED_STRING");
                when_any_ally_played.walk(
                    c,
                    &format!(
                        "{}.when_any_ally_played",
                        &format!("{}@CostReduction", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
                when_any_ally_digivolves_into.walk(
                    c,
                    &format!(
                        "{}.when_any_ally_digivolves_into",
                        &format!("{}@CostReduction", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
                condition.walk(
                    c,
                    &format!("{}.condition", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                optional.walk(
                    c,
                    &format!("{}.optional", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                once_per_turn.walk(
                    c,
                    &format!("{}.once_per_turn", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                amount.walk(
                    c,
                    &format!("{}.amount", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                amount_fn.walk(
                    c,
                    &format!("{}.amount_fn", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                pay_cost.walk(
                    c,
                    &format!("{}.pay_cost", &format!("{}@CostReduction", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@CostReduction", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@CostReduction", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            set_cost.walk(c, &format!("{}@CostReduction.set_cost", path), "UNCLASSIFIED_STRING");
}
            digimon_dsl::compiled::CompiledDeclarativeClause::Replacement {
                scope,
                active_when,
                trigger,
                optional,
                once_per_turn,
                process,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("Replacement"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@Replacement", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@Replacement", path)),
                    "UNCLASSIFIED_STRING",
                );
                trigger.walk(
                    c,
                    &format!("{}.trigger", &format!("{}@Replacement", path)),
                    "ENGINE_TIMING_NAME",
                );
                optional.walk(
                    c,
                    &format!("{}.optional", &format!("{}@Replacement", path)),
                    "UNCLASSIFIED_STRING",
                );
                once_per_turn.walk(
                    c,
                    &format!("{}.once_per_turn", &format!("{}@Replacement", path)),
                    "UNCLASSIFIED_STRING",
                );
                process.walk(
                    c,
                    &format!("{}.process", &format!("{}@Replacement", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@Replacement", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@Replacement", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::Partition {
                scope,
                active_when,
                sources,
                exclude_cause,
                process,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("Partition"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@Partition", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@Partition", path)),
                    "UNCLASSIFIED_STRING",
                );
                sources.walk(
                    c,
                    &format!("{}.sources", &format!("{}@Partition", path)),
                    "UNCLASSIFIED_STRING",
                );
                exclude_cause.walk(
                    c,
                    &format!("{}.exclude_cause", &format!("{}@Partition", path)),
                    "UNCLASSIFIED_STRING",
                );
                process.walk(
                    c,
                    &format!("{}.process", &format!("{}@Partition", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@Partition", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@Partition", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::AceOverflow {
                value,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("AceOverflow"),
                );
                value.walk(
                    c,
                    &format!("{}.value", &format!("{}@AceOverflow", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@AceOverflow", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@AceOverflow", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword {
                keyword,
                value,
                scope,
                overclock_cost_filter,
                active_when,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("GrantKeyword"),
                );
                keyword.walk(
                    c,
                    &format!("{}.keyword", &format!("{}@GrantKeyword", path)),
                    "ENGINE_KEYWORD_NAME",
                );
                value.walk(
                    c,
                    &format!("{}.value", &format!("{}@GrantKeyword", path)),
                    "UNCLASSIFIED_STRING",
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@GrantKeyword", path)),
                    "UNCLASSIFIED_STRING",
                );
                overclock_cost_filter.walk(
                    c,
                    &format!(
                        "{}.overclock_cost_filter",
                        &format!("{}@GrantKeyword", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@GrantKeyword", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@GrantKeyword", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@GrantKeyword", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::Delay {
                scope,
                active_when,
                trigger,
                process,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("Delay"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@Delay", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@Delay", path)),
                    "UNCLASSIFIED_STRING",
                );
                trigger.walk(
                    c,
                    &format!("{}.trigger", &format!("{}@Delay", path)),
                    "ENGINE_TIMING_NAME",
                );
                process.walk(
                    c,
                    &format!("{}.process", &format!("{}@Delay", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@Delay", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@Delay", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::LinkRequirement {
                scope,
                cost,
                filter,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("LinkRequirement"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@LinkRequirement", path)),
                    "UNCLASSIFIED_STRING",
                );
                cost.walk(
                    c,
                    &format!("{}.cost", &format!("{}@LinkRequirement", path)),
                    "UNCLASSIFIED_STRING",
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@LinkRequirement", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@LinkRequirement", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@LinkRequirement", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::LinkCondition {
                scope,
                cost,
                filter,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("LinkCondition"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@LinkCondition", path)),
                    "UNCLASSIFIED_STRING",
                );
                cost.walk(
                    c,
                    &format!("{}.cost", &format!("{}@LinkCondition", path)),
                    "UNCLASSIFIED_STRING",
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@LinkCondition", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@LinkCondition", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@LinkCondition", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::UseCostIncrease {
                scope,
                amount,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("UseCostIncrease"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@UseCostIncrease", path)),
                    "UNCLASSIFIED_STRING",
                );
                amount.walk(
                    c,
                    &format!("{}.amount", &format!("{}@UseCostIncrease", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@UseCostIncrease", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@UseCostIncrease", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::Succession {
                scope,
                active_when,
                filter,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("Succession"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@Succession", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@Succession", path)),
                    "UNCLASSIFIED_STRING",
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@Succession", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@Succession", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@Succession", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::FloodGate {
                scope,
                active_when,
                modifier,
                target,
                target_player,
                expiry,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("FloodGate"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@FloodGate", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@FloodGate", path)),
                    "UNCLASSIFIED_STRING",
                );
                modifier.walk(
                    c,
                    &format!("{}.modifier", &format!("{}@FloodGate", path)),
                    "ENGINE_MODIFIER_NAME",
                );
                target.walk(
                    c,
                    &format!("{}.target", &format!("{}@FloodGate", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
                target_player.walk(
                    c,
                    &format!("{}.target_player", &format!("{}@FloodGate", path)),
                    "UNCLASSIFIED_STRING",
                );
                expiry.walk(
                    c,
                    &format!("{}.expiry", &format!("{}@FloodGate", path)),
                    "ENGINE_EXPIRY_NAME",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@FloodGate", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@FloodGate", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::AltPathRegistration {
                scope,
                active_when,
                trigger,
                applies_to,
                registers,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("AltPathRegistration"),
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@AltPathRegistration", path)),
                    "UNCLASSIFIED_STRING",
                );
                active_when.walk(
                    c,
                    &format!("{}.active_when", &format!("{}@AltPathRegistration", path)),
                    "UNCLASSIFIED_STRING",
                );
                trigger.walk(
                    c,
                    &format!("{}.trigger", &format!("{}@AltPathRegistration", path)),
                    "ENGINE_TIMING_NAME",
                );
                applies_to.walk(
                    c,
                    &format!("{}.applies_to", &format!("{}@AltPathRegistration", path)),
                    "UNCLASSIFIED_STRING",
                );
                registers.walk(
                    c,
                    &format!("{}.registers", &format!("{}@AltPathRegistration", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@AltPathRegistration", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@AltPathRegistration", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
            digimon_dsl::compiled::CompiledDeclarativeClause::RawRust {
                fn_name,
                triggers,
                scope,
                summary,
                summary_key,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledDeclarativeClause",
                    "VARIANT",
                    serde_json::json!("RawRust"),
                );
                fn_name.walk(
                    c,
                    &format!("{}.fn_name", &format!("{}@RawRust", path)),
                    "HOOK_DECLARATIVE",
                );
                triggers.walk(
                    c,
                    &format!("{}.triggers", &format!("{}@RawRust", path)),
                    "UNCLASSIFIED_STRING",
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@RawRust", path)),
                    "UNCLASSIFIED_STRING",
                );
                summary.walk(
                    c,
                    &format!("{}.summary", &format!("{}@RawRust", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
                summary_key.walk(
                    c,
                    &format!("{}.summary_key", &format!("{}@RawRust", path)),
                    "METADATA_NO_EXECUTABLE_REFERENCE",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledScope {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledScope",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledScope::FaceUp => c.node(
                path,
                "digimon_dsl::compiled::CompiledScope",
                "ENUM_SCALAR",
                serde_json::json!("FaceUp"),
            ),
            digimon_dsl::compiled::CompiledScope::Inherited => c.node(
                path,
                "digimon_dsl::compiled::CompiledScope",
                "ENUM_SCALAR",
                serde_json::json!("Inherited"),
            ),
            digimon_dsl::compiled::CompiledScope::Both => c.node(
                path,
                "digimon_dsl::compiled::CompiledScope",
                "ENUM_SCALAR",
                serde_json::json!("Both"),
            ),
            digimon_dsl::compiled::CompiledScope::Linked => c.node(
                path,
                "digimon_dsl::compiled::CompiledScope",
                "ENUM_SCALAR",
                serde_json::json!("Linked"),
            ),
            digimon_dsl::compiled::CompiledScope::Security => c.node(
                path,
                "digimon_dsl::compiled::CompiledScope",
                "ENUM_SCALAR",
                serde_json::json!("Security"),
            ),
            digimon_dsl::compiled::CompiledScope::Trash => c.node(
                path,
                "digimon_dsl::compiled::CompiledScope",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledTiming {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledTiming",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledTiming::OnPlay => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnPlay"),
            ),
            digimon_dsl::compiled::CompiledTiming::WhenDigivolving => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("WhenDigivolving"),
            ),
            digimon_dsl::compiled::CompiledTiming::WhenAttacking => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("WhenAttacking"),
            ),
            digimon_dsl::compiled::CompiledTiming::EndOfAttack => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("EndOfAttack"),
            ),
            digimon_dsl::compiled::CompiledTiming::EndOfBattle => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("EndOfBattle"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAttack => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAttack"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnBlock => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnBlock"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAllyAttack => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAllyAttack"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnOpponentAttack => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnOpponentAttack"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDeletion => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDeletion"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAnyDeletion => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAnyDeletion"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAllyWonBattle => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAllyWonBattle"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDiscardHand => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDiscardHand"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAddToDeck => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAddToDeck"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnEnterFieldAnyone => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnEnterFieldAnyone"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAnyDigimonPlayed => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAnyDigimonPlayed"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAllyPlayed => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAllyPlayed"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnLeaveField => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnLeaveField"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnSuspend => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnSuspend"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnUnsuspend => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnUnsuspend"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAddToHand => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAddToHand"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnHatch => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnHatch"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnMove => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnMove"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDigivolve => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDigivolve"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDnaDigivolve => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDnaDigivolve"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDigixros => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDigixros"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnOpponentSecurityRemoved => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnOpponentSecurityRemoved"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnOwnSecurityRemoved => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnOwnSecurityRemoved"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDigivolutionCardTrashed => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDigivolutionCardTrashed"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAddDigivolutionCards => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAddDigivolutionCards"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDigivolutionCardReturnedToDeckBottom => c
                .node(
                    path,
                    "digimon_dsl::compiled::CompiledTiming",
                    "ENUM_SCALAR",
                    serde_json::json!("OnDigivolutionCardReturnedToDeckBottom"),
                ),
            digimon_dsl::compiled::CompiledTiming::OnSecurityCheck => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnSecurityCheck"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnCheckFaceUpSecurity => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnCheckFaceUpSecurity"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnLoseSecurity => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnLoseSecurity"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDiscardSecurity => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDiscardSecurity"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnDiscardDeck => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnDiscardDeck"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnSecurity => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnSecurity"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnOptionPlaced => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnOptionPlaced"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnOptionTrashed => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnOptionTrashed"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnUseOption => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnUseOption"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnPlaceSecurity => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnPlaceSecurity"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAddedToSecurity => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAddedToSecurity"),
            ),
            digimon_dsl::compiled::CompiledTiming::Main => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("Main"),
            ),
            digimon_dsl::compiled::CompiledTiming::StartOfYourTurn => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("StartOfYourTurn"),
            ),
            digimon_dsl::compiled::CompiledTiming::StartOfOpponentsTurn => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("StartOfOpponentsTurn"),
            ),
            digimon_dsl::compiled::CompiledTiming::StartOfYourMainPhase => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("StartOfYourMainPhase"),
            ),
            digimon_dsl::compiled::CompiledTiming::EndOfYourTurn => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("EndOfYourTurn"),
            ),
            digimon_dsl::compiled::CompiledTiming::EndOfOpponentsTurn => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("EndOfOpponentsTurn"),
            ),
            digimon_dsl::compiled::CompiledTiming::EndOfYourNextTurn => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("EndOfYourNextTurn"),
            ),
            digimon_dsl::compiled::CompiledTiming::EndOfOpponentsNextTurn => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("EndOfOpponentsNextTurn"),
            ),
            digimon_dsl::compiled::CompiledTiming::UntilNextUnsuspend => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("UntilNextUnsuspend"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAttackTargetChange => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAttackTargetChange"),
            ),
            digimon_dsl::compiled::CompiledTiming::MainFromHand => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("MainFromHand"),
            ),
            digimon_dsl::compiled::CompiledTiming::MainOnField => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("MainOnField"),
            ),
            digimon_dsl::compiled::CompiledTiming::MainFromTrash => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("MainFromTrash"),
            ),
            digimon_dsl::compiled::CompiledTiming::Counter => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("Counter"),
            ),
            digimon_dsl::compiled::CompiledTiming::BeforePayCost => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("BeforePayCost"),
            ),
            digimon_dsl::compiled::CompiledTiming::BeforePayCostObserve => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("BeforePayCostObserve"),
            ),
            digimon_dsl::compiled::CompiledTiming::Delayed => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("Delayed"),
            ),
            digimon_dsl::compiled::CompiledTiming::WhenLinked => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("WhenLinked"),
            ),
            digimon_dsl::compiled::CompiledTiming::WhenCardLinkedToThis => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("WhenCardLinkedToThis"),
            ),
            digimon_dsl::compiled::CompiledTiming::WhenWouldLinkToThis => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("WhenWouldLinkToThis"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnAnyLink => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnAnyLink"),
            ),
            digimon_dsl::compiled::CompiledTiming::OnLinkCardTrashed => c.node(
                path,
                "digimon_dsl::compiled::CompiledTiming",
                "ENUM_SCALAR",
                serde_json::json!("OnLinkCardTrashed"),
            ),
                    digimon_dsl::compiled::CompiledTiming::OnTopCardTrashed => c.node(path, "digimon_dsl::compiled::CompiledTiming", "ENUM_SCALAR", serde_json::json!("OnTopCardTrashed")),
            digimon_dsl::compiled::CompiledTiming::StartOfOpponentsMainPhase => c.node(path, "digimon_dsl::compiled::CompiledTiming", "ENUM_SCALAR", serde_json::json!("StartOfOpponentsMainPhase")),
}
    }
}
impl Walk for digimon_dsl::compiled::CompiledGrantKeywordValue {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledGrantKeywordValue",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledGrantKeywordValue { keyword, value } = self;
        keyword.walk(c, &format!("{}.keyword", path), "ENGINE_KEYWORD_NAME");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledEffectSourceKind {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledEffectSourceKind",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledEffectSourceKind::Digimon => c.node(
                path,
                "digimon_dsl::compiled::CompiledEffectSourceKind",
                "ENUM_SCALAR",
                serde_json::json!("Digimon"),
            ),
            digimon_dsl::compiled::CompiledEffectSourceKind::Tamer => c.node(
                path,
                "digimon_dsl::compiled::CompiledEffectSourceKind",
                "ENUM_SCALAR",
                serde_json::json!("Tamer"),
            ),
            digimon_dsl::compiled::CompiledEffectSourceKind::Option => c.node(
                path,
                "digimon_dsl::compiled::CompiledEffectSourceKind",
                "ENUM_SCALAR",
                serde_json::json!("Option"),
            ),
            digimon_dsl::compiled::CompiledEffectSourceKind::Rule => c.node(
                path,
                "digimon_dsl::compiled::CompiledEffectSourceKind",
                "ENUM_SCALAR",
                serde_json::json!("Rule"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledEffectController {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledEffectController",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledEffectController::Any => c.node(
                path,
                "digimon_dsl::compiled::CompiledEffectController",
                "ENUM_SCALAR",
                serde_json::json!("Any"),
            ),
            digimon_dsl::compiled::CompiledEffectController::Opponent => c.node(
                path,
                "digimon_dsl::compiled::CompiledEffectController",
                "ENUM_SCALAR",
                serde_json::json!("Opponent"),
            ),
            digimon_dsl::compiled::CompiledEffectController::Own => c.node(
                path,
                "digimon_dsl::compiled::CompiledEffectController",
                "ENUM_SCALAR",
                serde_json::json!("Own"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledAuraEffectImmunity {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAuraEffectImmunity",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledAuraEffectImmunity {
            source_kind,
            source_controller,
        } = self;
        source_kind.walk(c, &format!("{}.source_kind", path), "UNCLASSIFIED_STRING");
        source_controller.walk(
            c,
            &format!("{}.source_controller", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::compiled::CompiledDnaMaterialIdentity {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledDnaMaterialIdentity",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledDnaMaterialIdentity {
            for_result,
            level,
            name,
            result_scope,
        } = self;
        for_result.walk(c, &format!("{}.for_result", path), "MATCH_DNA_RESULT_NAME");
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        name.walk(c, &format!("{}.name", path), "CONDITIONAL_NAME_DEFINITION");
            result_scope.walk(c, &format!("{}.result_scope", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::compiled::CompiledFieldSelector {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledFieldSelector",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledFieldSelector::LowestDp => c.node(
                path,
                "digimon_dsl::compiled::CompiledFieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestDp"),
            ),
            digimon_dsl::compiled::CompiledFieldSelector::HighestDp => c.node(
                path,
                "digimon_dsl::compiled::CompiledFieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestDp"),
            ),
            digimon_dsl::compiled::CompiledFieldSelector::LowestPlayCost => c.node(
                path,
                "digimon_dsl::compiled::CompiledFieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestPlayCost"),
            ),
            digimon_dsl::compiled::CompiledFieldSelector::HighestPlayCost => c.node(
                path,
                "digimon_dsl::compiled::CompiledFieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestPlayCost"),
            ),
            digimon_dsl::compiled::CompiledFieldSelector::LowestMaterialCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledFieldSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestMaterialCount"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledAttackCostUpgrade {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAttackCostUpgrade",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledAttackCostUpgrade {
            dp,
            security_attack,
        } = self;
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        security_attack.walk(
            c,
            &format!("{}.security_attack", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::compiled::CompiledAppFuseZone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAppFuseZone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledAppFuseZone::Hand => c.node(
                path,
                "digimon_dsl::compiled::CompiledAppFuseZone",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::compiled::CompiledAppFuseZone::Trash => c.node(
                path,
                "digimon_dsl::compiled::CompiledAppFuseZone",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledBattleCompare {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledBattleCompare",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledBattleCompare::Dp => c.node(
                path,
                "digimon_dsl::compiled::CompiledBattleCompare",
                "ENUM_SCALAR",
                serde_json::json!("Dp"),
            ),
            digimon_dsl::compiled::CompiledBattleCompare::DigivolutionCards => c.node(
                path,
                "digimon_dsl::compiled::CompiledBattleCompare",
                "ENUM_SCALAR",
                serde_json::json!("DigivolutionCards"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledUnionZoneFilters {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledUnionZoneFilters",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledUnionZoneFilters {
            hand,
            trash,
            material,
        } = self;
        hand.walk(c, &format!("{}.hand", path), "UNCLASSIFIED_STRING");
        trash.walk(c, &format!("{}.trash", path), "UNCLASSIFIED_STRING");
        material.walk(c, &format!("{}.material", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledActivationCostKind {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledActivationCostKind",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledActivationCostKind::SuspendSelf => c.node(
                path,
                "digimon_dsl::compiled::CompiledActivationCostKind",
                "ENUM_SCALAR",
                serde_json::json!("SuspendSelf"),
            ),
            digimon_dsl::compiled::CompiledActivationCostKind::ReturnSelfToDeckBottom => c.node(
                path,
                "digimon_dsl::compiled::CompiledActivationCostKind",
                "ENUM_SCALAR",
                serde_json::json!("ReturnSelfToDeckBottom"),
            ),
            digimon_dsl::compiled::CompiledActivationCostKind::TrashSelf => c.node(
                path,
                "digimon_dsl::compiled::CompiledActivationCostKind",
                "ENUM_SCALAR",
                serde_json::json!("TrashSelf"),
            ),
            digimon_dsl::compiled::CompiledActivationCostKind::PlaceSelfAtSecurityBottom => c.node(
                path,
                "digimon_dsl::compiled::CompiledActivationCostKind",
                "ENUM_SCALAR",
                serde_json::json!("PlaceSelfAtSecurityBottom"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledRevealBucket {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledRevealBucket",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledRevealBucket {
            bind_as,
            filter,
            min,
            max,
        } = self;
        bind_as.walk(c, &format!("{}.bind_as", path), "EXISTING_INSTANCE_BINDING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        min.walk(c, &format!("{}.min", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::compiled::CompiledRevealSearchBucket {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledRevealSearchBucket",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledRevealSearchBucket {
            filter,
            to,
            max,
            optional,
            prompt,
        } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        to.walk(c, &format!("{}.to", path), "UNCLASSIFIED_STRING");
        max.walk(c, &format!("{}.max", path), "UNCLASSIFIED_STRING");
        optional.walk(c, &format!("{}.optional", path), "UNCLASSIFIED_STRING");
        prompt.walk(
            c,
            &format!("{}.prompt", path),
            "METADATA_NO_EXECUTABLE_REFERENCE",
        );
    }
}
impl Walk for digimon_dsl::compiled::CompiledRevealSearchDest {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledRevealSearchDest",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledRevealSearchDest::Hand => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealSearchDest",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::compiled::CompiledRevealSearchDest::Trash => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealSearchDest",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
            digimon_dsl::compiled::CompiledRevealSearchDest::Deck => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealSearchDest",
                "ENUM_SCALAR",
                serde_json::json!("Deck"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledRevealRemainder {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledRevealRemainder",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledRevealRemainder::Top => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealRemainder",
                "ENUM_SCALAR",
                serde_json::json!("Top"),
            ),
            digimon_dsl::compiled::CompiledRevealRemainder::Bottom => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealRemainder",
                "ENUM_SCALAR",
                serde_json::json!("Bottom"),
            ),
            digimon_dsl::compiled::CompiledRevealRemainder::Choose => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealRemainder",
                "ENUM_SCALAR",
                serde_json::json!("Choose"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledCanAttack {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledCanAttack",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledCanAttack {
            targets,
            without_suspending,
            ignore_summoning_sickness,
        } = self;
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
        without_suspending.walk(
            c,
            &format!("{}.without_suspending", path),
            "UNCLASSIFIED_STRING",
        );
        ignore_summoning_sickness.walk(
            c,
            &format!("{}.ignore_summoning_sickness", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::compiled::CompiledAttackTargetSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledAttackTargetSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledAttackTargetSpec::Any => c.node(
                path,
                "digimon_dsl::compiled::CompiledAttackTargetSpec",
                "ENUM_SCALAR",
                serde_json::json!("Any"),
            ),
            digimon_dsl::compiled::CompiledAttackTargetSpec::Player => c.node(
                path,
                "digimon_dsl::compiled::CompiledAttackTargetSpec",
                "ENUM_SCALAR",
                serde_json::json!("Player"),
            ),
            digimon_dsl::compiled::CompiledAttackTargetSpec::Digimon => c.node(
                path,
                "digimon_dsl::compiled::CompiledAttackTargetSpec",
                "ENUM_SCALAR",
                serde_json::json!("Digimon"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledBindingRef {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledBindingRef",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledBindingRef::Named(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingRef",
                    "VARIANT",
                    serde_json::json!("Named"),
                );
                f0.walk(c, &format!("{}@Named.0", path), "EXISTING_INSTANCE_BINDING");
            }
            digimon_dsl::compiled::CompiledBindingRef::SelfRef => c.node(
                path,
                "digimon_dsl::compiled::CompiledBindingRef",
                "ENUM_SCALAR",
                serde_json::json!("SelfRef"),
            ),
            digimon_dsl::compiled::CompiledBindingRef::Carrier => c.node(
                path,
                "digimon_dsl::compiled::CompiledBindingRef",
                "ENUM_SCALAR",
                serde_json::json!("Carrier"),
            ),
            digimon_dsl::compiled::CompiledBindingRef::Source => c.node(
                path,
                "digimon_dsl::compiled::CompiledBindingRef",
                "ENUM_SCALAR",
                serde_json::json!("Source"),
            ),
            digimon_dsl::compiled::CompiledBindingRef::EventTarget => c.node(
                path,
                "digimon_dsl::compiled::CompiledBindingRef",
                "ENUM_SCALAR",
                serde_json::json!("EventTarget"),
            ),
            digimon_dsl::compiled::CompiledBindingRef::EventCard => c.node(
                path,
                "digimon_dsl::compiled::CompiledBindingRef",
                "ENUM_SCALAR",
                serde_json::json!("EventCard"),
            ),
            digimon_dsl::compiled::CompiledBindingRef::Permanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingRef",
                    "VARIANT",
                    serde_json::json!("Permanent"),
                );
                f0.walk(
                    c,
                    &format!("{}@Permanent.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledBindingRef::Binding(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingRef",
                    "VARIANT",
                    serde_json::json!("Binding"),
                );
                f0.walk(
                    c,
                    &format!("{}@Binding.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledBindingRef::OfPermanent(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingRef",
                    "VARIANT",
                    serde_json::json!("OfPermanent"),
                );
                f0.walk(
                    c,
                    &format!("{}@OfPermanent.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledBindingRef::DeckTop(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingRef",
                    "VARIANT",
                    serde_json::json!("DeckTop"),
                );
                f0.walk(
                    c,
                    &format!("{}@DeckTop.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledBindingRef::SecurityTop(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledBindingRef",
                    "VARIANT",
                    serde_json::json!("SecurityTop"),
                );
                f0.walk(
                    c,
                    &format!("{}@SecurityTop.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledBindingRef::OwnBreeding => c.node(
                path,
                "digimon_dsl::compiled::CompiledBindingRef",
                "ENUM_SCALAR",
                serde_json::json!("OwnBreeding"),
            ),
                    digimon_dsl::compiled::CompiledBindingRef::TopOf(value) => {
                c.node(path, "digimon_dsl::compiled::CompiledBindingRef", "VARIANT", serde_json::json!("TopOf"));
                value.walk(c, &format!("{}@TopOf.0", path), "EXISTING_INSTANCE_BINDING");
            }
}
    }
}
impl Walk for digimon_dsl::compiled::CompiledModifierTarget {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledModifierTarget",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledModifierTarget::Binding(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierTarget",
                    "VARIANT",
                    serde_json::json!("Binding"),
                );
                f0.walk(
                    c,
                    &format!("{}@Binding.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledModifierTarget::Filter(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledModifierTarget",
                    "VARIANT",
                    serde_json::json!("Filter"),
                );
                f0.walk(c, &format!("{}@Filter.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledCostDelta {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledCostDelta",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledCostDelta::Free => c.node(
                path,
                "digimon_dsl::compiled::CompiledCostDelta",
                "ENUM_SCALAR",
                serde_json::json!("Free"),
            ),
            digimon_dsl::compiled::CompiledCostDelta::Printed => c.node(
                path,
                "digimon_dsl::compiled::CompiledCostDelta",
                "ENUM_SCALAR",
                serde_json::json!("Printed"),
            ),
            digimon_dsl::compiled::CompiledCostDelta::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledCostDelta",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::compiled::CompiledCostDelta::Reduce(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledCostDelta",
                    "VARIANT",
                    serde_json::json!("Reduce"),
                );
                f0.walk(c, &format!("{}@Reduce.0", path), role);
            }
            digimon_dsl::compiled::CompiledCostDelta::ReduceFn(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledCostDelta",
                    "VARIANT",
                    serde_json::json!("ReduceFn"),
                );
                f0.walk(c, &format!("{}@ReduceFn.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledStackPosition {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledStackPosition",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledStackPosition::Top => c.node(
                path,
                "digimon_dsl::compiled::CompiledStackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Top"),
            ),
            digimon_dsl::compiled::CompiledStackPosition::Bottom => c.node(
                path,
                "digimon_dsl::compiled::CompiledStackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Bottom"),
            ),
            digimon_dsl::compiled::CompiledStackPosition::Random => c.node(
                path,
                "digimon_dsl::compiled::CompiledStackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Random"),
            ),
            digimon_dsl::compiled::CompiledStackPosition::Choice => c.node(
                path,
                "digimon_dsl::compiled::CompiledStackPosition",
                "ENUM_SCALAR",
                serde_json::json!("Choice"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledRevealDestination {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledRevealDestination",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledRevealDestination::Hand => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::compiled::CompiledRevealDestination::DeckTop => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckTop"),
            ),
            digimon_dsl::compiled::CompiledRevealDestination::DeckBottom => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckBottom"),
            ),
            digimon_dsl::compiled::CompiledRevealDestination::PlayFree => c.node(
                path,
                "digimon_dsl::compiled::CompiledRevealDestination",
                "ENUM_SCALAR",
                serde_json::json!("PlayFree"),
            ),
            digimon_dsl::compiled::CompiledRevealDestination::BottomSourceOf(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledRevealDestination",
                    "VARIANT",
                    serde_json::json!("BottomSourceOf"),
                );
                f0.walk(c, &format!("{}@BottomSourceOf.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledRemainderDestination {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledRemainderDestination",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledRemainderDestination::DeckTop => c.node(
                path,
                "digimon_dsl::compiled::CompiledRemainderDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckTop"),
            ),
            digimon_dsl::compiled::CompiledRemainderDestination::DeckBottom => c.node(
                path,
                "digimon_dsl::compiled::CompiledRemainderDestination",
                "ENUM_SCALAR",
                serde_json::json!("DeckBottom"),
            ),
        }
    }
}

impl Walk for digimon_dsl::deck_rule::DeckCopyRule {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::deck_rule::DeckCopyRule", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::deck_rule::DeckCopyRule { max_copies, evidence_observation_id } = self;
        max_copies.walk(c, &format!("{}.max_copies", path), "UNCLASSIFIED_STRING");
        evidence_observation_id.walk(c, &format!("{}.evidence_observation_id", path), "METADATA_NO_EXECUTABLE_REFERENCE");
    }
}

impl Walk for digimon_dsl::clause::DnaMaterialResultScope {
    fn walk(&self, c: &mut Collector, path: &str, _role: &str) {
        match self {
            digimon_dsl::clause::DnaMaterialResultScope::OwnHand => c.node(path, "digimon_dsl::clause::DnaMaterialResultScope", "ENUM_SCALAR", serde_json::json!("OwnHand")),
        }
    }
}

impl Walk for digimon_dsl::compiled::CompiledDnaMaterialResultScope {
    fn walk(&self, c: &mut Collector, path: &str, _role: &str) {
        match self {
            digimon_dsl::compiled::CompiledDnaMaterialResultScope::OwnHand => c.node(path, "digimon_dsl::compiled::CompiledDnaMaterialResultScope", "ENUM_SCALAR", serde_json::json!("OwnHand")),
        }
    }
}
