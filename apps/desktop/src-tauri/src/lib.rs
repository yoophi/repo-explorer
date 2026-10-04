mod application;
mod domain;
mod infrastructure;

use application::{check_cancelled, ProgressSink};
use domain::{
    RepositoryRecord, RepositoryScanProgress, ScanRepositoriesRequest,
    UpdateRepositoryMetadataRequest,
};
use explorer_scan_job::{JobGuard, ScanRegistry, TerminalState};
use infrastructure::{FilesystemGitInspector, JsonCatalog, JsonMetadata};
use serde::Serialize;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

const SCAN_PROGRESS_EVENT: &str = "repository_scan_progress";
const SCAN_ITEM_EVENT: &str = "repository_scan_item";
const SCAN_TERMINAL_EVENT: &str = "repository_scan_terminal";

#[derive(Clone, Default)]
struct ScanJobs {
    registry: ScanRegistry,
    finalization: Arc<Mutex<()>>,
}

#[derive(Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanAcknowledgement {
    scan_id: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RepositoryScanProgressEvent {
    scan_id: String,
    #[serde(flatten)]
    progress: RepositoryScanProgress,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RepositoryScanItemEvent {
    scan_id: String,
    repository: RepositoryRecord,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RepositoryScanTerminal {
    scan_id: String,
    status: &'static str,
    repositories: Option<Vec<RepositoryRecord>>,
    error: Option<String>,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "repo-explorer",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
async fn list_repositories(app: tauri::AppHandle) -> Result<Vec<RepositoryRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let catalog = JsonCatalog::new(|| catalog_store_path(&app));
        application::list_repositories(&catalog, &FilesystemGitInspector)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn scan_repositories(
    app: tauri::AppHandle,
    jobs: tauri::State<'_, ScanJobs>,
    request: ScanRepositoriesRequest,
) -> Result<ScanAcknowledgement, String> {
    if request.scan_id.is_empty() {
        return Err("Scan ID is required".into());
    }
    let guard = jobs
        .registry
        .register(request.scan_id.clone())
        .map_err(|_| format!("Scan ID already active: {}", request.scan_id))?;
    let acknowledgement = ScanAcknowledgement {
        scan_id: request.scan_id.clone(),
    };
    let jobs = jobs.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let token = guard.token();
        let mut progress = TauriProgressSink {
            app: &app,
            scan_id: &request.scan_id,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            application::run_scan(
                &FilesystemGitInspector,
                &mut progress,
                &request,
                &token,
                || unix_timestamp(),
            )
        }))
        .unwrap_or_else(|_| Err(io::Error::other("Repository scan worker panicked")));

        let (terminal, result) = finish_scan(&jobs, guard, result, |work| {
            let catalog = JsonCatalog::new(|| catalog_store_path(&app));
            application::commit_scan(&catalog, work)
        });
        let event = match terminal {
            TerminalState::Completed => RepositoryScanTerminal {
                scan_id: request.scan_id,
                status: "completed",
                repositories: result.ok().map(|work| work.repositories),
                error: None,
            },
            TerminalState::Cancelled => RepositoryScanTerminal {
                scan_id: request.scan_id,
                status: "cancelled",
                repositories: None,
                error: None,
            },
            TerminalState::Failed => RepositoryScanTerminal {
                scan_id: request.scan_id,
                status: "failed",
                repositories: None,
                error: result.err().map(|error| error.to_string()),
            },
        };
        let _ = app.emit(SCAN_TERMINAL_EVENT, event);
    });
    Ok(acknowledgement)
}

#[tauri::command]
async fn cancel_repository_scan(
    jobs: tauri::State<'_, ScanJobs>,
    scan_id: String,
) -> Result<bool, String> {
    let jobs = jobs.inner().clone();
    tauri::async_runtime::spawn_blocking(move || cancel_scan(&jobs, &scan_id))
        .await
        .map_err(|error| error.to_string())
}

fn cancel_scan(jobs: &ScanJobs, scan_id: &str) -> bool {
    let _finalization = jobs
        .finalization
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    jobs.registry.cancel(scan_id)
}

fn finish_scan<T>(
    jobs: &ScanJobs,
    guard: JobGuard,
    result: io::Result<T>,
    commit: impl FnOnce(&T) -> io::Result<()>,
) -> (TerminalState, io::Result<T>) {
    // Cancellation and catalog commit share one boundary. Once committed,
    // the job is finished before a cancellation command can be accepted.
    let _finalization = jobs
        .finalization
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let token = guard.token();
    let result = result.and_then(|work| {
        check_cancelled(&token)?;
        commit(&work)?;
        Ok(work)
    });
    let terminal = guard.finish(&result);
    (terminal, result)
}

#[tauri::command]
async fn update_repository_metadata(
    app: tauri::AppHandle,
    request: UpdateRepositoryMetadataRequest,
) -> Result<RepositoryRecord, String> {
    tauri::async_runtime::spawn_blocking(move || update_repository_metadata_blocking(app, request))
        .await
        .map_err(|error| error.to_string())?
}

fn update_repository_metadata_blocking(
    app: tauri::AppHandle,
    request: UpdateRepositoryMetadataRequest,
) -> Result<RepositoryRecord, String> {
    let catalog = JsonCatalog::new(|| catalog_store_path(&app));
    application::update_metadata(
        &FilesystemGitInspector,
        &catalog,
        &JsonMetadata,
        request,
        || unix_timestamp(),
    )
}

struct TauriProgressSink<'a> {
    app: &'a tauri::AppHandle,
    scan_id: &'a str,
}

impl ProgressSink for TauriProgressSink<'_> {
    fn emit(&mut self, progress: RepositoryScanProgress) {
        let _ = self.app.emit(
            SCAN_PROGRESS_EVENT,
            RepositoryScanProgressEvent {
                scan_id: self.scan_id.to_string(),
                progress,
            },
        );
    }
    fn emit_item(&mut self, repository: RepositoryRecord) {
        let _ = self.app.emit(
            SCAN_ITEM_EVENT,
            RepositoryScanItemEvent {
                scan_id: self.scan_id.to_string(),
                repository,
            },
        );
    }
}

fn catalog_store_path(app: &tauri::AppHandle) -> io::Result<PathBuf> {
    let app_data_dir = app.path().app_data_dir().map_err(io::Error::other)?;

    Ok(app_data_dir.join("repositories.json"))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScanJobs::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_repositories,
            scan_repositories,
            cancel_repository_scan,
            update_repository_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::domain::*;
    use super::infrastructure::{
        discover_git_repositories, discover_git_repositories_with_progress,
        load_repository_metadata, read_readme, repositories_from_paths,
        repositories_from_paths_with_progress, save_repository_metadata,
        update_catalog_store_at_path,
    };
    use super::*;
    use explorer_json_store::load_json;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn cancelled_scan_does_not_commit_partial_catalog() {
        let temp_dir = tempdir().expect("create temp dir");
        let path = temp_dir.path().join("repositories.json");
        let jobs = ScanJobs::default();
        let guard = jobs.registry.register("cancelled").expect("register scan");
        assert!(cancel_scan(&jobs, "cancelled"));

        let (terminal, result) = finish_scan(
            &jobs,
            guard,
            Ok(PathBuf::from("/fixture/repo")),
            |repository| {
                update_catalog_store_at_path(&path, |store| {
                    merge_repository_paths(store, vec![repository.clone()]);
                })?;
                Ok(())
            },
        );

        assert_eq!(terminal, TerminalState::Cancelled);
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
        assert!(!path.exists());
    }

    #[test]
    fn cancelled_traversal_stops_before_reading_directories() {
        let temp_dir = tempdir().expect("create temp dir");
        let jobs = ScanJobs::default();
        let guard = jobs.registry.register("cancelled").expect("register scan");
        let token = guard.token();
        assert!(cancel_scan(&jobs, "cancelled"));
        let mut progress_count = 0;

        let result =
            discover_git_repositories_with_progress(temp_dir.path(), 2, Some(&token), |_| {
                progress_count += 1
            });

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
        assert_eq!(progress_count, 0);
    }

    #[test]
    fn completed_scan_commits_before_cancellation_can_be_accepted() {
        let temp_dir = tempdir().expect("create temp dir");
        let path = temp_dir.path().join("repositories.json");
        let jobs = ScanJobs::default();
        let guard = jobs.registry.register("completed").expect("register scan");
        let (terminal, result) = finish_scan(
            &jobs,
            guard,
            Ok(PathBuf::from("/fixture/repo")),
            |repository| {
                update_catalog_store_at_path(&path, |store| {
                    merge_repository_paths(store, vec![repository.clone()]);
                })?;
                Ok(())
            },
        );

        assert_eq!(terminal, TerminalState::Completed);
        assert!(result.is_ok());
        assert!(!cancel_scan(&jobs, "completed"));
        let catalog: CatalogStore = load_json(path).unwrap().unwrap();
        assert_eq!(catalog.repository_paths, vec!["/fixture/repo"]);
    }

    #[test]
    fn failed_scan_reports_failure_without_catalog_write() {
        let temp_dir = tempdir().expect("create temp dir");
        let path = temp_dir.path().join("repositories.json");
        let jobs = ScanJobs::default();
        let guard = jobs.registry.register("failed").expect("register scan");
        let (terminal, result) = finish_scan::<PathBuf>(
            &jobs,
            guard,
            Err(io::Error::other("fixture scan error")),
            |_| {
                update_catalog_store_at_path(&path, |_| {})?;
                Ok(())
            },
        );

        assert_eq!(terminal, TerminalState::Failed);
        assert_eq!(result.unwrap_err().to_string(), "fixture scan error");
        assert!(!path.exists());
    }

    #[test]
    fn discovers_git_repositories_by_depth_and_skips_build_directories() {
        let temp_dir = tempdir().expect("create temp dir");
        let root = temp_dir.path();
        let first_repo = root.join("team").join("api");
        let second_repo = root.join("team").join("web");
        let skipped_repo = root.join("node_modules").join("dependency");

        fs::create_dir_all(first_repo.join(".git")).expect("create first repo");
        fs::create_dir_all(second_repo.join(".git")).expect("create second repo");
        fs::create_dir_all(skipped_repo.join(".git")).expect("create skipped repo");

        let repositories = discover_git_repositories(root, 3).expect("discover repositories");
        let repository_paths = repositories
            .iter()
            .map(|path| {
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<BTreeSet<_>>();

        assert_eq!(
            repository_paths,
            BTreeSet::from(["team/api".into(), "team/web".into()])
        );
    }

    #[test]
    fn saves_and_loads_repository_metadata_in_repository_directory() {
        let temp_dir = tempdir().expect("create temp dir");
        let repository = temp_dir.path().join("repo");
        fs::create_dir_all(repository.join(".git")).expect("create repo");

        let metadata = RepositoryMetadata {
            description: "internal tool".into(),
            tags: vec!["infra".into(), "tool".into()],
            pinned: true,
        };

        save_repository_metadata(&repository, &metadata).expect("save metadata");
        let loaded = load_repository_metadata(&repository).expect("load metadata");
        let metadata_path = repository.join(METADATA_FILE_NAME);

        assert!(metadata_path.exists());
        assert_eq!(loaded.description, "internal tool");
        assert_eq!(loaded.tags, vec!["infra", "tool"]);
        assert!(loaded.pinned);
    }

    #[test]
    fn updates_existing_catalog_without_changing_its_schema() {
        let temp_dir = tempdir().expect("create temp dir");
        let path = temp_dir.path().join("repositories.json");
        fs::write(
            &path,
            r#"{"roots":[{"path":"/projects","maxDepth":2,"lastScannedAt":42}],"repositoryPaths":["/projects/first"]}"#,
        )
        .expect("write existing catalog");

        let store = update_catalog_store_at_path(&path, |store| {
            upsert_root(store, Path::new("/projects"), 4, 99);
            merge_repository_paths(store, vec![PathBuf::from("/projects/second")]);
        })
        .expect("update catalog");
        let persisted: CatalogStore = load_json(&path)
            .expect("load catalog")
            .expect("catalog exists");
        let json = fs::read_to_string(&path).expect("read catalog");

        assert_eq!(store.roots[0].max_depth, 4);
        assert_eq!(persisted.roots[0].last_scanned_at, 99);
        assert_eq!(
            persisted.repository_paths,
            vec!["/projects/first", "/projects/second"]
        );
        assert!(json.contains("\"maxDepth\""));
        assert!(json.contains("\"lastScannedAt\""));
        assert!(json.contains("\"repositoryPaths\""));
    }

    #[test]
    fn reads_readme_when_present() {
        let temp_dir = tempdir().expect("create temp dir");
        let repository = temp_dir.path().join("repo");
        fs::create_dir_all(repository.join(".git")).expect("create repo");
        fs::write(repository.join("README.md"), "# Repo\n\nDetails").expect("write readme");

        let readme = read_readme(&repository)
            .expect("read readme")
            .expect("readme exists");

        assert!(readme.path.ends_with("README.md"));
        assert_eq!(readme.content, "# Repo\n\nDetails");
    }

    #[test]
    fn inspects_origin_readme_and_metadata() {
        let temp_dir = tempdir().expect("create temp dir");
        let repository = temp_dir.path().join("repo");
        run_git(temp_dir.path(), ["init", "repo"]);
        run_git(
            &repository,
            ["remote", "add", "origin", "git@github.com:yoophi/repo.git"],
        );
        fs::write(repository.join("README.md"), "# Repo").expect("write readme");
        save_repository_metadata(
            &repository,
            &RepositoryMetadata {
                description: "metadata description".into(),
                tags: vec!["meta".into()],
                pinned: false,
            },
        )
        .expect("save metadata");

        let repositories = repositories_from_paths(
            vec![repository
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .to_string()],
            123,
        )
        .expect("inspect repositories");
        let repository = repositories.first().expect("repository");

        assert_eq!(
            repository.origin_url.as_deref(),
            Some("git@github.com:yoophi/repo.git")
        );
        assert_eq!(
            repository
                .readme
                .as_ref()
                .map(|readme| readme.content.as_str()),
            Some("# Repo")
        );
        assert_eq!(repository.metadata.description, "metadata description");
        assert_eq!(repository.last_seen_at, 123);
    }

    #[test]
    fn inspected_item_follows_progress_and_keeps_final_metadata() {
        let temp_dir = tempdir().expect("create temp dir");
        let repository = temp_dir.path().join("repo");
        run_git(temp_dir.path(), ["init", "repo"]);
        save_repository_metadata(
            &repository,
            &RepositoryMetadata {
                description: "streamed metadata".into(),
                tags: vec!["fixture".into()],
                pinned: true,
            },
        )
        .expect("save metadata");
        let events = std::cell::RefCell::new(Vec::new());
        let records = repositories_from_paths_with_progress(
            vec![repository.to_string_lossy().to_string()],
            77,
            None,
            true,
            |progress| events.borrow_mut().push(progress.phase.to_string()),
            |item| {
                assert_eq!(item.metadata.description, "streamed metadata");
                assert_eq!(item.last_seen_at, 77);
                events.borrow_mut().push(format!("item:{}", item.name));
            },
        )
        .expect("inspect fixture");
        assert_eq!(*events.borrow(), vec!["inspecting", "item:repo"]);
        assert_eq!(records[0].metadata.description, "streamed metadata");
    }

    #[test]
    fn assigns_linked_worktree_to_main_repository_parent() {
        let temp_dir = tempdir().expect("create temp dir");
        let main_repo = temp_dir.path().join("main");
        let linked_worktree = temp_dir.path().join("feature");

        run_git(temp_dir.path(), ["init", "main"]);
        run_git(&main_repo, ["config", "user.email", "test@example.com"]);
        run_git(&main_repo, ["config", "user.name", "Test User"]);
        fs::write(main_repo.join("README.md"), "# Main").expect("write readme");
        run_git(&main_repo, ["add", "README.md"]);
        run_git(&main_repo, ["commit", "-m", "initial"]);
        run_git(&main_repo, ["worktree", "add", "../feature"]);

        let main_repo = main_repo.canonicalize().unwrap();
        let linked_worktree = linked_worktree.canonicalize().unwrap();
        let repositories = repositories_from_paths(
            vec![
                main_repo.to_string_lossy().to_string(),
                linked_worktree.to_string_lossy().to_string(),
            ],
            456,
        )
        .expect("inspect repositories");

        let worktree = repositories
            .iter()
            .find(|repository| repository.path == linked_worktree.to_string_lossy())
            .expect("worktree");

        assert!(worktree.is_worktree);
        assert_eq!(
            worktree.parent_id.as_deref(),
            Some(main_repo.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn streamed_worktree_parent_matches_terminal_in_both_inspection_orders() {
        let temp_dir = tempdir().expect("create temp dir");
        let main_repo = temp_dir.path().join("main");
        let linked_worktree = temp_dir.path().join("feature");
        run_git(temp_dir.path(), ["init", "main"]);
        run_git(&main_repo, ["config", "user.email", "test@example.com"]);
        run_git(&main_repo, ["config", "user.name", "Test User"]);
        fs::write(main_repo.join("README.md"), "# Main").expect("write readme");
        run_git(&main_repo, ["add", "README.md"]);
        run_git(&main_repo, ["commit", "-m", "initial"]);
        run_git(&main_repo, ["worktree", "add", "../feature"]);
        let main_id = main_repo
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let worktree_id = linked_worktree
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string();

        for (paths, expected_emissions) in [
            (vec![main_id.clone(), worktree_id.clone()], 2),
            (vec![worktree_id.clone(), main_id.clone()], 3),
        ] {
            let items = std::cell::RefCell::new(Vec::new());
            let final_records = repositories_from_paths_with_progress(
                paths,
                456,
                None,
                true,
                |_| {},
                |item| items.borrow_mut().push(item),
            )
            .expect("inspect worktree fixture");
            let items = items.into_inner();
            assert_eq!(items.len(), expected_emissions);
            let streamed_worktree = items
                .iter()
                .rev()
                .find(|item| item.id == worktree_id)
                .unwrap();
            let final_worktree = final_records
                .iter()
                .find(|item| item.id == worktree_id)
                .unwrap();
            assert_eq!(
                streamed_worktree.parent_id.as_deref(),
                Some(main_id.as_str())
            );
            assert_eq!(streamed_worktree.parent_id, final_worktree.parent_id);
            if expected_emissions == 3 {
                assert_eq!(items[0].parent_id, None);
                assert_eq!(items[2].id, worktree_id);
            }
        }
    }

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .expect("run git");

        assert!(
            output.status.success(),
            "git command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
