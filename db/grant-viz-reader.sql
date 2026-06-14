-- Provision the read-only role the viz backend connects with (local stack, #5).
--
-- This is the local mirror of the GRANT performed on the OVH database: the viz
-- backend must only ever hold SELECT. No robot_rust schema lives in this repo —
-- the tables are created by robot_rust's bind-mounted migrations beforehand.
--
-- `:'reader_password'` is the psql-quoted value of the `-v reader_password=…`
-- passed by migrate.sh. Idempotent: safe to re-run against a persistent volume.

-- Create the LOGIN role only if it does not already exist (psql \gexec idiom;
-- variable substitution does not happen inside DO/$$ blocks, hence \gexec).
SELECT 'CREATE ROLE pompote_viz_reader LOGIN PASSWORD ' || :'reader_password'
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'pompote_viz_reader')
\gexec

-- Read-only access to the schema robot_rust's migrations populate. GRANT/ALTER
-- are idempotent, so re-running this script is harmless.
GRANT USAGE ON SCHEMA public TO pompote_viz_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO pompote_viz_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO pompote_viz_reader;
