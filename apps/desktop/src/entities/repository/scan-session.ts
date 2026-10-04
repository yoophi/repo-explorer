import type {
  RepositoryScanProgress,
  RepositoryScanItem,
  RepositoryScanTerminal,
  ScanAcknowledgement,
  ScanRepositoriesRequest,
} from "./api";

export type ScanTransport = {
  start: (request: ScanRepositoriesRequest) => Promise<ScanAcknowledgement>;
  cancel: (scanId: string) => Promise<boolean>;
};

type Callbacks = {
  onProgress: (progress: RepositoryScanProgress) => void;
  onItem: (item: RepositoryScanItem) => void;
  onTerminal: (terminal: RepositoryScanTerminal) => void;
  onStartError: (error: unknown) => void;
  onCancelError: (error: unknown) => void;
};

type ActiveScan = {
  id: string;
  acknowledged: boolean;
  cancelRequested: boolean;
  cancelAttempt: number;
};

export class ScanSession {
  private active: ActiveScan | null = null;
  private disposed = false;
  private readonly transport: ScanTransport;
  private readonly callbacks: Callbacks;

  constructor(transport: ScanTransport, callbacks: Callbacks) {
    this.transport = transport;
    this.callbacks = callbacks;
  }

  start(request: ScanRepositoriesRequest): boolean {
    if (this.disposed || this.active) return false;
    const job: ActiveScan = {
      id: request.scanId,
      acknowledged: false,
      cancelRequested: false,
      cancelAttempt: 0,
    };
    this.active = job;
    void this.transport.start(request).then((ack) => {
      if (this.active !== job) return;
      if (ack.scanId !== job.id) throw new Error("Scan acknowledgement ID mismatch");
      job.acknowledged = true;
      if (job.cancelRequested) void this.sendCancel(job);
    }).catch((error: unknown) => {
      if (this.active !== job) return;
      this.active = null;
      if (!this.disposed) this.callbacks.onStartError(error);
    });
    return true;
  }

  cancel(): boolean {
    const job = this.active;
    if (!job) return false;
    job.cancelRequested = true;
    // This may arrive before registration. The ack path retries it.
    void this.sendCancel(job);
    return true;
  }

  progress(progress: RepositoryScanProgress): void {
    if (!this.disposed && progress.scanId === this.active?.id) {
      this.callbacks.onProgress(progress);
    }
  }

  item(item: RepositoryScanItem): void {
    if (!this.disposed && item.scanId === this.active?.id) {
      this.callbacks.onItem(item);
    }
  }

  terminal(terminal: RepositoryScanTerminal): void {
    if (terminal.scanId !== this.active?.id) return;
    this.active = null;
    if (!this.disposed) this.callbacks.onTerminal(terminal);
  }

  dispose(): void {
    this.disposed = true;
    this.cancel();
  }

  private async sendCancel(job: ActiveScan): Promise<void> {
    const attempt = ++job.cancelAttempt;
    try {
      await this.transport.cancel(job.id);
    } catch (error) {
      if (this.active !== job || attempt !== job.cancelAttempt) return;
      if (job.acknowledged) {
        job.cancelRequested = false;
        if (!this.disposed) this.callbacks.onCancelError(error);
      }
      // Before ack, keep the request and retry after registration.
    }
  }
}

export async function installScanSubscriptions(
  subscriptions: Array<Promise<() => void>>,
  isActive: () => boolean,
): Promise<() => void> {
  const unlisteners: Array<() => void> = [];
  let accepting = true;
  const installed = subscriptions.map((subscription) => subscription.then((unlisten) => {
    if (accepting && isActive()) unlisteners.push(unlisten);
    else unlisten();
  }));
  const release = () => {
    accepting = false;
    unlisteners.splice(0).forEach((unlisten) => unlisten());
  };
  try {
    await Promise.all(installed);
    if (!isActive()) release();
    return release;
  } catch (error) {
    release();
    throw error;
  }
}
