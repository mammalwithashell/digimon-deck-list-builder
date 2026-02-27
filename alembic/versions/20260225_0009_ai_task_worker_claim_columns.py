"""add worker claim columns to ai_tasks

Revision ID: 20260225_0009
Revises: 20260225_0008
Create Date: 2026-02-25 23:00:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260225_0009"
down_revision = "20260225_0008"
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


def _has_index(table_name: str, index_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return index_name in {idx["name"] for idx in inspector.get_indexes(table_name)}


def upgrade() -> None:
    if not _has_table("ai_tasks"):
        return

    if not _has_column("ai_tasks", "worker_id"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            batch_op.add_column(sa.Column("worker_id", sa.String(), nullable=True))

    if not _has_column("ai_tasks", "claimed_at"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            batch_op.add_column(sa.Column("claimed_at", sa.DateTime(timezone=True), nullable=True))

    if not _has_index("ai_tasks", "ix_ai_tasks_worker_id"):
        op.create_index("ix_ai_tasks_worker_id", "ai_tasks", ["worker_id"])


def downgrade() -> None:
    if not _has_table("ai_tasks"):
        return

    if _has_index("ai_tasks", "ix_ai_tasks_worker_id"):
        op.drop_index("ix_ai_tasks_worker_id", table_name="ai_tasks")

    if _has_column("ai_tasks", "claimed_at"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            batch_op.drop_column("claimed_at")

    if _has_column("ai_tasks", "worker_id"):
        with op.batch_alter_table("ai_tasks") as batch_op:
            batch_op.drop_column("worker_id")
