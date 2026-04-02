/// The current Codex CLI version as embedded at compile time.
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CODEX_CLI_DISPLAY_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-local1");
