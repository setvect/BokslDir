pub(super) struct TextBufferEdit;

impl TextBufferEdit {
    pub(super) fn insert_char(value: &mut String, cursor_pos: &mut usize, c: char) {
        value.insert(*cursor_pos, c);
        *cursor_pos += c.len_utf8();
    }

    pub(super) fn backspace(value: &mut String, cursor_pos: &mut usize) {
        if *cursor_pos == 0 {
            return;
        }

        let prev = Self::prev_char_start(value, *cursor_pos);
        value.remove(prev);
        *cursor_pos = prev;
    }

    pub(super) fn delete(value: &mut String, cursor_pos: &mut usize) {
        if *cursor_pos < value.len() {
            value.remove(*cursor_pos);
        }
    }

    pub(super) fn left(value: &str, cursor_pos: &mut usize) {
        if *cursor_pos == 0 {
            return;
        }

        *cursor_pos = Self::prev_char_start(value, *cursor_pos);
    }

    pub(super) fn right(value: &str, cursor_pos: &mut usize) {
        if *cursor_pos >= value.len() {
            return;
        }

        *cursor_pos = value[*cursor_pos..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| *cursor_pos + i)
            .unwrap_or(value.len());
    }

    pub(super) fn home(cursor_pos: &mut usize) {
        *cursor_pos = 0;
    }

    pub(super) fn end(value: &str, cursor_pos: &mut usize) {
        *cursor_pos = value.len();
    }

    pub(super) fn delete_prev_word(value: &mut String, cursor_pos: &mut usize) {
        if *cursor_pos == 0 {
            return;
        }

        let original = *cursor_pos;
        let mut pos = original;

        // 1) 커서 왼쪽의 구분자들을 먼저 건너뜀
        while pos > 0 {
            let prev = Self::prev_char_start(value, pos);
            let ch = value[prev..pos].chars().next().unwrap_or_default();
            if Self::is_word_delimiter(ch) {
                pos = prev;
            } else {
                break;
            }
        }

        // 2) 실제 단어 시작까지 이동
        while pos > 0 {
            let prev = Self::prev_char_start(value, pos);
            let ch = value[prev..pos].chars().next().unwrap_or_default();
            if Self::is_word_delimiter(ch) {
                break;
            }
            pos = prev;
        }

        value.replace_range(pos..original, "");
        *cursor_pos = pos;
    }

    fn prev_char_start(value: &str, cursor_pos: usize) -> usize {
        value[..cursor_pos]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn is_word_delimiter(ch: char) -> bool {
        ch.is_whitespace()
            || matches!(
                ch,
                '/' | '\\'
                    | ':'
                    | ';'
                    | ','
                    | '.'
                    | '|'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | '"'
                    | '\''
                    | '`'
            )
    }
}

#[cfg(test)]
mod tests {
    use super::TextBufferEdit;

    #[test]
    fn test_insert_backspace_delete_utf8_cursor_boundary() {
        let mut value = "\u{AC00}\u{B098}".to_string();
        let mut cursor_pos = "\u{AC00}".len();

        TextBufferEdit::insert_char(&mut value, &mut cursor_pos, '\u{B2E4}');
        assert_eq!(value, "\u{AC00}\u{B2E4}\u{B098}");
        assert_eq!(cursor_pos, "\u{AC00}\u{B2E4}".len());

        TextBufferEdit::backspace(&mut value, &mut cursor_pos);
        assert_eq!(value, "\u{AC00}\u{B098}");
        assert_eq!(cursor_pos, "\u{AC00}".len());

        cursor_pos = 0;
        TextBufferEdit::delete(&mut value, &mut cursor_pos);
        assert_eq!(value, "\u{B098}");
        assert_eq!(cursor_pos, 0);
    }

    #[test]
    fn test_left_right_home_end_utf8_cursor_boundary() {
        let value = "a\u{AC00}b".to_string();
        let mut cursor_pos = value.len();

        TextBufferEdit::left(&value, &mut cursor_pos);
        assert_eq!(cursor_pos, "a\u{AC00}".len());

        TextBufferEdit::left(&value, &mut cursor_pos);
        assert_eq!(cursor_pos, "a".len());

        TextBufferEdit::right(&value, &mut cursor_pos);
        assert_eq!(cursor_pos, "a\u{AC00}".len());

        TextBufferEdit::home(&mut cursor_pos);
        assert_eq!(cursor_pos, 0);

        TextBufferEdit::end(&value, &mut cursor_pos);
        assert_eq!(cursor_pos, value.len());
    }

    #[test]
    fn test_delete_prev_word_utf8() {
        let mut value = "/tmp/\u{D55C}\u{AE00} \u{D3F4}\u{B354}/test".to_string();
        let mut cursor_pos = value.len();

        TextBufferEdit::delete_prev_word(&mut value, &mut cursor_pos);

        assert_eq!(value, "/tmp/\u{D55C}\u{AE00} \u{D3F4}\u{B354}/");
        assert_eq!(cursor_pos, value.len());
    }
}
