"""Persist immutable model training contracts; legacy rows stay unvalidated."""
from alembic import op
import sqlalchemy as sa
revision = "20261011_0022"
down_revision = "20260628_0021"
branch_labels = None
depends_on = None

def upgrade():
    for name, kind in [("observation_profile", sa.String()), ("tensor_version", sa.Integer()), ("feature_schema_version", sa.String()), ("tensor_layout_hash", sa.String())]:
        op.add_column("ai_models", sa.Column(name, kind, nullable=True))

def downgrade():
    for name in ["tensor_layout_hash", "feature_schema_version", "tensor_version", "observation_profile"]:
        op.drop_column("ai_models", name)
