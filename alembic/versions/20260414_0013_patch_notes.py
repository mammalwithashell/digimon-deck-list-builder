"""add releases and known_issues tables for patch notes

Revision ID: 20260414_0013
Revises: 20260227_0012
Create Date: 2026-04-14
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260414_0013"
down_revision = "20260227_0012"
branch_labels = None
depends_on = None


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def upgrade() -> None:
    if not _has_table("releases"):
        op.create_table(
            "releases",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column("version", sa.String(), nullable=False),
            sa.Column("release_date", sa.DateTime(timezone=True), nullable=False),
            sa.Column("title", sa.String(), nullable=True),
            sa.Column("added", sa.JSON(), nullable=False),
            sa.Column("changed", sa.JSON(), nullable=False),
            sa.Column("fixed", sa.JSON(), nullable=False),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.UniqueConstraint("version", name="uq_releases_version"),
        )
        op.create_index(
            "idx_releases_release_date", "releases", ["release_date"]
        )

    if not _has_table("known_issues"):
        op.create_table(
            "known_issues",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column("title", sa.String(), nullable=False),
            sa.Column("description", sa.Text(), nullable=False),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
        )
        op.create_index(
            "idx_known_issues_created_at", "known_issues", ["created_at"]
        )


def downgrade() -> None:
    if _has_table("known_issues"):
        op.drop_index("idx_known_issues_created_at", table_name="known_issues")
        op.drop_table("known_issues")
    if _has_table("releases"):
        op.drop_index("idx_releases_release_date", table_name="releases")
        op.drop_table("releases")
