import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Group, Panel, Separator } from "react-resizable-panels";
import {
  BookOpen,
  Check,
  ChevronRight,
  Copy,
  FileJson,
  FolderOpen,
  FolderGit2,
  GitBranch,
  MoreHorizontal,
  Pin,
  RefreshCw,
  Search,
  SquareTerminal,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@yoophi/ui/components/button";
import {
  getAppInfo,
  listRepositories,
  openRepositoryInFinder,
  openRepositoryInTerminal,
  scanRepositories,
  updateRepositoryMetadata,
  type RepositoryRecord,
  type RepositoryScanProgress,
  type TerminalApp,
} from "@/entities/repository";

type RepositoryTreeNode = {
  repository: RepositoryRecord;
  children: RepositoryTreeNode[];
};

const terminalOptions: { app: TerminalApp; label: string }[] = [
  { app: "terminal", label: "Terminal" },
  { app: "iterm2", label: "iTerm2" },
  { app: "ghostty", label: "Ghostty" },
  { app: "wezterm", label: "WezTerm" },
];

const repositoryScannedEvent = "repository_scanned";
const repositoryScanProgressEvent = "repository_scan_progress";

export function RepositoryPage() {
  const queryClient = useQueryClient();
  const [rootPath, setRootPath] = useState("/Users/yoophi/project");
  const [maxDepth, setMaxDepth] = useState(4);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedRepositoryId, setSelectedRepositoryId] = useState<string | null>(null);
  const [expandedRepositoryIds, setExpandedRepositoryIds] = useState<Set<string>>(() => new Set());
  const [description, setDescription] = useState("");
  const [tagsText, setTagsText] = useState("");
  const [pinned, setPinned] = useState(false);
  const [scanProgress, setScanProgress] = useState<RepositoryScanProgress | null>(null);
  const [repositoryActionError, setRepositoryActionError] = useState<string | null>(null);
  const [repositoryActionPending, setRepositoryActionPending] = useState<string | null>(null);
  const [repositoryCopyMessage, setRepositoryCopyMessage] = useState<string | null>(null);
  const [repositoryCopyError, setRepositoryCopyError] = useState<string | null>(null);
  const [scannedRepositoryIds, setScannedRepositoryIds] = useState<Set<string> | null>(null);

  const appInfoQuery = useQuery({
    queryKey: ["app-info"],
    queryFn: getAppInfo,
  });
  const repositoriesQuery = useQuery({
    queryKey: ["repositories"],
    queryFn: listRepositories,
  });
  const repositoryCatalog = repositoriesQuery.data ?? [];
  const repositories = useMemo(
    () =>
      scannedRepositoryIds
        ? repositoryCatalog.filter((repository) => scannedRepositoryIds.has(repository.id))
        : repositoryCatalog,
    [repositoryCatalog, scannedRepositoryIds],
  );
  const selectedRepository = useMemo(
    () => repositories.find((repository) => repository.id === selectedRepositoryId) ?? null,
    [repositories, selectedRepositoryId],
  );
  const tree = useMemo(() => buildRepositoryTree(repositories, searchQuery), [repositories, searchQuery]);
  const visibleRepositoryIds = useMemo(() => collectTreeIds(tree), [tree]);
  const scanMutation = useMutation({
    mutationFn: async () => {
      await queryClient.cancelQueries({ queryKey: ["repositories"] });
      setScannedRepositoryIds(new Set());
      queryClient.setQueryData(["repositories"], []);
      setSelectedRepositoryId(null);
      setScanProgress({
        phase: "started",
        currentPath: rootPath,
        visitedDirectories: 0,
        discoveredRepositories: 0,
        message: "Starting repository scan",
      });
      return scanRepositories({ rootPath, maxDepth });
    },
    onSuccess: (repositories) => {
      setScannedRepositoryIds(new Set(repositories.map((repository) => repository.id)));
      queryClient.setQueryData(["repositories"], repositories);
      setExpandedRepositoryIds(new Set(repositories.map((repository) => repository.id)));
      setSelectedRepositoryId((currentId) => currentId ?? repositories[0]?.id ?? null);
    },
  });
  const updateMetadataMutation = useMutation({
    mutationFn: () =>
      updateRepositoryMetadata({
        repositoryId: selectedRepository?.id ?? "",
        description,
        tags: tagsText.split(","),
        pinned,
      }),
    onSuccess: (updatedRepository) => {
      queryClient.setQueryData<RepositoryRecord[]>(["repositories"], (currentRepositories = []) =>
        currentRepositories.map((repository) =>
          repository.id === updatedRepository.id ? updatedRepository : repository,
        ),
      );
      void queryClient.invalidateQueries({ queryKey: ["repositories"] });
    },
  });

  useEffect(() => {
    let mounted = true;
    let unlistenProgress: (() => void) | undefined;
    let unlistenRepositoryScanned: (() => void) | undefined;

    void listen<RepositoryScanProgress>(repositoryScanProgressEvent, (event) => {
      if (mounted) {
        setScanProgress(event.payload);
      }
    }).then((unsubscribe) => {
      if (mounted) {
        unlistenProgress = unsubscribe;
      } else {
        unsubscribe();
      }
    });

    void listen<RepositoryRecord>(repositoryScannedEvent, (event) => {
      if (!mounted) {
        return;
      }

      const repository = event.payload;
      queryClient.setQueryData<RepositoryRecord[]>(["repositories"], (currentRepositories = []) =>
        upsertRepository(currentRepositories, repository),
      );
      setScannedRepositoryIds((currentIds) => {
        const nextIds = new Set(currentIds ?? []);
        nextIds.add(repository.id);
        return nextIds;
      });
      setExpandedRepositoryIds((currentIds) => {
        const nextIds = new Set(currentIds);
        nextIds.add(repository.id);
        if (repository.parentId) {
          nextIds.add(repository.parentId);
        }
        return nextIds;
      });
      setSelectedRepositoryId((currentId) => currentId ?? repository.id);
    }).then((unsubscribe) => {
      if (mounted) {
        unlistenRepositoryScanned = unsubscribe;
      } else {
        unsubscribe();
      }
    });

    return () => {
      mounted = false;
      unlistenProgress?.();
      unlistenRepositoryScanned?.();
    };
  }, [queryClient]);

  useEffect(() => {
    if (!selectedRepository && repositories[0]) {
      setSelectedRepositoryId(repositories[0].id);
    }
  }, [repositories, selectedRepository]);

  useEffect(() => {
    setRepositoryActionError(null);
    setRepositoryActionPending(null);
    setRepositoryCopyMessage(null);
    setRepositoryCopyError(null);
  }, [selectedRepositoryId]);

  useEffect(() => {
    if (!selectedRepositoryId || visibleRepositoryIds.has(selectedRepositoryId)) {
      return;
    }

    setSelectedRepositoryId(visibleRepositoryIds.values().next().value ?? null);
  }, [selectedRepositoryId, visibleRepositoryIds]);

  useEffect(() => {
    if (!selectedRepository) {
      setDescription("");
      setTagsText("");
      setPinned(false);
      return;
    }

    setDescription(selectedRepository.metadata.description);
    setTagsText(selectedRepository.metadata.tags.join(", "));
    setPinned(selectedRepository.metadata.pinned);
  }, [selectedRepository]);

  useEffect(() => {
    if (searchQuery.trim()) {
      setExpandedRepositoryIds(new Set(repositories.map((repository) => repository.id)));
    }
  }, [repositories, searchQuery]);

  async function selectRootDirectory() {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: "Select repository root",
    });

    if (typeof selectedPath === "string") {
      setRootPath(selectedPath);
    }
  }

  async function handleOpenRepositoryInFinder(repository: RepositoryRecord) {
    setRepositoryActionPending("finder");
    setRepositoryActionError(null);

    try {
      await openRepositoryInFinder(repository);
    } catch (error) {
      setRepositoryActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setRepositoryActionPending(null);
    }
  }

  async function handleOpenRepositoryInTerminal(repository: RepositoryRecord, terminalApp: TerminalApp) {
    setRepositoryActionPending(terminalApp);
    setRepositoryActionError(null);

    try {
      await openRepositoryInTerminal({
        repositoryId: repository.id,
        terminalApp,
      });
    } catch (error) {
      setRepositoryActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setRepositoryActionPending(null);
    }
  }

  async function handleCopyRepositoryPath(repository: RepositoryRecord) {
    setRepositoryCopyMessage(null);
    setRepositoryCopyError(null);

    try {
      await copyTextToClipboard(repository.path);
      setRepositoryCopyMessage("디렉토리 경로를 클립보드에 복사했습니다.");
      window.setTimeout(() => {
        setRepositoryCopyMessage(null);
      }, 2000);
    } catch (error) {
      setRepositoryCopyError(
        error instanceof Error ? error.message : "디렉토리 경로를 복사하지 못했습니다.",
      );
    }
  }

  function toggleExpanded(repositoryId: string) {
    setExpandedRepositoryIds((currentIds) => {
      const nextIds = new Set(currentIds);
      if (nextIds.has(repositoryId)) {
        nextIds.delete(repositoryId);
      } else {
        nextIds.add(repositoryId);
      }

      return nextIds;
    });
  }

  const scanError = scanMutation.error instanceof Error ? scanMutation.error.message : null;
  const listError =
    repositoriesQuery.error instanceof Error ? repositoriesQuery.error.message : null;
  const updateError =
    updateMetadataMutation.error instanceof Error ? updateMetadataMutation.error.message : null;

  return (
    <main className="h-screen bg-background text-foreground">
      <Group className="h-full" id="repository-workspace-layout" orientation="horizontal">
        <Panel
          className="min-w-0"
          defaultSize="420px"
          groupResizeBehavior="preserve-pixel-size"
          id="repository-navigation"
          maxSize="640px"
          minSize="280px"
        >
          <aside className="flex h-full min-h-0 flex-col bg-sidebar">
            <div className="border-b p-4">
              <div className="flex items-center gap-2 text-sm font-semibold">
                <FolderGit2 className="size-4" />
                Repo Explorer
              </div>

              <div className="mt-4 grid gap-3">
                <label className="grid gap-1 text-xs font-medium text-muted-foreground" htmlFor="root-path">
                  Directory
                  <div className="flex gap-2">
                    <input
                      id="root-path"
                      className="h-8 min-w-0 flex-1 rounded-md border bg-background px-2 text-sm font-normal text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
                      value={rootPath}
                      onChange={(event) => setRootPath(event.target.value)}
                    />
                    <Button variant="outline" size="icon" onClick={() => void selectRootDirectory()}>
                      <FolderGit2 className="size-4" />
                    </Button>
                  </div>
                </label>

                <div className="flex items-end gap-2">
                  <label className="grid flex-1 gap-1 text-xs font-medium text-muted-foreground" htmlFor="max-depth">
                    Max depth
                    <input
                      id="max-depth"
                      className="h-8 rounded-md border bg-background px-2 text-sm font-normal text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
                      min={0}
                      max={20}
                      type="number"
                      value={maxDepth}
                      onChange={(event) => setMaxDepth(Number(event.target.value))}
                    />
                  </label>
                  <Button disabled={!rootPath || scanMutation.isPending} onClick={() => scanMutation.mutate()}>
                    <Search className="size-4" />
                    Scan
                  </Button>
                </div>

                <label className="grid gap-1 text-xs font-medium text-muted-foreground" htmlFor="repo-search">
                  Search
                  <input
                    id="repo-search"
                    className="h-8 rounded-md border bg-background px-2 text-sm font-normal text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
                    placeholder="name, path, tag, origin, README"
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                  />
                </label>

                {scanError ? <div className="text-xs text-red-600">{scanError}</div> : null}
                {scanProgress ? (
                  <ScanProgressPanel progress={scanProgress} isScanning={scanMutation.isPending} />
                ) : null}
              </div>
            </div>

            <div className="flex items-center justify-between border-b px-4 py-2">
              <span className="text-xs font-medium text-muted-foreground">
                {visibleRepositoryIds.size} shown / {repositories.length} repos
              </span>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => void repositoriesQuery.refetch()}
                disabled={repositoriesQuery.isFetching}
              >
                <RefreshCw className="size-4" />
              </Button>
            </div>

            {listError ? <div className="p-4 text-xs text-red-600">{listError}</div> : null}

            <div className="min-h-0 flex-1 overflow-auto">
              {tree.length > 0 ? (
                tree.map((node) =>
                  renderRepositoryTreeNode({
                    node,
                    depth: 0,
                    expandedRepositoryIds,
                    selectedRepositoryId,
                    onSelect: setSelectedRepositoryId,
                    onToggle: toggleExpanded,
                  }),
                )
              ) : (
                <div className="p-4 text-sm text-muted-foreground">No repositories found.</div>
              )}
            </div>
          </aside>
        </Panel>

        <Separator
          className="w-1 cursor-col-resize bg-border transition-colors hover:bg-ring"
          id="repository-layout-resizer"
        />

        <Panel className="min-w-0" id="repository-detail" minSize="360px">
          <section className="flex h-full min-h-0 min-w-0 flex-col">
            <header className="flex h-12 shrink-0 items-center justify-between border-b px-4">
              <div>
                <h1 className="text-sm font-semibold">Repository Workspace</h1>
                <p className="text-xs text-muted-foreground">
                  {appInfoQuery.data
                    ? `${appInfoQuery.data.name} ${appInfoQuery.data.version}`
                    : "Local repository catalog"}
                </p>
              </div>
              <Button variant="outline" size="icon" onClick={() => void appInfoQuery.refetch()}>
                <RefreshCw className="size-4" />
              </Button>
            </header>

            <div className="min-h-0 flex-1 overflow-auto p-6">
              {selectedRepository ? (
                <RepositoryDetail
                  actionError={repositoryActionError}
                  actionPending={repositoryActionPending}
                  copyError={repositoryCopyError}
                  copyMessage={repositoryCopyMessage}
                  description={description}
                  pinned={pinned}
                  repository={selectedRepository}
                  tagsText={tagsText}
                  updateError={updateError}
                  updatePending={updateMetadataMutation.isPending}
                  onDescriptionChange={setDescription}
                  onCopyPath={handleCopyRepositoryPath}
                  onOpenFinder={handleOpenRepositoryInFinder}
                  onOpenTerminal={handleOpenRepositoryInTerminal}
                  onPinnedChange={setPinned}
                  onSave={() => updateMetadataMutation.mutate()}
                  onTagsTextChange={setTagsText}
                />
              ) : (
                <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                  Scan a directory to build the repository tree.
                </div>
              )}
            </div>
          </section>
        </Panel>
      </Group>
    </main>
  );
}

function ScanProgressPanel({
  isScanning,
  progress,
}: {
  isScanning: boolean;
  progress: RepositoryScanProgress;
}) {
  return (
    <div className="rounded-md border bg-background p-3 text-xs">
      <div className="flex items-center justify-between gap-2">
        <span className="font-medium">
          {isScanning || progress.phase !== "finished" ? "Scanning repositories" : "Scan complete"}
        </span>
        <span className="rounded-sm bg-secondary px-1.5 py-0.5">{progress.phase}</span>
      </div>
      <div className="mt-2 grid grid-cols-2 gap-2 text-muted-foreground">
        <div>Visited: {progress.visitedDirectories}</div>
        <div>Found: {progress.discoveredRepositories}</div>
      </div>
      {progress.currentPath ? (
        <div className="mt-2 break-all text-muted-foreground">{progress.currentPath}</div>
      ) : null}
      {progress.message ? <div className="mt-2 text-muted-foreground">{progress.message}</div> : null}
    </div>
  );
}

function RepositoryDetail({
  actionError,
  actionPending,
  copyError,
  copyMessage,
  description,
  pinned,
  repository,
  tagsText,
  updateError,
  updatePending,
  onDescriptionChange,
  onCopyPath,
  onOpenFinder,
  onOpenTerminal,
  onPinnedChange,
  onSave,
  onTagsTextChange,
}: {
  actionError: string | null;
  actionPending: string | null;
  copyError: string | null;
  copyMessage: string | null;
  description: string;
  pinned: boolean;
  repository: RepositoryRecord;
  tagsText: string;
  updateError: string | null;
  updatePending: boolean;
  onDescriptionChange: (description: string) => void;
  onCopyPath: (repository: RepositoryRecord) => Promise<void>;
  onOpenFinder: (repository: RepositoryRecord) => Promise<void>;
  onOpenTerminal: (repository: RepositoryRecord, terminalApp: TerminalApp) => Promise<void>;
  onPinnedChange: (pinned: boolean) => void;
  onSave: () => void;
  onTagsTextChange: (tagsText: string) => void;
}) {
  const [actionsOpen, setActionsOpen] = useState(false);

  async function runRepositoryAction(action: () => Promise<void>) {
    setActionsOpen(false);
    await action();
  }

  return (
    <div className="grid max-w-5xl gap-5">
      <div>
        <div className="flex min-w-0 items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2">
              <h2 className="truncate text-xl font-semibold">{repository.name}</h2>
              {repository.metadata.pinned ? <Pin className="size-4 shrink-0" /> : null}
              {repository.isWorktree ? (
                <span className="shrink-0 rounded-sm bg-secondary px-1.5 py-0.5 text-xs">worktree</span>
              ) : null}
            </div>
          </div>

          <div className="relative shrink-0">
            <Button
              aria-expanded={actionsOpen}
              aria-haspopup="menu"
              variant="outline"
              size="icon-sm"
              onClick={() => setActionsOpen((current) => !current)}
            >
              <MoreHorizontal className="size-4" />
            </Button>

            {actionsOpen ? (
              <div
                className="absolute right-0 top-9 z-10 w-52 overflow-hidden rounded-md border bg-card p-1 text-sm shadow-lg"
                role="menu"
              >
                <button
                  className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left hover:bg-muted disabled:opacity-50"
                  disabled={actionPending !== null}
                  role="menuitem"
                  type="button"
                  onClick={() => void runRepositoryAction(() => onOpenFinder(repository))}
                >
                  <FolderOpen className="size-4" />
                  Finder로 열기
                </button>

                <div className="my-1 border-t" />
                <div className="px-2 py-1 text-xs font-medium text-muted-foreground">터미널로 열기</div>
                {terminalOptions.map((option) => (
                  <button
                    className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left hover:bg-muted disabled:opacity-50"
                    disabled={actionPending !== null}
                    key={option.app}
                    role="menuitem"
                    type="button"
                    onClick={() =>
                      void runRepositoryAction(() => onOpenTerminal(repository, option.app))
                    }
                  >
                    <SquareTerminal className="size-4" />
                    {option.label}
                  </button>
                ))}
              </div>
            ) : null}
          </div>
        </div>
        <div className="mt-2 flex min-w-0 flex-wrap items-center gap-2">
          <div className="min-w-0 flex-1 break-all text-sm text-muted-foreground">{repository.path}</div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => void onCopyPath(repository)}
            aria-label="디렉토리 절대 경로 복사"
          >
            <Copy className="size-4" />
            경로 복사
          </Button>
        </div>
        {copyMessage ? <div className="mt-2 text-xs text-emerald-600">{copyMessage}</div> : null}
        {copyError ? <div className="mt-2 text-xs text-red-600">{copyError}</div> : null}
        {repository.originUrl ? (
          <div className="mt-2 flex items-center gap-2 break-all text-sm">
            <GitBranch className="size-4 shrink-0" />
            <span>{repository.originUrl}</span>
          </div>
        ) : (
          <div className="mt-2 text-sm text-muted-foreground">No git origin configured.</div>
        )}
        {actionError ? <div className="mt-2 text-xs text-red-600">{actionError}</div> : null}
      </div>

      <div className="grid gap-4 rounded-md border bg-card p-4">
        <div className="flex items-center gap-2 text-sm font-semibold">
          <FileJson className="size-4" />
          Metadata
        </div>
        <div className="break-all text-xs text-muted-foreground">{repository.metadataPath}</div>

        <label className="grid gap-1 text-sm font-medium">
          Description
          <textarea
            className="min-h-24 resize-y rounded-md border bg-background p-2 text-sm font-normal outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
            value={description}
            onChange={(event) => onDescriptionChange(event.target.value)}
          />
        </label>

        <label className="grid gap-1 text-sm font-medium">
          Tags
          <input
            className="h-8 rounded-md border bg-background px-2 text-sm font-normal outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
            placeholder="client, archived, infra"
            value={tagsText}
            onChange={(event) => onTagsTextChange(event.target.value)}
          />
        </label>

        <label className="flex items-center gap-2 text-sm">
          <input
            checked={pinned}
            className="size-4"
            type="checkbox"
            onChange={(event) => onPinnedChange(event.target.checked)}
          />
          Pinned
        </label>

        {updateError ? <div className="text-xs text-red-600">{updateError}</div> : null}

        <div>
          <Button disabled={updatePending} onClick={onSave}>
            <Check className="size-4" />
            Save metadata
          </Button>
        </div>
      </div>

      <div className="rounded-md border bg-card">
        <div className="flex items-center gap-2 border-b px-4 py-3 text-sm font-semibold">
          <BookOpen className="size-4" />
          README
        </div>
        {repository.readme ? (
          <div>
            <div className="border-b px-4 py-2 text-xs text-muted-foreground">
              {repository.readme.path}
            </div>
            <pre className="max-h-[520px] overflow-auto whitespace-pre-wrap p-4 text-sm leading-6">
              {repository.readme.content}
            </pre>
          </div>
        ) : (
          <div className="p-4 text-sm text-muted-foreground">
            No README found. Use metadata to describe this repository.
          </div>
        )}
      </div>
    </div>
  );
}

function renderRepositoryTreeNode({
  node,
  depth,
  expandedRepositoryIds,
  selectedRepositoryId,
  onSelect,
  onToggle,
}: {
  node: RepositoryTreeNode;
  depth: number;
  expandedRepositoryIds: Set<string>;
  selectedRepositoryId: string | null;
  onSelect: (repositoryId: string) => void;
  onToggle: (repositoryId: string) => void;
}) {
  const { repository } = node;
  const isSelected = repository.id === selectedRepositoryId;
  const isExpanded = expandedRepositoryIds.has(repository.id);
  const hasChildren = node.children.length > 0;
  const linkedWorktreeCount = node.children.filter(
    (childNode) => childNode.repository.isWorktree && childNode.repository.parentId === repository.id,
  ).length;

  return (
    <div key={repository.id}>
      <div
        className={`flex w-full items-start gap-1 border-b py-2 pr-3 text-left text-sm transition-colors ${
          isSelected ? "bg-background" : "hover:bg-muted"
        }`}
        style={{ paddingLeft: `${12 + depth * 18}px` }}
      >
        <button
          className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-sm hover:bg-secondary"
          disabled={!hasChildren}
          type="button"
          onClick={() => onToggle(repository.id)}
        >
          {hasChildren ? (
            <ChevronRight className={`size-4 transition-transform ${isExpanded ? "rotate-90" : ""}`} />
          ) : null}
        </button>
        <button className="min-w-0 flex-1 text-left" type="button" onClick={() => onSelect(repository.id)}>
          <div className="flex min-w-0 items-center gap-2">
            <div className="flex min-w-0 flex-1 items-center gap-2">
              {repository.metadata.pinned ? <Pin className="size-3.5 shrink-0" /> : null}
              {repository.isWorktree ? <GitBranch className="size-3.5 shrink-0" /> : null}
              <span className="truncate font-medium">{repository.name}</span>
            </div>
            {linkedWorktreeCount > 0 ? (
              <span className="shrink-0 rounded-sm bg-secondary px-1.5 py-0.5 text-[11px] text-secondary-foreground">
                worktree {linkedWorktreeCount}
              </span>
            ) : null}
            {repository.isWorktree ? (
              <span className="shrink-0 rounded-sm border px-1.5 py-0.5 text-[11px] text-muted-foreground">
                link
              </span>
            ) : null}
            {repository.originUrl ? (
              <span className="shrink-0 rounded-sm border px-1.5 py-0.5 text-[11px] text-muted-foreground">
                origin
              </span>
            ) : null}
            {repository.gitStatus.uncommittedChanges > 0 ? (
              <span className="shrink-0 rounded-sm border border-amber-300/70 bg-amber-50 px-1.5 py-0.5 text-[11px] text-amber-800">
                dirty {repository.gitStatus.uncommittedChanges}
              </span>
            ) : null}
            {repository.gitStatus.ahead > 0 ? (
              <span className="shrink-0 rounded-sm border px-1.5 py-0.5 text-[11px] text-muted-foreground">
                ahead {repository.gitStatus.ahead}
              </span>
            ) : null}
            {repository.gitStatus.behind > 0 ? (
              <span className="shrink-0 rounded-sm border px-1.5 py-0.5 text-[11px] text-muted-foreground">
                behind {repository.gitStatus.behind}
              </span>
            ) : null}
          </div>
          <div className="mt-1 truncate text-xs text-muted-foreground">{repository.relativePath}</div>
          {repository.originUrl ? (
            <div className="mt-1 truncate text-xs text-muted-foreground">{repository.originUrl}</div>
          ) : null}
          {repository.metadata.tags.length > 0 ? (
            <div className="mt-2 flex flex-wrap gap-1">
              {repository.metadata.tags.map((tag) => (
                <span className="rounded-sm bg-secondary px-1.5 py-0.5 text-[11px]" key={tag}>
                  {tag}
                </span>
              ))}
            </div>
          ) : null}
        </button>
      </div>
      {isExpanded
        ? node.children.map((childNode) =>
            renderRepositoryTreeNode({
              node: childNode,
              depth: depth + 1,
              expandedRepositoryIds,
              selectedRepositoryId,
              onSelect,
              onToggle,
            }),
          )
        : null}
    </div>
  );
}

function buildRepositoryTree(repositories: RepositoryRecord[], searchQuery: string) {
  const query = searchQuery.trim().toLowerCase();
  const nodesById = new Map<string, RepositoryTreeNode>(
    repositories.map((repository) => [repository.id, { repository, children: [] }]),
  );
  const roots: RepositoryTreeNode[] = [];

  for (const repository of repositories) {
    const node = nodesById.get(repository.id);
    if (!node) {
      continue;
    }

    const parentId = repository.parentId ?? nearestPathParentId(repository, repositories);
    const parentNode = parentId ? nodesById.get(parentId) : null;
    if (parentNode && parentNode.repository.id !== repository.id) {
      parentNode.children.push(node);
    } else {
      roots.push(node);
    }
  }

  sortTreeNodes(roots);

  if (!query) {
    return roots;
  }

  return filterTreeNodes(roots, query);
}

function nearestPathParentId(repository: RepositoryRecord, repositories: RepositoryRecord[]) {
  return repositories
    .filter((candidate) => repository.id !== candidate.id && repository.path.startsWith(`${candidate.path}/`))
    .sort((left, right) => right.path.length - left.path.length)[0]?.id;
}

function filterTreeNodes(nodes: RepositoryTreeNode[], query: string): RepositoryTreeNode[] {
  return nodes.flatMap((node) => {
    const children = filterTreeNodes(node.children, query);
    if (repositoryMatches(node.repository, query) || children.length > 0) {
      return [{ ...node, children }];
    }

    return [];
  });
}

function repositoryMatches(repository: RepositoryRecord, query: string) {
  return [
    repository.name,
    repository.path,
    repository.relativePath,
    repository.originUrl ?? "",
    repository.metadata.description,
    repository.metadata.tags.join(" "),
    repository.readme?.content ?? "",
  ]
    .join(" ")
    .toLowerCase()
    .includes(query);
}

function upsertRepository(repositories: RepositoryRecord[], repository: RepositoryRecord) {
  const repositoryIndex = repositories.findIndex((currentRepository) => currentRepository.id === repository.id);
  if (repositoryIndex === -1) {
    return [...repositories, repository];
  }

  return repositories.map((currentRepository, index) => (index === repositoryIndex ? repository : currentRepository));
}

async function copyTextToClipboard(text: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();

  try {
    if (!document.execCommand("copy")) {
      throw new Error("디렉토리 경로를 복사하지 못했습니다.");
    }
  } finally {
    document.body.removeChild(textarea);
  }
}

function sortTreeNodes(nodes: RepositoryTreeNode[]) {
  nodes.sort((left, right) =>
    Number(right.repository.metadata.pinned) - Number(left.repository.metadata.pinned) ||
    left.repository.name.localeCompare(right.repository.name) ||
    left.repository.path.localeCompare(right.repository.path),
  );

  for (const node of nodes) {
    sortTreeNodes(node.children);
  }
}

function collectTreeIds(nodes: RepositoryTreeNode[]) {
  const ids = new Set<string>();
  const visit = (node: RepositoryTreeNode) => {
    ids.add(node.repository.id);
    node.children.forEach(visit);
  };

  nodes.forEach(visit);
  return ids;
}
