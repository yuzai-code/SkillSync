// SQLite state database for tracking installations and sync history
// Implements: task 2.5

use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

// ---------------------------------------------------------------------------
// Data structs
// ---------------------------------------------------------------------------

/// A row from the `installed_resources` table.
#[derive(Debug, Clone)]
pub struct InstalledResource {
    pub id: i64,
    pub name: String,
    pub resource_type: String,
    pub scope: String,
    pub version: String,
    pub content_hash: Option<String>,
    pub installed_path: String,
    pub project_path: Option<String>,
    pub installed_at: String,
    pub updated_at: String,
}

/// A row from the `sync_history` table.
#[derive(Debug, Clone)]
pub struct SyncRecord {
    pub id: i64,
    pub operation: String,
    pub status: String,
    pub summary: Option<String>,
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// StateDb
// ---------------------------------------------------------------------------

/// Wraps a `rusqlite::Connection` and provides typed helpers for the
/// SkillSync state database (installed resources + sync history).
pub struct StateDb {
    conn: Connection,
}

impl StateDb {
    /// Open (or create) the SQLite database at `path` and run migrations.
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directories for state DB: {}",
                    path.display()
                )
            })?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open state database: {}", path.display()))?;

        // Enable WAL mode for better concurrency.
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("Failed to set WAL journal mode")?;

        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    // -- Migrations ----------------------------------------------------------

    fn run_migrations(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS installed_resources (
                    id             INTEGER PRIMARY KEY,
                    name           TEXT NOT NULL,
                    resource_type  TEXT NOT NULL,
                    scope          TEXT NOT NULL,
                    version        TEXT NOT NULL,
                    content_hash   TEXT,
                    installed_path TEXT NOT NULL,
                    project_path   TEXT,
                    installed_at   TEXT NOT NULL,
                    updated_at     TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS sync_history (
                    id        INTEGER PRIMARY KEY,
                    operation TEXT NOT NULL,
                    status    TEXT NOT NULL,
                    summary   TEXT,
                    timestamp TEXT NOT NULL
                );
                ",
            )
            .context("Failed to run state database migrations")?;
        Ok(())
    }

    // -- installed_resources CRUD -------------------------------------------

    /// Record (insert or update) an installed resource.
    ///
    /// If a resource with the same `name` and `project_path` already exists it
    /// is updated in place; otherwise a new row is inserted.
    pub fn record_install(
        &self,
        name: &str,
        resource_type: &str,
        scope: &str,
        version: &str,
        content_hash: Option<&str>,
        installed_path: &str,
        project_path: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // Check whether this resource already exists.
        let existing_id: Option<i64> = match project_path {
            Some(pp) => self
                .conn
                .query_row(
                    "SELECT id FROM installed_resources WHERE name = ?1 AND project_path = ?2",
                    params![name, pp],
                    |row| row.get(0),
                )
                .optional()
                .context("Failed to query existing installed resource")?,
            None => self
                .conn
                .query_row(
                    "SELECT id FROM installed_resources WHERE name = ?1 AND project_path IS NULL",
                    params![name],
                    |row| row.get(0),
                )
                .optional()
                .context("Failed to query existing installed resource")?,
        };

        if let Some(id) = existing_id {
            self.conn
                .execute(
                    "UPDATE installed_resources
                     SET resource_type = ?1, scope = ?2, version = ?3,
                         content_hash = ?4, installed_path = ?5, updated_at = ?6
                     WHERE id = ?7",
                    params![resource_type, scope, version, content_hash, installed_path, now, id],
                )
                .context("Failed to update installed resource")?;
        } else {
            self.conn
                .execute(
                    "INSERT INTO installed_resources
                        (name, resource_type, scope, version, content_hash,
                         installed_path, project_path, installed_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        name,
                        resource_type,
                        scope,
                        version,
                        content_hash,
                        installed_path,
                        project_path,
                        now,
                        now,
                    ],
                )
                .context("Failed to insert installed resource")?;
        }

        Ok(())
    }

    /// Look up a single installed resource by name and optional project path.
    pub fn get_installed(
        &self,
        name: &str,
        project_path: Option<&str>,
    ) -> Result<Option<InstalledResource>> {
        let row = match project_path {
            Some(pp) => self
                .conn
                .query_row(
                    "SELECT id, name, resource_type, scope, version, content_hash,
                            installed_path, project_path, installed_at, updated_at
                     FROM installed_resources
                     WHERE name = ?1 AND project_path = ?2",
                    params![name, pp],
                    row_to_installed_resource,
                )
                .optional()
                .context("Failed to query installed resource")?,
            None => self
                .conn
                .query_row(
                    "SELECT id, name, resource_type, scope, version, content_hash,
                            installed_path, project_path, installed_at, updated_at
                     FROM installed_resources
                     WHERE name = ?1 AND project_path IS NULL",
                    params![name],
                    row_to_installed_resource,
                )
                .optional()
                .context("Failed to query installed resource")?,
        };
        Ok(row)
    }

    /// List all installed resources, optionally filtered to a specific project.
    ///
    /// Pass `None` to list global (project_path IS NULL) resources.
    /// Pass `Some(path)` to list resources installed for that project.
    pub fn list_installed(
        &self,
        project_path: Option<&str>,
    ) -> Result<Vec<InstalledResource>> {
        let mut stmt;
        let rows: Vec<InstalledResource> = match project_path {
            Some(pp) => {
                stmt = self.conn.prepare(
                    "SELECT id, name, resource_type, scope, version, content_hash,
                            installed_path, project_path, installed_at, updated_at
                     FROM installed_resources
                     WHERE project_path = ?1
                     ORDER BY name",
                )?;
                stmt.query_map(params![pp], row_to_installed_resource)
                    .context("Failed to list installed resources")?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .context("Failed to collect installed resources")?
            }
            None => {
                stmt = self.conn.prepare(
                    "SELECT id, name, resource_type, scope, version, content_hash,
                            installed_path, project_path, installed_at, updated_at
                     FROM installed_resources
                     WHERE project_path IS NULL
                     ORDER BY name",
                )?;
                stmt.query_map([], row_to_installed_resource)
                    .context("Failed to list installed resources")?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .context("Failed to collect installed resources")?
            }
        };
        Ok(rows)
    }

    /// Remove an installed resource by name and optional project path.
    pub fn remove_installed(
        &self,
        name: &str,
        project_path: Option<&str>,
    ) -> Result<()> {
        match project_path {
            Some(pp) => {
                self.conn
                    .execute(
                        "DELETE FROM installed_resources WHERE name = ?1 AND project_path = ?2",
                        params![name, pp],
                    )
                    .context("Failed to remove installed resource")?;
            }
            None => {
                self.conn
                    .execute(
                        "DELETE FROM installed_resources WHERE name = ?1 AND project_path IS NULL",
                        params![name],
                    )
                    .context("Failed to remove installed resource")?;
            }
        }
        Ok(())
    }

    // -- sync_history --------------------------------------------------------

    /// Record a sync operation.
    pub fn record_sync(
        &self,
        operation: &str,
        status: &str,
        summary: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO sync_history (operation, status, summary, timestamp)
                 VALUES (?1, ?2, ?3, ?4)",
                params![operation, status, summary, now],
            )
            .context("Failed to record sync operation")?;
        Ok(())
    }

    /// Return the most recent sync records, newest first.
    pub fn recent_syncs(&self, limit: usize) -> Result<Vec<SyncRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, operation, status, summary, timestamp
                 FROM sync_history
                 ORDER BY id DESC
                 LIMIT ?1",
            )
            .context("Failed to prepare recent syncs query")?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(SyncRecord {
                    id: row.get(0)?,
                    operation: row.get(1)?,
                    status: row.get(2)?,
                    summary: row.get(3)?,
                    timestamp: row.get(4)?,
                })
            })
            .context("Failed to query recent syncs")?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Failed to collect sync records")?;

        Ok(rows)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a `rusqlite::Row` to an `InstalledResource`.
fn row_to_installed_resource(row: &rusqlite::Row<'_>) -> rusqlite::Result<InstalledResource> {
    Ok(InstalledResource {
        id: row.get(0)?,
        name: row.get(1)?,
        resource_type: row.get(2)?,
        scope: row.get(3)?,
        version: row.get(4)?,
        content_hash: row.get(5)?,
        installed_path: row.get(6)?,
        project_path: row.get(7)?,
        installed_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: create a StateDb backed by a temp directory.
    fn temp_db() -> (StateDb, TempDir) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("state.db");
        let db = StateDb::open(&db_path).unwrap();
        (db, dir)
    }

    #[test]
    fn test_open_creates_tables() {
        let (db, _dir) = temp_db();
        // Tables should exist — querying them should not error.
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM installed_resources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM sync_history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_record_and_get_installed_global() {
        let (db, _dir) = temp_db();

        db.record_install(
            "yuque",
            "skill",
            "global",
            "1.0.4",
            Some("abc123"),
            "/home/user/.claude/skills/yuque",
            None,
        )
        .unwrap();

        let res = db.get_installed("yuque", None).unwrap().unwrap();
        assert_eq!(res.name, "yuque");
        assert_eq!(res.resource_type, "skill");
        assert_eq!(res.scope, "global");
        assert_eq!(res.version, "1.0.4");
        assert_eq!(res.content_hash.as_deref(), Some("abc123"));
        assert_eq!(res.installed_path, "/home/user/.claude/skills/yuque");
        assert!(res.project_path.is_none());
    }

    #[test]
    fn test_record_and_get_installed_project() {
        let (db, _dir) = temp_db();

        db.record_install(
            "openspec",
            "mcp_server",
            "shared",
            "1.0.0",
            None,
            "/project/.mcp.json",
            Some("/project"),
        )
        .unwrap();

        // Look up with project_path
        let res = db.get_installed("openspec", Some("/project")).unwrap().unwrap();
        assert_eq!(res.name, "openspec");
        assert_eq!(res.project_path.as_deref(), Some("/project"));

        // Should not appear in global scope
        let none = db.get_installed("openspec", None).unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn test_record_install_upsert() {
        let (db, _dir) = temp_db();

        db.record_install("yuque", "skill", "global", "1.0.0", None, "/path/a", None)
            .unwrap();
        db.record_install("yuque", "skill", "global", "2.0.0", Some("newhash"), "/path/b", None)
            .unwrap();

        // Should still be exactly one row
        let all = db.list_installed(None).unwrap();
        assert_eq!(all.len(), 1);

        let res = db.get_installed("yuque", None).unwrap().unwrap();
        assert_eq!(res.version, "2.0.0");
        assert_eq!(res.content_hash.as_deref(), Some("newhash"));
        assert_eq!(res.installed_path, "/path/b");
        // installed_at should stay the same, updated_at should change
        assert_ne!(res.installed_at, res.updated_at);
    }

    #[test]
    fn test_list_installed() {
        let (db, _dir) = temp_db();

        db.record_install("a", "skill", "global", "1.0", None, "/a", None).unwrap();
        db.record_install("b", "plugin", "global", "2.0", None, "/b", None).unwrap();
        db.record_install("c", "mcp_server", "shared", "1.0", None, "/c", Some("/project"))
            .unwrap();

        let globals = db.list_installed(None).unwrap();
        assert_eq!(globals.len(), 2);
        // Sorted by name
        assert_eq!(globals[0].name, "a");
        assert_eq!(globals[1].name, "b");

        let project = db.list_installed(Some("/project")).unwrap();
        assert_eq!(project.len(), 1);
        assert_eq!(project[0].name, "c");
    }

    #[test]
    fn test_remove_installed() {
        let (db, _dir) = temp_db();

        db.record_install("yuque", "skill", "global", "1.0", None, "/a", None).unwrap();
        assert!(db.get_installed("yuque", None).unwrap().is_some());

        db.remove_installed("yuque", None).unwrap();
        assert!(db.get_installed("yuque", None).unwrap().is_none());
    }

    #[test]
    fn test_remove_installed_project_scoped() {
        let (db, _dir) = temp_db();

        db.record_install("x", "skill", "shared", "1.0", None, "/a", Some("/proj")).unwrap();
        db.record_install("x", "skill", "global", "1.0", None, "/b", None).unwrap();

        db.remove_installed("x", Some("/proj")).unwrap();

        // Project-scoped should be gone
        assert!(db.get_installed("x", Some("/proj")).unwrap().is_none());
        // Global should still exist
        assert!(db.get_installed("x", None).unwrap().is_some());
    }

    #[test]
    fn test_record_and_recent_syncs() {
        let (db, _dir) = temp_db();

        db.record_sync("pull", "success", Some("Pulled 3 resources")).unwrap();
        db.record_sync("push", "success", None).unwrap();
        db.record_sync("sync", "conflict", Some("Conflict in yuque")).unwrap();

        let syncs = db.recent_syncs(2).unwrap();
        assert_eq!(syncs.len(), 2);
        // Newest first
        assert_eq!(syncs[0].operation, "sync");
        assert_eq!(syncs[0].status, "conflict");
        assert_eq!(syncs[1].operation, "push");
    }

    #[test]
    fn test_recent_syncs_empty() {
        let (db, _dir) = temp_db();
        let syncs = db.recent_syncs(10).unwrap();
        assert!(syncs.is_empty());
    }

    #[test]
    fn test_open_idempotent() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("state.db");

        // Open twice — should not fail on CREATE TABLE IF NOT EXISTS.
        let db1 = StateDb::open(&db_path).unwrap();
        db1.record_install("a", "skill", "global", "1.0", None, "/a", None)
            .unwrap();
        drop(db1);

        let db2 = StateDb::open(&db_path).unwrap();
        let res = db2.get_installed("a", None).unwrap();
        assert!(res.is_some());
    }
}
