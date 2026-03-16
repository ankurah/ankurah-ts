import * as Y from 'yjs';

const doc1 = new Y.Doc();
const text = doc1.getText('test');
text.insert(0, 'hello');
const svAfterInsert = Y.encodeStateVector(doc1);
text.delete(0, 1);
const svAfterDelete = Y.encodeStateVector(doc1);
console.log('SV after insert:', Array.from(svAfterInsert));
console.log('SV after delete:', Array.from(svAfterDelete));
console.log('SVs equal?', svAfterInsert.length === svAfterDelete.length && Array.from(svAfterInsert).every((b, i) => b === svAfterDelete[i]));
const diff = Y.encodeStateAsUpdateV2(doc1, svAfterInsert);
console.log('delete-only diff:', Array.from(diff));
console.log('delete-only diff length:', diff.length);
const emptyDiff = Y.encodeStateAsUpdateV2(doc1, svAfterDelete);
console.log('empty diff:', Array.from(emptyDiff));
