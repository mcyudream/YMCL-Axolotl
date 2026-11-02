CREATE TABLE instance_content_refreshes (
	instance_id TEXT PRIMARY KEY REFERENCES instances(id) ON DELETE CASCADE,
	id TEXT NOT NULL UNIQUE,
	state TEXT NOT NULL,
	total INTEGER NOT NULL DEFAULT 0,
	processed INTEGER NOT NULL DEFAULT 0,
	failed INTEGER NOT NULL DEFAULT 0,
	error TEXT,
	updated_at INTEGER NOT NULL
);
CREATE TABLE instance_content_refresh_files (
	instance_id TEXT NOT NULL REFERENCES instance_content_refreshes(instance_id) ON DELETE CASCADE,
	relative_path TEXT NOT NULL,
	state TEXT NOT NULL,
	error TEXT,
	PRIMARY KEY(instance_id, relative_path)
);
