"""add releases and known_issues tables for patch notes

Revision ID: 20260415_0014
Revises: 20260414_0013a
Create Date: 2026-04-14

Serialized behind add_deck_alt_arts (20260414_0013a) to avoid the dual-head
state that shipped briefly when both 20260414_0013 migrations landed on
main independently. Runs cleanly either fresh or on top of a DB that had
patch_notes applied under the old revision string — the table-creation
functions guard on existence.
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260415_0014"
down_revision = "20260414_0013a"
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
