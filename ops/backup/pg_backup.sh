#!/bin/sh
set -eu

: "${POSTGRES_USER:?required}"
: "${POSTGRES_DB:?required}"
: "${SPACES_ENDPOINT:?required}"
: "${SPACES_BUCKET:?required}"
: "${SPACES_KEY:?required}"
: "${SPACES_SECRET:?required}"

export AWS_ACCESS_KEY_ID="$SPACES_KEY"
export AWS_SECRET_ACCESS_KEY="$SPACES_SECRET"

TS=$(date -u +%Y%m%dT%H%M%SZ)
KEY="backups/digimon-${TS}.sql.gz"

pg_dump -h postgres -U "$POSTGRES_USER" "$POSTGRES_DB" \
  | gzip -9 \
  | aws --endpoint-url "$SPACES_ENDPOINT" s3 cp - "s3://${SPACES_BUCKET}/${KEY}"

echo "backup complete: s3://${SPACES_BUCKET}/${KEY}"
