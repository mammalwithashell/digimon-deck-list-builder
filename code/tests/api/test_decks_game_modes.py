"""Smoke coverage for `decks._validate_for_mode` after deck legality moved
onto the Rust binding (shrink-legacy-engine-surface, task 2.7).

These exercise the per-mode branches — especially `no_restriction` (now routed
through `validate_deck_for_game_mode("no_restriction")` instead of a custom
Python `CardRestriction`) and the titan/edh thin Python wrappers over the Rust
binding + `CardDatabase` — to prove they run and classify without raising.
"""

from server.db.routers.decks import _validate_for_mode


def test_no_restriction_runs_and_returns_verdict():
    is_valid, errors = _validate_for_mode(["BT1-001"] * 50, "no_restriction", None)
    assert isinstance(is_valid, bool)
    assert isinstance(errors, list)


def test_standard_mode_rejects_too_many_copies():
    is_valid, errors = _validate_for_mode(["BT1-001"] * 50, "standard", None)
    assert is_valid is False
    assert errors  # restricted/per-card-copy violations surface


def test_edh_commander_flags_singleton_violation():
    is_valid, errors = _validate_for_mode(["BT1-001", "BT1-001"], "edh_commander", None)
    assert is_valid is False
    # The singleton rule fires for the duplicated card.
    assert any("singleton" in e.lower() or "copies" in e.lower() for e in errors)


def test_titan_mode_runs_with_size_override():
    # 80 cards, titan role => expected_main 80; size-error filtering must run
    # over the Rust validate_deck error strings without raising.
    is_valid, errors = _validate_for_mode(["BT1-001"] * 80, "titan", "titan")
    assert isinstance(is_valid, bool)
    assert isinstance(errors, list)
