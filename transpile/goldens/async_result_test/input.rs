// `#[test] fn t() -> anyhow::Result<()>` FAILS when it answers `Err`, and Rust's
// harness is what reads that answer. A bun test callback has no such reader, so
// the body becomes a function and the callback unwraps what it answers.
//
// An ASYNC body answers a promise OF the `Result`, and `await` binds looser than
// the call that follows it: `await f().unwrap()` is `await (f().unwrap())`, so
// `unwrap` was asked of the promise. A promise has none, so every async test
// that answered a `Result` threw — on `Ok` as well as on `Err`. The emitted
// tests below are the driver: they run, and at the parent they threw.

pub async fn parse(s: &str) -> Result<usize, String> {
    if s.is_empty() {
        Err("empty".to_string())
    } else {
        Ok(s.len())
    }
}

pub fn parse_now(s: &str) -> Result<usize, String> {
    if s.is_empty() {
        Err("empty".to_string())
    } else {
        Ok(s.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_async_test_answering_ok_passes() -> Result<(), String> {
        let n = parse("ab").await?;
        assert_eq!(n, 2);
        Ok(())
    }

    #[test]
    fn a_sync_test_answering_ok_passes() -> Result<(), String> {
        let n = parse_now("abc")?;
        assert_eq!(n, 3);
        Ok(())
    }

    #[tokio::test]
    async fn an_async_test_that_answers_nothing_still_runs() {
        assert_eq!(parse("abcd").await.unwrap(), 4);
    }
}
