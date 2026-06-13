"""add pauper and eden_singleton game modes

Revision ID: 20260613_0019
Revises: 20260502_0018
Create Date: 2026-06-13

Adds the `pauper` and `eden_singleton` formats to the game_mode check
constraints on `decks` and `game_sessions`. Mirrors 20260427_0017
(add_eden_game_mode). Downgrade normalizes rows in the two new modes back to
`standard` before restoring the previous constraint.
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260613_0019"
down_revision = "20260502_0018"
branch_labels = None
depends_on = None


NEW_DECK_MODES = (
    "game_mode IN ('standard', 'eden', 'eden_singleton', 'pauper', "
    "'edh_commander', 'titan', 'no_restriction')"
)
OLD_DECK_MODES = (
    "game_mode IN ('standard', 'eden', 'edh_commander', 'titan', 'no_restriction')"
)


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def upgrade() -> None:
    if _has_table("decks"):
        with op.batch_alter_table("decks") as batch_op:
            batch_op.drop_constraint("ck_decks_game_mode", type_="check")
            batch_op.create_check_constraint("ck_decks_game_mode", NEW_DECK_MODES)

    if _has_table("game_sessions"):
        with op.batch_alter_table("game_sessions") as batch_op:
            batch_op.drop_constraint("ck_game_sessions_mode", type_="check")
            batch_op.create_check_constraint("ck_game_sessions_mode", NEW_DECK_MODES)


def downgrade() -> None:
    if _has_table("decks"):
        op.execute(
            "UPDATE decks SET game_mode = 'standard' "
            "WHERE game_mode IN ('eden_singleton', 'pauper')"
        )
        with op.batch_alter_table("decks") as batch_op:
            batch_op.drop_constraint("ck_decks_game_mode", type_="check")
            batch_op.create_check_constraint("ck_decks_game_mode", OLD_DECK_MODES)

    if _has_table("game_sessions"):
        op.execute(
            "UPDATE game_sessions SET game_mode = 'standard' "
            "WHERE game_mode IN ('eden_singleton', 'pauper')"
        )
        with op.batch_alter_table("game_sessions") as batch_op:
            batch_op.drop_constraint("ck_game_sessions_mode", type_="check")
            batch_op.create_check_constraint("ck_game_sessions_mode", OLD_DECK_MODES)
