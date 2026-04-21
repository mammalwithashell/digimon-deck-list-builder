"""add app_releases and app_release_artifacts tables

Revision ID: 20260421_0015
Revises: 20260417_0014
Create Date: 2026-04-21

NOTE: The repo currently has two migration files that both declare
`revision = "20260417_0014"` (`20260417_0014_ai_models.py` and
`20260417_0014_add_deck_meta_tier.py`). Alembic's revision map silently
resolves the duplicate ID to the `ai_models` file, while still reporting
two heads. Setting `down_revision = "20260417_0014"` chains this migration
off the resolved revision, collapsing the ambiguous head set back to a
single head. The duplicate-revision-ID issue is pre-existing and out of
scope for this change; see `20260417_0014_add_deck_meta_tier.py`'s header
for context.
"""
from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260421_0015"
down_revision = "20260417_0014"
branch_labels = None
depends_on = None


def _has_table(table_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return table_name in inspector.get_table_names()


def upgrade() -> None:
    if not _has_table("app_releases"):
        op.create_table(
            "app_releases",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column("version", sa.String(), nullable=False),
            sa.Column("channel", sa.String(), nullable=False),
            sa.Column("engine_commit", sa.String(), nullable=False),
            sa.Column("min_version", sa.String(), nullable=False),
            sa.Column("release_notes", sa.Text(), nullable=False, server_default=""),
            sa.Column("published", sa.Boolean(), nullable=False, server_default="0"),
            sa.Column("published_at", sa.DateTime(timezone=True), nullable=True),
            sa.Column("state", sa.String(), nullable=False, server_default="pending"),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint(
                "state IN ('pending', 'uploaded', 'failed')",
                name="ck_app_releases_state",
            ),
            sa.UniqueConstraint("channel", "version", name="uq_app_releases_channel_version"),
        )
        op.create_index("idx_app_releases_channel_pub", "app_releases", ["channel", "published"])

    if not _has_table("app_release_artifacts"):
        op.create_table(
            "app_release_artifacts",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column(
                "release_id",
                sa.String(),
                sa.ForeignKey("app_releases.id", ondelete="CASCADE"),
                nullable=False,
            ),
            sa.Column("target", sa.String(), nullable=False),
            sa.Column("spaces_key", sa.String(), nullable=False),
            sa.Column("filename", sa.String(), nullable=False),
            sa.Column("file_sha256", sa.String(), nullable=True),
            sa.Column("file_size_bytes", sa.Integer(), nullable=True),
            sa.Column("signature_b64", sa.Text(), nullable=True),
            sa.CheckConstraint(
                "target IN ('windows-x86_64', 'linux-x86_64')",
                name="ck_app_release_artifacts_target",
            ),
            sa.UniqueConstraint("release_id", "target", name="uq_app_release_artifacts_release_target"),
            sa.UniqueConstraint("spaces_key", name="uq_app_release_artifacts_spaces_key"),
        )
        op.create_index("idx_app_release_artifacts_release", "app_release_artifacts", ["release_id"])


def downgrade() -> None:
    if _has_table("app_release_artifacts"):
        op.drop_index("idx_app_release_artifacts_release", table_name="app_release_artifacts")
        op.drop_table("app_release_artifacts")
    if _has_table("app_releases"):
        op.drop_index("idx_app_releases_channel_pub", table_name="app_releases")
        op.drop_table("app_releases")
