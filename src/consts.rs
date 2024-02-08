pub const PROGRAM_NAME: &str = "rust-gpu";

/**
When generating a package's unique ID, how many hex nibbles of the digest should be used *at most*?

The largest meaningful value is `40`.
*/
pub const ID_DIGEST_LEN_MAX: usize = 24;

/**
How old can stuff in the cache be before we automatically clear it out?

Measured in milliseconds.
*/
pub const MAX_CACHE_AGE_MS: u128 = 7 * 24 * 60 * 60 * 1000;

pub const TOOLCHAIN_VERSION: &str = "2023-09-30";
