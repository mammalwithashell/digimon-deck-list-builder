"""AI pipeline test conftest — marks all tests in this directory.

These tests are excluded from the default pytest run.
Run explicitly: python -m pytest tests/ai_pipeline -v
"""

import pytest

pytestmark = pytest.mark.ai_pipeline
