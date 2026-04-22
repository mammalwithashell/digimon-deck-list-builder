//! Compiled card IR — rkyv-friendly mirror of `CardSpec` used as the
//! on-disk / in-memory format for distributed card packs.
//!
//! Phase 1b Task 2: top-level types + stubs for nested. Task 3 populates
//! the stubs. Task 4 adds the lowering pass (CardSpec → CompiledCard).

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledCard {
    pub card: String,
    pub name: String,
    pub kind: CompiledCardKind,
    pub level: Option<u8>,
    pub color: Vec<CompiledColor>,
    pub cost: Option<i32>,
    pub dp: Option<i32>,
    pub traits: Vec<String>,
    pub form: Option<String>,
    pub attribute: Option<String>,
    pub ace_overflow: Option<i32>,
    pub identity: Option<CompiledIdentity>,
    pub alt_paths: Vec<CompiledAltPath>,
    pub effects: Vec<CompiledClause>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledCardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[archive(check_bytes)]
pub enum CompiledColor {
    Red,
    Blue,
    Yellow,
    Green,
    Black,
    Purple,
    White,
}

// Stubs for nested types — Task 3 populates with real content.

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledIdentity;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledAltPath;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[archive(check_bytes)]
pub struct CompiledClause;
