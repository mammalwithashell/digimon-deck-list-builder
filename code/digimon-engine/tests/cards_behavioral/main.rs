//! Behavioral tests for individual card implementations. The
//! `/batch-implement-cards-rust` skill targets this binary — each card's
//! TDD test file lands here alongside its `CardEffect` impl in
//! `src/cards/<set>/<card_id>.rs`.
//!
//! See `sample_bt17_015.rs` for the template shape new tests should
//! follow (doc-comment at top that summarizes the card text, two
//! companion tests covering the positive and negative branch of any
//! conditional effect).

mod de_digivolve;
#[path = "../support/dsl_card_data.rs"]
mod dsl_card_data;
mod dsl_omnimon_slice;
mod test_cards;
mod tokens;

// Sample template for forthcoming hand-written production card tests.
// Currently ignored pending the engine gaps that BT17-015 depends on.
mod sample_bt17_015;

// Per-set per-card test modules (added by /batch-implement-cards-rust-dsl)
mod ad1;
mod bt1;
mod bt13;
mod bt14;
mod bt15;
mod bt16;
mod bt17;
mod bt18;
mod bt20;
mod bt21;
mod bt23;
mod bt24;
mod bt5;
mod bt8;
mod bt9;
mod ex10;
mod ex11;
mod ex4;
mod ex7;
mod ex8;
mod ex9;
mod lm;
mod p;
mod st1;
mod st22;
mod bt12;
mod bt22;
mod ex1;
mod ex5;
mod st2;
mod st20;
mod st21;
