//! Behavioral tests for individual card implementations. The
//! `/batch-implement-cards-rust` skill targets this binary — each card's
//! TDD test file lands here alongside its `CardEffect` impl in
//! `src/cards/<set>/<card_id>.rs`.
//!
//! See `sample_bt17_015.rs` for the template shape new tests should
//! follow (doc-comment at top that summarizes the card text, two
//! companion tests covering the positive and negative branch of any
//! conditional effect).

mod app_fuse_primitive;
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
mod bt10;
mod bt11;
mod bt12;
mod bt13;
mod bt14;
mod bt15;
mod bt16;
mod bt17;
mod bt18;
mod bt19;
mod bt20;
mod bt21;
mod bt22;
mod bt23;
mod bt2;
mod bt24;
mod bt25;
mod bt3;
mod bt4;
mod bt5;
mod bt6;
mod bt7;
mod bt8;
mod bt9;
mod ex1;
mod ex10;
mod ex11;
mod ex3;
mod ex4;
mod ex5;
mod ex6;
mod ex7;
mod ex8;
mod ex9;
mod lm;
mod p;
mod rb1;
mod st1;
mod st12;
mod st13;
mod st17;
mod st18;
mod st19;
mod st2;
mod st20;
mod st21;
mod st22;
mod st23;
mod st24;
mod st3;
mod st4;
mod st5;
mod st6;
mod st9;
