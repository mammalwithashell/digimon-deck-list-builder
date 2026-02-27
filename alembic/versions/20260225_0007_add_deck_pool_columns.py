"""add deck_pool_id columns to agents and gauntlet_participants

Revision ID: 20260225_0007
Revises: 20260224_0006
Create Date: 2026-02-25 12:00:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260225_0007"
down_revision = "20260224_0006"
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


def _has_fk(table_name: str, fk_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return fk_name in {fk.get("name") for fk in inspector.get_foreign_keys(table_name)}


def upgrade() -> None:
    has_deck_pools = _has_table("deck_pools")

    if _has_table("agents") and not _has_column("agents", "deck_pool_id"):
        with op.batch_alter_table("agents") as batch_op:
            batch_op.add_column(sa.Column("deck_pool_id", sa.String(), nullable=True))
            if has_deck_pools and not _has_fk("agents", "fk_agents_deck_pool_id"):
                batch_op.create_foreign_key(
                    "fk_agents_deck_pool_id",
                    "deck_pools",
                    ["deck_pool_id"],
                    ["id"],
                    ondelete="SET NULL",
                )

    if _has_table("gauntlet_participants") and not _has_column("gauntlet_participants", "deck_pool_id"):
        with op.batch_alter_table("gauntlet_participants") as batch_op:
            batch_op.add_column(sa.Column("deck_pool_id", sa.String(), nullable=True))
            if has_deck_pools and not _has_fk("gauntlet_participants", "fk_gauntlet_participants_deck_pool_id"):
                batch_op.create_foreign_key(
                    "fk_gauntlet_participants_deck_pool_id",
                    "deck_pools",
                    ["deck_pool_id"],
                    ["id"],
                    ondelete="SET NULL",
                )


def downgrade() -> None:
    if _has_table("gauntlet_participants") and _has_column("gauntlet_participants", "deck_pool_id"):
        with op.batch_alter_table("gauntlet_participants") as batch_op:
            if _has_fk("gauntlet_participants", "fk_gauntlet_participants_deck_pool_id"):
                batch_op.drop_constraint("fk_gauntlet_participants_deck_pool_id", type_="foreignkey")
            batch_op.drop_column("deck_pool_id")

    if _has_table("agents") and _has_column("agents", "deck_pool_id"):
        with op.batch_alter_table("agents") as batch_op:
            if _has_fk("agents", "fk_agents_deck_pool_id"):
                batch_op.drop_constraint("fk_agents_deck_pool_id", type_="foreignkey")
            batch_op.drop_column("deck_pool_id")
