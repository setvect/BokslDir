//! 간단한 글로브 패턴 매칭 (외부 크레이트 없이 구현)
//!
//! `*` (0개 이상 임의 문자), `?` (임의 1문자) 지원.
//! 대소문자 무시.

/// 패턴에 글로브 와일드카드(`*` 또는 `?`)가 포함되어 있는지 확인
pub fn is_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

/// 글로브 패턴 매칭 (대소문자 무시, UTF-8 안전)
///
/// - `*` : 0개 이상의 임의 문자
/// - `?` : 정확히 1개의 임의 문자
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let text: Vec<char> = text.to_lowercase().chars().collect();
    glob_match_chars(&pattern, &text)
}

fn glob_match_chars(pattern: &[char], text: &[char]) -> bool {
    match (pattern.first(), text.first()) {
        // 패턴과 텍스트 모두 소진 → 매치
        (None, None) => true,
        // 패턴만 남음: 나머지가 모두 `*`이면 매치
        (Some(&'*'), _) if text.is_empty() => glob_match_chars(&pattern[1..], text),
        (Some(_), None) => pattern.iter().all(|&c| c == '*'),
        (None, Some(_)) => false,
        // `*` 매칭: 0문자 소비 또는 1문자 소비
        (Some(&'*'), Some(_)) => {
            glob_match_chars(&pattern[1..], text) // * = 0문자
                || glob_match_chars(pattern, &text[1..]) // * = 1+문자
        }
        // `?` 매칭: 정확히 1문자 소비
        (Some(&'?'), Some(_)) => glob_match_chars(&pattern[1..], &text[1..]),
        // 일반 문자 매칭
        (Some(&p), Some(&t)) => {
            if p == t {
                glob_match_chars(&pattern[1..], &text[1..])
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_glob_pattern() {
        assert!(is_glob_pattern("*.rs"));
        assert!(is_glob_pattern("test?"));
        assert!(is_glob_pattern("*.?"));
        assert!(!is_glob_pattern("hello"));
        assert!(!is_glob_pattern(""));
    }

    #[test]
    fn test_exact_match() {
        assert!(glob_match("hello", "hello"));
        assert!(glob_match("hello", "HELLO"));
        assert!(!glob_match("hello", "world"));
    }

    #[test]
    fn test_star_wildcard() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "test.rs"));
        assert!(!glob_match("*.rs", "main.txt"));
        assert!(glob_match("test*", "test_file.rs"));
        assert!(glob_match("test*", "test"));
        assert!(glob_match("*test*", "my_test_file"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", ""));
    }

    #[test]
    fn test_question_wildcard() {
        assert!(glob_match("?.rs", "a.rs"));
        assert!(!glob_match("?.rs", "ab.rs"));
        assert!(glob_match("test?", "tests"));
        assert!(!glob_match("test?", "test"));
        assert!(!glob_match("test?", "testab"));
    }

    #[test]
    fn test_combined_wildcards() {
        assert!(glob_match("*.??", "file.rs"));
        assert!(!glob_match("*.??", "file.txt"));
        assert!(glob_match("t*t", "test"));
        assert!(glob_match("t*t", "tt"));
        assert!(glob_match("*.*", "file.txt"));
        assert!(!glob_match("*.*", "noext"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(glob_match("*.RS", "file.rs"));
        assert!(glob_match("*.rs", "FILE.RS"));
        assert!(glob_match("Test*", "testing"));
    }

    #[test]
    fn test_korean_filenames() {
        assert!(glob_match("*테스트*", "나의_테스트_파일"));
        assert!(glob_match("*.txt", "한글파일.txt"));
    }

    #[test]
    fn test_edge_cases() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "a"));
        assert!(glob_match("*", ""));
        assert!(glob_match("**", "abc"));
        assert!(glob_match("***", ""));
    }
}
