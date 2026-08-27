//! The normalized per-step state projection, plus the parser for the DCGO
//! `StateDumper` sidecar that produces the other side of the comparison.
//!
//! # The governing rule: normalize representation, never semantics
//!
//! Both engines describe the same board, but they *write it down* differently:
//! zones come out in different orders, DP is stored printed-plus-modifiers on
//! one side and as a computed property on the other. Every one of those
//! differences is **representation**, and a differ that reports them produces
//! noise that buries the one real finding.
//!
//! So this projection normalizes representation aggressively:
//!
//! - `hand`, `trash` and `sources` are **sorted on construction**, which makes
//!   the derived `PartialEq` a multiset compare with no custom comparator that
//!   could drift away from the constructor.
//! - `field` is sorted by `(card_id, dp, suspended, sources)` — deliberately
//!   *not* by keywords, so an unmeasured keyword list can never reorder a side.
//!
//! and it normalizes **semantics** not at all:
//!
//! - Duplicate counts still matter. This is a multiset, not a set: holding two
//!   copies of a card is not the same board as holding one.
//! - `suspended` is semantics and must diff.
//! - `dp` is **effective** DP, after modifiers — the printed value would make
//!   every buffed Digimon read as a divergence. `-1` means "no DP" (a Tamer or
//!   an Option), matching DCGO's `Permanent.DP` which returns `-1` when
//!   `!HasDP`.
//! - `security` is a **count** only. The contents are hidden information, and
//!   diffing them would let the differ "confirm" a line whose legality
//!   depended on knowing them.
//!
//! # Keywords are `Option`: `None` means NOT MEASURED
//!
//! DCGO has no keyword collection — it has 22 independent bool properties
//! (`HasBlocker`, `HasJamming`, …), each of which walks the whole field's
//! effect lists. Dumping all of them per permanent per step is
//! `O(22 x field x effects)`, so the DCGO-side dumper puts keywords behind a
//! flag that defaults **off** and the sidecar usually omits them.
//!
//! An omitted keyword list therefore means *"nobody looked"*, not *"this
//! Digimon has no keywords"*. Modelling it as an empty `Vec` would make every
//! keyword-carrying permanent in every keyword-flag-off run report a false
//! divergence. It is an `Option<Vec<String>>`, and `None` compares equal to
//! any `Some(..)` — see [`PermanentProjection`]'s hand-written `PartialEq`.
//!
//! Because of that wildcard, equality here is a **compatibility relation**:
//! reflexive and symmetric but deliberately *not* transitive
//! (`Some(["Blocker"]) == None` and `None == Some(["Rush"])`, yet the two
//! `Some`s differ). That is why these types implement `PartialEq` but
//! **not** `Eq` — claiming `Eq` would assert a total equivalence the type does
//! not have, and would be unsound to rely on in a `HashMap` key.

use serde::{Deserialize, Serialize};

use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::memory_from_recording_perspective;
use digimon_engine::enums::{Keyword, PlayerId};
use digimon_engine::{Game, PermanentHandle};

/// The keyword vocabulary shared with the DCGO `StateDumper`.
///
/// Deliberately *not* every `Keyword` our engine knows: the DCGO side can only
/// report the keywords it has a `Has…` bool property for, so probing anything
/// outside this set would produce a value the other side can never match. The
/// string on the left is the wire name (DCGO's getter minus its `Has` prefix);
/// the `Keyword` on the right is our engine's equivalent. Note `Pierce` /
/// `Piercing` — the two engines spell that one differently.
pub const KEYWORD_PROBES: &[(&str, Keyword)] = &[
    ("Ascension", Keyword::Ascension),
    ("Blitz", Keyword::Blitz),
    ("Blocker", Keyword::Blocker),
    ("Engage", Keyword::Engage),
    ("Evade", Keyword::Evade),
    ("Fortitude", Keyword::Fortitude),
    ("Guard", Keyword::Guard),
    ("Iceclad", Keyword::Iceclad),
    ("Jamming", Keyword::Jamming),
    ("MindLink", Keyword::MindLink),
    ("Pierce", Keyword::Piercing),
    ("Raid", Keyword::Raid),
    ("Reboot", Keyword::Reboot),
    ("Retaliation", Keyword::Retaliation),
    ("Rush", Keyword::Rush),
];

/// One permanent on the battle area, normalized.
///
/// `card_id` is the **top** card of the digivolution stack; `sources` is
/// everything underneath it (so a freshly played Digimon has an empty
/// `sources`). `dp` is effective DP, `-1` when the permanent has no DP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermanentProjection {
    pub card_id: String,
    pub dp: i64,
    pub suspended: bool,
    #[serde(default)]
    pub sources: Vec<String>,
    /// `None` = not measured (the DCGO dumper's keyword flag was off).
    /// Never compares as a divergence — see the module docs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
}

impl PermanentProjection {
    fn normalize(&mut self) {
        self.sources.sort();
        if let Some(kw) = self.keywords.as_mut() {
            kw.sort();
            kw.dedup();
        }
    }

    /// The sort key used to order a seat's field. Excludes `keywords` on
    /// purpose: an unmeasured list must not be able to reorder one side
    /// relative to the other.
    fn sort_key(&self) -> (&str, i64, bool, &[String]) {
        (&self.card_id, self.dp, self.suspended, &self.sources)
    }
}

/// Hand-written so an unmeasured keyword list (`None`) is a wildcard rather
/// than "no keywords". Every other field compares exactly.
impl PartialEq for PermanentProjection {
    fn eq(&self, other: &Self) -> bool {
        if self.card_id != other.card_id
            || self.dp != other.dp
            || self.suspended != other.suspended
            || self.sources != other.sources
        {
            return false;
        }
        match (&self.keywords, &other.keywords) {
            // At least one side did not measure keywords — not a divergence.
            (None, _) | (_, None) => true,
            (Some(a), Some(b)) => a == b,
        }
    }
}

/// One player's visible state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeatProjection {
    /// A count, never the contents — security is hidden information.
    pub security: usize,
    pub hand: Vec<String>,
    pub trash: Vec<String>,
    pub field: Vec<PermanentProjection>,
}

impl SeatProjection {
    fn normalize(&mut self) {
        self.hand.sort();
        self.trash.sort();
        for perm in &mut self.field {
            perm.normalize();
        }
        // Stable sort: two permanents identical apart from keywords keep
        // their relative order rather than being shuffled by a tie-break the
        // other side cannot reproduce.
        self.field.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    }

    fn from_game(game: &Game, pid: PlayerId, measure_keywords: bool) -> Self {
        let data: &[CardData] = &game.card_data;
        let player = game.player(pid);

        let field = player
            .battle_area
            .iter()
            .enumerate()
            .map(|(index, perm)| {
                let handle = PermanentHandle {
                    player: pid,
                    index: index as u8,
                };
                // `split_last` rather than indexing: the top card is the last
                // source, everything before it is the digivolution stack.
                let (top, beneath) = perm
                    .card_sources
                    .split_last()
                    .expect("a permanent always has at least its top card");
                PermanentProjection {
                    card_id: top.card_id(data).to_string(),
                    // DCGO's `Permanent.DP` yields -1 when the permanent has
                    // no DP; `effective_dp` yields None for the same case.
                    dp: game.effective_dp(handle).map(i64::from).unwrap_or(-1),
                    suspended: perm.is_suspended,
                    sources: beneath
                        .iter()
                        .map(|src| src.card_id(data).to_string())
                        .collect(),
                    keywords: measure_keywords.then(|| {
                        KEYWORD_PROBES
                            .iter()
                            .filter(|(_, kw)| game.has_keyword(handle, *kw))
                            .map(|(name, _)| (*name).to_string())
                            .collect()
                    }),
                }
            })
            .collect();

        let mut seat = SeatProjection {
            security: player.security.len(),
            hand: player
                .hand
                .iter()
                .map(|src| src.card_id(data).to_string())
                .collect(),
            trash: player
                .trash
                .iter()
                .map(|src| src.card_id(data).to_string())
                .collect(),
            field,
        };
        seat.normalize();
        seat
    }
}

/// The whole board after one step, in the one shape both engines emit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateProjection {
    pub step: u32,
    pub turn: u64,
    pub phase: String,
    /// Memory **from player 0's perspective**: positive favors p0.
    ///
    /// Our engine stores the seesaw relative to `memory_pair.0`, which flips
    /// every turn; DCGO stores it relative to a fixed PlayerID. Comparing the
    /// raw numbers silently inverts on alternating turns, so both sides are
    /// pinned to p0 here via the engine's own conversion.
    pub memory: i64,
    pub p0: SeatProjection,
    pub p1: SeatProjection,
}

impl StateProjection {
    fn normalize(&mut self) {
        self.p0.normalize();
        self.p1.normalize();
    }

    /// Project our engine's live state. Keywords are left **unmeasured**
    /// (`None`), matching the DCGO dumper's default; use
    /// [`StateProjection::from_game_with_keywords`] for a scenario whose
    /// clause under test is a keyword clause.
    pub fn from_game(game: &Game, step: u32) -> StateProjection {
        Self::project(game, step, false)
    }

    /// Same, but probes [`KEYWORD_PROBES`] on every permanent. Only worth
    /// paying for when the DCGO side also ran with its keyword flag on —
    /// otherwise the extra work buys a wildcard comparison anyway.
    pub fn from_game_with_keywords(game: &Game, step: u32) -> StateProjection {
        Self::project(game, step, true)
    }

    fn project(game: &Game, step: u32, measure_keywords: bool) -> StateProjection {
        StateProjection {
            step,
            turn: u64::from(game.turn_count),
            phase: game.current_phase.py_name().to_string(),
            memory: i64::from(memory_from_recording_perspective(
                game.memory,
                game.memory_pair.0,
                0,
            )),
            p0: SeatProjection::from_game(game, 0, measure_keywords),
            p1: SeatProjection::from_game(game, 1, measure_keywords),
        }
    }

    /// Parse one `.state.jsonl` row. Trailing content after the JSON value is
    /// an error rather than being ignored.
    pub fn from_sidecar_line(line: &str) -> Result<StateProjection, String> {
        let mut projection: StateProjection =
            serde_json::from_str(line).map_err(|e| format!("malformed state row: {e}"))?;
        projection.normalize();
        Ok(projection)
    }
}

/// Parse a whole `.state.jsonl` sidecar.
///
/// Rows are whitespace-separated JSON values, so a pretty-printed row that
/// wraps across lines parses as one row. A row that will not parse is a hard
/// error: silently dropping it would shorten one side of the diff and
/// misalign every step after it, turning a parse bug into a phantom
/// engine divergence.
/// Drop the DCGO sidecar rows that precede the scenario's own origin.
///
/// A scenario line begins AFTER the mulligan: `ScenarioAdapter` resolves both
/// mulligans itself (keep) and DCGO is pinned to match, consuming no scripted
/// step for it. But DCGO still RECORDS the mulligan as ordinary `action` rows,
/// and `StateDumper` dumps state at those steps -- so the sidecar carries two
/// leading rows our trace has no counterpart for.
///
/// The differ aligns by step index, so without this the two traces sit two
/// steps apart and EVERY scenario reports a divergence at step 0 with
/// `security: ours=5 dcgo=0` (DCGO's step 0 is pre-mulligan, before security is
/// laid). That is an artifact of the harness, not a finding about the card, and
/// it would drown every real divergence beneath it.
///
/// Takes the recording alongside the sidecar because `source: "mulligan"` lives
/// on the `action` row, not in the state dump -- the sidecar's `phase` field
/// reads "Active" at that point and cannot identify it.
///
/// Returns the sidecar re-indexed from 0 so both traces share an origin.
pub fn align_to_scenario_origin(
    sidecar: Vec<StateProjection>,
    recording_text: &str,
) -> Result<Vec<StateProjection>, String> {
    let mut mulligan_steps = std::collections::HashSet::new();
    for line in recording_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("malformed recording row: {e}"))?;
        if row.get("type").and_then(|v| v.as_str()) == Some("action")
            && row.get("source").and_then(|v| v.as_str()) == Some("mulligan")
        {
            if let Some(s) = row.get("step").and_then(|v| v.as_u64()) {
                mulligan_steps.insert(s as u32);
            }
        }
    }

    let mut kept: Vec<StateProjection> = sidecar
        .into_iter()
        .filter(|p| !mulligan_steps.contains(&p.step))
        .collect();

    // Re-index from 0: the scenario's step N must line up with the Nth
    // post-mulligan decision, whatever absolute index DCGO gave it.
    for (i, p) in kept.iter_mut().enumerate() {
        p.step = i as u32;
    }
    Ok(kept)
}

/// Which of our rows pairs with which DCGO row, derived from the LOWERED
/// LINE rather than from either trace's length.
///
/// See [`pair_by_wire_rows`] for why this exists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepPairing {
    /// `(our row index, DCGO row index)` for every comparable step, in line
    /// order.
    pub pairs: Vec<(usize, usize)>,
    /// Our rows deliberately excluded: steps that put NOTHING on the DCGO
    /// wire (`LoweredStep::SimOnlyAction`).
    pub ours_unpairable: u32,
    /// DCGO rows deliberately excluded: the 2nd..Nth row of a step that
    /// consumes several. Both traces are PRE-decision snapshots, so a fold's
    /// second row is a mid-resolution state our trace never materializes.
    pub dcgo_unpairable: u32,
}

/// Pair the two traces by LOWERED SCENARIO STEP, not by decision count.
///
/// # Why positional alignment is wrong here
///
/// Every scenario step is one row in OUR trace (the pre-step snapshot), but a
/// step can consume one, two, or zero DCGO decision rows — the job emitter
/// already knows which, because it is the thing that writes them:
///
/// | `LoweredStep` | DCGO wire rows |
/// |---|---|
/// | `Action` | 1 |
/// | `Select` (plain, or a folded `decline:`) | 1 |
/// | `Select` (folded pick: `OptionalSkill(yes)` + the pick) | 2 |
/// | `EndOfTurnGate { attack: None }` | 1 |
/// | `EndOfTurnGate { attack: Some(_) }` | 2 |
/// | `SimOnlyAction` | 0 |
///
/// Aligning 1:1 after mulligan-stripping therefore slides the two traces apart
/// from the first multi-row step onward and manufactures divergences out of an
/// offset. Measured witness (EX12-011#effect#0): a 9-step line against a
/// 10-row DCGO trace put our post-battle step 8 opposite DCGO's *pre*-battle
/// mid-fold row, reporting `p1.trash: ours=[EX12-020] dcgo=[]` — while DCGO's
/// step 13, the row that step really lands on, read `p1.trash:["EX12-020"]`,
/// identical to ours on every projected field.
///
/// # The rule
///
/// Our row `i` pairs with the **first** DCGO row step `i` consumes. The
/// 2nd..Nth rows of a multi-row step are dropped, and so is our row for a
/// `SimOnlyAction` step — but both are COUNTED, in
/// [`StepPairing::dcgo_unpairable`] / [`StepPairing::ours_unpairable`], and
/// the differ prints them. A dropped row that vanished from the denominator
/// would turn "we could not compare that row" into "that row agreed".
///
/// Rows beyond what the line predicts are left unaccounted on purpose: they
/// then show up as a denominator shortfall, which is exactly what a DCGO trace
/// that ran long or short *is*.
pub fn pair_by_wire_rows(rows_per_step: &[usize], dcgo_len: usize) -> StepPairing {
    let mut pairing = StepPairing::default();
    let mut cursor = 0usize;
    for (i, &rows) in rows_per_step.iter().enumerate() {
        if rows == 0 {
            pairing.ours_unpairable += 1;
            continue;
        }
        if cursor < dcgo_len {
            pairing.pairs.push((i, cursor));
        }
        for extra in 1..rows {
            if cursor + extra < dcgo_len {
                pairing.dcgo_unpairable += 1;
            }
        }
        cursor += rows;
    }
    pairing
}

pub fn parse_sidecar(text: &str) -> Result<Vec<StateProjection>, String> {
    let stream = serde_json::Deserializer::from_str(text).into_iter::<StateProjection>();
    let mut rows = Vec::new();
    for (i, row) in stream.enumerate() {
        let mut row = row.map_err(|e| format!("state sidecar row {i}: {e}"))?;
        row.normalize();
        rows.push(row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: &str = r#"{"step":3,"turn":2,"phase":"Main","memory":-2,
        "p0":{"security":5,"hand":["ST1-02","ST1-03"],"trash":[],
              "field":[{"card_id":"EX12-035","dp":4000,"suspended":false,
                        "sources":["ST1-02"],"keywords":["Blocker"]}]},
        "p1":{"security":4,"hand":[],"trash":["BT16-082"],"field":[]}}"#;

    #[test]
    fn parses_a_sidecar_row() {
        let p = StateProjection::from_sidecar_line(ROW).unwrap();
        assert_eq!(p.step, 3);
        assert_eq!(p.memory, -2);
        assert_eq!(p.p0.security, 5);
        assert_eq!(p.p0.field.len(), 1);
        assert_eq!(p.p0.field[0].dp, 4000);
        assert_eq!(p.p1.trash, vec!["BT16-082"]);
    }

    #[test]
    fn hand_and_trash_compare_as_multisets_not_sequences() {
        // Zone ORDER is representation -- the two engines order zones
        // differently and an order-sensitive diff would be pure noise.
        let a = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A","B"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let b = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["B","A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        assert_eq!(a, b, "hand order must not be a divergence");
    }

    #[test]
    fn duplicate_counts_still_matter() {
        // Multiset, not set: holding two copies is not the same as one.
        let a = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A","A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        let b = StateProjection::from_sidecar_line(
            r#"{"step":0,"turn":1,"phase":"Main","memory":0,
                "p0":{"security":5,"hand":["A"],"trash":[],"field":[]},
                "p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#,
        )
        .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn suspended_is_semantics_and_must_differ() {
        let a = StateProjection::from_sidecar_line(ROW).unwrap();
        let b = StateProjection::from_sidecar_line(
            &ROW.replace("\"suspended\":false", "\"suspended\":true"),
        )
        .unwrap();
        assert_ne!(a, b, "suspended is semantics, not representation");
    }


    #[test]
    fn align_drops_leading_mulligan_rows_and_reindexes() {
        // The live shape: DCGO records two mulligan actions at steps 0-1, and
        // the sidecar dumps state at those steps. Our scenario trace has no
        // counterpart, so without alignment every scenario "diverges at step 0".
        let rec = concat!(
            r#"{"type":"action","step":0,"actor":0,"action_id":0,"source":"mulligan"}"#, "
",
            r#"{"type":"action","step":1,"actor":1,"action_id":0,"source":"mulligan"}"#, "
",
            r#"{"type":"action","step":2,"actor":0,"action_id":62,"source":"breeding"}"#, "
",
        );
        let side = concat!(
            r#"{"step":0,"turn":0,"phase":"Active","memory":0,"p0":{"security":0,"hand":[],"trash":[],"field":[]},"p1":{"security":0,"hand":[],"trash":[],"field":[]}}"#, "
",
            r#"{"step":1,"turn":0,"phase":"Active","memory":0,"p0":{"security":0,"hand":[],"trash":[],"field":[]},"p1":{"security":0,"hand":[],"trash":[],"field":[]}}"#, "
",
            r#"{"step":2,"turn":1,"phase":"Breeding","memory":0,"p0":{"security":5,"hand":[],"trash":[],"field":[]},"p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#, "
",
        );
        let out = align_to_scenario_origin(parse_sidecar(side).unwrap(), rec).unwrap();
        assert_eq!(out.len(), 1, "both mulligan rows must be dropped");
        assert_eq!(out[0].step, 0, "the survivor must be re-indexed to the origin");
        assert_eq!(out[0].p0.security, 5, "and it must be the post-mulligan row");
    }

    #[test]
    fn align_is_a_no_op_without_mulligan_rows() {
        let rec = concat!(
            r#"{"type":"action","step":0,"actor":0,"action_id":62,"source":"breeding"}"#, "
",
        );
        let side = concat!(
            r#"{"step":0,"turn":1,"phase":"Breeding","memory":0,"p0":{"security":5,"hand":[],"trash":[],"field":[]},"p1":{"security":5,"hand":[],"trash":[],"field":[]}}"#, "
",
        );
        let out = align_to_scenario_origin(parse_sidecar(side).unwrap(), rec).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].step, 0);
    }

    // ── pairing by lowered step ─────────────────────────────────────────

    #[test]
    fn a_one_row_per_step_line_pairs_positionally() {
        // The common case must stay exactly what it was.
        let p = pair_by_wire_rows(&[1, 1, 1], 3);
        assert_eq!(p.pairs, vec![(0, 0), (1, 1), (2, 2)]);
        assert_eq!(p.ours_unpairable, 0);
        assert_eq!(p.dcgo_unpairable, 0);
    }

    #[test]
    fn a_two_row_step_pairs_with_its_first_row_and_drops_the_intermediate() {
        // Measured witness EX12-011#effect#0: step 1 is an OptionalSkill+pick
        // fold, so DCGO writes two rows for it. Our row 1 belongs opposite the
        // FIRST of them; the second is a mid-resolution state our trace never
        // materializes, and comparing it can only manufacture a divergence.
        let p = pair_by_wire_rows(&[1, 2, 1], 4);
        assert_eq!(p.pairs, vec![(0, 0), (1, 1), (2, 3)]);
        assert_eq!(p.dcgo_unpairable, 1, "the fold's second row is accounted, not erased");
        assert_eq!(p.ours_unpairable, 0);
    }

    #[test]
    fn a_sim_only_step_drops_our_row_and_consumes_no_dcgo_row() {
        // Witness EX12-004#effect#2: the author balanced the row counts with a
        // sim-only step, which put our POST-attack row opposite DCGO's
        // PRE-attack one. Under the pairing rule the sim-only row simply has
        // no partner.
        let p = pair_by_wire_rows(&[1, 0, 1], 2);
        assert_eq!(p.pairs, vec![(0, 0), (2, 1)]);
        assert_eq!(p.ours_unpairable, 1);
        assert_eq!(p.dcgo_unpairable, 0);
    }

    #[test]
    fn a_short_dcgo_trace_leaves_our_rows_unpaired_rather_than_inventing_partners() {
        // Two steps predicted, one row delivered: the second step gets no
        // pair at all -- it must NOT slide onto a row belonging to another
        // step.
        let p = pair_by_wire_rows(&[1, 1, 1], 1);
        assert_eq!(p.pairs, vec![(0, 0)]);
        assert_eq!(p.ours_unpairable, 0, "an absent partner is a shortfall, not a deliberate drop");
    }

    #[test]
    fn intermediate_rows_are_only_counted_when_they_exist() {
        // A 2-row step whose second row DCGO never wrote must not be counted
        // as a deliberately-dropped row -- that would launder a truncated
        // trace into an accounted one.
        let p = pair_by_wire_rows(&[2], 1);
        assert_eq!(p.pairs, vec![(0, 0)]);
        assert_eq!(p.dcgo_unpairable, 0);
    }

    #[test]
    fn parse_sidecar_reads_every_row() {
        let text = format!("{}\n{}\n", ROW.replace("\"step\":3", "\"step\":0"), ROW);
        let rows = parse_sidecar(&text).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].step, 0);
        assert_eq!(rows[1].step, 3);
    }

    #[test]
    fn a_malformed_row_fails_loudly_rather_than_being_skipped() {
        // Silently dropping an unparseable row would shorten one side of the
        // diff and misalign every step after it.
        let text = format!("{ROW}\nnot json\n");
        assert!(parse_sidecar(&text).is_err());
    }

    #[test]
    fn unmeasured_keywords_are_a_wildcard_not_an_empty_set() {
        // DCGO's keyword dump is O(22 x field x effects) and defaults OFF, so
        // an absent list means "nobody looked". Treating it as "no keywords"
        // would report a keyword divergence on every permanent of every run
        // made without the flag -- pure noise, and worse, noise that looks
        // exactly like a real engine bug.
        let measured = StateProjection::from_sidecar_line(ROW).unwrap();
        let unmeasured =
            StateProjection::from_sidecar_line(&ROW.replace(",\"keywords\":[\"Blocker\"]", ""))
                .unwrap();
        assert_eq!(
            unmeasured.p0.field[0].keywords, None,
            "an omitted keyword list must parse as None, not as an empty Vec"
        );
        assert_eq!(
            measured, unmeasured,
            "an unmeasured keyword list must never diff against a measured one"
        );

        // ...but two sides that BOTH measured must still diff when they
        // disagree, including when one measured an genuinely empty set.
        let other =
            StateProjection::from_sidecar_line(&ROW.replace("\"Blocker\"", "\"Rush\"")).unwrap();
        assert_ne!(measured, other, "measured-vs-measured keywords must diff");

        let measured_empty =
            StateProjection::from_sidecar_line(&ROW.replace("[\"Blocker\"]", "[]")).unwrap();
        assert_ne!(
            measured, measured_empty,
            "a measured empty set is not the same claim as an unmeasured one"
        );
        assert_eq!(
            measured_empty, unmeasured,
            "unmeasured still wildcards against a measured empty set"
        );
    }

    /// `from_sidecar_line` is only half the contract -- if `from_game`
    /// disagrees with the real engine API the whole comparison is against a
    /// board nobody is playing. This runs the actual constructor over a real
    /// `Game` built from `cards.json`.
    #[test]
    fn from_game_projects_a_real_engine_state() {
        use crate::exam::test_support::{load_card_data, st1_decks};
        use digimon_engine::rules::Rules;
        use digimon_engine::Game;

        let card_data = load_card_data();
        let (main, egg) = st1_decks();
        let deck: Vec<String> = main.iter().chain(egg.iter()).cloned().collect();
        let game = Game::new_with_ordered_decks(
            &[deck.clone(), deck],
            &card_data,
            Rules::default(),
            Some(7),
            0,
        )
        .expect("ST-1 mirror should construct");

        let p = StateProjection::from_game(&game, 0);
        assert_eq!(p.step, 0);
        assert_eq!(p.turn, u64::from(game.turn_count));
        assert_eq!(p.phase, game.current_phase.py_name());
        assert_eq!(p.p0.security, game.player(0).security.len());
        assert_eq!(p.p0.hand.len(), game.player(0).hand.len());
        assert!(!p.p0.hand.is_empty(), "an opening hand should be dealt");
        // Sorted on construction, so the multiset compare in `PartialEq` is
        // honest for engine-built projections too, not just parsed ones.
        assert!(p.p0.hand.windows(2).all(|w| w[0] <= w[1]));
        // Keywords default to NOT MEASURED on our side as well, matching the
        // DCGO dumper's default-off flag.
        assert!(p.p0.field.iter().all(|f| f.keywords.is_none()));

        // Memory is pinned to player 0's perspective on both sides.
        let expected_memory = if game.memory_pair.0 == 0 {
            i64::from(game.memory)
        } else {
            -i64::from(game.memory)
        };
        assert_eq!(p.memory, expected_memory);

        // The keyword-measuring variant actually measures.
        let kw = StateProjection::from_game_with_keywords(&game, 0);
        assert!(kw.p0.field.iter().all(|f| f.keywords.is_some()));
    }
}
