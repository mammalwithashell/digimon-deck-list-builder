"""relax deck commander constraint for visual deck icons

Revision ID: 20260613_0020
Revises: 20260613_0019
Create Date: 2026-06-13

`commander_id` is now reused as a visual deck icon card id for every deck
format. The application layer validates that the id, when present, references
a card in the deck.
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260613_0020"
down_revision = "20260613_0019"
branch_labels = None
depends_on = None

OLD_COMMANDER_CONSTRAINT = (
    "(game_mode = 'edh_commander' AND commander_id IS NOT NULL) "
    "OR (game_mode != 'edh_commander' AND commander_id IS NULL)"
)


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def upgrade() -> None:
    if not _has_table("decks"):
        return
    with op.batch_alter_table("decks") as batch_op:
        batch_op.drop_constraint("ck_decks_commander", type_="check")


def downgrade() -> None:
    if not _has_table("decks"):
        return
    op.execute("UPDATE decks SET commander_id = NULL WHERE game_mode != 'edh_commander'")
    op.execute(
        "UPDATE decks SET commander_id = 'DOWNGRADE-COMMANDER' "
        "WHERE game_mode = 'edh_commander' AND commander_id IS NULL"
    )
    with op.batch_alter_table("decks") as batch_op:
        batch_op.create_check_constraint("ck_decks_commander", OLD_COMMANDER_CONSTRAINT)
