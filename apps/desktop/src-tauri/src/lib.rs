use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

const METADATA_FILE_NAME: &str = ".repo-explorer.json";
const SCAN_PROGRESS_EVENT: &str = "repository_scan_progress";
const REPOSITORY_SCANNED_EVENT: &str = "repository_scanned";

#[derive(Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanRepositoriesRequest {
    root_path: String,
    max_depth: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRepositoryMetadataRequest {
    repository_id: String,
    description: String,
    tags: Vec<String>,
    pinned: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenRepositoryInTerminalRequest {
    repository_id: String,
    terminal_app: TerminalApp,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum TerminalApp {
    Terminal,
    Iterm2,
    Ghostty,
    Wezterm,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogStore {
    roots: Vec<CatalogRoot>,
    repository_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogRoot {
    path: String,
    max_depth: usize,
    last_scanned_at: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryMetadata {
    description: String,
    tags: Vec<String>,
    pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryRecord {
    id: String,
    name: String,
    path: String,
    relative_path: String,
    parent_id: Option<String>,
    is_worktree: bool,
    origin_url: Option<String>,
    git_status: GitStatusSummary,
    readme: Option<ReadmeContent>,
    metadata: RepositoryMetadata,
    metadata_path: String,
    last_seen_at: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitStatusSummary {
    uncommitted_changes: usize,
    ahead: usize,
    behind: usize,
    has_upstream: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadmeContent {
    path: String,
    content: String,
}

#[derive(Debug)]
struct RepositoryInspection {
    path: PathBuf,
    git_dir: Option<PathBuf>,
    common_git_dir: Option<PathBuf>,
    origin_url: Option<String>,
    git_status: GitStatusSummary,
    readme: Option<ReadmeContent>,
    metadata: RepositoryMetadata,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryScanProgress {
    phase: &'static str,
    current_path: Option<String>,
    visited_directories: usize,
    discovered_repositories: usize,
    message: Option<String>,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "repo-explorer",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
fn list_repositories(app: tauri::AppHandle) -> Result<Vec<RepositoryRecord>, String> {
    let store = load_catalog_store(&app).map_err(|error| error.to_string())?;
    repositories_from_paths(store.repository_paths, 0).map_err(|error| error.to_string())
}

#[tauri::command]
async fn scan_repositories(
    app: tauri::AppHandle,
    request: ScanRepositoriesRequest,
) -> Result<Vec<RepositoryRecord>, String> {
    let app_for_scan = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let root_path = normalize_path(&request.root_path).map_err(|error| error.to_string())?;
        let max_depth = request.max_depth.unwrap_or(4);
        let now = unix_timestamp();
        emit_scan_progress(
            &app_for_scan,
            RepositoryScanProgress {
                phase: "started",
                current_path: Some(root_path.to_string_lossy().to_string()),
                visited_directories: 0,
                discovered_repositories: 0,
                message: Some("Starting repository scan".into()),
            },
        );

        let discovered_paths =
            discover_git_repositories_with_progress(&root_path, max_depth, |progress| {
                emit_scan_progress(&app_for_scan, progress);
            })
            .map_err(|error| error.to_string())?;
        let discovered_repository_paths = pathbufs_to_strings(&discovered_paths);

        let mut store = load_catalog_store(&app_for_scan).map_err(|error| error.to_string())?;
        upsert_root(&mut store, &root_path, max_depth, now);
        merge_repository_paths(&mut store, discovered_paths);
        save_catalog_store(&app_for_scan, &store).map_err(|error| error.to_string())?;

        let repositories = repositories_from_paths_with_progress_and_records(
            discovered_repository_paths,
            now,
            |progress| {
                emit_scan_progress(&app_for_scan, progress);
            },
            |repository| {
                emit_repository_scanned(&app_for_scan, repository);
            },
        )
        .map_err(|error| error.to_string())?;

        emit_scan_progress(
            &app_for_scan,
            RepositoryScanProgress {
                phase: "finished",
                current_path: None,
                visited_directories: 0,
                discovered_repositories: repositories.len(),
                message: Some("Repository scan finished".into()),
            },
        );

        Ok(repositories)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn update_repository_metadata(
    app: tauri::AppHandle,
    request: UpdateRepositoryMetadataRequest,
) -> Result<RepositoryRecord, String> {
    let repository_path =
        normalize_path(&request.repository_id).map_err(|error| error.to_string())?;
    if !is_git_repository(&repository_path) {
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
    save_repository_metadata(&repository_path, &metadata).map_err(|error| error.to_string())?;

    let mut store = load_catalog_store(&app).map_err(|error| error.to_string())?;
    merge_repository_paths(&mut store, vec![repository_path]);
    save_catalog_store(&app, &store).map_err(|error| error.to_string())?;

    let repositories = repositories_from_paths(store.repository_paths, unix_timestamp())
        .map_err(|error| error.to_string())?;
    repositories
        .into_iter()
        .find(|repository| repository.id == request.repository_id)
        .ok_or_else(|| format!("Repository not found: {}", request.repository_id))
}

#[tauri::command]
fn open_repository_in_terminal(request: OpenRepositoryInTerminalRequest) -> Result<(), String> {
    let repository_path =
        normalize_path(&request.repository_id).map_err(|error| error.to_string())?;
    if !is_git_repository(&repository_path) {
        return Err(format!(
            "Path is not a git repository: {}",
            repository_path.display()
        ));
    }

    open_terminal_at_path(&repository_path, request.terminal_app)
}

fn repositories_from_paths(
    paths: Vec<String>,
    last_seen_at: u64,
) -> io::Result<Vec<RepositoryRecord>> {
    repositories_from_paths_with_progress(paths, last_seen_at, |_| {})
}

fn repositories_from_paths_with_progress(
    paths: Vec<String>,
    last_seen_at: u64,
    mut on_progress: impl FnMut(RepositoryScanProgress),
) -> io::Result<Vec<RepositoryRecord>> {
    repositories_from_paths_with_progress_and_records(paths, last_seen_at, &mut on_progress, |_| {})
}

fn repositories_from_paths_with_progress_and_records(
    paths: Vec<String>,
    last_seen_at: u64,
    mut on_progress: impl FnMut(RepositoryScanProgress),
    mut on_repository: impl FnMut(RepositoryRecord),
) -> io::Result<Vec<RepositoryRecord>> {
    let inspections = inspect_repositories_with_progress(paths, &mut on_progress)?;
    let parent_ids = worktree_parent_ids(&inspections);
    let mut repositories = Vec::new();

    for inspection in inspections {
        let path_string = inspection.path.to_string_lossy().to_string();
        let parent_id = parent_ids.get(&path_string).cloned();
        let name = inspection
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("repository")
            .to_string();
        let relative_path = shortest_relative_path(&inspection.path);
        let is_worktree = parent_id.is_some()
            || inspection
                .git_dir
                .as_ref()
                .zip(inspection.common_git_dir.as_ref())
                .map(|(git_dir, common_git_dir)| git_dir != common_git_dir)
                .unwrap_or(false);
        let metadata_path = inspection
            .path
            .join(METADATA_FILE_NAME)
            .to_string_lossy()
            .to_string();

        let repository = RepositoryRecord {
            id: path_string.clone(),
            name,
            path: path_string,
            relative_path,
            parent_id,
            is_worktree,
            origin_url: inspection.origin_url,
            git_status: inspection.git_status,
            readme: inspection.readme,
            metadata: inspection.metadata,
            metadata_path,
            last_seen_at,
        };

        on_repository(repository.clone());
        repositories.push(repository);
    }

    sort_repositories(&mut repositories);
    Ok(repositories)
}

fn inspect_repositories_with_progress(
    paths: Vec<String>,
    on_progress: &mut impl FnMut(RepositoryScanProgress),
) -> io::Result<Vec<RepositoryInspection>> {
    let mut inspections = Vec::new();
    let total_paths = paths.len();

    for path in paths {
        let repository_path = match normalize_path(&path) {
            Ok(path) => path,
            Err(_) => continue,
        };

        if !is_git_repository(&repository_path) {
            continue;
        }

        on_progress(RepositoryScanProgress {
            phase: "inspecting",
            current_path: Some(repository_path.to_string_lossy().to_string()),
            visited_directories: 0,
            discovered_repositories: inspections.len() + 1,
            message: Some(format!(
                "Inspecting repository {} of {}",
                inspections.len() + 1,
                total_paths
            )),
        });

        inspections.push(RepositoryInspection {
            git_dir: git_path(&repository_path, "--git-dir"),
            common_git_dir: git_path(&repository_path, "--git-common-dir"),
            origin_url: git_output(&repository_path, ["config", "--get", "remote.origin.url"]),
            git_status: git_status_summary(&repository_path),
            readme: read_readme(&repository_path)?,
            metadata: load_repository_metadata(&repository_path)?,
            path: repository_path,
        });
    }

    Ok(inspections)
}

#[cfg(test)]
fn discover_git_repositories(root_path: &Path, max_depth: usize) -> io::Result<Vec<PathBuf>> {
    discover_git_repositories_with_progress(root_path, max_depth, |_| {})
}

fn discover_git_repositories_with_progress(
    root_path: &Path,
    max_depth: usize,
    mut on_progress: impl FnMut(RepositoryScanProgress),
) -> io::Result<Vec<PathBuf>> {
    let mut repositories = Vec::new();
    let mut pending = VecDeque::from([(root_path.to_path_buf(), 0)]);
    let mut visited_directories = 0;

    while let Some((path, depth)) = pending.pop_front() {
        visited_directories += 1;
        on_progress(RepositoryScanProgress {
            phase: "scanning",
            current_path: Some(path.to_string_lossy().to_string()),
            visited_directories,
            discovered_repositories: repositories.len(),
            message: None,
        });

        if is_git_repository(&path) {
            repositories.push(path.clone());
            on_progress(RepositoryScanProgress {
                phase: "found",
                current_path: Some(path.to_string_lossy().to_string()),
                visited_directories,
                discovered_repositories: repositories.len(),
                message: Some("Git repository found".into()),
            });
        }

        if depth >= max_depth {
            continue;
        }

        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => continue,
            Err(error) => return Err(error),
        };

        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_dir() {
                continue;
            }

            let child_path = entry.path();
            if should_skip_directory(&child_path) {
                continue;
            }

            pending.push_back((child_path, depth + 1));
        }
    }

    repositories.sort();
    repositories.dedup();
    Ok(repositories)
}

fn emit_scan_progress(app: &tauri::AppHandle, progress: RepositoryScanProgress) {
    let _ = app.emit(SCAN_PROGRESS_EVENT, progress);
}

fn emit_repository_scanned(app: &tauri::AppHandle, repository: RepositoryRecord) {
    let _ = app.emit(REPOSITORY_SCANNED_EVENT, repository);
}

fn worktree_parent_ids(inspections: &[RepositoryInspection]) -> HashMap<String, String> {
    let mut main_repositories_by_common_git_dir = HashMap::new();

    for inspection in inspections {
        let Some(git_dir) = &inspection.git_dir else {
            continue;
        };
        let Some(common_git_dir) = &inspection.common_git_dir else {
            continue;
        };

        if git_dir == common_git_dir {
            main_repositories_by_common_git_dir.insert(
                common_git_dir.to_string_lossy().to_string(),
                inspection.path.to_string_lossy().to_string(),
            );
        }
    }

    inspections
        .iter()
        .filter_map(|inspection| {
            let git_dir = inspection.git_dir.as_ref()?;
            let common_git_dir = inspection.common_git_dir.as_ref()?;
            if git_dir == common_git_dir {
                return None;
            }

            let parent_id = main_repositories_by_common_git_dir
                .get(&common_git_dir.to_string_lossy().to_string())?;
            let repository_id = inspection.path.to_string_lossy().to_string();

            if parent_id == &repository_id {
                return None;
            }

            Some((repository_id, parent_id.clone()))
        })
        .collect()
}

fn git_path(repository_path: &Path, arg: &str) -> Option<PathBuf> {
    let value = git_output(
        repository_path,
        ["rev-parse", "--path-format=absolute", arg],
    )?;
    Some(PathBuf::from(value))
}

fn git_output<const N: usize>(repository_path: &Path, args: [&str; N]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_path)
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn git_status_summary(repository_path: &Path) -> GitStatusSummary {
    let uncommitted_changes = git_output(
        repository_path,
        ["status", "--porcelain=v1", "--untracked-files=normal"],
    )
    .map(|status| {
        status
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    })
    .unwrap_or_default();

    let Some(upstream) = git_output(
        repository_path,
        [
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    ) else {
        return GitStatusSummary {
            uncommitted_changes,
            ..GitStatusSummary::default()
        };
    };

    let Some((behind, ahead)) = git_output(
        repository_path,
        ["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    )
    .and_then(|output| parse_ahead_behind_counts(&output)) else {
        return GitStatusSummary {
            uncommitted_changes,
            has_upstream: true,
            ..GitStatusSummary::default()
        };
    };

    GitStatusSummary {
        uncommitted_changes,
        ahead,
        behind,
        has_upstream: !upstream.is_empty(),
    }
}

fn parse_ahead_behind_counts(output: &str) -> Option<(usize, usize)> {
    let mut counts = output.split_whitespace();
    let behind = counts.next()?.parse().ok()?;
    let ahead = counts.next()?.parse().ok()?;

    Some((behind, ahead))
}

fn read_readme(repository_path: &Path) -> io::Result<Option<ReadmeContent>> {
    let Some(readme_path) = find_readme(repository_path)? else {
        return Ok(None);
    };

    let mut content = fs::read_to_string(&readme_path)?;
    const MAX_README_BYTES: usize = 200_000;
    if content.len() > MAX_README_BYTES {
        content.truncate(MAX_README_BYTES);
        content.push_str("\n\n[README truncated]");
    }

    Ok(Some(ReadmeContent {
        path: readme_path.to_string_lossy().to_string(),
        content,
    }))
}

fn find_readme(repository_path: &Path) -> io::Result<Option<PathBuf>> {
    let entries = fs::read_dir(repository_path)?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() {
                return None;
            }

            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.eq_ignore_ascii_case("readme")
                || file_name.to_ascii_lowercase().starts_with("readme.")
            {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase())
            .unwrap_or_default()
    });

    Ok(candidates.into_iter().next())
}

fn load_repository_metadata(repository_path: &Path) -> io::Result<RepositoryMetadata> {
    let path = repository_path.join(METADATA_FILE_NAME);
    if !path.exists() {
        return Ok(RepositoryMetadata::default());
    }

    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn save_repository_metadata(
    repository_path: &Path,
    metadata: &RepositoryMetadata,
) -> io::Result<()> {
    let content = serde_json::to_string_pretty(metadata)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    fs::write(repository_path.join(METADATA_FILE_NAME), content)
}

impl Default for RepositoryMetadata {
    fn default() -> Self {
        Self {
            description: String::new(),
            tags: Vec::new(),
            pinned: false,
        }
    }
}

fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

fn should_skip_directory(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "node_modules" | "target" | "dist" | ".next" | ".turbo" | "storybook-static")
    )
}

fn normalize_path(path: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(path);
    if path.exists() {
        return path.canonicalize();
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Path does not exist: {}", path.display()),
    ))
}

fn shortest_relative_path(path: &Path) -> String {
    let Ok(current_dir) = std::env::current_dir() else {
        return path.to_string_lossy().to_string();
    };

    path.strip_prefix(&current_dir)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn catalog_store_path(app: &tauri::AppHandle) -> io::Result<PathBuf> {
    let app_data_dir = app.path().app_data_dir().map_err(io::Error::other)?;

    fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir.join("repositories.json"))
}

fn load_catalog_store(app: &tauri::AppHandle) -> io::Result<CatalogStore> {
    let path = catalog_store_path(app)?;
    if !path.exists() {
        return Ok(CatalogStore::default());
    }

    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn save_catalog_store(app: &tauri::AppHandle, store: &CatalogStore) -> io::Result<()> {
    let path = catalog_store_path(app)?;
    let content = serde_json::to_string_pretty(store)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    fs::write(path, content)
}

fn upsert_root(store: &mut CatalogStore, path: &Path, max_depth: usize, last_scanned_at: u64) {
    let path = path.to_string_lossy().to_string();
    if let Some(root) = store.roots.iter_mut().find(|root| root.path == path) {
        root.max_depth = max_depth;
        root.last_scanned_at = last_scanned_at;
        return;
    }

    store.roots.push(CatalogRoot {
        path,
        max_depth,
        last_scanned_at,
    });
}

fn merge_repository_paths(store: &mut CatalogStore, paths: Vec<PathBuf>) {
    let mut path_set = store
        .repository_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    for path in paths {
        path_set.insert(path.to_string_lossy().to_string());
    }

    store.repository_paths = path_set.into_iter().collect();
}

fn pathbufs_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

fn sort_repositories(repositories: &mut [RepositoryRecord]) {
    repositories.sort_by(|left, right| {
        right
            .metadata
            .pinned
            .cmp(&left.metadata.pinned)
            .then_with(|| {
                left.path
                    .matches('/')
                    .count()
                    .cmp(&right.path.matches('/').count())
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.path.cmp(&right.path))
    });
}

#[cfg(target_os = "macos")]
fn open_terminal_at_path(path: &Path, terminal_app: TerminalApp) -> Result<(), String> {
    let app_name = match terminal_app {
        TerminalApp::Terminal => "Terminal",
        TerminalApp::Iterm2 => "iTerm",
        TerminalApp::Ghostty => "Ghostty",
        TerminalApp::Wezterm => "WezTerm",
    };

    let status = Command::new("open")
        .arg("-a")
        .arg(app_name)
        .arg(path)
        .status()
        .map_err(|error| format!("Failed to open {app_name}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Failed to open {app_name}: {status}"))
    }
}

#[cfg(not(target_os = "macos"))]
fn open_terminal_at_path(_path: &Path, _terminal_app: TerminalApp) -> Result<(), String> {
    Err("Opening a selected terminal app is only supported on macOS.".into())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut tags = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();

    tags.sort();
    tags.dedup();
    tags
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_repositories,
            scan_repositories,
            open_repository_in_terminal,
            update_repository_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
    fn summarizes_uncommitted_git_changes() {
        let temp_dir = tempdir().expect("create temp dir");
        let repository = temp_dir.path().join("repo");

        run_git(temp_dir.path(), ["init", "repo"]);
        run_git(&repository, ["config", "user.email", "test@example.com"]);
        run_git(&repository, ["config", "user.name", "Test User"]);
        fs::write(repository.join("README.md"), "# Repo").expect("write readme");
        run_git(&repository, ["add", "README.md"]);
        run_git(&repository, ["commit", "-m", "initial"]);

        fs::write(repository.join("README.md"), "# Repo\n\nChanged").expect("modify readme");
        fs::write(
            repository.join("repo-explorer-status-fixture.txt"),
            "fixture",
        )
        .expect("write fixture");
        run_git(&repository, ["add", "repo-explorer-status-fixture.txt"]);

        let status = git_status_summary(&repository);

        assert_eq!(status.uncommitted_changes, 2);
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert!(!status.has_upstream);
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
