// MIRRORS: ankurah/proto/src/human_id.rs

function compress(bytes: Uint8Array, target: number): Uint8Array {
  const segSize = Math.trunc(bytes.length / target);
  return bytes.chunks(segSize).map((c) => [...c].fold(0, (acc, x) => acc ^ x));
}

export function humanize(bytes: Uint8Array, wordsOut: number): string {
  return [...compress(bytes, wordsOut)].map((x) => WORDLIST[x]).join('-');
}

export function hex(bytes: Uint8Array): string {
  return [...bytes].map((x) => `${x}`).join('');
}

export const WORDLIST: string[] = undefined as any; // TODO

