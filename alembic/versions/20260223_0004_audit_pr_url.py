"""add pr_url to ai_fix_apply_audits

Revision ID: 20260223_0004
Revises: 20260222_0003
Create Date: 2026-02-23 10:00:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260223_0004"
down_revision = "20260222_0003"
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


def upgrade() -> None:
    if _has_table("ai_fix_apply_audits") and not _has_column("ai_fix_apply_audits", "pr_url"):
        with op.batch_alter_table("ai_fix_apply_audits") as batch_op:
            batch_op.add_column(sa.Column("pr_url", sa.String(), nullable=True))


def downgrade() -> None:
    if _has_table("ai_fix_apply_audits") and _has_column("ai_fix_apply_audits", "pr_url"):
        with op.batch_alter_table("ai_fix_apply_audits") as batch_op:
            batch_op.drop_column("pr_url")
