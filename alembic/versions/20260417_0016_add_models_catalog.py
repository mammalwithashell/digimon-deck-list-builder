"""add model_records and model_versions tables for ONNX catalog

Revision ID: 20260417_0016
Revises: 20260417_0015
Create Date: 2026-04-17 13:00:00
"""

from alembic import op
import sqlalchemy as sa


revision = "20260417_0016"
down_revision = "20260417_0015"
branch_labels = None
depends_on = None


def _has_table(name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return name in inspector.get_table_names()


def upgrade() -> None:
    if not _has_table("model_records"):
        op.create_table(
            "model_records",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column("slug", sa.String(), unique=True, nullable=False),
            sa.Column("name", sa.String(), nullable=False),
            sa.Column("arch", sa.String(), nullable=False),
            sa.Column("description", sa.Text(), nullable=False, server_default=""),
            sa.Column("deck_pool", sa.String(), nullable=True),
            sa.Column("min_engine_version", sa.String(), nullable=False, server_default="0.1.0"),
            sa.Column("is_public", sa.Integer(), nullable=False, server_default="1"),
            sa.Column("is_deprecated", sa.Integer(), nullable=False, server_default="0"),
            sa.Column(
                "created_by_user_id",
                sa.String(),
                sa.ForeignKey("users.id", ondelete="SET NULL"),
                nullable=True,
            ),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint("arch IN ('mlp', 'lstm')", name="ck_model_records_arch"),
        )
        op.create_index("idx_model_records_slug", "model_records", ["slug"])

    if not _has_table("model_versions"):
        op.create_table(
            "model_versions",
            sa.Column("id", sa.String(), primary_key=True),
            sa.Column(
                "model_id",
                sa.String(),
                sa.ForeignKey("model_records.id", ondelete="CASCADE"),
                nullable=False,
            ),
            sa.Column("version", sa.String(), nullable=False),
            sa.Column("storage_key", sa.String(), nullable=False),
            sa.Column("size_bytes", sa.Integer(), nullable=False),
            sa.Column("sha256", sa.String(), nullable=False),
            sa.Column("min_engine_version", sa.String(), nullable=True),
            sa.Column("onnx_input_shapes", sa.JSON(), nullable=True),
            sa.Column("changelog", sa.Text(), nullable=False, server_default=""),
            sa.Column("is_deprecated", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("released_at", sa.DateTime(timezone=True), nullable=False),
            sa.UniqueConstraint("model_id", "version", name="uq_model_versions_model_version"),
        )
        op.create_index("idx_model_versions_model_id", "model_versions", ["model_id"])
        op.create_index("idx_model_versions_released_at", "model_versions", ["released_at"])


def downgrade() -> None:
    if _has_table("model_versions"):
        op.drop_index("idx_model_versions_released_at", table_name="model_versions")
        op.drop_index("idx_model_versions_model_id", table_name="model_versions")
        op.drop_table("model_versions")
    if _has_table("model_records"):
        op.drop_index("idx_model_records_slug", table_name="model_records")
        op.drop_table("model_records")
