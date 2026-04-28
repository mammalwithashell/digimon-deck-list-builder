"""add deck library folders and pinned decks

Revision ID: 20260426_0016
Revises: 20260421_0015
Create Date: 2026-04-26
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260426_0016"
down_revision = "20260421_0015"
branch_labels = None
depends_on = None


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def upgrade() -> None:
    if not _has_table("deck_folders"):
        op.create_table(
            "deck_folders",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column("owner_id", sa.String(), sa.ForeignKey("users.id", ondelete="CASCADE"), nullable=False),
            sa.Column("name", sa.String(), nullable=False),
            sa.Column("sort_order", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.UniqueConstraint("owner_id", "name", name="uq_deck_folders_owner_name"),
        )
        op.create_index("idx_deck_folders_owner_id", "deck_folders", ["owner_id"])

    with op.batch_alter_table("decks") as batch_op:
        if not _has_column("decks", "folder_id"):
            batch_op.add_column(sa.Column("folder_id", sa.String(), nullable=True))
            batch_op.create_foreign_key(
                "fk_decks_folder_id_deck_folders",
                "deck_folders",
                ["folder_id"],
                ["id"],
                ondelete="SET NULL",
            )
        if not _has_column("decks", "is_pinned"):
            batch_op.add_column(sa.Column("is_pinned", sa.Integer(), nullable=False, server_default="0"))


def downgrade() -> None:
    with op.batch_alter_table("decks") as batch_op:
        if _has_column("decks", "is_pinned"):
            batch_op.drop_column("is_pinned")
        if _has_column("decks", "folder_id"):
            batch_op.drop_constraint("fk_decks_folder_id_deck_folders", type_="foreignkey")
            batch_op.drop_column("folder_id")

    if _has_table("deck_folders"):
        op.drop_index("idx_deck_folders_owner_id", table_name="deck_folders")
        op.drop_table("deck_folders")
