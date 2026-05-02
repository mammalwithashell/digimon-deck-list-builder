"""add eden game mode

Revision ID: 20260427_0017
Revises: 20260426_0016
Create Date: 2026-04-27
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260427_0017"
down_revision = "20260426_0016"
branch_labels = None
depends_on = None


DECK_MODES = "game_mode IN ('standard', 'eden', 'edh_commander', 'titan', 'no_restriction')"
OLD_DECK_MODES = "game_mode IN ('standard', 'edh_commander', 'titan', 'no_restriction')"


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def upgrade() -> None:
    if _has_table("decks"):
        with op.batch_alter_table("decks") as batch_op:
            batch_op.drop_constraint("ck_decks_game_mode", type_="check")
            batch_op.create_check_constraint("ck_decks_game_mode", DECK_MODES)

    if _has_table("game_sessions"):
        with op.batch_alter_table("game_sessions") as batch_op:
            batch_op.drop_constraint("ck_game_sessions_mode", type_="check")
            batch_op.create_check_constraint("ck_game_sessions_mode", DECK_MODES)


def downgrade() -> None:
    if _has_table("decks"):
        op.execute("UPDATE decks SET game_mode = 'standard' WHERE game_mode = 'eden'")
        with op.batch_alter_table("decks") as batch_op:
            batch_op.drop_constraint("ck_decks_game_mode", type_="check")
            batch_op.create_check_constraint("ck_decks_game_mode", OLD_DECK_MODES)

    if _has_table("game_sessions"):
        op.execute("UPDATE game_sessions SET game_mode = 'standard' WHERE game_mode = 'eden'")
        with op.batch_alter_table("game_sessions") as batch_op:
            batch_op.drop_constraint("ck_game_sessions_mode", type_="check")
            batch_op.create_check_constraint("ck_game_sessions_mode", OLD_DECK_MODES)
