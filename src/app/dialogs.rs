use super::text_edit::TextBufferEdit;
use super::*;

impl App {
    // === 다이얼로그 입력 처리 메서드 ===

    /// 입력 다이얼로그: 문자 입력
    pub fn dialog_input_char(&mut self, c: char) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::insert_char(value, cursor_pos, c);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 백스페이스
    pub fn dialog_input_backspace(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::backspace(value, cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 이전 단어 삭제 (Ctrl+W)
    pub fn dialog_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete_prev_word(value, cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: Delete
    pub fn dialog_input_delete(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete(value, cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 커서 왼쪽
    pub fn dialog_input_left(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::left(value, cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 커서 오른쪽
    pub fn dialog_input_right(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::right(value, cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: Home
    pub fn dialog_input_home(&mut self) {
        if let Some(DialogKind::Input { cursor_pos, .. }) = &mut self.dialog {
            TextBufferEdit::home(cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: End
    pub fn dialog_input_end(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::end(value, cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 버튼 선택 변경 (Tab)
    pub fn dialog_input_toggle_button(&mut self) {
        if let Some(DialogKind::Input {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    /// 입력 다이얼로그: 선택된 버튼 반환
    pub fn get_dialog_input_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::Input {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    pub fn get_dialog_input_purpose(&self) -> Option<InputPurpose> {
        if let Some(DialogKind::Input { purpose, .. }) = &self.dialog {
            Some(*purpose)
        } else {
            None
        }
    }

    pub(super) fn archive_create_field_count(use_password: bool) -> usize {
        if use_password {
            5
        } else {
            3
        }
    }

    pub fn archive_create_dialog_next_field(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            use_password,
            ..
        }) = &mut self.dialog
        {
            let count = Self::archive_create_field_count(*use_password);
            *focused_field = (*focused_field + 1) % count;
        }
    }

    pub fn archive_create_dialog_prev_field(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            use_password,
            ..
        }) = &mut self.dialog
        {
            let count = Self::archive_create_field_count(*use_password);
            *focused_field = if *focused_field == 0 {
                count - 1
            } else {
                *focused_field - 1
            };
        }
    }

    pub fn archive_create_dialog_toggle_password(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            use_password,
            focused_field,
            ..
        }) = &mut self.dialog
        {
            *use_password = !*use_password;
            if !*use_password && *focused_field > 2 {
                *focused_field = 2;
            }
        }
    }

    pub fn archive_create_dialog_toggle_button(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn archive_create_dialog_char(&mut self, c: char) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_value,
            path_cursor_pos,
            use_password,
            password_value,
            password_cursor_pos,
            password_confirm_value,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => {
                    TextBufferEdit::insert_char(path_value, path_cursor_pos, c);
                }
                2 if *use_password => {
                    TextBufferEdit::insert_char(password_value, password_cursor_pos, c);
                }
                3 if *use_password => {
                    TextBufferEdit::insert_char(
                        password_confirm_value,
                        password_confirm_cursor_pos,
                        c,
                    );
                }
                _ => {}
            }
        }
    }

    pub fn archive_create_dialog_backspace(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_value,
            path_cursor_pos,
            use_password,
            password_value,
            password_cursor_pos,
            password_confirm_value,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => TextBufferEdit::backspace(path_value, path_cursor_pos),
                2 if *use_password => {
                    TextBufferEdit::backspace(password_value, password_cursor_pos)
                }
                3 if *use_password => {
                    TextBufferEdit::backspace(password_confirm_value, password_confirm_cursor_pos)
                }
                _ => {}
            }
        }
    }

    pub fn archive_create_dialog_delete(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_value,
            path_cursor_pos,
            use_password,
            password_value,
            password_cursor_pos,
            password_confirm_value,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => TextBufferEdit::delete(path_value, path_cursor_pos),
                2 if *use_password => TextBufferEdit::delete(password_value, password_cursor_pos),
                3 if *use_password => {
                    TextBufferEdit::delete(password_confirm_value, password_confirm_cursor_pos)
                }
                _ => {}
            }
        }
    }

    pub fn archive_create_dialog_left(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_value,
            path_cursor_pos,
            use_password,
            password_value,
            password_cursor_pos,
            password_confirm_value,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => TextBufferEdit::left(path_value, path_cursor_pos),
                2 if *use_password => TextBufferEdit::left(password_value, password_cursor_pos),
                3 if *use_password => {
                    TextBufferEdit::left(password_confirm_value, password_confirm_cursor_pos)
                }
                4 => {
                    self.archive_create_dialog_toggle_button();
                }
                _ => {}
            }
        }
    }

    pub fn archive_create_dialog_right(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_value,
            path_cursor_pos,
            use_password,
            password_value,
            password_cursor_pos,
            password_confirm_value,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => TextBufferEdit::right(path_value, path_cursor_pos),
                2 if *use_password => TextBufferEdit::right(password_value, password_cursor_pos),
                3 if *use_password => {
                    TextBufferEdit::right(password_confirm_value, password_confirm_cursor_pos)
                }
                4 => {
                    self.archive_create_dialog_toggle_button();
                }
                _ => {}
            }
        }
    }

    pub fn archive_create_dialog_home(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_cursor_pos,
            use_password,
            password_cursor_pos,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => TextBufferEdit::home(path_cursor_pos),
                2 if *use_password => TextBufferEdit::home(password_cursor_pos),
                3 if *use_password => TextBufferEdit::home(password_confirm_cursor_pos),
                _ => {}
            }
        }
    }

    pub fn archive_create_dialog_end(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_value,
            path_cursor_pos,
            use_password,
            password_value,
            password_cursor_pos,
            password_confirm_value,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => TextBufferEdit::end(path_value, path_cursor_pos),
                2 if *use_password => TextBufferEdit::end(password_value, password_cursor_pos),
                3 if *use_password => {
                    TextBufferEdit::end(password_confirm_value, password_confirm_cursor_pos)
                }
                _ => {}
            }
        }
    }

    pub fn archive_create_dialog_delete_prev_word(&mut self) {
        if let Some(DialogKind::ArchiveCreateOptions {
            focused_field,
            path_value,
            path_cursor_pos,
            use_password,
            password_value,
            password_cursor_pos,
            password_confirm_value,
            password_confirm_cursor_pos,
            ..
        }) = &mut self.dialog
        {
            match *focused_field {
                0 => TextBufferEdit::delete_prev_word(path_value, path_cursor_pos),
                2 if *use_password => {
                    TextBufferEdit::delete_prev_word(password_value, password_cursor_pos)
                }
                3 if *use_password => TextBufferEdit::delete_prev_word(
                    password_confirm_value,
                    password_confirm_cursor_pos,
                ),
                _ => {}
            }
        }
    }

    pub fn confirm_archive_create_dialog(&mut self) {
        let Some(DialogKind::ArchiveCreateOptions {
            path_value,
            use_password,
            password_value,
            password_confirm_value,
            base_path,
            ..
        }) = &self.dialog
        else {
            self.close_dialog();
            return;
        };

        let path_value = path_value.clone();
        let use_password = *use_password;
        let password_value = password_value.clone();
        let password_confirm_value = password_confirm_value.clone();
        let base_path = base_path.clone();

        let Some(flow) = self.archive_flow.clone() else {
            self.close_dialog();
            return;
        };
        let ArchiveFlowContext::CreatePending { sources } = flow else {
            self.close_dialog();
            return;
        };

        let resolved_path = self.resolve_input_path(&path_value, &base_path);
        let resolved_path_str = resolved_path.to_string_lossy().to_string();

        if resolved_path.exists() {
            self.set_toast(&format!("Archive already exists: {}", resolved_path_str));
            if let Some(DialogKind::ArchiveCreateOptions { focused_field, .. }) = &mut self.dialog {
                *focused_field = 0;
            }
            return;
        }

        let Some(format) = detect_archive_format(&resolved_path) else {
            self.dialog = Some(DialogKind::error(
                "Error",
                format!(
                    "Unsupported archive format:\n{}\n\nSupported: zip/tar/tar.gz/tar.zst/7z/jar/war",
                    resolved_path_str
                ),
            ));
            return;
        };

        let password = if use_password {
            if !supports_password(format) {
                self.set_toast(&format!(
                    "Format '{}' does not support password (zip/7z only).",
                    format.display_name()
                ));
                if let Some(DialogKind::ArchiveCreateOptions { focused_field, .. }) =
                    &mut self.dialog
                {
                    *focused_field = 1;
                }
                return;
            }
            if password_value.is_empty() {
                self.set_toast("Password is empty.");
                if let Some(DialogKind::ArchiveCreateOptions { focused_field, .. }) =
                    &mut self.dialog
                {
                    *focused_field = 2;
                }
                return;
            }
            if password_value != password_confirm_value {
                self.set_toast("Password and confirmation do not match.");
                if let Some(DialogKind::ArchiveCreateOptions { focused_field, .. }) =
                    &mut self.dialog
                {
                    *focused_field = 3;
                }
                return;
            }
            Some(password_value)
        } else {
            None
        };

        self.archive_flow = None;
        self.start_archive_create_worker(ArchiveCreateRequest {
            sources,
            output_path: resolved_path,
            password,
        });
    }

    /// 확인 다이얼로그: 버튼 선택 변경
    pub fn dialog_confirm_toggle(&mut self) {
        if let Some(DialogKind::Confirm {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    /// 확인 다이얼로그 확정 처리
    pub fn confirm_confirm_dialog(&mut self) {
        self.close_dialog();
    }

    /// 충돌 다이얼로그: 옵션 이동
    pub fn dialog_conflict_next(&mut self) {
        if let Some(DialogKind::Conflict {
            selected_option, ..
        }) = &mut self.dialog
        {
            *selected_option = (*selected_option + 1) % 5;
        }
    }

    /// 충돌 다이얼로그: 옵션 이동 (이전)
    pub fn dialog_conflict_prev(&mut self) {
        if let Some(DialogKind::Conflict {
            selected_option, ..
        }) = &mut self.dialog
        {
            *selected_option = if *selected_option == 0 {
                4
            } else {
                *selected_option - 1
            };
        }
    }

    /// 현재 다이얼로그 입력값 반환 (Input 다이얼로그용)
    pub fn get_dialog_input_value(&self) -> Option<String> {
        if let Some(DialogKind::Input { value, .. }) = &self.dialog {
            Some(value.clone())
        } else {
            None
        }
    }

    /// 현재 다이얼로그 선택 버튼 반환 (Confirm 다이얼로그용)
    pub fn get_dialog_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::Confirm {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    /// 현재 다이얼로그 선택 옵션 반환 (Conflict 다이얼로그용)
    pub fn get_dialog_conflict_option(&self) -> Option<ConflictResolution> {
        if let Some(DialogKind::Conflict {
            selected_option, ..
        }) = &self.dialog
        {
            Some(match selected_option {
                0 => ConflictResolution::Overwrite,
                1 => ConflictResolution::Skip,
                2 => ConflictResolution::OverwriteAll,
                3 => ConflictResolution::SkipAll,
                _ => ConflictResolution::Cancel,
            })
        } else {
            None
        }
    }
}
