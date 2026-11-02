CREATE TABLE instance_schematic_sources (
	file_id TEXT PRIMARY KEY REFERENCES instance_files(id) ON DELETE CASCADE,
	source TEXT NOT NULL,
	project_id TEXT NOT NULL,
	file_name TEXT NOT NULL,
	sha1 TEXT NOT NULL
);
