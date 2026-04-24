PRAGMA journal_mode=WAL;

CREATE TABLE IF NOT EXISTS files (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS symbols (
  id INTEGER PRIMARY KEY,
  file_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  signature TEXT,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(file_id, name, kind),
  FOREIGN KEY(file_id) REFERENCES files(id)
);

CREATE TABLE IF NOT EXISTS edges (
  id INTEGER PRIMARY KEY,
  src_symbol_id INTEGER,
  dst_symbol_id INTEGER,
  type TEXT NOT NULL,
  metadata_json TEXT,
  UNIQUE(src_symbol_id, dst_symbol_id, type)
);

CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  summary TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS runs (
  id INTEGER PRIMARY KEY,
  task_id INTEGER,
  command TEXT NOT NULL,
  status TEXT NOT NULL,
  agent TEXT,
  exit_code INTEGER,
  duration_ms INTEGER,
  original_tokens INTEGER,
  packed_tokens INTEGER,
  reduction_pct REAL,
  fallback_used INTEGER NOT NULL DEFAULT 0,
  pack_path TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS failures (
  id INTEGER PRIMARY KEY,
  run_id INTEGER,
  message TEXT NOT NULL,
  root_cause TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS notes (
  id INTEGER PRIMARY KEY,
  task_id INTEGER,
  body TEXT NOT NULL,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS memory_directives (
  id INTEGER PRIMARY KEY,
  key TEXT NOT NULL UNIQUE,
  body TEXT NOT NULL,
  scope TEXT NOT NULL DEFAULT 'project',
  source TEXT NOT NULL DEFAULT 'manual',
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS snippets (
  id INTEGER PRIMARY KEY,
  file_id INTEGER,
  symbol_id INTEGER,
  content TEXT NOT NULL,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS embeddings_metadata (
  id INTEGER PRIMARY KEY,
  snippet_id INTEGER,
  model TEXT NOT NULL,
  vector_dim INTEGER,
  checksum TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE VIRTUAL TABLE IF NOT EXISTS snippets_fts USING fts5(
  content,
  content='snippets',
  content_rowid='id'
);

CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_edges_src ON edges(src_symbol_id);
CREATE INDEX IF NOT EXISTS idx_edges_dst ON edges(dst_symbol_id);
CREATE INDEX IF NOT EXISTS idx_failures_created_at ON failures(created_at);
CREATE INDEX IF NOT EXISTS idx_notes_created_at ON notes(created_at);
CREATE INDEX IF NOT EXISTS idx_memory_scope_updated_at ON memory_directives(scope, updated_at);
CREATE INDEX IF NOT EXISTS idx_runs_created_at ON runs(created_at);

CREATE TRIGGER IF NOT EXISTS snippets_ai AFTER INSERT ON snippets BEGIN
  INSERT INTO snippets_fts(rowid, content) VALUES (new.id, new.content);
END;

CREATE TRIGGER IF NOT EXISTS snippets_ad AFTER DELETE ON snippets BEGIN
  INSERT INTO snippets_fts(snippets_fts, rowid, content) VALUES('delete', old.id, old.content);
END;

CREATE TRIGGER IF NOT EXISTS snippets_au AFTER UPDATE ON snippets BEGIN
  INSERT INTO snippets_fts(snippets_fts, rowid, content) VALUES('delete', old.id, old.content);
  INSERT INTO snippets_fts(rowid, content) VALUES (new.id, new.content);
END;
