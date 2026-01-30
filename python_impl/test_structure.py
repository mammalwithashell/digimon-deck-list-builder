import sys
import os

# Add repo root to path so we can import modules
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from python_impl.engine.data.enums import CardColor, CardKind
from python_impl.engine.core.entity_base import CEntity_Base
from python_impl.engine.core.card_source import CardSource
from python_impl.engine.core.permanent import Permanent
from python_impl.engine.core.player import Player
from python_impl.engine.interfaces.card_effect import ICardEffect
from python_impl.engine.data.card_database import CardDatabase

def test_structure():
    print("Testing structure...")

    # Create Player
    player = Player()
    player.player_name = "TestPlayer"
    print(f"Player created: {player.player_name}")

    # Create Entity
    entity = CEntity_Base()
    entity.card_name_eng = "Agumon"
    entity.card_kind = CardKind.Digimon
    entity.level = 3
    entity.play_cost = 3
    entity.dp = 2000
    entity.card_colors.append(CardColor.Red)
    print(f"Entity created: {entity.card_name_eng}, Level: {entity.level}, DP: {entity.dp}")

    # Create CardSource
    card = CardSource()
    card.set_base_data(entity, player)
    print(f"CardSource created linked to {card.card_names[0]}")
    print(f"Card is Digimon: {card.is_digimon}")
    print(f"Card has play cost: {card.has_play_cost}")

    # Create Permanent
    permanent = Permanent([card])
    print(f"Permanent created with top card: {permanent.top_card.card_names[0]}")
    print(f"Permanent Level: {permanent.level}")
    print(f"Permanent DP: {permanent.dp}")

    # Test Database
    db = CardDatabase()

    print("Structure test passed!")

if __name__ == "__main__":
    test_structure()
