// TS-ONLY: Maps Rust's tokio::sync::Mutex to JS async serialization

/**
 * Async mutex for serializing operations across await points.
 * 1:1 equivalent of Rust's tokio::sync::Mutex<()>.
 */
export class AsyncMutex {
  private queue: Promise<void> = Promise.resolve();

  async acquire(): Promise<() => void> {
    let release!: () => void;
    const next = new Promise<void>((resolve) => {
      release = resolve;
    });
    const prev = this.queue;
    this.queue = next;
    await prev;
    return release;
  }
}
