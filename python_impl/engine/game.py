from __future__ import annotations
from typing import TYPE_CHECKING, Optional, Union
import random
from python_impl.engine.data.enums import GamePhase, EffectTiming, AttackResolution
from python_impl.engine.core.player import Player
from python_impl.engine.core.permanent import Permanent

if TYPE_CHECKING:
    pass

class Game:
    def __init__(self):
        self.player1: Player = Player()
        self.player2: Player = Player()
        self.player1.player_name = "Player 1"
        self.player2.player_name = "Player 2"
        self.player1.player_id = 1
        self.player2.player_id = 2

        self.turn_player: Player = self.player1
        self.opponent_player: Player = self.player2

        # Memory relative to turn player.
        # Positive means turn player has memory.
        # Negative means opponent has memory (and turn should end).
        self.memory: int = 0

        self.turn_count: int = 0
        self.current_phase: GamePhase = GamePhase.Start
        self.game_over: bool = False
        self.winner: Optional[Player] = None

    def start_game(self):
        # Determine first player (random for now)
        if random.choice([True, False]):
            self.turn_player = self.player1
            self.opponent_player = self.player2
        else:
            self.turn_player = self.player2
            self.opponent_player = self.player1

        print(f"Game Started. First player: {self.turn_player.player_name}")

        # These methods will be implemented in Player
        self.player1.setup_game()
        self.player2.setup_game()

        self.turn_count = 1
        self.current_phase = GamePhase.Start
        self.memory = 0

        # Start the loop
        self.phase_start()

    def next_phase(self):
        if self.game_over:
            return

        print(f"Phase Transition: {self.current_phase.name} -> ", end="")

        if self.current_phase == GamePhase.Start:
            self.current_phase = GamePhase.Draw
            print(f"{self.current_phase.name}")
            self.phase_draw()
        elif self.current_phase == GamePhase.Draw:
            self.current_phase = GamePhase.Breeding
            print(f"{self.current_phase.name}")
            self.phase_breeding()
        elif self.current_phase == GamePhase.Breeding:
            self.current_phase = GamePhase.Main
            print(f"{self.current_phase.name}")
            self.phase_main()
        elif self.current_phase == GamePhase.Main:
            self.current_phase = GamePhase.End
            print(f"{self.current_phase.name}")
            self.phase_end()
        elif self.current_phase == GamePhase.End:
            self.switch_turn()
            print(f"{self.current_phase.name} (New Turn)")
            self.phase_start()

    def phase_start(self):
        self.current_phase = GamePhase.Start
        self.turn_player.unsuspend_all()
        # Trigger Start of Turn effects
        self.execute_effects(EffectTiming.OnStartTurn)

        # Memory Check: If memory is <= 0 (meaning on opponent side or zero), reset to 3.
        # Wait, if I start my turn, memory MUST be on my side (positive) or 0?
        # Rule: "If the memory gauge is at zero or on the opponent's side, it moves to 3 on your side."
        if self.memory <= 0:
            self.memory = 3
            print(f"Memory reset to 3 for {self.turn_player.player_name}.")

        self.next_phase()

    def phase_draw(self):
        # Player going first DOES NOT draw on their first turn (Turn 1).
        if self.turn_count == 1:
            print("First turn: No draw.")
        else:
            if not self.turn_player.draw():
                self.declare_winner(self.opponent_player)
                return
            # Trigger OnDraw effects?
            self.execute_effects(EffectTiming.OnDraw)

        self.next_phase()

    def phase_breeding(self):
        # This phase allows Hatching or Moving.
        # In a real game, this is a decision point.
        # For this engine, we assume the Agent/Runner will call actions.
        # But if we just 'pass' here, we skip the phase logic.
        # We need to signal that we are in Breeding Phase.
        print("Breeding Phase: Waiting for action (hatch/move/pass).")
        # In headless/interactive, we return here. The Runner calls game.hatch() then game.next_phase() (to skip to Main)?
        # Or game.breeding_action_done().
        # For simplicity, we proceed to Main automatically IF no agent interaction hooks are present?
        # No, we must stop flow.
        pass

    def phase_main(self):
        # Main phase is where most actions happen.
        print("Main Phase: Waiting for actions.")
        self.execute_effects(EffectTiming.OnStartMainPhase)
        pass

    def phase_end(self):
        # Resolve end of turn effects.
        self.execute_effects(EffectTiming.OnEndTurn)
        self.next_phase()

    def switch_turn(self):
        self.turn_player, self.opponent_player = self.opponent_player, self.turn_player
        self.turn_count += 1
        # Memory is inverted
        self.memory = -self.memory

        self.turn_player.is_my_turn = True
        self.opponent_player.is_my_turn = False

    def pass_turn(self):
        # Player chooses to pass.
        # Memory set to 3 on opponent side.
        # From current player perspective, memory = -3.
        if self.memory >= 0:
            self.memory = -3
            print(f"{self.turn_player.player_name} passed turn. Memory set to -3.")

        self.current_phase = GamePhase.End
        self.next_phase()

    def check_turn_end(self):
        if self.memory < 0:
            print("Memory on opponent side. Turn ending...")
            self.current_phase = GamePhase.End
            self.next_phase()

    def execute_effects(self, timing: EffectTiming):
        # Naive implementation: iterate all permanents of turn player
        # In full engine, iterate ALL permanents (both players) and check conditions
        all_perms = self.turn_player.battle_area + self.opponent_player.battle_area

        # Also check effects from Hand/Trash/Security if timing matches (e.g. OnDeletion)

        for perm in all_perms:
            effects = perm.effect_list(timing)
            for effect in effects:
                # Context should refer to the owner of the effect/permanent
                owner = perm.top_card.owner if perm.top_card and perm.top_card.owner else None
                if owner is None:
                    # Fallback logic to find owner
                    if perm in self.turn_player.battle_area:
                        owner = self.turn_player
                    elif perm in self.opponent_player.battle_area:
                        owner = self.opponent_player
                    else:
                        owner = self.turn_player # Default safely

                context = {"game": self, "player": owner, "permanent": perm}
                if effect.can_use_condition is None or effect.can_use_condition(context):
                    print(f"Triggering effect: {effect.effect_name}")
                    if effect.on_process_callback:
                        effect.on_process_callback()

    def declare_winner(self, winner: Player):
        self.game_over = True
        self.winner = winner
        print(f"Game Over! Winner: {self.winner.player_name}")

    def resolve_attack(self, attacker: Permanent, target: Union[Permanent, Player]):
        if not attacker.can_attack(None):
            print("Attacker cannot attack (suspended or summoning sickness).")
            return

        print(f"Attack declared by {attacker.top_card.card_names[0]}")
        attacker.suspend()

        # Trigger When Attacking
        self.execute_effects(EffectTiming.OnUseAttack) # Or OnAllyAttack

        # Reaction Block? (Skip for PoC)

        # Battle
        if isinstance(target, Player):
            result = target.security_attack(attacker)
            if result == AttackResolution.AttackerDeleted:
                self.turn_player.delete_permanent(attacker)
            elif result == AttackResolution.GameEnd:
                self.declare_winner(self.turn_player)
        elif isinstance(target, Permanent):
            print(f"Battling Digimon: {attacker.dp} vs {target.dp}")
            if attacker.dp > target.dp:
                print(f"Defender {target.top_card.card_names[0]} deleted.")
                self.opponent_player.delete_permanent(target)
            elif attacker.dp < target.dp:
                print(f"Attacker {attacker.top_card.card_names[0]} deleted.")
                self.turn_player.delete_permanent(attacker)
            else:
                print("Draw! Both deleted.")
                self.opponent_player.delete_permanent(target)
                self.turn_player.delete_permanent(attacker)

        self.execute_effects(EffectTiming.OnEndAttack)
        self.check_turn_end()

    # Wrapper actions for Headless Runner / Agent
    def action_play_card(self, card_index: int):
        if self.current_phase != GamePhase.Main:
            print("Not Main Phase.")
            return

        if card_index < 0 or card_index >= len(self.turn_player.hand_cards):
            print("Invalid card index.")
            return

        card = self.turn_player.hand_cards[card_index]
        # Check cost
        cost = card.get_cost_itself
        # In Digimon, you can go negative to play, passing turn.
        # But if memory is ALREADY negative (shouldn't happen in Main phase start?), you can't?
        # Actually, rule is you can perform action if you can pay cost. Going negative ends turn.

        self.turn_player.play_card(card)
        self.memory -= cost
        print(f"Played {card.card_names[0]}. Memory: {self.memory}")

        self.execute_effects(EffectTiming.OnEnterFieldAnyone)
        self.check_turn_end()

    def action_digivolve(self, permanent_index: int, card_index: int):
        if self.current_phase != GamePhase.Main:
            return

        # validation...
        if permanent_index >= len(self.turn_player.battle_area): return
        if card_index >= len(self.turn_player.hand_cards): return

        perm = self.turn_player.battle_area[permanent_index]
        card = self.turn_player.hand_cards[card_index]

        cost = self.turn_player.digivolve(perm, card)
        self.memory -= cost
        print(f"Digivolution cost: {cost}. Memory: {self.memory}")

        self.execute_effects(EffectTiming.WhenDigivolving)
        self.check_turn_end()

    def action_attack_player(self, attacker_index: int):
        if self.current_phase != GamePhase.Main: return
        if attacker_index < 0 or attacker_index >= len(self.turn_player.battle_area):
            print("Invalid attacker index.")
            return
        attacker = self.turn_player.battle_area[attacker_index]
        self.resolve_attack(attacker, self.opponent_player)

    def action_pass_turn(self):
        self.pass_turn()
