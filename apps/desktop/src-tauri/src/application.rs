//! Tauri-free repository use cases and the outbound contracts they require.
use crate::domain::{
    normalize_tags, pathbufs_to_strings, CatalogStore, RepositoryMetadata, RepositoryRecord,
    RepositoryScanProgress, ScanRepositoriesRequest, UpdateRepositoryMetadataRequest,
};
use explorer_scan_job::CancellationToken;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) trait RepositoryInspector {
    fn normalize_path(&self, path: &str) -> io::Result<PathBuf>;
    fn is_git_repository(&self, path: &Path) -> bool;
    fn discover(
        &self,
        root: &Path,
        max_depth: usize,
        token: &CancellationToken,
        progress: &mut dyn FnMut(RepositoryScanProgress),
    ) -> io::Result<Vec<PathBuf>>;
    fn inspect(
        &self,
        paths: Vec<String>,
        last_seen_at: u64,
        token: Option<&CancellationToken>,
        sink: &mut dyn ProgressSink,
    ) -> io::Result<Vec<RepositoryRecord>>;
}

pub(crate) trait CatalogRepository {
    fn load(&self) -> io::Result<CatalogStore>;
    /// Implementations must keep each read-modify-write in one update_json lock.
    fn record_scan(
        &self,
        root: &Path,
        max_depth: usize,
        now: u64,
        paths: &[PathBuf],
    ) -> io::Result<()>;
    fn add_repository(&self, path: &Path) -> io::Result<CatalogStore>;
}

pub(crate) trait MetadataRepository {
    fn save(&self, path: &Path, metadata: &RepositoryMetadata) -> io::Result<()>;
}

pub(crate) trait ProgressSink {
    fn emit(&mut self, progress: RepositoryScanProgress);
    fn emit_item(&mut self, item: RepositoryRecord);
    fn wants_items(&self) -> bool {
        true
    }
}

impl ProgressSink for () {
    fn emit(&mut self, _progress: RepositoryScanProgress) {}
    fn emit_item(&mut self, _item: RepositoryRecord) {}
    fn wants_items(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub(crate) struct ScanWork {
    pub(crate) root_path: PathBuf,
    pub(crate) max_depth: usize,
    pub(crate) now: u64,
    pub(crate) discovered_paths: Vec<PathBuf>,
    pub(crate) repositories: Vec<RepositoryRecord>,
}

pub(crate) fn run_scan(
    inspector: &dyn RepositoryInspector,
    sink: &mut dyn ProgressSink,
    request: &ScanRepositoriesRequest,
    token: &CancellationToken,
    timestamp: impl FnOnce() -> u64,
) -> io::Result<ScanWork> {
    check_cancelled(token)?;
    let root_path = inspector.normalize_path(&request.root_path)?;
    let max_depth = request.max_depth.unwrap_or(4);
    let now = timestamp();
    sink.emit(RepositoryScanProgress {
        phase: "started",
        current_path: Some(root_path.to_string_lossy().to_string()),
        visited_directories: 0,
        discovered_repositories: 0,
        message: Some("Starting repository scan".into()),
    });
    let discovered_paths =
        inspector.discover(&root_path, max_depth, token, &mut |event| sink.emit(event))?;
    let repositories = inspector.inspect(
        pathbufs_to_strings(&discovered_paths),
        now,
        Some(token),
        sink,
    )?;
    check_cancelled(token)?;
    Ok(ScanWork {
        root_path,
        max_depth,
        now,
        discovered_paths,
        repositories,
    })
}

pub(crate) fn commit_scan(catalog: &dyn CatalogRepository, work: &ScanWork) -> io::Result<()> {
    catalog.record_scan(
        &work.root_path,
        work.max_depth,
        work.now,
        &work.discovered_paths,
    )
}

pub(crate) fn list_repositories(
    catalog: &dyn CatalogRepository,
    inspector: &dyn RepositoryInspector,
) -> io::Result<Vec<RepositoryRecord>> {
    let store = catalog.load()?;
    inspector.inspect(store.repository_paths, 0, None, &mut ())
}

pub(crate) fn update_metadata(
    inspector: &dyn RepositoryInspector,
    catalog: &dyn CatalogRepository,
    metadata_store: &dyn MetadataRepository,
    request: UpdateRepositoryMetadataRequest,
    timestamp: impl FnOnce() -> u64,
) -> Result<RepositoryRecord, String> {
    let repository_path = inspector
        .normalize_path(&request.repository_id)
        .map_err(|error| error.to_string())?;
    if !inspector.is_git_repository(&repository_path) {
        return Err(format!(
            "Repository not found: {}",
            repository_path.display()
        ));
    }
    let metadata = RepositoryMetadata {
        description: request.description.trim().to_string(),
        tags: normalize_tags(request.tags),
        pinned: request.pinned,
    };
    metadata_store
        .save(&repository_path, &metadata)
        .map_err(|error| error.to_string())?;
    let store = catalog
        .add_repository(&repository_path)
        .map_err(|error| error.to_string())?;
    let repositories = inspector
        .inspect(store.repository_paths, timestamp(), None, &mut ())
        .map_err(|error| error.to_string())?;
    repositories
        .into_iter()
        .find(|repository| repository.id == request.repository_id)
        .ok_or_else(|| format!("Repository not found: {}", request.repository_id))
}

pub(crate) fn check_cancelled(token: &CancellationToken) -> io::Result<()> {
    if token.is_cancelled() {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Repository scan cancelled",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{merge_repository_paths, upsert_root};
    use explorer_scan_job::ScanRegistry;
    use std::sync::{Arc, Mutex};

    struct FakeInspector {
        path: PathBuf,
        discovered: Vec<PathBuf>,
        records: Vec<RepositoryRecord>,
        fail_discover: bool,
        fail_inspect_after_item: bool,
        valid_repository: bool,
    }

    impl RepositoryInspector for FakeInspector {
        fn normalize_path(&self, _path: &str) -> io::Result<PathBuf> {
            Ok(self.path.clone())
        }
        fn is_git_repository(&self, _path: &Path) -> bool {
            self.valid_repository
        }
        fn discover(
            &self,
            _root: &Path,
            _depth: usize,
            _token: &CancellationToken,
            progress: &mut dyn FnMut(RepositoryScanProgress),
        ) -> io::Result<Vec<PathBuf>> {
            if self.fail_discover {
                return Err(io::Error::other("fake discovery failed"));
            }
            progress(RepositoryScanProgress {
                phase: "found",
                current_path: None,
                visited_directories: 1,
                discovered_repositories: 1,
                message: None,
            });
            Ok(self.discovered.clone())
        }
        fn inspect(
            &self,
            _paths: Vec<String>,
            _now: u64,
            _token: Option<&CancellationToken>,
            sink: &mut dyn ProgressSink,
        ) -> io::Result<Vec<RepositoryRecord>> {
            for record in &self.records {
                sink.emit_item(record.clone());
            }
            if self.fail_inspect_after_item {
                return Err(io::Error::other("fake inspection failed after item"));
            }
            Ok(self.records.clone())
        }
    }

    #[derive(Default)]
    struct FakeCatalog {
        value: Mutex<CatalogStore>,
        operations: Arc<Mutex<Vec<&'static str>>>,
        fail_add: bool,
    }

    impl CatalogRepository for FakeCatalog {
        fn load(&self) -> io::Result<CatalogStore> {
            Ok(self.value.lock().unwrap().clone())
        }
        fn record_scan(
            &self,
            root: &Path,
            depth: usize,
            now: u64,
            paths: &[PathBuf],
        ) -> io::Result<()> {
            self.operations.lock().unwrap().push("record_scan");
            let mut value = self.value.lock().unwrap();
            upsert_root(&mut value, root, depth, now);
            merge_repository_paths(&mut value, paths.to_vec());
            Ok(())
        }
        fn add_repository(&self, path: &Path) -> io::Result<CatalogStore> {
            self.operations.lock().unwrap().push("add_repository");
            if self.fail_add {
                return Err(io::Error::other("fake catalog path failed"));
            }
            let mut value = self.value.lock().unwrap();
            merge_repository_paths(&mut value, vec![path.to_path_buf()]);
            Ok(value.clone())
        }
    }

    struct FakeMetadata {
        saved: Mutex<Option<RepositoryMetadata>>,
        operations: Arc<Mutex<Vec<&'static str>>>,
        fail: bool,
    }

    impl MetadataRepository for FakeMetadata {
        fn save(&self, _path: &Path, metadata: &RepositoryMetadata) -> io::Result<()> {
            self.operations.lock().unwrap().push("save_metadata");
            if self.fail {
                return Err(io::Error::other("fake metadata write failed"));
            }
            *self.saved.lock().unwrap() = Some(metadata.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeProgress(Vec<&'static str>);
    impl ProgressSink for FakeProgress {
        fn emit(&mut self, progress: RepositoryScanProgress) {
            self.0.push(progress.phase);
        }
        fn emit_item(&mut self, _item: RepositoryRecord) {
            self.0.push("item");
        }
    }

    fn record() -> RepositoryRecord {
        RepositoryRecord {
            id: "/fake/repo".into(),
            name: "repo".into(),
            path: "/fake/repo".into(),
            relative_path: "repo".into(),
            parent_id: None,
            is_worktree: false,
            origin_url: None,
            git_status: Default::default(),
            readme: None,
            metadata: RepositoryMetadata::default(),
            metadata_path: "/fake/repo/.repo-explorer.json".into(),
            last_seen_at: 123,
        }
    }

    fn inspector() -> FakeInspector {
        FakeInspector {
            path: PathBuf::from("/fake"),
            discovered: vec![PathBuf::from("/fake/repo")],
            records: vec![record()],
            fail_discover: false,
            fail_inspect_after_item: false,
            valid_repository: true,
        }
    }

    #[test]
    fn scan_uses_fake_ports_and_commits_only_after_complete_result() {
        let registry = ScanRegistry::default();
        let guard = registry.register("scan").unwrap();
        let catalog = FakeCatalog::default();
        let mut sink = FakeProgress::default();
        let work = run_scan(
            &inspector(),
            &mut sink,
            &ScanRepositoriesRequest {
                scan_id: "scan".into(),
                root_path: "/input".into(),
                max_depth: Some(2),
            },
            &guard.token(),
            || 123,
        )
        .unwrap();
        assert_eq!(sink.0, vec!["started", "found", "item"]);
        assert!(catalog.load().unwrap().repository_paths.is_empty());
        assert_eq!(work.repositories[0].id, "/fake/repo");
        commit_scan(&catalog, &work).unwrap();
        let stored = catalog.load().unwrap();
        assert_eq!(stored.roots[0].path, "/fake");
        assert_eq!(stored.roots[0].max_depth, 2);
        assert_eq!(stored.repository_paths, vec!["/fake/repo"]);
    }

    #[test]
    fn fake_scan_failure_and_cancellation_do_not_commit() {
        let registry = ScanRegistry::default();
        let guard = registry.register("scan").unwrap();
        let catalog = FakeCatalog::default();
        let mut failing = inspector();
        failing.fail_discover = true;
        let request = ScanRepositoriesRequest {
            scan_id: "scan".into(),
            root_path: "/input".into(),
            max_depth: None,
        };
        assert_eq!(
            run_scan(
                &failing,
                &mut FakeProgress::default(),
                &request,
                &guard.token(),
                || 123
            )
            .unwrap_err()
            .to_string(),
            "fake discovery failed"
        );
        assert!(registry.cancel("scan"));
        assert_eq!(
            run_scan(
                &inspector(),
                &mut FakeProgress::default(),
                &request,
                &guard.token(),
                || 123
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::Interrupted
        );
        assert!(catalog.operations.lock().unwrap().is_empty());
    }

    #[test]
    fn item_emission_before_inspection_failure_never_commits_catalog() {
        let guard = ScanRegistry::default().register("scan").unwrap();
        let catalog = FakeCatalog::default();
        let mut failing = inspector();
        failing.fail_inspect_after_item = true;
        let mut sink = FakeProgress::default();
        let result = run_scan(
            &failing,
            &mut sink,
            &ScanRepositoriesRequest {
                scan_id: "scan".into(),
                root_path: "/input".into(),
                max_depth: None,
            },
            &guard.token(),
            || 123,
        );
        assert_eq!(
            result.unwrap_err().to_string(),
            "fake inspection failed after item"
        );
        assert_eq!(sink.0, vec!["started", "found", "item"]);
        assert!(catalog.operations.lock().unwrap().is_empty());
    }

    #[test]
    fn metadata_use_case_saves_normalized_data_then_updates_catalog() {
        let operations = Arc::new(Mutex::new(Vec::new()));
        let catalog = FakeCatalog {
            operations: operations.clone(),
            ..Default::default()
        };
        let metadata = FakeMetadata {
            saved: Mutex::new(None),
            operations: operations.clone(),
            fail: false,
        };
        let request = UpdateRepositoryMetadataRequest {
            repository_id: "/fake/repo".into(),
            description: "  internal tool  ".into(),
            tags: vec![" infra ".into(), "tool".into(), "infra".into()],
            pinned: true,
        };
        let mut inspector = inspector();
        inspector.path = PathBuf::from("/fake/repo");
        let clock_operations = operations.clone();
        let updated = update_metadata(&inspector, &catalog, &metadata, request, || {
            clock_operations.lock().unwrap().push("timestamp");
            123
        })
        .unwrap();
        assert_eq!(updated.id, "/fake/repo");
        assert_eq!(
            *operations.lock().unwrap(),
            vec!["save_metadata", "add_repository", "timestamp"]
        );
        assert_eq!(catalog.load().unwrap().repository_paths, vec!["/fake/repo"]);
        let saved = metadata.saved.lock().unwrap();
        assert_eq!(saved.as_ref().unwrap().description, "internal tool");
        assert_eq!(saved.as_ref().unwrap().tags, vec!["infra", "tool"]);
        assert!(saved.as_ref().unwrap().pinned);
    }

    #[test]
    fn metadata_write_failure_does_not_update_catalog() {
        let operations = Arc::new(Mutex::new(Vec::new()));
        let catalog = FakeCatalog {
            operations: operations.clone(),
            ..Default::default()
        };
        let metadata = FakeMetadata {
            saved: Mutex::new(None),
            operations: operations.clone(),
            fail: true,
        };
        let request = UpdateRepositoryMetadataRequest {
            repository_id: "/fake/repo".into(),
            description: "x".into(),
            tags: vec![],
            pinned: false,
        };
        let mut inspector = inspector();
        inspector.path = PathBuf::from("/fake/repo");
        assert!(update_metadata(&inspector, &catalog, &metadata, request, || 123).is_err());
        assert_eq!(*operations.lock().unwrap(), vec!["save_metadata"]);
    }

    #[test]
    fn catalog_failure_happens_after_metadata_save() {
        let operations = Arc::new(Mutex::new(Vec::new()));
        let catalog = FakeCatalog {
            operations: operations.clone(),
            fail_add: true,
            ..Default::default()
        };
        let metadata = FakeMetadata {
            saved: Mutex::new(None),
            operations: operations.clone(),
            fail: false,
        };
        let mut inspector = inspector();
        inspector.path = PathBuf::from("/fake/repo");
        let request = UpdateRepositoryMetadataRequest {
            repository_id: "/fake/repo".into(),
            description: "saved first".into(),
            tags: vec![],
            pinned: false,
        };

        let error = update_metadata(&inspector, &catalog, &metadata, request, || {
            panic!("timestamp must follow catalog update")
        })
        .unwrap_err();
        assert_eq!(error, "fake catalog path failed");
        assert_eq!(
            *operations.lock().unwrap(),
            vec!["save_metadata", "add_repository"]
        );
        assert_eq!(
            metadata.saved.lock().unwrap().as_ref().unwrap().description,
            "saved first"
        );
        assert!(catalog.load().unwrap().repository_paths.is_empty());
    }
}

pub(crate) trait TerminalLauncher {
    fn open(&self, path: &Path, app: crate::domain::TerminalApp) -> Result<(), String>;
}

pub(crate) fn open_repository_in_terminal(
    inspector: &dyn RepositoryInspector,
    launcher: &dyn TerminalLauncher,
    request: crate::domain::OpenRepositoryInTerminalRequest,
) -> Result<(), String> {
    let path = inspector
        .normalize_path(&request.repository_id)
        .map_err(|e| e.to_string())?;
    if !inspector.is_git_repository(&path) {
        return Err(format!("Path is not a git repository: {}", path.display()));
    }
    launcher.open(&path, request.terminal_app)
}
