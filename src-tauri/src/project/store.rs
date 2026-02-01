//! SQLite storage for projects, artifacts, and sessions.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use super::types::{Artifact, ArtifactData, ArtifactType, Project, Session};
use super::{ProjectError, Result};

/// Project storage backed by SQLite.
pub struct ProjectStore {
    conn: Mutex<Connection>,
}

impl ProjectStore {
    /// Open or create the project database at the given path.
    pub fn open(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ProjectError(format!("Cannot create directory: {}", e)))?;
        }

        let conn = Connection::open(&db_path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Initialize the database schema.
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            -- Projects table
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Artifacts table
            CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                data_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                parent_artifact_id TEXT,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL
            );

            -- Context links (which artifacts were used as context when creating another)
            CREATE TABLE IF NOT EXISTS artifact_context (
                artifact_id TEXT NOT NULL,
                context_artifact_id TEXT NOT NULL,
                PRIMARY KEY (artifact_id, context_artifact_id),
                FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE,
                FOREIGN KEY (context_artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
            );

            -- Sessions (AI conversations that produced artifacts)
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                artifact_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                transcript TEXT NOT NULL,
                FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
            );

            -- Indexes for fast lookups
            CREATE INDEX IF NOT EXISTS idx_artifacts_project ON artifacts(project_id);
            CREATE INDEX IF NOT EXISTS idx_artifacts_type ON artifacts(artifact_type);
            CREATE INDEX IF NOT EXISTS idx_sessions_artifact ON sessions(artifact_id);

            PRAGMA foreign_keys = ON;
            "#,
        )?;

        Ok(())
    }

    // =========================================================================
    // Project operations
    // =========================================================================

    /// Create a new project.
    pub fn create_project(&self, project: &Project) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                &project.id,
                &project.name,
                &project.created_at,
                &project.updated_at
            ],
        )?;
        Ok(())
    }

    /// Get a project by ID.
    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, created_at, updated_at FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    /// List all projects, ordered by most recently updated.
    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, created_at, updated_at FROM projects ORDER BY updated_at DESC")?;
        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    /// Update a project's name.
    pub fn update_project(&self, id: &str, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE projects SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, now, id],
        )?;
        Ok(())
    }

    /// Delete a project and all its artifacts.
    pub fn delete_project(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    // =========================================================================
    // Artifact operations
    // =========================================================================

    /// Create a new artifact.
    pub fn create_artifact(&self, artifact: &Artifact) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let data_json =
            serde_json::to_string(&artifact.data).map_err(|e| ProjectError::new(e.to_string()))?;

        conn.execute(
            "INSERT INTO artifacts (id, project_id, title, artifact_type, data_json, created_at, updated_at, parent_artifact_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &artifact.id,
                &artifact.project_id,
                &artifact.title,
                artifact.artifact_type().as_str(),
                data_json,
                &artifact.created_at,
                &artifact.updated_at,
                &artifact.parent_artifact_id,
            ],
        )?;

        // Update the project's updated_at timestamp
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![now, &artifact.project_id],
        )?;

        Ok(())
    }

    /// Get an artifact by ID.
    pub fn get_artifact(&self, id: &str) -> Result<Option<Artifact>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, project_id, title, data_json, created_at, updated_at, parent_artifact_id
             FROM artifacts WHERE id = ?1",
            params![id],
            |row| {
                let data_json: String = row.get(3)?;
                let data: ArtifactData = serde_json::from_str(&data_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(Artifact {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    parent_artifact_id: row.get(6)?,
                    data,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    /// List artifacts in a project, ordered by most recently updated.
    pub fn list_artifacts(&self, project_id: &str) -> Result<Vec<Artifact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, data_json, created_at, updated_at, parent_artifact_id
             FROM artifacts WHERE project_id = ?1 ORDER BY updated_at DESC",
        )?;
        let artifacts = stmt
            .query_map(params![project_id], |row| {
                let data_json: String = row.get(3)?;
                let data: ArtifactData = serde_json::from_str(&data_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(Artifact {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    parent_artifact_id: row.get(6)?,
                    data,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(artifacts)
    }

    /// List artifacts by type in a project.
    #[allow(dead_code)]
    pub fn list_artifacts_by_type(
        &self,
        project_id: &str,
        artifact_type: ArtifactType,
    ) -> Result<Vec<Artifact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, data_json, created_at, updated_at, parent_artifact_id
             FROM artifacts WHERE project_id = ?1 AND artifact_type = ?2 ORDER BY updated_at DESC",
        )?;
        let artifacts = stmt
            .query_map(params![project_id, artifact_type.as_str()], |row| {
                let data_json: String = row.get(3)?;
                let data: ArtifactData = serde_json::from_str(&data_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(Artifact {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    parent_artifact_id: row.get(6)?,
                    data,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(artifacts)
    }

    /// Update an artifact's title and/or content.
    pub fn update_artifact(
        &self,
        id: &str,
        title: Option<&str>,
        data: Option<&ArtifactData>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        match (title, data) {
            (Some(title), Some(data)) => {
                let data_json =
                    serde_json::to_string(data).map_err(|e| ProjectError::new(e.to_string()))?;
                conn.execute(
                    "UPDATE artifacts SET title = ?1, data_json = ?2, artifact_type = ?3, updated_at = ?4 WHERE id = ?5",
                    params![title, data_json, data.artifact_type().as_str(), now, id],
                )?;
            }
            (Some(title), None) => {
                conn.execute(
                    "UPDATE artifacts SET title = ?1, updated_at = ?2 WHERE id = ?3",
                    params![title, now, id],
                )?;
            }
            (None, Some(data)) => {
                let data_json =
                    serde_json::to_string(data).map_err(|e| ProjectError::new(e.to_string()))?;
                conn.execute(
                    "UPDATE artifacts SET data_json = ?1, artifact_type = ?2, updated_at = ?3 WHERE id = ?4",
                    params![data_json, data.artifact_type().as_str(), now, id],
                )?;
            }
            (None, None) => {
                // Nothing to update
                return Ok(());
            }
        }

        // Update the project's updated_at timestamp
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = (SELECT project_id FROM artifacts WHERE id = ?2)",
            params![now, id],
        )?;

        Ok(())
    }

    /// Delete an artifact.
    pub fn delete_artifact(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM artifacts WHERE id = ?1", params![id])?;
        Ok(())
    }

    // =========================================================================
    // Context operations
    // =========================================================================

    /// Add a context link (artifact X was created using artifact Y as context).
    pub fn add_context(&self, artifact_id: &str, context_artifact_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO artifact_context (artifact_id, context_artifact_id) VALUES (?1, ?2)",
            params![artifact_id, context_artifact_id],
        )?;
        Ok(())
    }

    /// Get the artifacts that were used as context when creating an artifact.
    pub fn get_context_artifacts(&self, artifact_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT context_artifact_id FROM artifact_context WHERE artifact_id = ?1")?;
        let ids = stmt
            .query_map(params![artifact_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    /// Get the artifacts that use this artifact as context.
    #[allow(dead_code)]
    pub fn get_dependent_artifacts(&self, context_artifact_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT artifact_id FROM artifact_context WHERE context_artifact_id = ?1")?;
        let ids = stmt
            .query_map(params![context_artifact_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    // =========================================================================
    // Session operations
    // =========================================================================

    /// Create a new session.
    pub fn create_session(&self, session: &Session) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, artifact_id, created_at, transcript) VALUES (?1, ?2, ?3, ?4)",
            params![
                &session.id,
                &session.artifact_id,
                &session.created_at,
                &session.transcript,
            ],
        )?;
        Ok(())
    }

    /// Get sessions for an artifact, ordered by most recent first.
    pub fn get_sessions(&self, artifact_id: &str) -> Result<Vec<Session>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, artifact_id, created_at, transcript FROM sessions WHERE artifact_id = ?1 ORDER BY created_at DESC",
        )?;
        let sessions = stmt
            .query_map(params![artifact_id], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    artifact_id: row.get(1)?,
                    created_at: row.get(2)?,
                    transcript: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(sessions)
    }
}
