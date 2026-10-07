//! Temporary local3 completion diagnostics. Remove after the stuck-busy bug is verified fixed.
//! Opt in with CODEX_TUI_COMPLETION_DIAGNOSTICS_DIR pointing to an absolute local directory.

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde_json::Value;
use serde_json::json;

static SINK: OnceLock<Option<mpsc::SyncSender<Value>>> = OnceLock::new();
static DROPPED: AtomicU64 = AtomicU64::new(0);
const FILE_LIMIT: u64 = 2_621_440;

fn sink() -> Option<&'static mpsc::SyncSender<Value>> {
    SINK.get_or_init(|| {
        let directory = PathBuf::from(std::env::var_os("CODEX_TUI_COMPLETION_DIAGNOSTICS_DIR")?);
        if !directory.is_absolute() {
            return None;
        }
        let (sender, receiver) = mpsc::sync_channel::<Value>(128);
        std::thread::Builder::new().name("local3-completion-log".into()).spawn(move || {
            if std::fs::create_dir_all(&directory).is_err() {
                return;
            }
            let Ok(lock) = OpenOptions::new().create(true).truncate(false).read(true).write(true)
                .open(directory.join("completion.lock")) else { return; };
            let current = directory.join("completion.jsonl");
            let previous = directory.join("completion.previous.jsonl");
            let mut recent: VecDeque<(String, Instant, u64)> = VecDeque::new();
            for mut record in receiver {
                let key = format!("{}:{}:{}:{}", record["thread_id"], record["turn_id"], record["phase"], record["state"]);
                let now = Instant::now();
                if let Some(entry) = recent.iter_mut().find(|entry| entry.0 == key) {
                    if now.duration_since(entry.1) < Duration::from_secs(10) {
                        entry.2 = entry.2.saturating_add(1);
                        continue;
                    }
                    record["suppressed"] = json!(entry.2);
                    entry.1 = now;
                    entry.2 = 0;
                } else {
                    if recent.len() == 256 { recent.pop_front(); }
                    recent.push_back((key, now, 0));
                }
                record["dropped_records"] = json!(DROPPED.swap(0, Ordering::Relaxed));
                let Ok(mut line) = serde_json::to_vec(&record) else { continue; };
                if line.len() > 4096 { continue; }
                line.push(b'\n');
                // The lock and all filesystem work are confined to this background writer.
                if lock.lock().is_err() { return; }
                let result = (|| -> std::io::Result<()> {
                    let length = std::fs::metadata(&current).map(|m| m.len()).unwrap_or(0);
                    if length + line.len() as u64 > FILE_LIMIT {
                        match std::fs::remove_file(&previous) {
                            Ok(()) => {},
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                            Err(error) => return Err(error),
                        }
                        std::fs::rename(&current, &previous)?;
                    }
                    OpenOptions::new().create(true).append(true).open(&current)?.write_all(&line)
                })();
                let _ = lock.unlock();
                if result.is_err() { return; }
            }
        }).ok()?;
        Some(sender)
    }).as_ref()
}

pub fn enabled() -> bool {
    sink().is_some()
}

/// Records only caller-selected IDs and booleans, never a full notification or transcript.
pub fn record(phase: &'static str, thread_id: &str, turn_id: Option<&str>, state: Value) {
    let Some(sender) = sink() else { return; };
    let record = json!({
        "time_ms": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),
        "pid": std::process::id(), "version": concat!(env!("CARGO_PKG_VERSION"), "-local3"),
        "phase": phase, "thread_id": thread_id, "turn_id": turn_id, "state": state,
    });
    if sender.try_send(record).is_err() {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}
