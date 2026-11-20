use crate::project::{spawn_project_load, Project, ProjectSaveWorker};
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

/// Owns the active project's loading/saving lifecycle and edit revision.
pub(super) struct ProjectSession {
    pub(super) load_rx: Option<Receiver<Result<crate::project::ProjectLoadResult, String>>>,
    pub(super) save_worker: ProjectSaveWorker,
    pub(super) name: String,
    pub(super) pending_path: Option<String>,
    pub(super) saves_in_flight: HashSet<(String, u64)>,
    pub(super) dirty: bool,
    pub(super) revision: u64,
}

impl ProjectSession {
    pub(super) fn new(project_file: Option<String>) -> Self {
        let project_path = project_file.unwrap_or_else(Project::default_project_file);
        Self {
            load_rx: Some(spawn_project_load(project_path, true)),
            save_worker: ProjectSaveWorker::new(),
            name: String::new(),
            pending_path: None,
            saves_in_flight: HashSet::new(),
            dirty: false,
            revision: 0,
        }
    }
}
