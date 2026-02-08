// Formatters - 파일 크기, 날짜, 권한 포맷팅

use chrono::{DateTime, Local};
use std::fs::Permissions;
use std::time::SystemTime;

/// 파일 크기를 읽기 쉬운 형식으로 포맷팅 (숫자와 단위 사이 공백)
///
/// # Examples
/// ```
/// use boksldir::utils::formatter::format_file_size;
///
/// assert_eq!(format_file_size(0), "0 B");
/// assert_eq!(format_file_size(512), "512 B");
/// assert_eq!(format_file_size(1536), "1.5 KB");
/// assert_eq!(format_file_size(1_048_576), "1.0 MB");
/// ```
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes == 0 {
        "0 B".to_string()
    } else if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        let kb = bytes as f64 / KB as f64;
        format!("{:.1} KB", kb)
    } else if bytes < GB {
        let mb = bytes as f64 / MB as f64;
        format!("{:.1} MB", mb)
    } else {
        let gb = bytes as f64 / GB as f64;
        format!("{:.1} GB", gb)
    }
}

/// 시스템 시간을 통일된 날짜 형식으로 포맷팅
///
/// 항상 "YYYY-MM-DD HH:MM" 형식 (16자 고정)
///
/// # Examples
/// ```
/// use std::time::SystemTime;
/// use boksldir::utils::formatter::format_date;
///
/// let now = SystemTime::now();
/// let formatted = format_date(now);
/// // 항상 "2026-02-08 14:30" 형식 (16자)
/// assert_eq!(formatted.len(), 16);
/// ```
pub fn format_date(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

/// 시스템 시간을 전체 날짜/시간 형식으로 포맷팅 (Properties 다이얼로그 전용)
///
/// 항상 "YYYY-MM-DD HH:MM:SS" 형식 (19자)
pub fn format_date_full(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 개수에 따라 단수/복수형 반환
///
/// # Examples
/// ```
/// use boksldir::utils::formatter::pluralize;
///
/// assert_eq!(pluralize(1, "file", "files"), "1 file");
/// assert_eq!(pluralize(3, "file", "files"), "3 files");
/// assert_eq!(pluralize(0, "item", "items"), "0 items");
/// ```
pub fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{} {}", count, singular)
    } else {
        format!("{} {}", count, plural)
    }
}

/// 숫자를 천단위 콤마로 포맷팅
///
/// # Examples
/// ```
/// use boksldir::utils::formatter::format_number_with_commas;
///
/// assert_eq!(format_number_with_commas(0), "0");
/// assert_eq!(format_number_with_commas(999), "999");
/// assert_eq!(format_number_with_commas(1234), "1,234");
/// assert_eq!(format_number_with_commas(1234567), "1,234,567");
/// ```
pub fn format_number_with_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(b as char);
    }
    result
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
        assert_eq!(format_file_size(0), "0 B");
    }

    #[test]
    fn test_format_file_size_bytes() {
        assert_eq!(format_file_size(1), "1 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1023), "1023 B");
    }

    #[test]
    fn test_format_file_size_kb() {
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(2048), "2.0 KB");
        assert_eq!(format_file_size(1_048_575), "1024.0 KB");
    }

    #[test]
    fn test_format_file_size_mb() {
        assert_eq!(format_file_size(1_048_576), "1.0 MB");
        assert_eq!(format_file_size(3_670_016), "3.5 MB");
        assert_eq!(format_file_size(1_073_741_823), "1024.0 MB");
    }

    #[test]
    fn test_format_file_size_gb() {
        assert_eq!(format_file_size(1_073_741_824), "1.0 GB");
        assert_eq!(format_file_size(2_147_483_648), "2.0 GB");
    }

    #[test]
    fn test_format_date() {
        let now = SystemTime::now();
        let formatted = format_date(now);
        // 항상 "YYYY-MM-DD HH:MM" 형식 (16자)
        assert_eq!(formatted.len(), 16);
        assert!(formatted.contains('-'));
        assert!(formatted.contains(':'));
    }

    #[test]
    fn test_format_date_full() {
        let now = SystemTime::now();
        let formatted = format_date_full(now);
        // 항상 "YYYY-MM-DD HH:MM:SS" 형식 (19자)
        assert_eq!(formatted.len(), 19);
    }

    #[test]
    fn test_pluralize() {
        assert_eq!(pluralize(0, "file", "files"), "0 files");
        assert_eq!(pluralize(1, "file", "files"), "1 file");
        assert_eq!(pluralize(2, "file", "files"), "2 files");
        assert_eq!(pluralize(1, "item", "items"), "1 item");
        assert_eq!(pluralize(5, "dir", "dirs"), "5 dirs");
    }

    #[test]
    fn test_format_number_with_commas() {
        assert_eq!(format_number_with_commas(0), "0");
        assert_eq!(format_number_with_commas(999), "999");
        assert_eq!(format_number_with_commas(1000), "1,000");
        assert_eq!(format_number_with_commas(1234), "1,234");
        assert_eq!(format_number_with_commas(1234567), "1,234,567");
        assert_eq!(format_number_with_commas(1000000000), "1,000,000,000");
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
