import { MemoryStorageEngine } from '@ankurah/storage-memory';
import { CallbackObserver } from '@ankurah/signals';
import { Node } from '../../src/node.ts';
import { PermissiveAgent } from '../../src/policy.ts';
import { defineModel, yrsText } from '../../src/define-model.ts';
import { YjsBackend } from '../../src/property/backend/yjs.ts';

const Album = defineModel('album', {
  name: yrsText(),
  year: yrsText(),
});

const node = new Node({
  storageEngine: new MemoryStorageEngine(),
  policyAgent: new PermissiveAgent(),
  durable: true,
});

const context = node.context();

// Create
let albumId: any;
{
  const trx = context.begin();
  const albumBorrow = await trx.create(Album, { name: 'Test', year: '2024' });
  albumId = albumBorrow.inner.id();
  await trx.commit();
}

const album = await context.get(Album, albumId);
console.log('initial name:', album.name());

let renderCount = 0;
const observer = new CallbackObserver(() => {
  renderCount++;
  const name = album.name();
  console.log(`render #${renderCount}: name=${name}`);
});
observer.trigger();
console.log('after initial trigger, renderCount:', renderCount);

// Edit
const trx2 = context.begin();
const albumMut2 = await trx2.get(Album, albumId);
const yjs2 = albumMut2.inner.entity().getBackend(YjsBackend);
yjs2.delete('name', 0, 4);
yjs2.insert('name', 0, 'Changed');
console.log('about to commit trx2');
await trx2.commit();
console.log('after commit, renderCount:', renderCount);

// Wait a tick
await new Promise(r => setTimeout(r, 100));
console.log('after 100ms, renderCount:', renderCount);

process.exit(0);
