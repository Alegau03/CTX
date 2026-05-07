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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDirective {
    pub id: i64,
    pub key: String,
    pub body: String,
    pub scope: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInsert {
    pub command: String,
    pub status: String,
    pub agent: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub original_tokens: Option<usize>,
    pub packed_tokens: Option<usize>,
    pub reduction_pct: Option<f64>,
    pub fallback_used: bool,
    pub pack_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: i64,
    pub command: String,
    pub status: String,
    pub agent: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub original_tokens: Option<usize>,
    pub packed_tokens: Option<usize>,
    pub reduction_pct: Option<f64>,
    pub fallback_used: bool,
    pub pack_path: Option<String>,
    pub created_at: String,
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
            .context("failed to initialize sqlite schema")?;
        self.migrate_runs_table()
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

    pub fn remove_file(&mut self, path: &str) -> Result<bool> {
        let Some(file_id) = self.file_id(path)? else {
            return Ok(false);
        };

        let tx = self
            .conn
            .transaction()
            .context("failed to start graph prune transaction")?;
        tx.execute(
            "DELETE FROM edges
             WHERE src_symbol_id IN (SELECT id FROM symbols WHERE file_id = ?1)
                OR dst_symbol_id IN (SELECT id FROM symbols WHERE file_id = ?1)",
            params![file_id],
        )
        .context("failed to delete file edges")?;
        tx.execute(
            "DELETE FROM embeddings_metadata
             WHERE snippet_id IN (SELECT id FROM snippets WHERE file_id = ?1)",
            params![file_id],
        )
        .context("failed to delete file embedding metadata")?;
        tx.execute("DELETE FROM snippets WHERE file_id = ?1", params![file_id])
            .context("failed to delete file snippets")?;
        tx.execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])
            .context("failed to delete file symbols")?;
        tx.execute("DELETE FROM files WHERE id = ?1", params![file_id])
            .context("failed to delete file row")?;
        tx.commit()
            .context("failed to commit graph prune transaction")?;
        Ok(true)
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

    pub fn find_symbols_by_exact_name(&self, name: &str, limit: usize) -> Result<Vec<SymbolHit>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT s.id, f.path, s.name, s.kind, COALESCE(s.signature, '')
                 FROM symbols s
                 JOIN files f ON f.id = s.file_id
                 WHERE s.name = ?1
                 ORDER BY s.updated_at DESC, s.id DESC
                 LIMIT ?2",
            )
            .context("failed to prepare find_symbols_by_exact_name")?;

        let rows = stmt
            .query_map(params![name, limit as i64], |row| {
                Ok(SymbolHit {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    name: row.get(2)?,
                    kind: row.get(3)?,
                    signature: row.get(4)?,
                })
            })
            .context("failed to run find_symbols_by_exact_name")?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("failed to decode exact symbol row")?);
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
        self.record_invocation_run(&RunInsert {
            command: command.to_string(),
            status: status.to_string(),
            agent: None,
            exit_code: None,
            duration_ms: None,
            original_tokens: None,
            packed_tokens: None,
            reduction_pct: None,
            fallback_used: false,
            pack_path: None,
        })
    }

    pub fn record_invocation_run(&self, run: &RunInsert) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO runs(
                   command, status, agent, exit_code, duration_ms, original_tokens,
                   packed_tokens, reduction_pct, fallback_used, pack_path, created_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, CURRENT_TIMESTAMP)",
                params![
                    run.command,
                    run.status,
                    run.agent,
                    run.exit_code,
                    run.duration_ms.map(|value| value as i64),
                    run.original_tokens.map(|value| value as i64),
                    run.packed_tokens.map(|value| value as i64),
                    run.reduction_pct,
                    if run.fallback_used { 1 } else { 0 },
                    run.pack_path,
                ],
            )
            .context("failed to insert run")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn recent_runs(&self, limit: usize) -> Result<Vec<RunRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, command, status, agent, exit_code, duration_ms, original_tokens,
                        packed_tokens, reduction_pct, fallback_used, pack_path, created_at
                 FROM runs
                 ORDER BY id DESC
                 LIMIT ?1",
            )
            .context("failed to prepare recent_runs")?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let duration_ms = row.get::<_, Option<i64>>(5)?.map(|value| value as u64);
                let original_tokens = row.get::<_, Option<i64>>(6)?.map(|value| value as usize);
                let packed_tokens = row.get::<_, Option<i64>>(7)?.map(|value| value as usize);
                let fallback_used = row.get::<_, i64>(9)? != 0;

                Ok(RunRecord {
                    id: row.get(0)?,
                    command: row.get(1)?,
                    status: row.get(2)?,
                    agent: row.get(3)?,
                    exit_code: row.get(4)?,
                    duration_ms,
                    original_tokens,
                    packed_tokens,
                    reduction_pct: row.get(8)?,
                    fallback_used,
                    pack_path: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })
            .context("failed to query recent_runs")?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.context("failed to decode run row")?);
        }
        Ok(out)
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

    pub fn upsert_memory_directive(
        &self,
        key: &str,
        body: &str,
        scope: &str,
        source: &str,
    ) -> Result<i64> {
        self.conn
            .execute(
                "INSERT INTO memory_directives(key, body, scope, source, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET
                   body = excluded.body,
                   scope = excluded.scope,
                   source = excluded.source,
                   updated_at = CURRENT_TIMESTAMP",
                params![key, body, scope, source],
            )
            .context("failed to upsert memory directive")?;

        self.conn
            .query_row(
                "SELECT id FROM memory_directives WHERE key = ?1",
                params![key],
                |row| row.get::<_, i64>(0),
            )
            .context("failed to fetch memory directive id")
    }

    pub fn get_memory_directive(&self, key: &str) -> Result<Option<MemoryDirective>> {
        self.conn
            .query_row(
                "SELECT id, key, body, scope, source, created_at, updated_at
                 FROM memory_directives
                 WHERE key = ?1",
                params![key],
                |row| {
                    Ok(MemoryDirective {
                        id: row.get(0)?,
                        key: row.get(1)?,
                        body: row.get(2)?,
                        scope: row.get(3)?,
                        source: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .context("failed to fetch memory directive by key")
    }

    pub fn list_memory_directives(
        &self,
        scope: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryDirective>> {
        let mut out = Vec::new();
        if let Some(scope_filter) = scope {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, key, body, scope, source, created_at, updated_at
                     FROM memory_directives
                     WHERE scope = ?1
                     ORDER BY updated_at DESC, id DESC
                     LIMIT ?2",
                )
                .context("failed to prepare scoped memory directives query")?;

            let rows = stmt
                .query_map(params![scope_filter, limit as i64], |row| {
                    Ok(MemoryDirective {
                        id: row.get(0)?,
                        key: row.get(1)?,
                        body: row.get(2)?,
                        scope: row.get(3)?,
                        source: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                })
                .context("failed to query scoped memory directives")?;

            for row in rows {
                out.push(row.context("failed to decode scoped memory directive row")?);
            }
            return Ok(out);
        }

        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, key, body, scope, source, created_at, updated_at
                 FROM memory_directives
                 ORDER BY updated_at DESC, id DESC
                 LIMIT ?1",
            )
            .context("failed to prepare memory directives query")?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(MemoryDirective {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    body: row.get(2)?,
                    scope: row.get(3)?,
                    source: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .context("failed to query memory directives")?;

        for row in rows {
            out.push(row.context("failed to decode memory directive row")?);
        }

        Ok(out)
    }

    pub fn search_memory_directives(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryDirective>> {
        let query = query.trim();
        if query.is_empty() {
            return self.list_memory_directives(None, limit);
        }

        let terms = query
            .split_whitespace()
            .filter(|t| t.len() >= 2)
            .map(|t| t.to_lowercase())
            .collect::<Vec<_>>();

        if terms.is_empty() {
            return self.list_memory_directives(None, limit);
        }

        let mut weighted = Vec::new();
        for directive in self.list_memory_directives(None, 500)? {
            let hay = format!(
                "{} {} {} {}",
                directive.key, directive.body, directive.scope, directive.source
            )
            .to_lowercase();
            let score = terms.iter().filter(|t| hay.contains(t.as_str())).count();
            if score > 0 {
                weighted.push((score, directive));
            }
        }

        weighted.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.updated_at.cmp(&a.1.updated_at))
                .then_with(|| b.1.id.cmp(&a.1.id))
        });

        Ok(weighted
            .into_iter()
            .take(limit)
            .map(|(_, directive)| directive)
            .collect())
    }

    pub fn delete_memory_directive(&self, key: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM memory_directives WHERE key = ?1", params![key])
            .context("failed to delete memory directive")?;
        Ok(affected > 0)
    }

    pub fn delete_memory_directives_by_prefix(&self, prefix: &str) -> Result<usize> {
        let pattern = format!("{prefix}.%");
        let affected = self
            .conn
            .execute(
                "DELETE FROM memory_directives WHERE key = ?1 OR key LIKE ?2",
                params![prefix, pattern],
            )
            .context("failed to delete memory directives by prefix")?;
        Ok(affected)
    }

    fn migrate_runs_table(&self) -> Result<()> {
        self.ensure_column("runs", "agent", "ALTER TABLE runs ADD COLUMN agent TEXT")?;
        self.ensure_column(
            "runs",
            "exit_code",
            "ALTER TABLE runs ADD COLUMN exit_code INTEGER",
        )?;
        self.ensure_column(
            "runs",
            "duration_ms",
            "ALTER TABLE runs ADD COLUMN duration_ms INTEGER",
        )?;
        self.ensure_column(
            "runs",
            "original_tokens",
            "ALTER TABLE runs ADD COLUMN original_tokens INTEGER",
        )?;
        self.ensure_column(
            "runs",
            "packed_tokens",
            "ALTER TABLE runs ADD COLUMN packed_tokens INTEGER",
        )?;
        self.ensure_column(
            "runs",
            "reduction_pct",
            "ALTER TABLE runs ADD COLUMN reduction_pct REAL",
        )?;
        self.ensure_column(
            "runs",
            "fallback_used",
            "ALTER TABLE runs ADD COLUMN fallback_used INTEGER NOT NULL DEFAULT 0",
        )?;
        self.ensure_column(
            "runs",
            "pack_path",
            "ALTER TABLE runs ADD COLUMN pack_path TEXT",
        )?;
        self.conn
            .execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_runs_agent_created_at ON runs(agent, created_at);",
            )
            .context("failed to create runs agent index")?;
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, ddl: &str) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .context("failed to inspect table columns")?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .context("failed to query table columns")?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        if !columns.iter().any(|existing| existing == column) {
            self.conn
                .execute_batch(ddl)
                .with_context(|| format!("failed to add column {table}.{column}"))?;
        }
        Ok(())
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
