"""add alt-art columns to decks and deck_versions

Revision ID: 20260414_0013
Revises: 20260227_0012
Create Date: 2026-04-14 12:00:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260414_0013"
down_revision = "20260227_0012"
branch_labels = None
depends_on = None


def _has_table(name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return name in inspector.get_table_names()


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def _add_alt_art_columns(table_name: str) -> None:
    if not _has_table(table_name):
        return
    with op.batch_alter_table(table_name) as batch_op:
        if not _has_column(table_name, "main_deck_alt_arts"):
            batch_op.add_column(
                sa.Column(
                    "main_deck_alt_arts",
                    sa.Text(),
                    nullable=True,
                    server_default="[]",
                )
            )
        if not _has_column(table_name, "egg_deck_alt_arts"):
            batch_op.add_column(
                sa.Column(
                    "egg_deck_alt_arts",
                    sa.Text(),
                    nullable=True,
                    server_default="[]",
                )
            )


def _drop_alt_art_columns(table_name: str) -> None:
    if not _has_table(table_name):
        return
    with op.batch_alter_table(table_name) as batch_op:
        if _has_column(table_name, "egg_deck_alt_arts"):
            batch_op.drop_column("egg_deck_alt_arts")
        if _has_column(table_name, "main_deck_alt_arts"):
            batch_op.drop_column("main_deck_alt_arts")


def upgrade() -> None:
    _add_alt_art_columns("decks")
    _add_alt_art_columns("deck_versions")


def downgrade() -> None:
    _drop_alt_art_columns("deck_versions")
    _drop_alt_art_columns("decks")
