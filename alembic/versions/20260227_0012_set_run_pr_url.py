"""add pr_url to ai_set_runs for consolidated PR

Revision ID: 20260227_0012
Revises: 20260226_0011
Create Date: 2026-02-27
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260227_0012"
down_revision = "20260226_0011"
branch_labels = None
depends_on = None


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def upgrade() -> None:
    if not _has_column("ai_set_runs", "pr_url"):
        op.add_column("ai_set_runs", sa.Column("pr_url", sa.String(), nullable=True))


def downgrade() -> None:
    op.drop_column("ai_set_runs", "pr_url")
