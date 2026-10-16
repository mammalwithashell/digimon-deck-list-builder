//! Optional harness observations. These identities never participate in rules,
//! action masks, tensors, replay verification, or gameplay identity allocation.
use super::Game;
use crate::{card_source::CardHandle, enums::PlayerId, permanent::PermanentHandle};
use std::sync::Arc;

/// One completed native play, captured before its entry effects run.
/// Opaque process-local identity: cloneable, deliberately not serializable.
#[derive(Clone, Debug)]
pub struct NativePlayEntry {
    player: PlayerId,
    card: CardHandle,
    incarnation: Arc<()>,
    session: Arc<()>,
}

#[cfg(test)]
mod tests {
    use crate::{
        debug_runner::{make_test_card, DebugRunner},
        enums::GamePhase,
        permanent::PermanentHandle,
    };

    #[test]
    fn observation_never_allocates_gameplay_identity_and_dna_revokes_body() {
        let mut card = make_test_card("OBS-UNIT", "Observed Unit");
        card.play_cost = 0;
        let mut r = DebugRunner::builder()
            .add_card(card)
            .hand(0, &["OBS-UNIT"; 3])
            .deck(0, &["OBS-UNIT"; 5])
            .memory(5)
            .start();
        r.game.current_phase = GamePhase::Main;
        let mut control = r.game.clone();
        r.game.enable_native_play_observation(true);
        for _ in 0..2 {
            r.game.play_from_hand(0, 0).unwrap();
            control.play_from_hand(0, 0).unwrap();
        }
        assert_eq!(r.game.next_battle_identity, control.next_battle_identity);
        let a = PermanentHandle {
            player: 0,
            index: 0,
        };
        assert_eq!(
            r.game.capture_digivolve_host_identity(a),
            control.capture_digivolve_host_identity(a)
        );
        assert_eq!(r.game.verification_digest(), control.verification_digest());
        let original = r.game.take_native_play_entries().remove(0);
        let merged = r
            .game
            .dna_digivolve_inner(
                a,
                PermanentHandle {
                    player: 0,
                    index: 1,
                },
                0,
                0,
                0,
                true,
            )
            .unwrap();
        // Expose that same original physical card again after the DNA merge.
        // DNA created a new body, unlike ordinary evolution/de-digivolution.
        r.game.players[0].battle_area[merged.index as usize]
            .card_sources
            .retain(|c| c.handle() == original.card());
        assert!(r.game.resolve_native_play_entry(&original).is_none());
    }
}

impl NativePlayEntry {
    pub fn player(&self) -> PlayerId {
        self.player
    }
    pub fn card(&self) -> CardHandle {
        self.card
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NativePlayObservation {
    session: Arc<()>,
    entries: Vec<NativePlayEntry>,
}

impl Game {
    /// Begin a fresh observation session, or disable it. Either operation
    /// revokes old entries. Disabled by default and after reset_for_replay.
    pub fn enable_native_play_observation(&mut self, enabled: bool) {
        self.native_play_observation = enabled.then(|| NativePlayObservation {
            session: Arc::new(()),
            entries: Vec::new(),
        });
    }

    /// Drain successful native commits since the preceding drain. Consumers
    /// must bind them to the exact in-flight native action, never a later play
    /// of the same card. No failed, cancelled, or effect-initiated play is logged.
    pub fn take_native_play_entries(&mut self) -> Vec<NativePlayEntry> {
        self.native_play_observation
            .as_mut()
            .map(|state| std::mem::take(&mut state.entries))
            .unwrap_or_default()
    }

    /// Locate the same surviving body with its original physical top card.
    /// Indices may move. A new play of that card is a different incarnation.
    /// Evolving and then restoring the original top on this body is allowed.
    pub fn resolve_native_play_entry(&self, entry: &NativePlayEntry) -> Option<PermanentHandle> {
        let state = self.native_play_observation.as_ref()?;
        if !Arc::ptr_eq(&state.session, &entry.session) {
            return None;
        }
        let mut matches = self
            .players
            .iter()
            .enumerate()
            .flat_map(|(player, state)| {
                state
                    .battle_area
                    .iter()
                    .enumerate()
                    .map(move |(index, permanent)| (player, index, permanent))
            })
            .filter(|(_, _, permanent)| {
                permanent
                    .card_sources
                    .last()
                    .is_some_and(|card| card.handle() == entry.card)
                    && permanent
                        .native_play_incarnation
                        .as_ref()
                        .is_some_and(|token| Arc::ptr_eq(token, &entry.incarnation))
            });
        let (player, index, _) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(PermanentHandle {
            player: PlayerId::try_from(player).ok()?,
            index: u8::try_from(index).ok()?,
        })
    }

    pub(crate) fn observe_native_play_commit(
        &mut self,
        entered: PermanentHandle,
        card: CardHandle,
    ) {
        let Some(state) = &self.native_play_observation else {
            return;
        };
        let session = state.session.clone();
        let incarnation = Arc::new(());
        let permanent = &mut self.player_mut(entered.player).battle_area[entered.index as usize];
        debug_assert_eq!(permanent.top_card().handle(), card);
        permanent.native_play_incarnation = Some(incarnation.clone());
        self.native_play_observation
            .as_mut()
            .expect("enabled observer")
            .entries
            .push(NativePlayEntry {
                player: entered.player,
                card,
                incarnation,
                session,
            });
    }
}
