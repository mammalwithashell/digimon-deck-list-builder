"""add ai_models.starter_deck slug for built-in (non-DB) deck routing

Revision ID: 20260628_0021
Revises: 20260613_0020
Create Date: 2026-06-28

Adds a free-form `starter_deck` column to `ai_models`. Unlike `deck_id` (an FK
to a real `decks` row), this names a *built-in* bundled deck (e.g. the
AI-Starter deck id "starter_st3_heavens_yellow") that has no DB row, so the
desktop AI-Starter mode can route each starter deck to its trained specialist
model via the public manifest.
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260628_0021"
down_revision = "20260613_0020"
branch_labels = None
depends_on = None


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def upgrade() -> None:
    with op.batch_alter_table("ai_models") as batch_op:
        if not _has_column("ai_models", "starter_deck"):
            batch_op.add_column(sa.Column("starter_deck", sa.String(), nullable=True))


def downgrade() -> None:
    with op.batch_alter_table("ai_models") as batch_op:
        if _has_column("ai_models", "starter_deck"):
            batch_op.drop_column("starter_deck")
