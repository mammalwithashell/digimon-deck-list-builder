//! Scenario capture + application (the inverse of `staging.rs`).
//!
//! `Game::to_scenario()` serializes the current full-information game into
//! the `qa/scenarios/` fixture schema; `Game::apply_scenario()` re-stages a
//! fixture onto an already-constructed game. The two are exact inverses for
//! the asserted zones (hand, deck order, security, trash, field stacks,
//! breeding) and scalar state (memory, phase, turn, first player).
//!
//! Known limits (documented, not faithfully round-tripped): egg-deck order
//! (the fixture schema has no egg-deck zone — `decks` carries the egg cards
//! so construction repopulates the egg deck, but reshuffled), and tokens
//! (not deck-sourced; excluded from the reconstructed `decks` pool).

#![allow(unused_imports)]
use super::*;
use crate::card_source::CardSource;
use crate::enums::GamePhase;
use crate::player::Player;
use serde_json::{json, Map, Value};

impl Game {
    /// Capture the current full-information state as a scenario fixture
    /// (`qa/scenarios/` schema) with an empty assertion list. The inverse of
    /// the staging importer: re-applying the result reproduces this board's
    /// asserted zones and scalar state.
    pub fn to_scenario(&self) -> Value {
        let first_player = self.turn_order.first().copied().unwrap_or(0);
        let mut decks = Map::new();
        let mut zones = Map::new();
        for (idx, player) in self.players.iter().enumerate() {
            let key = (idx + 1).to_string();
            decks.insert(key.clone(), json!(self.reconstruct_deck_pool(player)));
            zones.insert(key, self.capture_player_zones(player));
        }
        json!({
            "schema_version": 1,
            "id": "captured",
            "title": "Captured game state",
            "readiness": "expected_pass",
            "blocked_reason": "",
            "notes": "Captured via Game::to_scenario(). Egg-deck order and tokens are not round-tripped.",
            "decks": Value::Object(decks),
            "seed": 0,
            "state": {
                "memory": self.memory,
                "phase": format!("{:?}", self.current_phase),
                "turn": self.turn_count,
                "first_player": (first_player as u64) + 1,
            },
            "zones": Value::Object(zones),
            "assertions": { "engine": [], "ui": [] },
        })
    }

    /// Re-stage a scenario fixture onto this (already-constructed,
    /// post-mulligan) game. Mirrors the Python `_apply_zone` /
    /// `_apply_scalar_state` loader: any present zone REPLACES the dealt
    /// contents (clear-then-populate). Validates the staged board and
    /// returns `Err(reason)` on a rule-illegal result.
    pub fn apply_scenario(&mut self, fixture: &Value) -> Result<(), String> {
        if let Some(zones) = fixture.get("zones").and_then(Value::as_object) {
            for (pid_str, zspec) in zones {
                let pid_1based: u8 = pid_str
                    .parse()
                    .map_err(|_| format!("apply_scenario: bad player key {pid_str:?}"))?;
                let pid = pid_1based
                    .checked_sub(1)
                    .filter(|p| (*p as usize) < self.players.len())
                    .ok_or_else(|| format!("apply_scenario: player key {pid_str:?} out of range"))?;
                self.apply_zone_spec(pid, zspec)?;
            }
        }

        if let Some(state) = fixture.get("state") {
            if let Some(fp) = state.get("first_player").and_then(Value::as_u64) {
                let rust_fp = (fp.max(1) - 1) as u8;
                self.stage_set_first_player(rust_fp);
            }
            if let Some(turn) = state.get("turn").and_then(Value::as_u64) {
                self.turn_count = turn as u16;
            }
            if let Some(phase) = state.get("phase").and_then(Value::as_str) {
                self.current_phase = parse_scenario_phase(phase)?;
            }
            if let Some(mem) = state.get("memory").and_then(Value::as_i64) {
                self.set_memory(mem as i16);
            }
        }

        self.stage_validate()
    }

    fn apply_zone_spec(&mut self, pid: PlayerId, z: &Value) -> Result<(), String> {
        if let Some(hand) = z.get("hand").and_then(Value::as_array) {
            self.stage_clear_zone(pid, "hand")?;
            for c in hand {
                let id = c.as_str().ok_or("apply_scenario: non-string hand card")?;
                self.stage_inject_card(pid, id, "hand")?;
            }
        }
        // `deck_top` is ordered top->bottom (index 0 = next draw = vec tail),
        // so inject bottom-first: iterate reversed.
        if let Some(deck) = z.get("deck_top").and_then(Value::as_array) {
            self.stage_clear_zone(pid, "deck")?;
            for c in deck.iter().rev() {
                let id = c.as_str().ok_or("apply_scenario: non-string deck card")?;
                self.stage_inject_card(pid, id, "deck_top")?;
            }
        }
        // `security` is ordered top->bottom (top = vec tail): inject bottom-first.
        if let Some(sec) = z.get("security").and_then(Value::as_array) {
            self.stage_clear_zone(pid, "security")?;
            for c in sec.iter().rev() {
                let id = c.as_str().ok_or("apply_scenario: non-string security card")?;
                self.stage_inject_card(pid, id, "security_top")?;
            }
        }
        if let Some(trash) = z.get("trash").and_then(Value::as_array) {
            self.stage_clear_zone(pid, "trash")?;
            for c in trash {
                let id = c.as_str().ok_or("apply_scenario: non-string trash card")?;
                self.stage_inject_card(pid, id, "trash")?;
            }
        }
        if let Some(field) = z.get("field").and_then(Value::as_array) {
            self.stage_clear_zone(pid, "battle_area")?;
            for fs in field {
                let stack_vals = fs
                    .get("stack")
                    .and_then(Value::as_array)
                    .ok_or("apply_scenario: field stack missing 'stack'")?;
                let stack: Vec<&str> = stack_vals.iter().filter_map(Value::as_str).collect();
                if stack.is_empty() {
                    return Err("apply_scenario: empty field stack".to_string());
                }
                let suspended = fs.get("is_suspended").and_then(Value::as_bool).unwrap_or(false);
                let turn_played =
                    fs.get("turn_played").and_then(Value::as_u64).unwrap_or(0) as u16;
                self.stage_place_field_stack(pid, &stack, suspended, turn_played);
            }
        }
        // `breeding` present (even null) REPLACES the breeding area.
        if let Some(br) = z.get("breeding") {
            self.stage_clear_zone(pid, "breeding")?;
            if let Some(arr) = br.as_array() {
                if !arr.is_empty() {
                    let stack: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
                    self.stage_place_in_breeding(pid, &stack);
                }
            }
        }
        Ok(())
    }

    /// Reconstruct a player's full deck pool (main + egg) from every zone,
    /// so a captured fixture can be re-constructed. Tokens are excluded
    /// (they are not deck-sourced).
    fn reconstruct_deck_pool(&self, p: &Player) -> Vec<String> {
        let mut ids = Vec::new();
        let mut push = |cs: &CardSource, ids: &mut Vec<String>| {
            if !cs.is_token {
                ids.push(self.card_data[cs.data_index].card_id.clone());
            }
        };
        for cs in &p.deck {
            push(cs, &mut ids);
        }
        for cs in &p.digitama_deck {
            push(cs, &mut ids);
        }
        for cs in &p.security {
            push(cs, &mut ids);
        }
        for cs in &p.hand {
            push(cs, &mut ids);
        }
        for cs in &p.trash {
            push(cs, &mut ids);
        }
        for perm in &p.battle_area {
            for cs in &perm.card_sources {
                push(cs, &mut ids);
            }
        }
        if let Some(perm) = &p.breeding_area {
            for cs in &perm.card_sources {
                push(cs, &mut ids);
            }
        }
        ids
    }

    fn cs_id(&self, cs: &CardSource) -> String {
        self.card_data[cs.data_index].card_id.clone()
    }

    fn capture_player_zones(&self, p: &Player) -> Value {
        let hand: Vec<String> = p.hand.iter().map(|cs| self.cs_id(cs)).collect();
        // top->bottom: vec tail is the top, so reverse the vec.
        let deck_top: Vec<String> = p.deck.iter().rev().map(|cs| self.cs_id(cs)).collect();
        let security: Vec<String> = p.security.iter().rev().map(|cs| self.cs_id(cs)).collect();
        let trash: Vec<String> = p.trash.iter().map(|cs| self.cs_id(cs)).collect();
        let field: Vec<Value> = p
            .battle_area
            .iter()
            .map(|perm| {
                json!({
                    "stack": perm.card_sources.iter().map(|cs| self.cs_id(cs)).collect::<Vec<_>>(),
                    "is_suspended": perm.is_suspended,
                    "turn_played": perm.turn_played,
                })
            })
            .collect();
        let breeding = match &p.breeding_area {
            Some(perm) => json!(perm.card_sources.iter().map(|cs| self.cs_id(cs)).collect::<Vec<_>>()),
            None => Value::Null,
        };
        json!({
            "hand": hand,
            "deck_top": deck_top,
            "security": security,
            "trash": trash,
            "field": field,
            "breeding": breeding,
        })
    }
}

/// Parse a fixture phase string into a stageable turn phase (the inverse of
/// `format!("{:?}", phase)` for the stageable subset).
fn parse_scenario_phase(s: &str) -> Result<GamePhase, String> {
    Ok(match s {
        "Mulligan" => GamePhase::Mulligan,
        "Unsuspend" => GamePhase::Unsuspend,
        "Draw" => GamePhase::Draw,
        "Breeding" => GamePhase::Breeding,
        "Main" => GamePhase::Main,
        "EndTurn" => GamePhase::EndTurn,
        other => {
            return Err(format!(
                "apply_scenario: unstageable phase '{other}'; stage a turn phase \
                 (Mulligan/Unsuspend/Draw/Breeding/Main/EndTurn)"
            ))
        }
    })
}
