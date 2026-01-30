import unittest
import numpy as np
from fastapi.testclient import TestClient
from python_impl.digimon_gym import GameState
from python_impl.engine.data.card_database import CardDatabase
from python_impl.api import app

class TestDigimonGym(unittest.TestCase):
    def setUp(self):
        self.game = GameState()
        self.client = TestClient(app)

    def test_card_database_loading(self):
        db = CardDatabase()
        cards = db.get_all_cards()
        self.assertGreater(len(cards), 0)
        self.assertIn("ST1-03", cards)
        self.assertEqual(cards["ST1-03"].card_name_eng, "Agumon")

    def test_game_state_initialization(self):
        obs = self.game.reset()
        self.assertIsInstance(obs, dict)
        self.assertIn("p0_hand", obs)
        self.assertIsInstance(obs["p0_hand"], np.ndarray)
        self.assertEqual(self.game.turn, 0)

    def test_game_step(self):
        self.game.reset()
        obs, reward, done = self.game.step(0)
        self.assertEqual(self.game.turn, 1)
        self.assertIsInstance(reward, float)
        self.assertIsInstance(done, bool)

    def test_api_simulate(self):
        response = self.client.post("/simulate", json={
            "deck1": "ST1-03\nST1-01",
            "deck2": "ST1-03\nST1-01",
            "num_simulations": 10
        })
        self.assertEqual(response.status_code, 200)
        data = response.json()
        self.assertIn("p1_win_rate", data)
        self.assertIn("logs", data)
        self.assertEqual(len(data["logs"]), 5) # We capped logs at 5

if __name__ == '__main__':
    unittest.main()
