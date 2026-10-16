"""Same-shape semantic-version migration against the repository contract module."""
import importlib.util
import os
import sys
import types
import unittest
from unittest.mock import patch

import model_contract as contract

class ActionSemanticVersionTests(unittest.TestCase):
    def setUp(self):
        self.layout = dict(profile_id='test_same_shape', tensor_version=2,
                           feature_schema_version='test-v1', tensor_size=100,
                           layout_hash='test-layout')
        engine = types.SimpleNamespace(ACTION_SPACE_SIZE=2192, ACTION_SCHEMA_VERSION=2,
                                       get_observation_layout=lambda _: self.layout)
        self.mock = patch.dict(sys.modules, {'digimon_engine': engine})
        self.mock.start(); self.addCleanup(self.mock.stop)
        self.metadata = dict(observation_profile='test_same_shape', tensor_version=2,
                             feature_schema_version='test-v1', tensor_size=100,
                             tensor_layout_hash='test-layout', action_space_size=2192,
                             action_schema_version=2)

    def test_old_extension_failure_is_a_compatibility_error(self):
        engine = sys.modules['digimon_engine']
        del engine.ACTION_SCHEMA_VERSION
        with self.assertRaisesRegex(ValueError, 'ACTION_SCHEMA_VERSION'):
            contract.current_contract('test_same_shape')

    def test_exact_declared_action_semantics_survive_validation(self):
        self.assertEqual(contract.validate_contract(self.metadata), self.metadata)

    def test_same_shapes_and_layout_do_not_admit_previous_action_meanings(self):
        old = dict(self.metadata, action_schema_version=1)
        model = types.SimpleNamespace(digimon_contract=old,
                                      observation_space=types.SimpleNamespace(shape=(100,)),
                                      action_space=types.SimpleNamespace(n=2192))
        with self.assertRaises(ValueError):
            contract.validate_checkpoint(model)
        self.assertEqual(model.digimon_contract, old)

    def test_missing_semantics_cannot_be_inferred_from_shape(self):
        missing = dict(self.metadata); del missing['action_schema_version']
        with self.assertRaises(ValueError):
            contract.validate_contract(missing)

    def test_malformed_action_version_is_rejected(self):
        for value in [None, True, '2', 0, -1, 2.0]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                contract.validate_contract(dict(self.metadata, action_schema_version=value))

if __name__ == '__main__':
    unittest.main(verbosity=2)
