-- Per-domain YAP sessions. Tokens are stored server-side (app database),
-- mirroring the Modrinth credentials storage; never across domains.
CREATE TABLE ymcl_sessions (
    domain_id TEXT PRIMARY KEY,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    expires_at INTEGER,
    session_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
