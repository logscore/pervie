use elevate::RunningAs;
use std::sync::OnceLock;

static IS_ROOT: OnceLock<bool> = OnceLock::new();

/// Returns whether the current process is running as root/admin.
/// The result is cached on first call using OnceLock.
pub fn is_root() -> bool {
    *IS_ROOT.get_or_init(|| matches!(elevate::check(), RunningAs::Root | RunningAs::Suid))
}

/// Attempts to escalate privileges using sudo/doas/pkexec if not already root.
/// This MUST be called BEFORE entering raw mode (before enable_raw_mode()).
pub fn escalate_if_needed() -> Result<(), Box<dyn std::error::Error>> {
    if !is_root() {
        println!("Pervie requires root privileges for disk discovery and flashing operations.");
        elevate::escalate_if_needed()?;
    }
    Ok(())
}

/// Convert bytes to human-readable format (KB, MB, GB, TB)
pub fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_human() {
        assert_eq!(bytes_to_human(0), "0 B");
        assert_eq!(bytes_to_human(512), "512 B");
        assert_eq!(bytes_to_human(1024), "1.00 KB");
        assert_eq!(bytes_to_human(1536), "1.50 KB");
        assert_eq!(bytes_to_human(1048576), "1.00 MB");
        assert_eq!(bytes_to_human(1073741824), "1.00 GB");
        assert_eq!(bytes_to_human(1099511627776), "1.00 TB");
    }
}
