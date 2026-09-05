// MIRRORS: ankurah/async_result_test/src/input.rs
import { Result } from '@ankurah/base';

export async function parse(s: string): Promise<Result<number, string>> {
  if (s.length === 0) {
    return Result.Err('empty');
  } else {
    return Result.Ok(s.length);
  }
}

export function parseNow(s: string): Result<number, string> {
  if (s.length === 0) {
    return Result.Err('empty');
  } else {
    return Result.Ok(s.length);
  }
}

