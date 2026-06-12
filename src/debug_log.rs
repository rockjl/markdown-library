use std::fs::OpenOptions;
use std::io::Write;

fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, millis)
}

/// Write a timestamped line to `voice_debug.log` (appended, file truncated on first open).
pub fn log_msg(msg: &str) {
    use std::sync::Mutex;
    static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);
    if let Ok(mut guard) = FILE.lock() {
        if guard.is_none() {
            *guard = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open("voice_debug.log")
                .ok();
        }
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "[{}] {}", timestamp(), msg);
            let _ = f.flush();
        }
    }
}
