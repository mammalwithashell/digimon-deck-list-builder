"""add rbac and ai workflow tables

Revision ID: 20260222_0002
Revises: 20260222_0001
Create Date: 2026-02-22 00:20:00
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


revision = "20260222_0002"
down_revision = "20260222_0001"
branch_labels = None
depends_on = None


def _has_table(name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return name in inspector.get_table_names()


def upgrade() -> None:
    if not _has_table("roles"):
        op.create_table(
            "roles",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("name", sa.String(), nullable=False),
            sa.Column("description", sa.Text(), nullable=False, server_default=""),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.PrimaryKeyConstraint("id"),
            sa.UniqueConstraint("name"),
        )
        op.create_index("idx_roles_name", "roles", ["name"])

    if not _has_table("user_roles"):
        op.create_table(
            "user_roles",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("user_id", sa.String(), nullable=False),
            sa.Column("role_id", sa.String(), nullable=False),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.ForeignKeyConstraint(["role_id"], ["roles.id"], ondelete="CASCADE"),
            sa.ForeignKeyConstraint(["user_id"], ["users.id"], ondelete="CASCADE"),
            sa.PrimaryKeyConstraint("id"),
            sa.UniqueConstraint("user_id", "role_id", name="uq_user_roles_user_role"),
        )
        op.create_index("idx_user_roles_user_id", "user_roles", ["user_id"])
        op.create_index("idx_user_roles_role_id", "user_roles", ["role_id"])

    if not _has_table("issues"):
        op.create_table(
            "issues",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("card_id", sa.String(), nullable=False),
            sa.Column("description", sa.Text(), nullable=False),
            sa.Column("source", sa.String(), nullable=False),
            sa.Column("severity", sa.String(), nullable=False, server_default="medium"),
            sa.Column("status", sa.String(), nullable=False, server_default="new"),
            sa.Column("created_by", sa.String(), nullable=True),
            sa.Column("approved_by", sa.String(), nullable=True),
            sa.Column("triage_notes", sa.Text(), nullable=False, server_default=""),
            sa.Column("resolution_notes", sa.Text(), nullable=False, server_default=""),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("resolved_at", sa.DateTime(timezone=True), nullable=True),
            sa.CheckConstraint("source IN ('player', 'judge', 'system')", name="ck_issues_source"),
            sa.CheckConstraint("severity IN ('low', 'medium', 'high', 'critical')", name="ck_issues_severity"),
            sa.CheckConstraint(
                "status IN ('new', 'approved_for_ai', 'rejected', 'resolved')",
                name="ck_issues_status",
            ),
            sa.ForeignKeyConstraint(["created_by"], ["users.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["approved_by"], ["users.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_issues_status", "issues", ["status"])
        op.create_index("idx_issues_card_id", "issues", ["card_id"])
        op.create_index("idx_issues_created_by", "issues", ["created_by"])

    if not _has_table("ai_tasks"):
        op.create_table(
            "ai_tasks",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("task_type", sa.String(), nullable=False),
            sa.Column("payload_json", sa.Text(), nullable=False, server_default="{}"),
            sa.Column("status", sa.String(), nullable=False, server_default="queued"),
            sa.Column("result_json", sa.Text(), nullable=True),
            sa.Column("sanitized_input_json", sa.Text(), nullable=False, server_default="{}"),
            sa.Column("retrieval_refs_json", sa.Text(), nullable=False, server_default="[]"),
            sa.Column("model_name", sa.String(), nullable=True),
            sa.Column("cost_estimate_usd", sa.Float(), nullable=False, server_default="0"),
            sa.Column("cost_actual_usd", sa.Float(), nullable=False, server_default="0"),
            sa.Column("input_tokens", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("output_tokens", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("error_text", sa.Text(), nullable=True),
            sa.Column("attempts", sa.Integer(), nullable=False, server_default="0"),
            sa.Column("max_attempts", sa.Integer(), nullable=False, server_default="3"),
            sa.Column("created_by", sa.String(), nullable=True),
            sa.Column("started_at", sa.DateTime(timezone=True), nullable=True),
            sa.Column("completed_at", sa.DateTime(timezone=True), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint(
                "task_type IN ('review_batch', 'qa_analysis', 'engine_audit')",
                name="ck_ai_tasks_task_type",
            ),
            sa.CheckConstraint(
                "status IN ('queued', 'running', 'completed', 'failed')",
                name="ck_ai_tasks_status",
            ),
            sa.ForeignKeyConstraint(["created_by"], ["users.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_ai_tasks_status", "ai_tasks", ["status"])
        op.create_index("idx_ai_tasks_task_type", "ai_tasks", ["task_type"])
        op.create_index("idx_ai_tasks_created_at", "ai_tasks", ["created_at"])

    if not _has_table("engine_backlog_items"):
        op.create_table(
            "engine_backlog_items",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("title", sa.String(), nullable=False),
            sa.Column("description", sa.Text(), nullable=False, server_default=""),
            sa.Column("mechanic", sa.String(), nullable=False, server_default=""),
            sa.Column("status", sa.String(), nullable=False, server_default="open"),
            sa.Column("source_task_id", sa.String(), nullable=True),
            sa.Column("created_by", sa.String(), nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("updated_at", sa.DateTime(timezone=True), nullable=False),
            sa.CheckConstraint(
                "status IN ('open', 'in_progress', 'closed')",
                name="ck_engine_backlog_status",
            ),
            sa.ForeignKeyConstraint(["source_task_id"], ["ai_tasks.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["created_by"], ["users.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_engine_backlog_status", "engine_backlog_items", ["status"])
        op.create_index("idx_engine_backlog_source_task", "engine_backlog_items", ["source_task_id"])

    if not _has_table("script_promotion_audits"):
        op.create_table(
            "script_promotion_audits",
            sa.Column("id", sa.String(), nullable=False),
            sa.Column("card_id", sa.String(), nullable=False),
            sa.Column("set_id", sa.String(), nullable=False),
            sa.Column("module_name", sa.String(), nullable=False),
            sa.Column("generated_hash", sa.String(), nullable=False),
            sa.Column("frozen_hash", sa.String(), nullable=False),
            sa.Column("manifest_version", sa.Integer(), nullable=False, server_default="1"),
            sa.Column("ai_task_id", sa.String(), nullable=True),
            sa.Column("promoted_by", sa.String(), nullable=True),
            sa.Column("notes", sa.Text(), nullable=False, server_default=""),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.ForeignKeyConstraint(["ai_task_id"], ["ai_tasks.id"], ondelete="SET NULL"),
            sa.ForeignKeyConstraint(["promoted_by"], ["users.id"], ondelete="SET NULL"),
            sa.PrimaryKeyConstraint("id"),
        )
        op.create_index("idx_script_promotions_card_id", "script_promotion_audits", ["card_id"])
        op.create_index("idx_script_promotions_created_at", "script_promotion_audits", ["created_at"])


def downgrade() -> None:
    if _has_table("script_promotion_audits"):
        op.drop_index("idx_script_promotions_created_at", table_name="script_promotion_audits")
        op.drop_index("idx_script_promotions_card_id", table_name="script_promotion_audits")
        op.drop_table("script_promotion_audits")

    if _has_table("engine_backlog_items"):
        op.drop_index("idx_engine_backlog_source_task", table_name="engine_backlog_items")
        op.drop_index("idx_engine_backlog_status", table_name="engine_backlog_items")
        op.drop_table("engine_backlog_items")

    if _has_table("ai_tasks"):
        op.drop_index("idx_ai_tasks_created_at", table_name="ai_tasks")
        op.drop_index("idx_ai_tasks_task_type", table_name="ai_tasks")
        op.drop_index("idx_ai_tasks_status", table_name="ai_tasks")
        op.drop_table("ai_tasks")

    if _has_table("issues"):
        op.drop_index("idx_issues_created_by", table_name="issues")
        op.drop_index("idx_issues_card_id", table_name="issues")
        op.drop_index("idx_issues_status", table_name="issues")
        op.drop_table("issues")

    if _has_table("user_roles"):
        op.drop_index("idx_user_roles_role_id", table_name="user_roles")
        op.drop_index("idx_user_roles_user_id", table_name="user_roles")
        op.drop_table("user_roles")

    if _has_table("roles"):
        op.drop_index("idx_roles_name", table_name="roles")
        op.drop_table("roles")
