import type {
  RepositoryScanProgress,
  RepositoryScanItem,
  RepositoryScanTerminal,
  ScanAcknowledgement,
  ScanRepositoriesRequest,
} from "./api";
import { ScanLifecycle } from "@yoophi/scan-client";

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

export class ScanSession {
  private readonly lifecycle: ScanLifecycle<ScanRepositoriesRequest>;
  private readonly callbacks: Callbacks;

  constructor(transport: ScanTransport, callbacks: Callbacks) {
    this.callbacks = callbacks;
    this.lifecycle = new ScanLifecycle(transport, {
      onStartError: callbacks.onStartError,
      onCancelError: callbacks.onCancelError,
    }, { cancelBeforeAcknowledgement: true });
  }

  start(request: ScanRepositoriesRequest): boolean {
    return this.lifecycle.start(request);
  }

  cancel(): boolean { return this.lifecycle.cancel(); }

  progress(progress: RepositoryScanProgress): void {
    if (this.lifecycle.accepts(progress.scanId)) {
      this.callbacks.onProgress(progress);
    }
  }

  item(item: RepositoryScanItem): void {
    if (this.lifecycle.accepts(item.scanId)) {
      this.callbacks.onItem(item);
    }
  }

  terminal(terminal: RepositoryScanTerminal): void {
    if (this.lifecycle.finish(terminal.scanId)) this.callbacks.onTerminal(terminal);
  }

  dispose(): void { this.lifecycle.dispose(); }
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
