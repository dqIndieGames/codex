/// The current Codex CLI version as embedded at compile time.
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CODEX_CLI_DISPLAY_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (local.1)");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_version_includes_local_suffix() {
        assert_eq!(
            CODEX_CLI_DISPLAY_VERSION,
            concat!(env!("CARGO_PKG_VERSION"), " (local.1)")
        );
    }
}
