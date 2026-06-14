#!/bin/sh
# One-shot migration step for the local stack (issue #5).
#
# Applies robot_rust's forward migrations to the local Timescale, then creates
# the read-only `pompote_viz_reader` role. This mirrors the OVH setup locally;
# robot_rust's schema is bind-mounted at runtime and never committed to this repo.
#
# psql reads its connection from the PG* env vars (set by docker-compose).
set -eu

echo "Applying robot_rust migrations (*.up.sql) to '${PGDATABASE}' ..."
# Lexical order == sqlx version order (timestamp-prefixed filenames). We apply
# only the forward (.up.sql) files; sqlx's own tracking table is not needed for
# a throwaway local database.
for f in $(ls /migrations/*.up.sql | sort); do
	echo "  -> $(basename "$f")"
	psql -v ON_ERROR_STOP=1 -f "$f"
done

echo "Provisioning read-only role pompote_viz_reader ..."
psql -v ON_ERROR_STOP=1 -v reader_password="${VIZ_READER_PASSWORD}" -f /db/grant-viz-reader.sql

echo "Local database ready (schema applied + read-only role granted)."
