"""add matchmaking fields: decks.meta_tier/meta_archetype + users.rating

Revision ID: 20260417_0014
Revises: 20260414_0013
Create Date: 2026-04-17

NOTE: The repo currently has two migrations sharing revision ID
`20260414_0013` (`add_deck_alt_arts` and `patch_notes`). Alembic reports
both as heads and warns on `alembic heads`. This migration chains off that
ambiguous revision — the duplicate-head issue is pre-existing and out of
scope for this change. Whichever `_0013` runs last, this migration runs
after it.
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260417_0014"
down_revision = "20260414_0013"
branch_labels = None
depends_on = None


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def upgrade() -> None:
    with op.batch_alter_table("decks") as batch_op:
        if not _has_column("decks", "meta_tier"):
            batch_op.add_column(sa.Column("meta_tier", sa.String(), nullable=True))
        if not _has_column("decks", "meta_archetype"):
            batch_op.add_column(sa.Column("meta_archetype", sa.String(), nullable=True))
    with op.batch_alter_table("users") as batch_op:
        if not _has_column("users", "rating"):
            batch_op.add_column(
                sa.Column("rating", sa.Float(), nullable=False, server_default="1500.0")
            )


def downgrade() -> None:
    with op.batch_alter_table("users") as batch_op:
        if _has_column("users", "rating"):
            batch_op.drop_column("rating")
    with op.batch_alter_table("decks") as batch_op:
        if _has_column("decks", "meta_archetype"):
            batch_op.drop_column("meta_archetype")
        if _has_column("decks", "meta_tier"):
            batch_op.drop_column("meta_tier")
