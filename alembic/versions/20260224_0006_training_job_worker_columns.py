"""add worker_id, claimed_at, device columns to training_jobs

Revision ID: 20260224_0006
Revises: 20260224_0005
Create Date: 2026-02-24 18:00:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260224_0006"
down_revision = "20260224_0005"
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
    if not _has_table("training_jobs"):
        return

    if not _has_column("training_jobs", "worker_id"):
        with op.batch_alter_table("training_jobs") as batch_op:
            batch_op.add_column(sa.Column("worker_id", sa.String(), nullable=True))

    if not _has_column("training_jobs", "claimed_at"):
        with op.batch_alter_table("training_jobs") as batch_op:
            batch_op.add_column(sa.Column("claimed_at", sa.DateTime(timezone=True), nullable=True))

    if not _has_column("training_jobs", "device"):
        with op.batch_alter_table("training_jobs") as batch_op:
            batch_op.add_column(sa.Column("device", sa.String(), nullable=True))

    if not _has_index("training_jobs", "idx_training_jobs_status_created"):
        op.create_index(
            "idx_training_jobs_status_created",
            "training_jobs",
            ["status", "created_at"],
        )

    if not _has_index("training_jobs", "ix_training_jobs_worker_id"):
        op.create_index(
            "ix_training_jobs_worker_id",
            "training_jobs",
            ["worker_id"],
        )


def downgrade() -> None:
    if not _has_table("training_jobs"):
        return

    if _has_index("training_jobs", "ix_training_jobs_worker_id"):
        op.drop_index("ix_training_jobs_worker_id", table_name="training_jobs")

    if _has_index("training_jobs", "idx_training_jobs_status_created"):
        op.drop_index("idx_training_jobs_status_created", table_name="training_jobs")

    if _has_column("training_jobs", "device"):
        with op.batch_alter_table("training_jobs") as batch_op:
            batch_op.drop_column("device")

    if _has_column("training_jobs", "claimed_at"):
        with op.batch_alter_table("training_jobs") as batch_op:
            batch_op.drop_column("claimed_at")

    if _has_column("training_jobs", "worker_id"):
        with op.batch_alter_table("training_jobs") as batch_op:
            batch_op.drop_column("worker_id")
