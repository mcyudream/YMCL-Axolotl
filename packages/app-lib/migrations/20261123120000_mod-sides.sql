-- Manual rules are independent of installed file names and content refreshes.
-- An empty scope is the local, cross-instance rule list.
CREATE TABLE mod_side_overrides (
    scope TEXT NOT NULL,
    identity TEXT NOT NULL,
    side TEXT NOT NULL CHECK (side IN ('client', 'server', 'both', 'unknown')),
    PRIMARY KEY (scope, identity)
);

-- Keep provider detections available to cache-only/offline content snapshots.
CREATE TABLE mod_side_detections (
    sha1 TEXT PRIMARY KEY NOT NULL,
    metadata TEXT NOT NULL
);
