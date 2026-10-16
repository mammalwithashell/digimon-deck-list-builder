"""Require declared action semantics; legacy rows remain NULL and incompatible."""
from alembic import op
import sqlalchemy as sa
revision = "20261012_0023"
down_revision = "20261011_0022"
branch_labels = None
depends_on = None
def upgrade():
    op.add_column("ai_models", sa.Column("action_schema_version", sa.Integer(), nullable=True))
def downgrade():
    op.drop_column("ai_models", "action_schema_version")
