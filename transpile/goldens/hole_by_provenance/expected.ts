// MIRRORS: ankurah/hole_by_provenance/src/input.rs
import { checkedAdd } from '@ankurah/base';

export function unsupported_(label: string): number | null {
  if (label === 'missing') {
    return null;
  } else {
    return 3;
  }
}

export function askedMissing(): number | null {
  const _r0 = unsupported_('missing');
  if (_r0 == null) return null;
  const n = _r0;
  return checkedAdd(n, 1, 'u32');
}

export function askedPresent(): number | null {
  const _r0 = unsupported_('present');
  if (_r0 == null) return null;
  const n = _r0;
  return checkedAdd(n, 1, 'u32');
}

