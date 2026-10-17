//! Dedicated paged numeric choices. No game state is mutated by page construction.
//! IDs are phase-aware aliases, not additions to ACTION_SPACE_SIZE.
use crate::action::space::HAND_EFFECT_START;

pub const COUNT_CHOICES_PER_PAGE: usize = 28;
pub const COUNT_PREVIOUS_ACTION: u16 = HAND_EFFECT_START + 28;
pub const COUNT_NEXT_ACTION: u16 = HAND_EFFECT_START + 29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountChoiceValue {
    pub declared: u64,
    pub payable: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountChoiceOperation {
    Commit(CountChoiceValue),
    PreviousPage,
    NextPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountChoiceHost {
    pub player: crate::enums::PlayerId,
    pub field_index: u8,
    pub battle_identity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountChoiceMetadata {
    pub operation: CountChoiceOperation,
    pub page_index: u64,
    pub page_count: u64,
    pub host: Option<CountChoiceHost>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountChoiceRow {
    pub action_id: u16,
    pub metadata: CountChoiceMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountChoicePage {
    values: Vec<CountChoiceValue>,
    page_index: usize,
    pub host: Option<CountChoiceHost>,
}

impl CountChoicePage {
    /// Exact values are supplied by the owning game operation after legality
    /// evaluation. Counts must fit the DSL Literal(i64) binding. Every physical
    /// source Vec on supported targets already satisfies that allocation bound.
    /// Presentation metadata is never used to decide game legality.
    pub fn new(values: Vec<CountChoiceValue>, host: Option<CountChoiceHost>) -> Option<Self> {
        if values.is_empty()
            || values
                .iter()
                .any(|v| v.payable > v.declared || v.declared > i64::MAX as u64)
            || values.windows(2).any(|w| w[0].declared >= w[1].declared)
        {
            return None;
        }
        Some(Self {
            values,
            page_index: 0,
            host,
        })
    }

    pub fn page_index(&self) -> usize {
        self.page_index
    }
    pub fn page_count(&self) -> usize {
        self.values.len() / COUNT_CHOICES_PER_PAGE
            + usize::from(self.values.len() % COUNT_CHOICES_PER_PAGE != 0)
    }
    pub fn values(&self) -> &[CountChoiceValue] {
        &self.values
    }

    pub fn rows(&self) -> Vec<CountChoiceRow> {
        let start = self.page_index * COUNT_CHOICES_PER_PAGE;
        let end = self
            .values
            .len()
            .min(start.saturating_add(COUNT_CHOICES_PER_PAGE));
        let metadata = |operation| CountChoiceMetadata {
            operation,
            page_index: self.page_index as u64,
            page_count: self.page_count() as u64,
            host: self.host,
        };
        let mut rows: Vec<_> = self.values[start..end]
            .iter()
            .enumerate()
            .map(|(slot, value)| CountChoiceRow {
                action_id: HAND_EFFECT_START + slot as u16,
                metadata: metadata(CountChoiceOperation::Commit(*value)),
            })
            .collect();
        if self.page_index > 0 {
            rows.push(CountChoiceRow {
                action_id: COUNT_PREVIOUS_ACTION,
                metadata: metadata(CountChoiceOperation::PreviousPage),
            });
        }
        if self.page_index + 1 < self.page_count() {
            rows.push(CountChoiceRow {
                action_id: COUNT_NEXT_ACTION,
                metadata: metadata(CountChoiceOperation::NextPage),
            });
        }
        rows
    }

    /// Resolve against this page's actual offered IDs. Returning an operation
    /// does not apply payment. The caller must revalidate the physical pins.
    pub fn resolve(&mut self, action_id: u16) -> Option<CountChoiceOperation> {
        let operation = self
            .rows()
            .into_iter()
            .find(|row| row.action_id == action_id)?
            .metadata
            .operation;
        match operation {
            CountChoiceOperation::PreviousPage => self.page_index -= 1,
            CountChoiceOperation::NextPage => self.page_index += 1,
            CountChoiceOperation::Commit(_) => {}
        }
        Some(operation)
    }
}

/// Four normalized little-endian 16-bit limbs are exact and remain in the
/// observation Box bounds. Never cast an unbounded count directly to f32.
pub fn encode_count_u64(value: u64) -> [f32; 4] {
    std::array::from_fn(|i| ((value >> (i * 16)) & 0xffff) as f32 / 65535.0)
}

/// Install an explicit, clone-faithful count choice. A singleton still prompts.
/// The owner derives legal values and later revalidates any physical payment.
pub fn install_count_choice(
    ctx: &mut crate::effect_context::EffectContext<'_>,
    page: CountChoicePage,
    optional: bool,
    prompt: String,
    bind_as: Option<String>,
    inner_tail: Vec<digimon_dsl::compiled::CompiledStep>,
    bindings: crate::dsl_cards::bindings::Bindings,
    runtime: crate::dsl_cards::step::StepRuntime,
) {
    use crate::resume::{
        ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack,
    };
    use crate::selection::{PendingSelection, SelectionKind};
    let choices = choice_entries(&page, ctx.source_card, ctx.source_kind, optional);
    let valid_action_ids = choices.iter().map(|c| c.action_id).collect();
    let previous_phase = ctx.game.current_phase;
    let selecting_player = ctx.override_selecting_player().unwrap_or(ctx.player);
    let prov = ResumeProvenance {
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        controller: ctx.player,
        override_pin: ctx.override_selecting_player(),
    };
    let trigger_context = ctx.game.current_trigger_context.clone();
    ctx.game.current_phase = crate::enums::GamePhase::EffectChoice;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: optional,
        prompt,
        effect_choices: Some(choices),
        source_card: ctx.source_card,
        source_permanent: ctx.source_permanent,
        source_kind: ctx.source_kind,
        callback: Box::new(|_, _| panic!("count choice requires its typed resume frame")),
        on_decline: None,
        zone_owner: None,
    });
    ctx.game.pending_selection_resume = Some(ResumeStack {
        frames: vec![ResumeFrame::RunTail {
            prov,
            select_kind: ResumeSelectKind::CountChoice(page),
            bind_as,
            inner_tail: std::sync::Arc::new(inner_tail),
            outer_conts: Vec::new(),
            bindings,
            runtime,
            trigger_context,
            decline: if optional {
                ResumeDecline::RunTail {
                    tail: std::sync::Arc::new(Vec::new()),
                    aborts_clause: true,
                }
            } else {
                ResumeDecline::None
            },
        }],
    });
}

fn choice_entries(
    page: &CountChoicePage,
    source_card: crate::card_source::CardHandle,
    source_kind: crate::enums::EffectSourceKind,
    optional: bool,
) -> Vec<crate::selection::EffectChoiceEntry> {
    page.rows()
        .into_iter()
        .map(|row| crate::selection::EffectChoiceEntry {
            label: match row.metadata.operation {
                CountChoiceOperation::Commit(v) => {
                    format!("Declare {} ({} payable)", v.declared, v.payable)
                }
                CountChoiceOperation::PreviousPage => "Previous page".into(),
                CountChoiceOperation::NextPage => "Next page".into(),
            },
            action_id: row.action_id,
            source_card: Some(source_card),
            source_kind: Some(source_kind),
            timing: None,
            is_optional: optional,
            keyword: None,
            observation_metadata: Default::default(),
            count_choice: Some(row.metadata),
        })
        .collect()
}

/// Called after generic player/action validation but BEFORE taking the prompt,
/// restoring phase, executing hooks, or draining any game lifecycle. Only a
/// dedicated count frame can reinterpret58/59 as navigation.
pub(crate) fn navigate_count_choice(game: &mut crate::game::Game, action_id: u16) -> bool {
    if action_id != COUNT_PREVIOUS_ACTION && action_id != COUNT_NEXT_ACTION {
        return false;
    }
    let Some(stack) = game.pending_selection_resume.as_mut() else {
        return false;
    };
    let Some(page) = active_count_page_mut(stack) else {
        return false;
    };
    if !matches!(
        page.resolve(action_id),
        Some(CountChoiceOperation::PreviousPage | CountChoiceOperation::NextPage)
    ) {
        return false;
    }
    let Some(pending) = game.pending_selection.as_mut() else {
        return false;
    };
    let choices = choice_entries(
        page,
        pending.source_card,
        pending.source_kind,
        pending.is_optional,
    );
    pending.valid_action_ids = choices.iter().map(|c| c.action_id).collect();
    pending.effect_choices = Some(choices);
    true
}

pub(crate) fn active_count_page(stack: &crate::resume::ResumeStack) -> Option<&CountChoicePage> {
    use crate::resume::{ResumeFrame as F, ResumeSelectKind as K};
    match stack.frames.last()? {
        F::RunTail {
            select_kind:
                K::CountChoice(page)
                | K::SourcePrefixCount { page, .. }
                | K::DeDigivolveCount { page, .. },
            ..
        } => Some(page),
        F::ReplacementBody { inner, .. }
        | F::FieldTopSourceBatch { inner, .. }
        | F::DelayCancelAfterSelection { inner, .. }
        | F::DelayHandDigivolveAfterSelection { inner, .. }
        | F::DelayDnaAfterSelection { inner, .. }
        | F::DelayFieldDnaAfterPayment { inner, .. }
        | F::DelayPlayFromHandAfterSelection { inner, .. }
        | F::DelayPlayFromUnionAfterSelection { inner, .. }
        | F::DelayBodyAfterCost { inner, .. } => active_count_page(inner),
        _ => None,
    }
}

pub(crate) fn active_count_page_mut(
    stack: &mut crate::resume::ResumeStack,
) -> Option<&mut CountChoicePage> {
    use crate::resume::{ResumeFrame as F, ResumeSelectKind as K};
    match stack.frames.last_mut()? {
        F::RunTail {
            select_kind:
                K::CountChoice(page)
                | K::SourcePrefixCount { page, .. }
                | K::DeDigivolveCount { page, .. },
            ..
        } => Some(page),
        F::ReplacementBody { inner, .. }
        | F::FieldTopSourceBatch { inner, .. }
        | F::DelayCancelAfterSelection { inner, .. }
        | F::DelayHandDigivolveAfterSelection { inner, .. }
        | F::DelayDnaAfterSelection { inner, .. }
        | F::DelayFieldDnaAfterPayment { inner, .. }
        | F::DelayPlayFromHandAfterSelection { inner, .. }
        | F::DelayPlayFromUnionAfterSelection { inner, .. }
        | F::DelayBodyAfterCost { inner, .. } => active_count_page_mut(inner),
        _ => None,
    }
}
