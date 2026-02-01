// Formatters - 파일 크기, 날짜, 권한 포맷팅

use chrono::{DateTime, Local};
use std::fs::Permissions;
use std::time::SystemTime;

/// 파일 크기를 읽기 쉬운 형식으로 포맷팅
///
/// # Examples
/// ```
/// use boksldir::utils::formatter::format_file_size;
///
/// assert_eq!(format_file_size(0), "0B");
/// assert_eq!(format_file_size(512), "512B");
/// assert_eq!(format_file_size(1536), "1.5KB");
/// assert_eq!(format_file_size(1_048_576), "1.0MB");
/// ```
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes == 0 {
        "0B".to_string()
    } else if bytes < KB {
        format!("{}B", bytes)
    } else if bytes < MB {
        let kb = bytes as f64 / KB as f64;
        format!("{:.1}KB", kb)
    } else if bytes < GB {
        let mb = bytes as f64 / MB as f64;
        format!("{:.1}MB", mb)
    } else {
        let gb = bytes as f64 / GB as f64;
        format!("{:.1}GB", gb)
    }
}

/// 시스템 시간을 읽기 쉬운 날짜 형식으로 포맷팅
///
/// 오늘이면 "HH:MM" 형식, 아니면 "YYYY-MM-DD" 형식
///
/// # Examples
/// ```
/// use std::time::SystemTime;
/// use boksldir::utils::formatter::format_date;
///
/// let now = SystemTime::now();
/// let formatted = format_date(now);
/// // 오늘이면 "14:30", 아니면 "2026-01-30" 같은 형식
/// assert!(!formatted.is_empty());
/// ```
pub fn format_date(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    let today = Local::now().date_naive();
    let time_date = datetime.date_naive();

    if time_date == today {
        // 오늘이면 시간 표시
        datetime.format("%H:%M").to_string()
    } else {
        // 다른 날이면 날짜 표시
        datetime.format("%Y-%m-%d").to_string()
    }
}

/// Unix 권한을 문자열로 포맷팅 (Unix 전용)
///
/// Windows에서는 항상 "-"를 반환
///
/// # Examples
/// ```no_run
/// use std::fs::Permissions;
/// use boksldir::utils::formatter::format_permissions;
///
/// let perms = std::fs::metadata("some_file").unwrap().permissions();
/// let formatted = format_permissions(Some(&perms));
/// // Unix: "rwxr-xr-x", Windows: "-"
/// ```
pub fn format_permissions(permissions: Option<&Permissions>) -> String {
    match permissions {
        None => "-".to_string(),
        Some(perms) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = perms.mode();
                unix_mode_to_string(mode)
            }
            #[cfg(not(unix))]
            {
                // Windows는 Unix 스타일 권한이 없음
                let _ = perms; // 경고 방지
                "-".to_string()
            }
        }
    }
}

/// Unix 모드를 rwxr-xr-x 형식으로 변환
#[cfg(unix)]
fn unix_mode_to_string(mode: u32) -> String {
    let user = triplet(mode, 0o100, 0o200, 0o400);
    let group = triplet(mode, 0o010, 0o020, 0o040);
    let other = triplet(mode, 0o001, 0o002, 0o004);
    format!("{}{}{}", user, group, other)
}

/// 권한 triplet (rwx) 생성
#[cfg(unix)]
fn triplet(mode: u32, exec: u32, write: u32, read: u32) -> String {
    let r = if mode & read != 0 { "r" } else { "-" };
    let w = if mode & write != 0 { "w" } else { "-" };
    let x = if mode & exec != 0 { "x" } else { "-" };
    format!("{}{}{}", r, w, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size_zero() {
        assert_eq!(format_file_size(0), "0B");
    }

    #[test]
    fn test_format_file_size_bytes() {
        assert_eq!(format_file_size(1), "1B");
        assert_eq!(format_file_size(512), "512B");
        assert_eq!(format_file_size(1023), "1023B");
    }

    #[test]
    fn test_format_file_size_kb() {
        assert_eq!(format_file_size(1024), "1.0KB");
        assert_eq!(format_file_size(1536), "1.5KB");
        assert_eq!(format_file_size(2048), "2.0KB");
        assert_eq!(format_file_size(1_048_575), "1024.0KB");
    }

    #[test]
    fn test_format_file_size_mb() {
        assert_eq!(format_file_size(1_048_576), "1.0MB");
        assert_eq!(format_file_size(3_670_016), "3.5MB");
        assert_eq!(format_file_size(1_073_741_823), "1024.0MB");
    }

    #[test]
    fn test_format_file_size_gb() {
        assert_eq!(format_file_size(1_073_741_824), "1.0GB");
        assert_eq!(format_file_size(2_147_483_648), "2.0GB");
    }

    #[test]
    fn test_format_date() {
        let now = SystemTime::now();
        let formatted = format_date(now);
        // 오늘 날짜는 HH:MM 형식 (5자)
        assert!(formatted.len() == 5 && formatted.contains(':'));
    }

    #[test]
    fn test_format_permissions_none() {
        assert_eq!(format_permissions(None), "-");
    }

    #[cfg(unix)]
    #[test]
    fn test_unix_mode_to_string() {
        // 0o755 = rwxr-xr-x
        assert_eq!(unix_mode_to_string(0o755), "rwxr-xr-x");
        // 0o644 = rw-r--r--
        assert_eq!(unix_mode_to_string(0o644), "rw-r--r--");
        // 0o777 = rwxrwxrwx
        assert_eq!(unix_mode_to_string(0o777), "rwxrwxrwx");
        // 0o000 = ---------
        assert_eq!(unix_mode_to_string(0o000), "---------");
    }
}
