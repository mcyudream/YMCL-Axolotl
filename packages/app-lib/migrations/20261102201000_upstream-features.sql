ALTER TABLE settings ADD COLUMN default_game_language TEXT NOT NULL DEFAULT '';
ALTER TABLE settings ADD COLUMN backup_repository_path TEXT;

CREATE TABLE pending_backup_repository_cleanups (
	path TEXT PRIMARY KEY,
	created_at INTEGER NOT NULL
);
