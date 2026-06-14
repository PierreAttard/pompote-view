#!/bin/sh
# One-shot migration step for the local stack (issue #5).
#
# Applies robot_rust's forward migrations to the local Timescale, then creates
# the read-only `pompote_viz_reader` role. This mirrors the OVH setup locally;
# robot_rust's schema is bind-mounted at runtime and never committed to this repo.
#
# psql reads its connection from the PG* env vars (set by docker-compose).
set -eu

# Skip the (non-idempotent) robot_rust migrations when the schema is already
# present, so re-running `docker compose up` on a persisted volume does not fail
# and block viz_api. Detection: any base table in `public` means it's migrated.
# (robot_rust's *.up.sql are not ours to make idempotent.)
applied=$(psql -tAc "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE';")
if [ "${applied:-0}" -gt 0 ]; then
	echo "Schema already present (${applied} table(s) in public) — skipping migrations."
else
	echo "Applying robot_rust migrations (*.up.sql) to '${PGDATABASE}' ..."
	# Glob expansion is lexically sorted == sqlx version order (timestamp-prefixed
	# filenames). Only the forward (.up.sql) files are applied; sqlx's own tracking
	# table is not needed for a throwaway local database.
	for f in /migrations/*.up.sql; do
		echo "  -> $(basename "$f")"
		psql -v ON_ERROR_STOP=1 -f "$f"
	done
fi

# Always (re-)apply the read-only role + grants — this script is idempotent.
echo "Provisioning read-only role pompote_viz_reader ..."
psql -v ON_ERROR_STOP=1 -v reader_password="${VIZ_READER_PASSWORD}" -f /db/grant-viz-reader.sql

echo "Local database ready (schema applied + read-only role granted)."
