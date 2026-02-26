"""Add retranspile columns and learn run table."""

revision = "20260225_0010"
down_revision = "20260225_0009"
branch_labels = None
depends_on = None

import sqlalchemy as sa
from alembic import op


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
    # AISetRun columns
    if not _has_column("ai_set_runs", "score_threshold"):
        op.add_column("ai_set_runs", sa.Column("score_threshold", sa.Float, nullable=True))
    if not _has_column("ai_set_runs", "retranspile_total"):
        op.add_column("ai_set_runs", sa.Column("retranspile_total", sa.Integer, nullable=False, server_default="0"))
    if not _has_column("ai_set_runs", "retranspile_completed"):
        op.add_column("ai_set_runs", sa.Column("retranspile_completed", sa.Integer, nullable=False, server_default="0"))
    if not _has_column("ai_set_runs", "retranspile_failed"):
        op.add_column("ai_set_runs", sa.Column("retranspile_failed", sa.Integer, nullable=False, server_default="0"))

    # AISetRunItem columns
    if not _has_column("ai_set_run_items", "transpile_score"):
        op.add_column("ai_set_run_items", sa.Column("transpile_score", sa.Float, nullable=True))
    if not _has_column("ai_set_run_items", "retranspile_task_id"):
        op.add_column("ai_set_run_items", sa.Column("retranspile_task_id", sa.String, nullable=True))

    # AITranspilerLearnRun table
    if not _has_table("ai_transpiler_learn_runs"):
        op.create_table(
            "ai_transpiler_learn_runs",
            sa.Column("id", sa.String, primary_key=True),
            sa.Column("source_set_run_id", sa.String, sa.ForeignKey("ai_set_runs.id", ondelete="SET NULL"), nullable=True),
            sa.Column("status", sa.String, nullable=False, server_default="clustering"),
            sa.Column("clusters_found", sa.Integer, nullable=False, server_default="0"),
            sa.Column("patches_proposed", sa.Integer, nullable=False, server_default="0"),
            sa.Column("pr_url", sa.String, nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("completed_at", sa.DateTime(timezone=True), nullable=True),
        )


def downgrade() -> None:
    op.drop_table("ai_transpiler_learn_runs")
    op.drop_column("ai_set_run_items", "retranspile_task_id")
    op.drop_column("ai_set_run_items", "transpile_score")
    op.drop_column("ai_set_runs", "retranspile_failed")
    op.drop_column("ai_set_runs", "retranspile_completed")
    op.drop_column("ai_set_runs", "retranspile_total")
    op.drop_column("ai_set_runs", "score_threshold")
