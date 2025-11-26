use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root_paths: Vec<PathBuf>,
    pub name: String,
}

impl Workspace {
    pub fn new(root_paths: Vec<PathBuf>, name: Option<String>) -> Result<Self, WorkspaceError> {
        for path in &root_paths {
            if !path.exists() {
                return Err(WorkspaceError::PathNotFound(path.clone()));
            }
            if !path.is_dir() {
                return Err(WorkspaceError::NotADirectory(path.clone()));
            }
        }

        let name = name.unwrap_or_else(|| {
            if root_paths.len() == 1 {
                root_paths[0]
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
                    .to_string()
            } else {
                "multi-root-workspace".to_string()
            }
        });

        Ok(Self { root_paths, name })
    }

    pub fn single_root<P: AsRef<Path>>(path: P) -> Result<Self, WorkspaceError> {
        Self::new(vec![path.as_ref().to_path_buf()], None)
    }

    pub fn contains_file(&self, file_path: &Path) -> bool {
        self.root_paths
            .iter()
            .any(|root| file_path.starts_with(root))
    }

    pub fn relative_path(&self, file_path: &Path) -> Option<PathBuf> {
        for root in &self.root_paths {
            if let Ok(relative) = file_path.strip_prefix(root) {
                return Some(relative.to_path_buf());
            }
        }
        None
    }

    pub fn get_files(&self) -> Result<Vec<PathBuf>, WorkspaceError> {
        let mut files = Vec::new();

        for root in &self.root_paths {
            let entries = walkdir::WalkDir::new(root)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok());

            for entry in entries {
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(files)
    }

    pub fn find_files_by_extension(&self, extension: &str) -> Result<Vec<PathBuf>, WorkspaceError> {
        let all_files = self.get_files()?;
        let filtered: Vec<PathBuf> = all_files
            .into_iter()
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == extension)
                    .unwrap_or(false)
            })
            .collect();

        Ok(filtered)
    }

    pub fn create_file(&self, relative_path: &Path, content: &str) -> Result<(), WorkspaceError> {
        for root in &self.root_paths {
            let full_path = root.join(relative_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, content)?;
            return Ok(());
        }
        Err(WorkspaceError::PathNotFound(relative_path.to_path_buf()))
    }

    pub fn delete_file(&self, relative_path: &Path) -> Result<(), WorkspaceError> {
        for root in &self.root_paths {
            let full_path = root.join(relative_path);
            if full_path.exists() {
                std::fs::remove_file(&full_path)?;
                return Ok(());
            }
        }
        Err(WorkspaceError::PathNotFound(relative_path.to_path_buf()))
    }
}

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),
    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
