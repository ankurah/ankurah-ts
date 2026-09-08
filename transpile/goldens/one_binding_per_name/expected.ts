// MIRRORS: ankurah/one_binding_per_name/src/input.rs
import { checkedAdd } from '@ankurah/base';
import { plain, unsupported_, with_ } from './words';

export function caller(): bigint {
  return checkedAdd(checkedAdd(plain(), with_(), 'i64'), unsupported_(), 'i64');
}

