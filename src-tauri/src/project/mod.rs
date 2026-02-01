//! Project and Artifact storage using SQLite.
//!
//! Projects are goal-oriented collections of artifacts.
//! Artifacts are the persistent outputs of AI work (markdown documents, commits, etc.).

mod store;
mod types;

use std::sync::OnceLock;

use tauri::{AppHandle, Manager};

// Re-export types
pub use store::ProjectStore;
pub use types::{Artifact, ArtifactData, ArtifactType, Project, Session};

// =============================================================================
// Error type
// =============================================================================

#[derive(Debug)]
pub struct ProjectError(pub String);

impl ProjectError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ProjectError {}

impl From<rusqlite::Error> for ProjectError {
    fn from(e: rusqlite::Error) -> Self {
        ProjectError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ProjectError>;

// =============================================================================
// Global store
// =============================================================================

/// Global store instance - initialized during app setup.
static STORE: OnceLock<std::result::Result<ProjectStore, String>> = OnceLock::new();

/// Initialize the global store with the app's data directory.
/// Call this once during Tauri app setup.
pub fn init_store(app_handle: &AppHandle) -> Result<()> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| ProjectError::new(format!("Cannot get app data dir: {}", e)))?;

    let db_path = app_data_dir.join("projects.db");

    STORE.get_or_init(|| ProjectStore::open(db_path).map_err(|e| e.0));

    // Check if initialization succeeded
    get_store()?;
    Ok(())
}

/// Get the global store. Must call init_store first during app setup.
pub fn get_store() -> Result<&'static ProjectStore> {
    let result = STORE
        .get()
        .ok_or_else(|| ProjectError::new("Project store not initialized"))?;

    match result {
        Ok(store) => Ok(store),
        Err(msg) => Err(ProjectError::new(msg.clone())),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_get_project() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ProjectStore::open(db_path).unwrap();

        let project = Project::new("Test Project");

        store.create_project(&project).unwrap();
        let retrieved = store.get_project(&project.id).unwrap().unwrap();

        assert_eq!(retrieved.id, project.id);
        assert_eq!(retrieved.name, "Test Project");
    }

    #[test]
    fn test_list_projects() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ProjectStore::open(db_path).unwrap();

        let p1 = Project::new("Project 1");
        let p2 = Project::new("Project 2");

        store.create_project(&p1).unwrap();
        store.create_project(&p2).unwrap();

        let projects = store.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_create_and_get_artifact() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ProjectStore::open(db_path).unwrap();

        let project = Project::new("Test Project");
        store.create_project(&project).unwrap();

        let artifact = Artifact::new_markdown(&project.id, "Test Artifact", "# Hello\n\nWorld");
        store.create_artifact(&artifact).unwrap();

        let retrieved = store.get_artifact(&artifact.id).unwrap().unwrap();
        assert_eq!(retrieved.id, artifact.id);
        assert_eq!(retrieved.title, "Test Artifact");

        if let ArtifactData::Markdown { content } = &retrieved.data {
            assert_eq!(content, "# Hello\n\nWorld");
        } else {
            panic!("Expected markdown artifact");
        }
    }

    #[test]
    fn test_update_artifact() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ProjectStore::open(db_path).unwrap();

        let project = Project::new("Test Project");
        store.create_project(&project).unwrap();

        let artifact = Artifact::new_markdown(&project.id, "Original", "Content");
        store.create_artifact(&artifact).unwrap();

        let new_data = ArtifactData::Markdown {
            content: "Updated content".to_string(),
        };
        store
            .update_artifact(&artifact.id, Some("Updated Title"), Some(&new_data))
            .unwrap();

        let retrieved = store.get_artifact(&artifact.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated Title");

        if let ArtifactData::Markdown { content } = &retrieved.data {
            assert_eq!(content, "Updated content");
        } else {
            panic!("Expected markdown artifact");
        }
    }

    #[test]
    fn test_delete_project_cascades() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ProjectStore::open(db_path).unwrap();

        let project = Project::new("Test Project");
        store.create_project(&project).unwrap();

        let artifact = Artifact::new_markdown(&project.id, "Test", "Content");
        store.create_artifact(&artifact).unwrap();

        store.delete_project(&project.id).unwrap();

        // Artifact should be deleted too
        let retrieved = store.get_artifact(&artifact.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_context_links() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ProjectStore::open(db_path).unwrap();

        let project = Project::new("Test Project");
        store.create_project(&project).unwrap();

        let a1 = Artifact::new_markdown(&project.id, "Research", "Research content");
        let a2 = Artifact::new_markdown(&project.id, "Plan", "Plan based on research");

        store.create_artifact(&a1).unwrap();
        store.create_artifact(&a2).unwrap();

        // a2 was created using a1 as context
        store.add_context(&a2.id, &a1.id).unwrap();

        let context = store.get_context_artifacts(&a2.id).unwrap();
        assert_eq!(context, vec![a1.id.clone()]);

        let dependents = store.get_dependent_artifacts(&a1.id).unwrap();
        assert_eq!(dependents, vec![a2.id.clone()]);
    }

    #[test]
    fn test_sessions() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = ProjectStore::open(db_path).unwrap();

        let project = Project::new("Test Project");
        store.create_project(&project).unwrap();

        let artifact = Artifact::new_markdown(&project.id, "Test", "Content");
        store.create_artifact(&artifact).unwrap();

        let session = Session::new(&artifact.id, "User: Create a plan\nAI: Here's your plan...");
        store.create_session(&session).unwrap();

        let sessions = store.get_sessions(&artifact.id).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session.id);
    }
}
