//! Behavioral tests for individual card implementations. The
//! `/batch-implement-cards-rust` skill targets this binary — each card's
//! TDD test file lands here alongside its `CardEffect` impl in
//! `src/cards/<set>/<card_id>.rs`.
//!
//! See `sample_bt17_015.rs` for the template shape new tests should
//! follow (doc-comment at top that summarizes the card text, two
//! companion tests covering the positive and negative branch of any
//! conditional effect).

mod test_cards;
mod tokens;

// Sample template for forthcoming hand-written production card tests.
// Currently ignored pending the engine gaps that BT17-015 depends on.
mod sample_bt17_015;
