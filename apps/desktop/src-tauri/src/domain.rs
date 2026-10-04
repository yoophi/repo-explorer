//! Repository data contracts and pure catalog/metadata rules.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub(crate) const METADATA_FILE_NAME: &str = ".repo-explorer.json";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanRepositoriesRequest {
    pub(crate) scan_id: String,
    pub(crate) root_path: String,
    pub(crate) max_depth: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateRepositoryMetadataRequest {
    pub(crate) repository_id: String,
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
    pub(crate) pinned: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogStore {
    pub(crate) roots: Vec<CatalogRoot>,
    pub(crate) repository_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogRoot {
    pub(crate) path: String,
    pub(crate) max_depth: usize,
    pub(crate) last_scanned_at: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryMetadata {
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
    pub(crate) pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) relative_path: String,
    pub(crate) parent_id: Option<String>,
    pub(crate) is_worktree: bool,
    pub(crate) origin_url: Option<String>,
    pub(crate) git_status: GitStatusSummary,
    pub(crate) readme: Option<ReadmeContent>,
    pub(crate) metadata: RepositoryMetadata,
    pub(crate) metadata_path: String,
    pub(crate) last_seen_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadmeContent {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RepositoryInspection {
    pub(crate) path: PathBuf,
    pub(crate) git_dir: Option<PathBuf>,
    pub(crate) common_git_dir: Option<PathBuf>,
    pub(crate) origin_url: Option<String>,
    pub(crate) git_status: GitStatusSummary,
    pub(crate) readme: Option<ReadmeContent>,
    pub(crate) metadata: RepositoryMetadata,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryScanProgress {
    pub(crate) phase: &'static str,
    pub(crate) current_path: Option<String>,
    pub(crate) visited_directories: usize,
    pub(crate) discovered_repositories: usize,
    pub(crate) message: Option<String>,
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

pub(crate) fn upsert_root(
    store: &mut CatalogStore,
    path: &Path,
    max_depth: usize,
    last_scanned_at: u64,
) {
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

pub(crate) fn merge_repository_paths(store: &mut CatalogStore, paths: Vec<PathBuf>) {
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

pub(crate) fn pathbufs_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

pub(crate) fn sort_repositories(repositories: &mut [RepositoryRecord]) {
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

pub(crate) fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut tags = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();

    tags.sort();
    tags.dedup();
    tags
}

/// Assemble inspected repositories without depending on Git or the filesystem.
pub(crate) fn records_from_inspections(
    inspections: Vec<RepositoryInspection>,
    last_seen_at: u64,
    relative_path: impl Fn(&Path) -> String,
) -> Vec<RepositoryRecord> {
    let parent_ids = worktree_parent_ids(&inspections);
    let mut repositories = inspections
        .into_iter()
        .map(|inspection| {
            let parent_id = parent_ids
                .get(&inspection.path.to_string_lossy().to_string())
                .cloned();
            record_from_inspection(inspection, last_seen_at, parent_id, &relative_path)
        })
        .collect::<Vec<_>>();

    sort_repositories(&mut repositories);
    repositories
}

pub(crate) fn preview_record_from_inspection(
    inspection: &RepositoryInspection,
    last_seen_at: u64,
    parent_id: Option<String>,
    relative_path: impl Fn(&Path) -> String,
) -> RepositoryRecord {
    record_from_inspection(inspection.clone(), last_seen_at, parent_id, &relative_path)
}

fn record_from_inspection(
    inspection: RepositoryInspection,
    last_seen_at: u64,
    parent_id: Option<String>,
    relative_path: &impl Fn(&Path) -> String,
) -> RepositoryRecord {
    let path_string = inspection.path.to_string_lossy().to_string();
    let name = inspection
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_string();
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
    RepositoryRecord {
        id: path_string.clone(),
        name,
        path: path_string,
        relative_path: relative_path(&inspection.path),
        parent_id,
        is_worktree,
        origin_url: inspection.origin_url,
        git_status: inspection.git_status,
        readme: inspection.readme,
        metadata: inspection.metadata,
        metadata_path,
        last_seen_at,
    }
}

#[derive(Default)]
pub(crate) struct StreamingParentIndex {
    mains_by_common_git_dir: HashMap<PathBuf, String>,
    worktrees_by_common_git_dir: HashMap<PathBuf, Vec<(usize, String)>>,
}

impl StreamingParentIndex {
    /// Returns this item's known parent and earlier worktrees whose parent changed.
    pub(crate) fn observe(
        &mut self,
        inspection: &RepositoryInspection,
        index: usize,
    ) -> (Option<String>, Vec<(usize, Option<String>)>) {
        let (Some(git_dir), Some(common_git_dir)) =
            (&inspection.git_dir, &inspection.common_git_dir)
        else {
            return (None, Vec::new());
        };
        let repository_id = inspection.path.to_string_lossy().to_string();
        if git_dir == common_git_dir {
            self.mains_by_common_git_dir
                .insert(common_git_dir.clone(), repository_id.clone());
            return (
                None,
                self.worktrees_by_common_git_dir
                    .get(common_git_dir)
                    .map(|worktrees| {
                        worktrees
                            .iter()
                            .map(|(index, id)| {
                                (
                                    *index,
                                    (id != &repository_id).then(|| repository_id.clone()),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            );
        }
        self.worktrees_by_common_git_dir
            .entry(common_git_dir.clone())
            .or_default()
            .push((index, repository_id.clone()));
        let parent = self
            .mains_by_common_git_dir
            .get(common_git_dir)
            .filter(|parent| *parent != &repository_id)
            .cloned();
        (parent, Vec::new())
    }
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

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitStatusSummary {
    pub(crate) uncommitted_changes: usize,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
    pub(crate) has_upstream: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenRepositoryInTerminalRequest {
    pub(crate) repository_id: String,
    pub(crate) terminal_app: TerminalApp,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TerminalApp {
    Terminal,
    Iterm2,
    Ghostty,
    Wezterm,
}
