//! Persistent local account numbers for terminal titles; never sent to the model.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;

static PREFIX: OnceLock<String> = OnceLock::new();

pub(crate) fn initialize(codex_home: &Path, account: Option<&str>) -> io::Result<()> {
    let number = if let Some(account) = account {
        let directory = codex_home.join("accounts");
        std::fs::create_dir_all(&directory)?;
        // A separate lock survives atomic replacement of the number registry.
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("title-sequences.lock"))?;
        lock.lock()?;
        let path = directory.join("title-sequences.json");
        let mut numbers: BTreeMap<String, u64> = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => return Err(error),
        };
        let unique: std::collections::BTreeSet<_> = numbers.values().collect();
        if unique.len() != numbers.len() || numbers.values().any(|number| *number < 2) {
            return Err(io::Error::other("invalid account title number registry"));
        }
        if let Some(number) = numbers.get(account) {
            *number
        } else {
            let number = numbers.values().copied().max().unwrap_or(1)
                .checked_add(1).ok_or_else(|| io::Error::other("account title numbers exhausted"))?;
            numbers.insert(account.to_owned(), number);
            let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
            serde_json::to_writer(&mut temporary, &numbers).map_err(io::Error::other)?;
            temporary.flush()?;
            temporary.as_file().sync_all()?;
            temporary.persist(&path).map_err(io::Error::other)?;
            number
        }
    } else {
        1
    };
    let _ = PREFIX.set(format!("[{number}] "));
    Ok(())
}

pub(crate) fn prefix() -> &'static str {
    PREFIX.get().map(String::as_str).unwrap_or("")
}
