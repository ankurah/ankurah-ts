// MIRRORS: ankurah/tests/tests/basic.rs
// Integration test: create, edit, subscribe, commit lifecycle

import { describe, expect, test } from 'bun:test';
import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { CallbackObserver } from '@ankurah/signals';
import { Node } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';

// ── Model ──
// Mirrors: common.rs `struct Album { name: String, year: String }`
// Default active type for String in Rust is YrsString.
const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

// ── TestWatcher ──
// Mirrors: common.rs TestWatcher<T, U> — accumulates notifications with async waiting.
// Divergence: Uses Promise-based waiting instead of tokio::sync::Notify [E8].

class TestWatcher<T, U = T> {
  private changes: T[] = [];
  private waiters: Array<() => void> = [];
  private transform: (item: T) => U;

  constructor(transform?: (item: T) => U) {
    this.transform = transform ?? ((x: T) => x as unknown as U);
  }

  static new<T>(): TestWatcher<T, T> {
    return new TestWatcher<T, T>();
  }

  static withTransform<T, U>(transform: (item: T) => U): TestWatcher<T, U> {
    return new TestWatcher<T, U>(transform);
  }

  notify(item: T): void {
    this.changes.push(item);
    // Wake all waiters
    const waiters = this.waiters.splice(0);
    for (const w of waiters) w();
  }

  count(): number {
    return this.changes.length;
  }

  drain(): U[] {
    return this.changes.splice(0).map(item => this.transform(item));
  }

  async waitForCount(count: number, timeoutMs = 10000): Promise<boolean> {
    if (this.changes.length >= count) return true;
    return new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => resolve(false), timeoutMs);
      const check = () => {
        if (this.changes.length >= count) {
          clearTimeout(timer);
          resolve(true);
        } else {
          this.waiters.push(check);
        }
      };
      this.waiters.push(check);
    });
  }

  async takeOne(timeoutMs = 10000): Promise<U> {
    const ok = await this.waitForCount(1, timeoutMs);
    if (!ok || this.changes.length === 0) {
      throw new Error(`takeOne() timed out waiting for items (waited ${timeoutMs}ms, got ${this.changes.length} items)`);
    }
    const item = this.changes.splice(0, 1)[0];
    return this.transform(item);
  }

  async take(count: number, timeoutMs = 10000): Promise<U[]> {
    const ok = await this.waitForCount(count, timeoutMs);
    if (!ok) {
      throw new Error(`take(${count}) timed out (waited ${timeoutMs}ms, got ${this.changes.length} items)`);
    }
    return this.changes.splice(0, count).map(item => this.transform(item));
  }

  // Mirrors Rust quiesce(): waits 100ms then returns count
  async quiesce(): Promise<number> {
    await new Promise(resolve => setTimeout(resolve, 100));
    return this.count();
  }
}

// ── Test ──
// Mirrors: basic.rs test_sled()

describe('basic integration', () => {
  test('test_sled', async () => {
    // Rust: let node = Node::new_durable(Arc::new(SledStorageEngine::new_test()?), PermissiveAgent::new());
    // Divergence: MemoryStorageEngine instead of SledStorageEngine [E5]
    const node = new Node({
      storageEngine: new MemoryStorageEngine(),
      policyAgent: new PermissiveAgent(),
      durable: true,
    });

    // Rust: node.system.create().await?;
    // Divergence: SystemManager not yet ported — skip [E8]

    // Rust: let context = node.context_async(c).await;
    const context = node.context();

    // Create an Album
    let albumId;
    {
      const trx = context.begin();
      const albumBorrow = await trx.create(Album, { name: 'The rest of the bowl', year: '2024' });
      albumId = albumBorrow.inner.id();
      await trx.commit();
    }

    // Rust: let album = context.get::<AlbumView>(album_id).await?;
    const album = await context.get(Album, albumId);

    // Rust: let view_watcher = TestWatcher::transform(|v: AlbumView| (v.clone(), v.name().unwrap(), v.year().unwrap()));
    // Divergence: We capture the entity's current property values at notification time [E8].
    const viewWatcher = TestWatcher.withTransform(() => {
      // At notification time, read current values from the album view
      const name = album.name();
      const year = album.year();
      return { name: name ?? '', year: year ?? '' };
    });

    // Rust: let render_watcher = TestWatcher::new();
    const renderWatcher = TestWatcher.new<string>();

    // Rust: let _h1 = album.subscribe(&view_watcher);
    // Divergence: View doesn't directly implement Subscribe; subscribe on entity broadcast [E8].
    const h1 = album.entity().broadcast.reference().listen({
      type: 'NotifyOnly',
      callback: () => viewWatcher.notify(undefined as any),
    });

    // Rust: CallbackObserver + observer.trigger()
    const observer = new CallbackObserver(() => {
      const name = album.name();
      const year = album.year();
      renderWatcher.notify(`name: ${name ?? ''}, year: ${year ?? ''}`);
    });
    observer.trigger();

    // Rust: assert_eq!(render_watcher.take_one().await, "name: The rest of the bowl, year: 2024");
    expect(await renderWatcher.takeOne()).toBe('name: The rest of the bowl, year: 2024');

    // Second transaction: edit name - delete the "b" from "bowl"
    // Rust: let trx2 = context.begin();
    // Rust: let album_mut2 = album.edit(&trx2)?;
    const trx2 = context.begin();
    const albumMut2 = await trx2.get(Album, albumId);
    const yjs2 = albumMut2.inner.entity().getBackend(YjsBackend);

    // Rust: album_mut2.name().delete(16, 1)?; // remove the "typo" b from bowl
    yjs2.delete('name', 16, 1);

    // We haven't committed the transaction yet — neither watcher should have received any changes
    // Rust: assert_eq!(view_watcher.quiesce().await, 0);
    expect(await viewWatcher.quiesce()).toBe(0);
    // Rust: assert_eq!(render_watcher.quiesce().await, 0);
    expect(await renderWatcher.quiesce()).toBe(0);

    // Commit the transaction
    // Rust: trx2.commit().await?;
    await trx2.commit();

    // Now we should have one change since we performed a delete operation
    // Rust: assert_eq!(view_watcher.take_one().await, (album.clone(), "The rest of the owl".to_owned(), "2024".to_owned()));
    const viewChange1 = await viewWatcher.takeOne();
    expect(viewChange1.name).toBe('The rest of the owl');
    expect(viewChange1.year).toBe('2024');

    // Rust: assert_eq!(render_watcher.take_one().await, "name: The rest of the owl, year: 2024");
    expect(await renderWatcher.takeOne()).toBe('name: The rest of the owl, year: 2024');

    // Third transaction: change year
    // Rust: let trx3 = context.begin();
    // Rust: let album_mut3 = album.edit(&trx3)?;
    const trx3 = context.begin();
    const albumMut3 = await trx3.get(Album, albumId);
    const yjs3 = albumMut3.inner.entity().getBackend(YjsBackend);

    // Rust: album_mut3.year().replace("2025")?;
    // Divergence: Yjs has no replace(); use delete + insert [E5]
    yjs3.delete('year', 0, 4);
    yjs3.insert('year', 0, '2025');

    // Rust: trx3.commit().await?;
    await trx3.commit();

    // Rust: assert_eq!(view_watcher.take_one().await, (album.clone(), "The rest of the owl".to_owned(), "2025".to_owned()));
    const viewChange2 = await viewWatcher.takeOne();
    expect(viewChange2.name).toBe('The rest of the owl');
    expect(viewChange2.year).toBe('2025');

    // Rust: assert_eq!(render_watcher.take_one().await, "name: The rest of the owl, year: 2025");
    expect(await renderWatcher.takeOne()).toBe('name: The rest of the owl, year: 2025');

    // Cleanup
    h1.drop();
  });
});
