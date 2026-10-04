//! Filesystem, Git, and JSON outbound adapters for the application ports.
use crate::application::{
    check_cancelled, CatalogRepository, MetadataRepository, RepositoryInspector,
};
use crate::domain::{
    merge_repository_paths, preview_record_from_inspection, upsert_root, CatalogStore,
    ReadmeContent, RepositoryInspection, RepositoryMetadata, RepositoryRecord,
    RepositoryScanProgress, StreamingParentIndex, METADATA_FILE_NAME,
};
use crate::domain::{GitStatusSummary, TerminalApp};
use explorer_fs_core::{walk_directories, WalkError, WalkPolicy};
use explorer_json_store::{load_json, save_json, update_json};
use explorer_scan_job::CancellationToken;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
pub(crate) fn repositories_from_paths(
    paths: Vec<String>,
    last_seen_at: u64,
) -> io::Result<Vec<RepositoryRecord>> {
    repositories_from_paths_with_progress(paths, last_seen_at, None, false, |_| {}, |_| {})
}

pub(crate) fn repositories_from_paths_with_progress(
    paths: Vec<String>,
    last_seen_at: u64,
    token: Option<&CancellationToken>,
    emit_items: bool,
    mut on_progress: impl FnMut(RepositoryScanProgress),
    mut on_item: impl FnMut(RepositoryRecord),
) -> io::Result<Vec<RepositoryRecord>> {
    let mut parents = StreamingParentIndex::default();
    let inspections =
        inspect_repositories_with_progress(paths, token, &mut on_progress, &mut |inspections| {
            if emit_items {
                let index = inspections.len() - 1;
                let inspection = &inspections[index];
                let (parent_id, revisions) = parents.observe(inspection, index);
                let item = preview_record_from_inspection(
                    inspection,
                    last_seen_at,
                    parent_id,
                    shortest_relative_path,
                );
                on_item(item);
                for (prior_index, parent_id) in revisions {
                    let prior = &inspections[prior_index];
                    on_item(preview_record_from_inspection(
                        prior,
                        last_seen_at,
                        parent_id,
                        shortest_relative_path,
                    ));
                }
            }
        })?;
    Ok(crate::domain::records_from_inspections(
        inspections,
        last_seen_at,
        shortest_relative_path,
    ))
}

fn inspect_repositories_with_progress(
    paths: Vec<String>,
    token: Option<&CancellationToken>,
    on_progress: &mut impl FnMut(RepositoryScanProgress),
    on_inspected: &mut impl FnMut(&[RepositoryInspection]),
) -> io::Result<Vec<RepositoryInspection>> {
    let mut inspections = Vec::new();
    let total_paths = paths.len();

    for path in paths {
        if let Some(token) = token {
            check_cancelled(token)?;
        }
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
        if let Some(token) = token {
            check_cancelled(token)?;
        }
        on_inspected(&inspections);
    }

    Ok(inspections)
}

#[cfg(test)]
pub(crate) fn discover_git_repositories(
    root_path: &Path,
    max_depth: usize,
) -> io::Result<Vec<PathBuf>> {
    discover_git_repositories_with_progress(root_path, max_depth, None, |_| {})
}

pub(crate) fn discover_git_repositories_with_progress(
    root_path: &Path,
    max_depth: usize,
    token: Option<&CancellationToken>,
    mut on_progress: impl FnMut(RepositoryScanProgress),
) -> io::Result<Vec<PathBuf>> {
    let mut repositories = Vec::new();
    let mut visited_directories = 0;
    let result = walk_directories(
        root_path,
        WalkPolicy::RepositoryBfs { max_depth },
        &|| token.is_some_and(|token| token.is_cancelled()),
        &should_skip_directory,
        &mut |path, _depth| {
            visited_directories += 1;
            on_progress(RepositoryScanProgress {
                phase: "scanning",
                current_path: Some(path.to_string_lossy().to_string()),
                visited_directories,
                discovered_repositories: repositories.len(),
                message: None,
            });

            if is_git_repository(&path) {
                repositories.push(path.to_path_buf());
                on_progress(RepositoryScanProgress {
                    phase: "found",
                    current_path: Some(path.to_string_lossy().to_string()),
                    visited_directories,
                    discovered_repositories: repositories.len(),
                    message: Some("Git repository found".into()),
                });
            }

            Ok::<(), io::Error>(())
        },
    );
    match result {
        Ok(()) => {}
        Err(WalkError::Cancelled) => {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Repository scan cancelled",
            ))
        }
        Err(WalkError::Io(error) | WalkError::Callback(error)) => return Err(error),
    }

    repositories.sort();
    repositories.dedup();
    Ok(repositories)
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

pub(crate) fn read_readme(repository_path: &Path) -> io::Result<Option<ReadmeContent>> {
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

pub(crate) fn load_repository_metadata(repository_path: &Path) -> io::Result<RepositoryMetadata> {
    let path = repository_path.join(METADATA_FILE_NAME);
    Ok(load_json(path)?.unwrap_or_default())
}

pub(crate) fn save_repository_metadata(
    repository_path: &Path,
    metadata: &RepositoryMetadata,
) -> io::Result<()> {
    save_json(repository_path.join(METADATA_FILE_NAME), metadata)
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

pub(crate) fn update_catalog_store_at_path(
    path: &Path,
    update: impl FnOnce(&mut CatalogStore),
) -> io::Result<CatalogStore> {
    update_json(path, |stored: Option<CatalogStore>| {
        let mut store = stored.unwrap_or_default();
        update(&mut store);
        Ok((store.clone(), store))
    })
}

pub(crate) struct FilesystemGitInspector;

impl RepositoryInspector for FilesystemGitInspector {
    fn normalize_path(&self, path: &str) -> io::Result<PathBuf> {
        normalize_path(path)
    }
    fn is_git_repository(&self, path: &Path) -> bool {
        is_git_repository(path)
    }
    fn discover(
        &self,
        root: &Path,
        max_depth: usize,
        token: &CancellationToken,
        progress: &mut dyn FnMut(RepositoryScanProgress),
    ) -> io::Result<Vec<PathBuf>> {
        discover_git_repositories_with_progress(root, max_depth, Some(token), progress)
    }
    fn inspect(
        &self,
        paths: Vec<String>,
        last_seen_at: u64,
        token: Option<&CancellationToken>,
        sink: &mut dyn crate::application::ProgressSink,
    ) -> io::Result<Vec<RepositoryRecord>> {
        let emit_items = sink.wants_items();
        let sink = std::cell::RefCell::new(sink);
        repositories_from_paths_with_progress(
            paths,
            last_seen_at,
            token,
            emit_items,
            |progress| sink.borrow_mut().emit(progress),
            |item| sink.borrow_mut().emit_item(item),
        )
    }
}

pub(crate) struct JsonCatalog<F: Fn() -> io::Result<PathBuf>> {
    resolve_path: F,
}

impl<F: Fn() -> io::Result<PathBuf>> JsonCatalog<F> {
    pub(crate) fn new(resolve_path: F) -> Self {
        Self { resolve_path }
    }

    fn path_for_operation(&self) -> io::Result<PathBuf> {
        let path = (self.resolve_path)()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(path)
    }
}

impl<F: Fn() -> io::Result<PathBuf>> CatalogRepository for JsonCatalog<F> {
    fn load(&self) -> io::Result<CatalogStore> {
        let path = self.path_for_operation()?;
        Ok(load_json(&path)?.unwrap_or_default())
    }

    fn record_scan(
        &self,
        root: &Path,
        max_depth: usize,
        now: u64,
        paths: &[PathBuf],
    ) -> io::Result<()> {
        let path = self.path_for_operation()?;
        update_catalog_store_at_path(&path, |store| {
            upsert_root(store, root, max_depth, now);
            merge_repository_paths(store, paths.to_vec());
        })
        .map(|_| ())
    }

    fn add_repository(&self, path: &Path) -> io::Result<CatalogStore> {
        let catalog_path = self.path_for_operation()?;
        update_catalog_store_at_path(&catalog_path, |store| {
            merge_repository_paths(store, vec![path.to_path_buf()]);
        })
    }
}

pub(crate) struct JsonMetadata;

impl MetadataRepository for JsonMetadata {
    fn save(&self, path: &Path, metadata: &RepositoryMetadata) -> io::Result<()> {
        save_repository_metadata(path, metadata)
    }
}

pub(crate) fn git_status_summary(repository_path: &Path) -> GitStatusSummary {
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

#[cfg(target_os = "macos")]
pub(crate) fn open_terminal_at_path(path: &Path, terminal_app: TerminalApp) -> Result<(), String> {
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
pub(crate) fn open_terminal_at_path(
    _path: &Path,
    _terminal_app: TerminalApp,
) -> Result<(), String> {
    Err("Opening a selected terminal app is only supported on macOS.".into())
}

pub(crate) struct PlatformTerminalLauncher;
impl crate::application::TerminalLauncher for PlatformTerminalLauncher {
    fn open(&self, path: &Path, app: TerminalApp) -> Result<(), String> {
        open_terminal_at_path(path, app)
    }
}
