"""add fix_iteration and max_fix_iterations to ai_set_runs

Revision ID: 20260226_0011
Revises: 20260225_0010
Create Date: 2026-02-26
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260226_0011"
down_revision = "20260225_0010"
branch_labels = None
depends_on = None


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def upgrade() -> None:
    if not _has_column("ai_set_runs", "fix_iteration"):
        op.add_column("ai_set_runs", sa.Column("fix_iteration", sa.Integer(), nullable=False, server_default="0"))
    if not _has_column("ai_set_runs", "max_fix_iterations"):
        op.add_column("ai_set_runs", sa.Column("max_fix_iterations", sa.Integer(), nullable=False, server_default="3"))


def downgrade() -> None:
    op.drop_column("ai_set_runs", "max_fix_iterations")
    op.drop_column("ai_set_runs", "fix_iteration")
