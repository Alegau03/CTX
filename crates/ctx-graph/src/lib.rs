use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHit {
    pub id: i64,
    pub file_path: String,
    pub name: String,
    pub kind: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetHit {
    pub snippet_id: i64,
    pub file_path: String,
    pub symbol_name: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub message: String,
    pub root_cause: Option<String>,
}

pub struct GraphStore {
    conn: Connection,
}

impl GraphStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create graph parent dir {}", parent.display())
            })?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open sqlite db at {}", path.display()))?;

        Ok(Self { conn })
    }

    pub fn init_schema(&self) -> Result<()> {
        self.conn
            .execute_batch(include_str!("schema.sql"))
            .context("failed to initialize sqlite schema")
    }

    pub fn index_file(&self, path: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO files(path, updated_at) VALUES (?1, CURRENT_TIMESTAMP)
                 ON CONFLICT(path) DO UPDATE SET updated_at = CURRENT_TIMESTAMP",
                params![path],
            )
            .context("failed to index file")?;
        Ok(())
    }

    pub fn query_files(&self, term: &str) -> Result<Vec<String>> {
        let pattern = format!("%{}%", term);
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM files WHERE path LIKE ?1 ORDER BY path ASC")
            .context("failed to prepare query")?;

        let mut rows = stmt
            .query(params![pattern])
            .context("failed to query files")?;
        let mut out = Vec::new();

        while let Some(row) = rows.next().context("failed to read row")? {
            out.push(row.get::<_, String>(0).context("failed to decode path")?);
        }

        Ok(out)
    }

    pub fn upsert_symbol(
        &self,
        file_path: &str,
        name: &str,
        kind: &str,
        signature: &str,
    ) -> Result<i64> {
        self.index_file(file_path)?;
        let file_id = self
            .file_id(file_path)?
            .context("file id should exist after index_file")?;

        self.conn
            .execute(
                "INSERT INTO symbols(file_id, name, kind, signature, updated_at)
                 VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
                 ON CONFLICT(file_id, name, kind) DO UPDATE SET
                   signature = excluded.signature,
                   updated_at = CURRENT_TIMESTAMP",
                params![file_id, name, kind, signature],
            )
            .context("failed to upsert symbol")?;

        self.conn
            .query_row(
                "SELECT id FROM symbols WHERE file_id = ?1 AND name = ?2 AND kind = ?3",
                params![file_id, name, kind],
                |row| row.get::<_, i64>(0),
            )
            .context("failed to fetch upserted symbol id")
    }

    pub fn search_symbols(&self, term: &str) -> Result<Vec<SymbolHit>> {
        let pattern = format!("%{}%", term);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.id, f.path, s.name, s.kind, COALESCE(s.signature, '')
                 FROM symbols s
                 JOIN files f ON f.id = s.file_id
                 WHERE s.name LIKE ?1 OR s.signature LIKE ?1 OR f.path LIKE ?1
                 ORDER BY s.updated_at DESC, s.id DESC",
            )
            .context("failed to prepare search_symbols")?;

        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(SymbolHit {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    signature: row.get(4)?,
                })
            })
            .context("failed to run search_symbols")?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("failed to decode symbol row")?);
        }
        Ok(out)
    }

    pub fn link_symbols(
        &self,
        src_symbol_id: i64,
        dst_symbol_id: i64,
        edge_type: &str,
        metadata_json: Option<&str>,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO edges(src_symbol_id, dst_symbol_id, type, metadata_json)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(src_symbol_id, dst_symbol_id, type) DO UPDATE SET
                   metadata_json = excluded.metadata_json",
                params![src_symbol_id, dst_symbol_id, edge_type, metadata_json],
            )
            .context("failed to link symbols")?;
        Ok(())
    }

    pub fn related_symbols(&self, symbol_name: &str, limit: usize) -> Result<Vec<SymbolHit>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT dst.id, f.path, dst.name, dst.kind, COALESCE(dst.signature, '')
                 FROM symbols src
                 JOIN edges e ON e.src_symbol_id = src.id
                 JOIN symbols dst ON dst.id = e.dst_symbol_id
                 JOIN files f ON f.id = dst.file_id
                 WHERE src.name = ?1
                 LIMIT ?2",
            )
            .context("failed to prepare related_symbols")?;

        let rows = stmt
            .query_map(params![symbol_name, limit as i64], |row| {
                Ok(SymbolHit {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    signature: row.get(4)?,
                })
            })
            .context("failed to execute related_symbols")?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("failed to decode related symbol row")?);
        }

        Ok(out)
    }

    pub fn add_snippet(
        &self,
        file_path: &str,
        symbol_name: Option<&str>,
        content: &str,
    ) -> Result<i64> {
        self.index_file(file_path)?;
        let file_id = self
            .file_id(file_path)?
            .context("file id should exist after index_file")?;
        let symbol_id = if let Some(name) = symbol_name {
            self.conn
                .query_row(
                    "SELECT id FROM symbols WHERE file_id = ?1 AND name = ?2 LIMIT 1",
                    params![file_id, name],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .context("failed to fetch symbol id for snippet")?
        } else {
            None
        };

        self.conn
            .execute(
                "INSERT INTO snippets(file_id, symbol_id, content, created_at)
                 VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
                params![file_id, symbol_id, content],
            )
            .context("failed to insert snippet")?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn search_snippets(&self, term: &str, limit: usize) -> Result<Vec<SnippetHit>> {
        let escaped = term.replace('"', "\"");
        let query = if escaped.trim().is_empty() {
            "*".to_string()
        } else {
            escaped
        };

        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.id, f.path, sym.name, s.content
                 FROM snippets_fts fts
                 JOIN snippets s ON s.id = fts.rowid
                 JOIN files f ON f.id = s.file_id
                 LEFT JOIN symbols sym ON sym.id = s.symbol_id
                 WHERE snippets_fts MATCH ?1
                 LIMIT ?2",
            )
            .context("failed to prepare search_snippets")?;

        let rows = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(SnippetHit {
                    snippet_id: row.get(0)?,
                    file_path: row.get(1)?,
                    symbol_name: row.get(2)?,
                    content: row.get(3)?,
                })
            })
            .context("failed to query snippets fts")?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("failed to decode snippet row")?);
        }
        Ok(out)
    }

    pub fn record_run(&self, command: &str, status: &str) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO runs(command, status, created_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
                params![command, status],
            )
            .context("failed to insert run")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_failure(
        &self,
        run_id: i64,
        message: &str,
        root_cause: Option<&str>,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO failures(run_id, message, root_cause, created_at)
                 VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
                params![run_id, message, root_cause],
            )
            .context("failed to insert failure")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn recent_failures(&self, limit: usize) -> Result<Vec<FailureRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT message, root_cause
                 FROM failures
                 ORDER BY id DESC
                 LIMIT ?1",
            )
            .context("failed to prepare recent_failures")?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(FailureRecord {
                    message: row.get(0)?,
                    root_cause: row.get(1)?,
                })
            })
            .context("failed to query recent_failures")?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("failed to decode failure row")?);
        }
        Ok(out)
    }

    pub fn record_decision(&self, title: &str, summary: &str) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO tasks(title, summary, created_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
                params![title, summary],
            )
            .context("failed to insert task decision")?;
        let task_id = self.conn.last_insert_rowid();

        self.conn
            .execute(
                "INSERT INTO notes(task_id, body, created_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
                params![task_id, summary],
            )
            .context("failed to insert decision note")?;

        Ok(task_id)
    }

    pub fn recent_decisions(&self, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.title, n.body
                 FROM notes n
                 JOIN tasks t ON t.id = n.task_id
                 ORDER BY n.id DESC
                 LIMIT ?1",
            )
            .context("failed to prepare recent_decisions")?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let title: String = row.get(0)?;
                let body: String = row.get(1)?;
                Ok(format!("{title}: {body}"))
            })
            .context("failed to query recent_decisions")?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("failed to decode decision row")?);
        }
        Ok(out)
    }

    fn file_id(&self, file_path: &str) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM files WHERE path = ?1",
                params![file_path],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .context("failed to fetch file id")
    }
}
