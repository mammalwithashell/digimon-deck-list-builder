from __future__ import annotations
from typing import TYPE_CHECKING, Optional, Union, List
import random

# Support both import prefixes
try:
    from python_impl.engine.data.enums import GamePhase, EffectTiming, AttackResolution, PendingAction
    from python_impl.engine.core.player import Player
    from python_impl.engine.core.permanent import Permanent
except ImportError:
    from digimon_gym.engine.data.enums import GamePhase, EffectTiming, AttackResolution, PendingAction
    from digimon_gym.engine.core.player import Player
    from digimon_gym.engine.core.permanent import Permanent

if TYPE_CHECKING:
    from .core.card_source import CardSource


class Game:
    def __init__(self):
        self.player1: Player = Player()
        self.player2: Player = Player()
        self.player1.player_name = "Player 1"
        self.player2.player_name = "Player 2"
        self.player1.player_id = 1
        self.player2.player_id = 2

        # Wire up cross-references
        self.player1.enemy = self.player2
        self.player2.enemy = self.player1
        self.player1.game = self
        self.player2.game = self

        self.turn_player: Player = self.player1
        self.opponent_player: Player = self.player2

        self.memory: int = 0
        self.turn_count: int = 0
        self.current_phase: GamePhase = GamePhase.Start
        self.pending_action: PendingAction = PendingAction.NO_ACTION
        self.game_over: bool = False
        self.winner: Optional[Player] = None

    def start_game(self):
        if random.choice([True, False]):
            self.turn_player = self.player1
            self.opponent_player = self.player2
        else:
            self.turn_player = self.player2
            self.opponent_player = self.player1

        self.player1.setup_game()
        self.player2.setup_game()

        self.turn_count = 1
        self.current_phase = GamePhase.Start
        self.memory = 0
        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False

        self.phase_start()

    def next_phase(self):
        if self.game_over:
            return

        if self.current_phase == GamePhase.Start:
            self.current_phase = GamePhase.Draw
            self.phase_draw()
        elif self.current_phase == GamePhase.Draw:
            self.current_phase = GamePhase.Breeding
            self.phase_breeding()
        elif self.current_phase == GamePhase.Breeding:
            self.current_phase = GamePhase.Main
            self.phase_main()
        elif self.current_phase == GamePhase.Main:
            self.current_phase = GamePhase.End
            self.phase_end()
        elif self.current_phase == GamePhase.End:
            self.switch_turn()
            self.phase_start()

    def phase_start(self):
        self.current_phase = GamePhase.Start
        self.turn_player.unsuspend_all()
        self._reset_effect_turn_counts()
        self._clear_temp_dp()
        self.execute_effects(EffectTiming.OnStartTurn)

        if self.memory <= 0:
            self.memory = 3

        self.next_phase()

    def phase_draw(self):
        if self.turn_count == 1:
            pass  # First turn: no draw
        else:
            if not self.turn_player.draw():
                self.declare_winner(self.opponent_player)
                return
            self.execute_effects(EffectTiming.OnDraw)

        self.next_phase()

    def phase_breeding(self):
        pass  # Waiting for agent action

    def phase_main(self):
        self.execute_effects(EffectTiming.OnStartMainPhase)
        pass  # Waiting for agent actions

    def phase_end(self):
        self.execute_effects(EffectTiming.OnEndTurn)
        self.next_phase()

    def switch_turn(self):
        self.turn_player, self.opponent_player = self.opponent_player, self.turn_player
        self.turn_count += 1
        self.memory = -self.memory
        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False

    def pass_turn(self):
        if self.memory >= 0:
            self.memory = -3
        self.current_phase = GamePhase.End
        self.next_phase()

    def check_turn_end(self):
        if self.memory < 0:
            self.current_phase = GamePhase.End
            self.next_phase()

    # ─── Effect Execution ────────────────────────────────────────────

    def execute_effects(self, timing: EffectTiming, extra_context: Optional[dict] = None):
        """Execute all effects matching the given timing across all permanents."""
        all_perms: List[Permanent] = list(self.turn_player.battle_area) + list(self.opponent_player.battle_area)

        for perm in all_perms:
            effects = perm.effect_list(timing)
            for effect in effects:
                if not effect.can_activate_this_turn():
                    continue

                owner = self._find_owner(perm)
                context = {
                    "game": self,
                    "player": owner,
                    "permanent": perm,
                    "card": effect.effect_source_card,
                    "turn_player": self.turn_player,
                    "opponent_player": self.opponent_player,
                }
                if extra_context:
                    context.update(extra_context)

                if effect.can_use_condition is None or effect.can_use_condition(context):
                    effect.record_activation()
                    if effect.on_process_callback:
                        effect.on_process_callback(context)

    def execute_deletion_effects(self, deleted_permanent: Permanent, owner: Player):
        """Execute OnDeletion effects for a permanent that was just deleted."""
        # Effects from the deleted permanent's card stack
        for source in deleted_permanent.card_sources:
            effects = source.effect_list(EffectTiming.OnDestroyedAnyone)
            for effect in effects:
                if not effect.can_activate_this_turn():
                    continue
                context = {
                    "game": self,
                    "player": owner,
                    "permanent": deleted_permanent,
                    "card": effect.effect_source_card,
                    "turn_player": self.turn_player,
                    "opponent_player": self.opponent_player,
                    "deleted_permanent": deleted_permanent,
                }
                if effect.can_use_condition is None or effect.can_use_condition(context):
                    effect.record_activation()
                    if effect.on_process_callback:
                        effect.on_process_callback(context)

    def _find_owner(self, perm: Permanent) -> Player:
        """Determine which player owns a permanent."""
        if perm.top_card and perm.top_card.owner:
            return perm.top_card.owner
        if perm in self.turn_player.battle_area:
            return self.turn_player
        if perm in self.opponent_player.battle_area:
            return self.opponent_player
        return self.turn_player

    def _reset_effect_turn_counts(self):
        """Reset once-per-turn counters for all effects at start of turn."""
        for player in [self.player1, self.player2]:
            for perm in player.battle_area:
                for source in perm.card_sources:
                    for effect in source.effect_list(EffectTiming.NoTiming):
                        effect.reset_turn_count()
            if player.breeding_area:
                for source in player.breeding_area.card_sources:
                    for effect in source.effect_list(EffectTiming.NoTiming):
                        effect.reset_turn_count()

    def _clear_temp_dp(self):
        """Clear temporary DP modifiers at start of turn."""
        for player in [self.player1, self.player2]:
            for perm in player.battle_area:
                perm.clear_temp_dp()

    # ─── Game Actions ────────────────────────────────────────────────

    def declare_winner(self, winner: Player):
        self.game_over = True
        self.winner = winner

    def resolve_attack(self, attacker: Permanent, target: Union[Permanent, Player]):
        if not attacker.can_attack():
            return

        attacker.suspend()

        # Trigger When Attacking (OnAllyAttack for the attacker's effects)
        self.execute_effects(EffectTiming.OnAllyAttack, {"attacker": attacker})

        if isinstance(target, Player):
            result = target.security_attack(attacker)
            if result == AttackResolution.AttackerDeleted:
                self.turn_player.delete_permanent(attacker)
            elif result == AttackResolution.GameEnd:
                self.declare_winner(self.turn_player)
        elif isinstance(target, Permanent):
            if attacker.dp > target.dp:
                self.opponent_player.delete_permanent(target)
            elif attacker.dp < target.dp:
                self.turn_player.delete_permanent(attacker)
            else:
                self.opponent_player.delete_permanent(target)
                self.turn_player.delete_permanent(attacker)

        self.execute_effects(EffectTiming.OnEndAttack)
        self.check_turn_end()

    def action_play_card(self, card_index: int):
        if self.current_phase != GamePhase.Main:
            return
        if card_index < 0 or card_index >= len(self.turn_player.hand_cards):
            return

        card = self.turn_player.hand_cards[card_index]
        cost = card.get_cost_itself

        self.turn_player.play_card(card)
        self.memory -= cost

        self.execute_effects(EffectTiming.OnEnterFieldAnyone, {"played_card": card})
        self.check_turn_end()

    def action_digivolve(self, permanent_index: int, card_index: int):
        if self.current_phase != GamePhase.Main:
            return
        if permanent_index >= len(self.turn_player.battle_area):
            return
        if card_index >= len(self.turn_player.hand_cards):
            return

        perm = self.turn_player.battle_area[permanent_index]
        card = self.turn_player.hand_cards[card_index]

        cost = self.turn_player.digivolve(perm, card)
        self.memory -= cost

        self.execute_effects(EffectTiming.WhenDigivolving, {"digivolved_permanent": perm})
        self.execute_effects(EffectTiming.OnEnterFieldAnyone, {"played_card": card})
        self.check_turn_end()

    def action_attack_player(self, attacker_index: int):
        if self.current_phase != GamePhase.Main:
            return
        if attacker_index < 0 or attacker_index >= len(self.turn_player.battle_area):
            return
        attacker = self.turn_player.battle_area[attacker_index]
        self.resolve_attack(attacker, self.opponent_player)

    def action_pass_turn(self):
        self.pass_turn()
