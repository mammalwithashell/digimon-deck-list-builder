// Explicit typed patterns: all fields and variants, no rest/wildcard patterns.
use super::{Collector, Walk};
impl Walk for digimon_dsl::formula::FormulaSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::FormulaSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::formula::FormulaSpec::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::formula::FormulaSpec::BasePerDelta {
                base,
                per,
                bucket,
                delta,
            } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("BasePerDelta"),
                );
                base.walk(
                    c,
                    &format!("{}.base", &format!("{}@BasePerDelta", path)),
                    "UNCLASSIFIED_STRING",
                );
                per.walk(
                    c,
                    &format!("{}.per", &format!("{}@BasePerDelta", path)),
                    "UNCLASSIFIED_STRING",
                );
                bucket.walk(
                    c,
                    &format!("{}.bucket", &format!("{}@BasePerDelta", path)),
                    "UNCLASSIFIED_STRING",
                );
                delta.walk(
                    c,
                    &format!("{}.delta", &format!("{}@BasePerDelta", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::BindingDp { binding_dp } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("BindingDp"),
                );
                binding_dp.walk(
                    c,
                    &format!("{}.binding_dp", &format!("{}@BindingDp", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::BindingPlayCost { binding_play_cost } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("BindingPlayCost"),
                );
                binding_play_cost.walk(
                    c,
                    &format!("{}.binding_play_cost", &format!("{}@BindingPlayCost", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::BindingValue { binding_value } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("BindingValue"),
                );
                binding_value.walk(
                    c,
                    &format!("{}.binding_value", &format!("{}@BindingValue", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::formula::FormulaSpec::SourceDp { source_dp } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("SourceDp"),
                );
                source_dp.walk(
                    c,
                    &format!("{}.source_dp", &format!("{}@SourceDp", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::SourceMaterialCount {
                source_material_count,
            } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("SourceMaterialCount"),
                );
                source_material_count.walk(
                    c,
                    &format!(
                        "{}.source_material_count",
                        &format!("{}@SourceMaterialCount", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::EventTargetLevel { event_target_level } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("EventTargetLevel"),
                );
                event_target_level.walk(
                    c,
                    &format!(
                        "{}.event_target_level",
                        &format!("{}@EventTargetLevel", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::SourceColorCount { source_color_count } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("SourceColorCount"),
                );
                source_color_count.walk(
                    c,
                    &format!(
                        "{}.source_color_count",
                        &format!("{}@SourceColorCount", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::SourceStackCount { source_stack_count } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("SourceStackCount"),
                );
                source_stack_count.walk(
                    c,
                    &format!(
                        "{}.source_stack_count",
                        &format!("{}@SourceStackCount", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::SourceStackDpSum {
                source_stack_dp_sum,
            } => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("SourceStackDpSum"),
                );
                source_stack_dp_sum.walk(
                    c,
                    &format!(
                        "{}.source_stack_dp_sum",
                        &format!("{}@SourceStackDpSum", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::FormulaSpec::Compound(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::FormulaSpec",
                    "VARIANT",
                    serde_json::json!("Compound"),
                );
                f0.walk(c, &format!("{}@Compound.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::formula::SourceDpSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::SourceDpSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::formula::SourceDpSpec {} = self;
    }
}
impl Walk for digimon_dsl::formula::SourceStackDpSumSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::SourceStackDpSumSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::formula::SourceStackDpSumSpec { target, filter } = self;
        target.walk(c, &format!("{}.target", path), "EXISTING_INSTANCE_BINDING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::formula::CompoundFormula {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::CompoundFormula",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::formula::CompoundFormula::FloorDiv(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("FloorDiv"),
                );
                f0.walk(c, &format!("{}@FloorDiv.0", path), role);
            }
            digimon_dsl::formula::CompoundFormula::Max(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("Max"),
                );
                f0.walk(c, &format!("{}@Max.0", path), role);
            }
            digimon_dsl::formula::CompoundFormula::Min(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("Min"),
                );
                f0.walk(c, &format!("{}@Min.0", path), role);
            }
            digimon_dsl::formula::CompoundFormula::Subtract(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("Subtract"),
                );
                f0.walk(c, &format!("{}@Subtract.0", path), role);
            }
            digimon_dsl::formula::CompoundFormula::Multiply(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("Multiply"),
                );
                f0.walk(c, &format!("{}@Multiply.0", path), role);
            }
            digimon_dsl::formula::CompoundFormula::Aggregate(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("Aggregate"),
                );
                f0.walk(c, &format!("{}@Aggregate.0", path), role);
            }
            digimon_dsl::formula::CompoundFormula::AggregateScoped(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("AggregateScoped"),
                );
                f0.walk(c, &format!("{}@AggregateScoped.0", path), role);
            }
            digimon_dsl::formula::CompoundFormula::RawRust(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::CompoundFormula",
                    "VARIANT",
                    serde_json::json!("RawRust"),
                );
                f0.walk(c, &format!("{}@RawRust.0", path), "HOOK_FORMULA");
            }
        }
    }
}
impl Walk for digimon_dsl::formula::AggregateFormulaSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::AggregateFormulaSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::formula::AggregateFormulaSpec { selector, scope } = self;
        selector.walk(c, &format!("{}.selector", path), "UNCLASSIFIED_STRING");
        scope.walk(c, &format!("{}.scope", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::formula::CardCountInZoneSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::CardCountInZoneSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::formula::CardCountInZoneSpec { zone, of, filter } = self;
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::formula::PerSelector {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::PerSelector",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::formula::PerSelector::MaterialCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("MaterialCount"),
            ),
            digimon_dsl::formula::PerSelector::StackSize => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("StackSize"),
            ),
            digimon_dsl::formula::PerSelector::AllyCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("AllyCount"),
            ),
            digimon_dsl::formula::PerSelector::SuspendedCount { of, exclude_source } => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("SuspendedCount"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@SuspendedCount", path)),
                    "UNCLASSIFIED_STRING",
                );
                exclude_source.walk(
                    c,
                    &format!("{}.exclude_source", &format!("{}@SuspendedCount", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::PerSelector::DigivolutionColorCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("DigivolutionColorCount"),
            ),
            digimon_dsl::formula::PerSelector::SourceColorCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SourceColorCount"),
            ),
            digimon_dsl::formula::PerSelector::ReturnedCardColorCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("ReturnedCardColorCount"),
            ),
            digimon_dsl::formula::PerSelector::EffectSuspendedCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("EffectSuspendedCount"),
            ),
            digimon_dsl::formula::PerSelector::SameLevelPairsInSources => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SameLevelPairsInSources"),
            ),
            digimon_dsl::formula::PerSelector::SharedTrashCount { bucket } => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("SharedTrashCount"),
                );
                bucket.walk(
                    c,
                    &format!("{}.bucket", &format!("{}@SharedTrashCount", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::PerSelector::CardCountInZone(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("CardCountInZone"),
                );
                f0.walk(c, &format!("{}@CardCountInZone.0", path), role);
            }
            digimon_dsl::formula::PerSelector::DistinctColorsCount(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("DistinctColorsCount"),
                );
                f0.walk(c, &format!("{}@DistinctColorsCount.0", path), role);
            }
            digimon_dsl::formula::PerSelector::DistinctNamesCount(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("DistinctNamesCount"),
                );
                f0.walk(c, &format!("{}@DistinctNamesCount.0", path), role);
            }
            digimon_dsl::formula::PerSelector::SourceStackCount(f0) => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("SourceStackCount"),
                );
                f0.walk(c, &format!("{}@SourceStackCount.0", path), role);
            }
            digimon_dsl::formula::PerSelector::PlayerMemory { of } => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("PlayerMemory"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@PlayerMemory", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::PerSelector::OwnLinkCardCount { of } => {
                c.node(
                    path,
                    "digimon_dsl::formula::PerSelector",
                    "VARIANT",
                    serde_json::json!("OwnLinkCardCount"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@OwnLinkCardCount", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::formula::PerSelector::SourceLinkCardCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SourceLinkCardCount"),
            ),
            digimon_dsl::formula::PerSelector::SourceRulesColorCount => c.node(
                path,
                "digimon_dsl::formula::PerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SourceRulesColorCount"),
            ),
        }
    }
}
impl Walk for digimon_dsl::formula::SourceStackCountSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::SourceStackCountSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::formula::SourceStackCountSpec { filter } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::formula::AggregateSelector {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::formula::AggregateSelector",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::formula::AggregateSelector::LowestDp => c.node(
                path,
                "digimon_dsl::formula::AggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestDp"),
            ),
            digimon_dsl::formula::AggregateSelector::HighestDp => c.node(
                path,
                "digimon_dsl::formula::AggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestDp"),
            ),
            digimon_dsl::formula::AggregateSelector::LowestLevel => c.node(
                path,
                "digimon_dsl::formula::AggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestLevel"),
            ),
            digimon_dsl::formula::AggregateSelector::HighestLevel => c.node(
                path,
                "digimon_dsl::formula::AggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("HighestLevel"),
            ),
            digimon_dsl::formula::AggregateSelector::FewestMaterials => c.node(
                path,
                "digimon_dsl::formula::AggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("FewestMaterials"),
            ),
            digimon_dsl::formula::AggregateSelector::LowestPlayCost => c.node(
                path,
                "digimon_dsl::formula::AggregateSelector",
                "ENUM_SCALAR",
                serde_json::json!("LowestPlayCost"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledFormula {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledFormula",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledFormula::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::compiled::CompiledFormula::BasePerDelta { base, per, delta } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("BasePerDelta"),
                );
                base.walk(
                    c,
                    &format!("{}.base", &format!("{}@BasePerDelta", path)),
                    "UNCLASSIFIED_STRING",
                );
                per.walk(
                    c,
                    &format!("{}.per", &format!("{}@BasePerDelta", path)),
                    "UNCLASSIFIED_STRING",
                );
                delta.walk(
                    c,
                    &format!("{}.delta", &format!("{}@BasePerDelta", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledFormula::FloorDiv(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("FloorDiv"),
                );
                f0.walk(c, &format!("{}@FloorDiv.0", path), role);
            }
            digimon_dsl::compiled::CompiledFormula::Max(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("Max"),
                );
                f0.walk(c, &format!("{}@Max.0", path), role);
            }
            digimon_dsl::compiled::CompiledFormula::Min(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("Min"),
                );
                f0.walk(c, &format!("{}@Min.0", path), role);
            }
            digimon_dsl::compiled::CompiledFormula::Subtract(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("Subtract"),
                );
                f0.walk(c, &format!("{}@Subtract.0", path), role);
            }
            digimon_dsl::compiled::CompiledFormula::Multiply(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("Multiply"),
                );
                f0.walk(c, &format!("{}@Multiply.0", path), role);
            }
            digimon_dsl::compiled::CompiledFormula::Aggregate(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("Aggregate"),
                );
                f0.walk(c, &format!("{}@Aggregate.0", path), role);
            }
            digimon_dsl::compiled::CompiledFormula::AggregateScoped { selector, scope } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("AggregateScoped"),
                );
                selector.walk(
                    c,
                    &format!("{}.selector", &format!("{}@AggregateScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
                scope.walk(
                    c,
                    &format!("{}.scope", &format!("{}@AggregateScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledFormula::BindingDp(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("BindingDp"),
                );
                f0.walk(
                    c,
                    &format!("{}@BindingDp.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledFormula::BindingPlayCost(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("BindingPlayCost"),
                );
                f0.walk(
                    c,
                    &format!("{}@BindingPlayCost.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledFormula::BindingValue(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("BindingValue"),
                );
                f0.walk(
                    c,
                    &format!("{}@BindingValue.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
            }
            digimon_dsl::compiled::CompiledFormula::SourceDp => c.node(
                path,
                "digimon_dsl::compiled::CompiledFormula",
                "ENUM_SCALAR",
                serde_json::json!("SourceDp"),
            ),
            digimon_dsl::compiled::CompiledFormula::SourceMaterialCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledFormula",
                "ENUM_SCALAR",
                serde_json::json!("SourceMaterialCount"),
            ),
            digimon_dsl::compiled::CompiledFormula::EventTargetLevel => c.node(
                path,
                "digimon_dsl::compiled::CompiledFormula",
                "ENUM_SCALAR",
                serde_json::json!("EventTargetLevel"),
            ),
            digimon_dsl::compiled::CompiledFormula::SourceColorCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledFormula",
                "ENUM_SCALAR",
                serde_json::json!("SourceColorCount"),
            ),
            digimon_dsl::compiled::CompiledFormula::SourceStackCount { target, filter } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("SourceStackCount"),
                );
                target.walk(
                    c,
                    &format!("{}.target", &format!("{}@SourceStackCount", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@SourceStackCount", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledFormula::SourceStackDpSum { target, filter } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("SourceStackDpSum"),
                );
                target.walk(
                    c,
                    &format!("{}.target", &format!("{}@SourceStackDpSum", path)),
                    "EXISTING_INSTANCE_BINDING",
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@SourceStackDpSum", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledFormula::RawRust(f0) => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledFormula",
                    "VARIANT",
                    serde_json::json!("RawRust"),
                );
                f0.walk(c, &format!("{}@RawRust.0", path), "HOOK_FORMULA");
            }
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledPerSelector {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledPerSelector",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::compiled::CompiledPerSelector::MaterialCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("MaterialCount"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::StackSize => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("StackSize"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::AllyCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("AllyCount"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::SuspendedCount { of, exclude_source } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("SuspendedCount"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@SuspendedCount", path)),
                    "UNCLASSIFIED_STRING",
                );
                exclude_source.walk(
                    c,
                    &format!("{}.exclude_source", &format!("{}@SuspendedCount", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::DigivolutionColorCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("DigivolutionColorCount"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::SourceColorCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SourceColorCount"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::ReturnedCardColorCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("ReturnedCardColorCount"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::EffectSuspendedCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("EffectSuspendedCount"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::SameLevelPairsInSources => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SameLevelPairsInSources"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::SharedTrashCount { bucket } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("SharedTrashCount"),
                );
                bucket.walk(
                    c,
                    &format!("{}.bucket", &format!("{}@SharedTrashCount", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::CardCountInZone => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("CardCountInZone"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::CardCountInZoneScoped { zone, of } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("CardCountInZoneScoped"),
                );
                zone.walk(
                    c,
                    &format!("{}.zone", &format!("{}@CardCountInZoneScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@CardCountInZoneScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::FilteredCardCountInZoneScoped {
                zone,
                of,
                filter,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("FilteredCardCountInZoneScoped"),
                );
                zone.walk(
                    c,
                    &format!(
                        "{}.zone",
                        &format!("{}@FilteredCardCountInZoneScoped", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@FilteredCardCountInZoneScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
                filter.walk(
                    c,
                    &format!(
                        "{}.filter",
                        &format!("{}@FilteredCardCountInZoneScoped", path)
                    ),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::DistinctColorsCountScoped {
                zone,
                of,
                filter,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("DistinctColorsCountScoped"),
                );
                zone.walk(
                    c,
                    &format!("{}.zone", &format!("{}@DistinctColorsCountScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@DistinctColorsCountScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@DistinctColorsCountScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::DistinctNamesCountScoped {
                zone,
                of,
                filter,
            } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("DistinctNamesCountScoped"),
                );
                zone.walk(
                    c,
                    &format!("{}.zone", &format!("{}@DistinctNamesCountScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@DistinctNamesCountScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@DistinctNamesCountScoped", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::SourceStackCountFiltered { filter } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("SourceStackCountFiltered"),
                );
                filter.walk(
                    c,
                    &format!("{}.filter", &format!("{}@SourceStackCountFiltered", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::PlayerMemory { of } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("PlayerMemory"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@PlayerMemory", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::OwnLinkCardCount { of } => {
                c.node(
                    path,
                    "digimon_dsl::compiled::CompiledPerSelector",
                    "VARIANT",
                    serde_json::json!("OwnLinkCardCount"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@OwnLinkCardCount", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
            digimon_dsl::compiled::CompiledPerSelector::SourceLinkCardCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SourceLinkCardCount"),
            ),
            digimon_dsl::compiled::CompiledPerSelector::SourceRulesColorCount => c.node(
                path,
                "digimon_dsl::compiled::CompiledPerSelector",
                "ENUM_SCALAR",
                serde_json::json!("SourceRulesColorCount"),
            ),
        }
    }
}
