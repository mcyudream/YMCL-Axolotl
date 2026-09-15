-- YMCL domain registry: one yda deployment is one domain. The personal
-- domain is virtual (id 'personal') and is never stored in ymcl_domains.
CREATE TABLE ymcl_domains (
    id TEXT PRIMARY KEY,
    origin TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL DEFAULT '',
    logo_url TEXT,
    capabilities_json TEXT,
    manifest_json TEXT,
    added_at INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE ymcl_state (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    active_domain_id TEXT NOT NULL DEFAULT 'personal'
);

INSERT INTO ymcl_state (id, active_domain_id) VALUES (0, 'personal');
