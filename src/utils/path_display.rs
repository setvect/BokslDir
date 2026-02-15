use std::path::Path;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const ELLIPSIS: &str = "...";
const PATH_ELLIPSIS: &str = "/...";

/// 문자열을 최대 너비에 맞춰 중간 생략한다.
pub fn truncate_middle(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    if text.width() <= max_width {
        return text.to_string();
    }

    if max_width < 5 {
        return take_prefix_by_width(text, max_width);
    }

    let side_width = (max_width - ELLIPSIS.width()) / 2;
    let start = take_prefix_by_width(text, side_width);
    let end = take_suffix_by_width(text, side_width);
    format!("{}{}{}", start, ELLIPSIS, end)
}

/// 경로를 최대 너비에 맞춰 축약한다.
/// 규칙: HOME 경로는 `~`로 표시하고, 길면 `앞/..../뒤` 형태로 생략한다.
pub fn truncate_path(path: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let path = shorten_home(path);
    if path.width() <= max_width {
        return path;
    }

    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 2 {
        return truncate_from_start(&path, max_width);
    }

    let first = if path.starts_with('~') {
        "~".to_string()
    } else if path.starts_with('/') {
        format!("/{}", parts[0])
    } else {
        parts[0].to_string()
    };

    let first_width = first.width() + PATH_ELLIPSIS.width();
    if first_width >= max_width {
        return truncate_from_start(&path, max_width);
    }
    let available_width = max_width.saturating_sub(first_width);

    let mut end_parts: Vec<&str> = Vec::new();
    let mut current_width = 0;
    for part in parts.iter().rev() {
        let part_width = part.width() + 1; // '/'
        if current_width + part_width > available_width {
            break;
        }
        end_parts.insert(0, part);
        current_width += part_width;
    }

    if end_parts.is_empty() {
        return truncate_from_start(&path, max_width);
    }

    format!("{}{}/{}", first, PATH_ELLIPSIS, end_parts.join("/"))
}

/// Path를 문자열로 변환 후 경로 축약 규칙을 적용한다.
pub fn truncate_path_buf(path: &Path, max_width: usize) -> String {
    truncate_path(&path.to_string_lossy(), max_width)
}

fn shorten_home(path: &str) -> String {
    let home_dir = std::env::var("HOME").unwrap_or_default();
    if home_dir.is_empty() {
        return path.to_string();
    }

    if path == home_dir {
        "~".to_string()
    } else if let Some(rest) = path.strip_prefix(&home_dir) {
        if rest.starts_with('/') {
            format!("~{}", rest)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

fn truncate_from_start(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_string();
    }
    if max_width <= ELLIPSIS.width() {
        return take_prefix_by_width(text, max_width);
    }

    let suffix_width = max_width - ELLIPSIS.width();
    format!("{}{}", ELLIPSIS, take_suffix_by_width(text, suffix_width))
}

fn take_prefix_by_width(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + ch_width > max_width {
            break;
        }
        result.push(ch);
        width += ch_width;
    }
    result
}

fn take_suffix_by_width(text: &str, max_width: usize) -> String {
    let mut rev_chars: Vec<char> = Vec::new();
    let mut width = 0;
    for ch in text.chars().rev() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + ch_width > max_width {
            break;
        }
        rev_chars.push(ch);
        width += ch_width;
    }
    rev_chars.reverse();
    rev_chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_middle() {
        assert_eq!(truncate_middle("short", 10), "short");
        assert_eq!(truncate_middle("verylongstring", 10), "ver...ing");
        assert_eq!(truncate_middle("verylongstring", 4), "very");
    }

    #[test]
    fn test_truncate_middle_width_bound() {
        let value = "가나다라마바사아자차카타파하";
        let truncated = truncate_middle(value, 12);
        assert!(truncated.width() <= 12);
    }

    #[test]
    fn test_truncate_path_short() {
        let path = "/tmp/docs";
        assert_eq!(truncate_path(path, 20), path);
    }

    #[test]
    fn test_truncate_path_long() {
        let path = "/Users/boksl/IdeaProjects/BokslDir/temp/BokslPlanningPoker/server/node_modules";
        let truncated = truncate_path(path, 30);
        assert!(truncated.contains("/.../"));
        assert!(truncated.ends_with("/node_modules"));
        assert!(truncated.width() <= 30);
    }
}
