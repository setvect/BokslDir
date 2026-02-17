use super::*;
use crate::utils::error::BokslDirError;
use ratatui::style::Color;
use std::fs;
use std::io::Write;
use tempfile::TempDir;
use zip::write::SimpleFileOptions as ZipFileOptions;
use zip::{AesMode, CompressionMethod, ZipWriter};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

fn make_test_app() -> App {
    App::new_for_test()
}

fn run_file_operation_until_done(app: &mut App) {
    let mut guard = 0usize;
    while app.pending_operation.is_some() && guard < 10_000 {
        app.process_next_file();
        guard += 1;
    }
    assert!(guard < 10_000, "operation loop guard exceeded");
}

fn run_archive_operation_until_done(app: &mut App) {
    let mut guard = 0usize;
    while app.archive_worker.is_some() && guard < 10_000 {
        app.process_next_archive();
        std::thread::sleep(std::time::Duration::from_millis(1));
        guard += 1;
    }
    assert!(guard < 10_000, "archive operation loop guard exceeded");
}

fn child_path(base: &std::path::Path, name: &str) -> std::path::PathBuf {
    base.join(name)
}

fn create_dirs(base: &std::path::Path, names: &[&str]) -> Vec<std::path::PathBuf> {
    names
        .iter()
        .map(|name| {
            let path = child_path(base, name);
            fs::create_dir_all(&path).unwrap();
            path
        })
        .collect()
}

/// 재귀 경로 검사 테스트: 디렉토리를 자기 자신 내부로 복사
#[test]
fn test_is_recursive_path_into_self() {
    let temp = TempDir::new().unwrap();
    let parent = child_path(temp.path(), "parent");
    let child = parent.join("child");

    fs::create_dir_all(&child).unwrap();

    // parent -> parent/child 는 재귀 복사
    assert!(App::is_recursive_path(&parent, &child));
}

/// 재귀 경로 검사 테스트: 서로 다른 디렉토리는 OK
#[test]
fn test_is_recursive_path_different_dirs() {
    let temp = TempDir::new().unwrap();
    let dirs = create_dirs(temp.path(), &["dir_a", "dir_b"]);
    let dir_a = dirs[0].clone();
    let dir_b = dirs[1].clone();

    // dir_a -> dir_b 는 재귀 아님
    assert!(!App::is_recursive_path(&dir_a, &dir_b));
}

/// 재귀 경로 검사 테스트: 파일은 재귀 검사 대상 아님
#[test]
fn test_is_recursive_path_file_not_checked() {
    let temp = TempDir::new().unwrap();
    let file = child_path(temp.path(), "file.txt");
    let dest = child_path(temp.path(), "dest");

    fs::write(&file, "test").unwrap();
    fs::create_dir_all(&dest).unwrap();

    // 파일은 항상 false
    assert!(!App::is_recursive_path(&file, &dest));
}

/// 재귀 경로 검사 테스트: 형제 디렉토리는 OK
#[test]
fn test_is_recursive_path_sibling_dirs() {
    let temp = TempDir::new().unwrap();
    let dirs = create_dirs(temp.path(), &["parent", "sibling"]);
    let parent = dirs[0].clone();
    let sibling = dirs[1].clone();

    // parent -> sibling 은 재귀 아님
    assert!(!App::is_recursive_path(&parent, &sibling));
}

/// 재귀 경로 검사 테스트: 같은 디렉토리 (자기 자신)
#[test]
fn test_is_recursive_path_same_dir() {
    let temp = TempDir::new().unwrap();
    let dir = create_dirs(temp.path(), &["dir"])[0].clone();

    // dir -> dir 자체도 재귀로 간주
    assert!(App::is_recursive_path(&dir, &dir));
}

/// check_recursive_operation 테스트: 재귀 발견 시 에러 메시지 반환
#[test]
fn test_check_recursive_operation_detects_recursive() {
    let temp = TempDir::new().unwrap();
    let parent = child_path(temp.path(), "parent");
    let child = parent.join("child");

    fs::create_dir_all(&child).unwrap();

    let sources = vec![parent.clone()];
    let result = App::check_recursive_operation(&sources, OperationType::Copy, &child);

    assert!(result.is_some());
    assert!(result.unwrap().contains("Cannot copy"));
}

/// check_recursive_operation 테스트: 정상 복사는 None 반환
#[test]
fn test_check_recursive_operation_allows_valid() {
    let temp = TempDir::new().unwrap();
    let dirs = create_dirs(temp.path(), &["source", "dest"]);
    let source = dirs[0].clone();
    let dest = dirs[1].clone();

    let sources = vec![source];
    let result = App::check_recursive_operation(&sources, OperationType::Copy, &dest);

    assert!(result.is_none());
}

/// check_recursive_operation 테스트: 여러 소스 중 하나라도 재귀면 에러
#[test]
fn test_check_recursive_operation_multiple_sources() {
    let temp = TempDir::new().unwrap();
    let dirs = create_dirs(temp.path(), &["ok", "bad"]);
    let ok_dir = dirs[0].clone();
    let bad_dir = dirs[1].clone();
    let dest = bad_dir.join("child");

    fs::create_dir_all(&dest).unwrap();

    let sources = vec![ok_dir, bad_dir.clone()];
    let result = App::check_recursive_operation(&sources, OperationType::Move, &dest);

    assert!(result.is_some());
    assert!(result.unwrap().contains("Cannot move"));
}

#[test]
fn test_tab_create_close_and_guard_last() {
    let mut app = make_test_app();

    assert_eq!(app.left_tabs.len(), 1);
    app.new_tab_active_panel();
    assert_eq!(app.left_tabs.len(), 2);
    assert_eq!(app.left_tabs.active_index(), 1);

    app.close_tab_active_panel();
    assert_eq!(app.left_tabs.len(), 1);

    app.close_tab_active_panel();
    assert_eq!(app.left_tabs.len(), 1);
    assert_eq!(app.toast_display(), Some("Cannot close last tab"));
}

#[test]
fn test_tab_prev_next_and_switch() {
    let mut app = make_test_app();
    app.new_tab_active_panel();
    app.new_tab_active_panel();
    assert_eq!(app.left_tabs.active_index(), 2);

    app.prev_tab_active_panel();
    assert_eq!(app.left_tabs.active_index(), 1);
    app.next_tab_active_panel();
    assert_eq!(app.left_tabs.active_index(), 2);

    app.switch_tab_active_panel(0);
    assert_eq!(app.left_tabs.active_index(), 0);
    app.switch_tab_active_panel(9);
    assert_eq!(app.left_tabs.active_index(), 0);
}

#[test]
fn test_tab_state_persists_per_tab() {
    let mut app = make_test_app();

    app.active_panel_state_mut()
        .set_filter(Some("alpha".to_string()));
    app.new_tab_active_panel();
    app.active_panel_state_mut()
        .set_filter(Some("beta".to_string()));

    app.prev_tab_active_panel();
    assert_eq!(app.active_panel_state().filter.as_deref(), Some("alpha"));

    app.next_tab_active_panel();
    assert_eq!(app.active_panel_state().filter.as_deref(), Some("beta"));
}

#[test]
fn test_tab_max_limit_is_five() {
    let mut app = make_test_app();

    for _ in 0..4 {
        app.new_tab_active_panel();
    }
    assert_eq!(app.left_tabs.len(), 5);

    app.new_tab_active_panel();
    assert_eq!(app.left_tabs.len(), 5);
    assert_eq!(app.toast_display(), Some("Max 5 tabs per panel"));
}

#[test]
fn test_tab_list_dialog_select_and_switch() {
    let mut app = make_test_app();
    app.new_tab_active_panel();
    app.new_tab_active_panel();
    assert_eq!(app.left_tabs.active_index(), 2);

    app.show_tab_list();
    app.tab_list_move_up();
    app.tab_list_move_up();
    app.tab_list_confirm();

    assert_eq!(app.left_tabs.active_index(), 0);
    assert!(app.dialog.is_none());
}

#[test]
fn test_directory_navigation_records_history() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let root = temp.path().to_path_buf();
    let child = root.join("child");
    fs::create_dir_all(&child).unwrap();

    app.go_to_mount_point(root.clone());
    assert_eq!(app.active_panel_state().current_path, root);

    let fs = FileSystem::new();
    let _ = app.active_panel_state_mut().refresh(&fs);
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.name == "child")
        .unwrap();
    app.active_panel_state_mut().selected_index = entry_index + offset;
    app.enter_selected();

    assert_eq!(app.active_panel_state().current_path, child);
    let history = &app.active_panel_state().history_entries;
    assert!(history.contains(&root));
    assert!(history.contains(&app.active_panel_state().current_path));
}

#[test]
fn test_default_multi_archive_name_uses_current_dir_name_or_archive_for_root() {
    let regular = App::default_multi_archive_name(Path::new("/tmp/my_docs"));
    assert_eq!(regular, "my_docs.zip");

    #[cfg(unix)]
    {
        let root = App::default_multi_archive_name(Path::new("/"));
        assert_eq!(root, "archive.zip");
    }
}

#[test]
fn test_next_unique_archive_path_recursively_avoids_duplicates() {
    let temp = TempDir::new().unwrap();
    let base = temp.path();

    fs::write(base.join("archive.zip"), b"1").unwrap();
    fs::write(base.join("archive_(1).zip"), b"2").unwrap();
    fs::write(base.join("archive_(2).zip"), b"3").unwrap();

    let next = App::next_unique_archive_path(base, "archive.zip");
    assert_eq!(
        next.file_name().and_then(OsStr::to_str),
        Some("archive_(3).zip")
    );
}

#[test]
fn test_confirm_archive_create_dialog_blocks_existing_output_path() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();

    let src_file = base.join("a.txt");
    fs::write(&src_file, b"a").unwrap();
    let archive_path = base.join("base.zip");
    fs::write(&archive_path, b"existing").unwrap();

    app.archive_flow = Some(ArchiveFlowContext::CreatePending {
        sources: vec![src_file],
    });
    app.dialog = Some(DialogKind::ArchiveCreateOptions {
        path_value: archive_path.to_string_lossy().to_string(),
        path_cursor_pos: archive_path.to_string_lossy().len(),
        use_password: false,
        password_value: String::new(),
        password_cursor_pos: 0,
        password_confirm_value: String::new(),
        password_confirm_cursor_pos: 0,
        focused_field: 4,
        selected_button: 0,
        base_path: base.clone(),
    });

    app.confirm_archive_create_dialog();

    assert!(
        app.archive_worker.is_none(),
        "archive worker must not start"
    );
    assert!(matches!(
        app.dialog,
        Some(DialogKind::ArchiveCreateOptions { .. })
    ));
    assert!(app
        .toast_display()
        .is_some_and(|msg| msg.contains("Archive already exists")));
}

#[test]
fn test_enter_selected_opens_archive_preview_for_archive_file() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();

    let zip_path = base.join("sample.zip");
    let file = std::fs::File::create(&zip_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"hello").unwrap();
    writer.finish().unwrap();

    app.go_to_mount_point(base.clone());
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == zip_path)
        .expect("archive entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;

    app.enter_selected();

    assert!(app.dialog.is_none());
    assert!(app.archive_panel_view.is_some());
    assert!(app
        .active_panel_state()
        .current_path
        .to_string_lossy()
        .contains("sample.zip::/"));
}

#[test]
fn test_start_copy_in_archive_view_opens_copy_destination_dialog() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let target = temp.path().join("target");
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(&target).unwrap();

    let zip_path = base.join("sample.zip");
    let file = std::fs::File::create(&zip_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"hello").unwrap();
    writer.finish().unwrap();

    app.go_to_mount_point(base.clone());
    app.toggle_panel();
    app.go_to_mount_point(target.clone());
    app.toggle_panel();

    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == zip_path)
        .expect("archive entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;
    app.enter_selected();
    app.active_panel_state_mut().selected_index = 1;

    app.start_copy();

    assert!(matches!(
        app.dialog,
        Some(DialogKind::Input {
            purpose: InputPurpose::OperationDestination,
            ..
        })
    ));
    assert!(matches!(
        app.archive_flow,
        Some(ArchiveFlowContext::CopyFromPanel { .. })
    ));
}

#[test]
fn test_archive_copy_shows_conflict_dialog_on_duplicate_destination() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let target = temp.path().join("target");
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(&target).unwrap();

    let zip_path = base.join("sample.zip");
    let file = std::fs::File::create(&zip_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"from-archive").unwrap();
    writer.finish().unwrap();
    fs::write(target.join("inside.txt"), "existing").unwrap();

    app.go_to_mount_point(base.clone());
    app.toggle_panel();
    app.go_to_mount_point(target.clone());
    app.toggle_panel();

    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == zip_path)
        .expect("archive entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;
    app.enter_selected();
    app.active_panel_state_mut().selected_index = 1;

    app.start_copy();
    app.confirm_input_dialog(target.to_string_lossy().to_string());
    app.process_next_file();

    assert!(matches!(app.dialog, Some(DialogKind::Conflict { .. })));
}

#[test]
fn test_extract_no_password_zip_does_not_prompt_password() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("plain.zip");
    let dest_dir = temp.path().join("out");
    fs::create_dir_all(&dest_dir).unwrap();

    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"hello").unwrap();
    writer.finish().unwrap();

    app.archive_flow = Some(ArchiveFlowContext::ExtractPending {
        archive_path: archive_path.clone(),
        format: ArchiveFormat::Zip,
    });
    app.dialog = Some(DialogKind::archive_extract_path_input(
        dest_dir.to_string_lossy().to_string(),
        temp.path().to_path_buf(),
    ));

    app.confirm_input_dialog(dest_dir.to_string_lossy().to_string());

    assert!(!matches!(
        app.dialog,
        Some(DialogKind::Input {
            purpose: InputPurpose::ArchivePassword,
            ..
        })
    ));
    assert!(app.archive_worker.is_some(), "extract worker should start");
}

#[test]
fn test_start_archive_extract_uses_inactive_panel_path_as_default() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let left_dir = temp.path().join("left");
    let right_dir = temp.path().join("right");
    fs::create_dir_all(&left_dir).unwrap();
    fs::create_dir_all(&right_dir).unwrap();

    let archive_path = left_dir.join("plain.zip");
    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"hello").unwrap();
    writer.finish().unwrap();

    app.go_to_mount_point(left_dir.clone());
    app.toggle_panel();
    app.go_to_mount_point(right_dir.clone());
    app.toggle_panel();

    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == archive_path)
        .expect("archive entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;

    app.start_archive_extract();

    match &app.dialog {
        Some(DialogKind::Input {
            value,
            base_path,
            purpose,
            ..
        }) => {
            assert_eq!(*purpose, InputPurpose::ArchiveExtractDestination);
            assert_eq!(Path::new(value), right_dir.as_path());
            assert_eq!(base_path, &right_dir);
        }
        other => panic!("expected extract input dialog, got {:?}", other),
    }
}

#[test]
fn test_extract_encrypted_zip_prompts_password() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("secret.zip");
    let dest_dir = temp.path().join("out");
    fs::create_dir_all(&dest_dir).unwrap();

    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .with_aes_encryption(AesMode::Aes256, "pw1234");
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"secret").unwrap();
    writer.finish().unwrap();

    app.archive_flow = Some(ArchiveFlowContext::ExtractPending {
        archive_path: archive_path.clone(),
        format: ArchiveFormat::Zip,
    });
    app.dialog = Some(DialogKind::archive_extract_path_input(
        dest_dir.to_string_lossy().to_string(),
        temp.path().to_path_buf(),
    ));

    app.confirm_input_dialog(dest_dir.to_string_lossy().to_string());

    assert!(matches!(
        app.dialog,
        Some(DialogKind::Input {
            purpose: InputPurpose::ArchivePassword,
            ..
        })
    ));
    assert!(matches!(
        app.archive_flow,
        Some(ArchiveFlowContext::ExtractNeedsPassword { .. })
    ));
}

#[test]
fn test_extract_conflict_prompts_file_exists_dialog() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("plain.zip");
    let dest_dir = temp.path().join("out");
    fs::create_dir_all(&dest_dir).unwrap();

    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"new-content").unwrap();
    writer.finish().unwrap();
    fs::write(dest_dir.join("inside.txt"), "old-content").unwrap();

    app.archive_flow = Some(ArchiveFlowContext::ExtractPending {
        archive_path: archive_path.clone(),
        format: ArchiveFormat::Zip,
    });
    app.dialog = Some(DialogKind::archive_extract_path_input(
        dest_dir.to_string_lossy().to_string(),
        temp.path().to_path_buf(),
    ));

    app.confirm_input_dialog(dest_dir.to_string_lossy().to_string());

    assert!(matches!(app.dialog, Some(DialogKind::Conflict { .. })));
    assert!(matches!(
        app.archive_flow,
        Some(ArchiveFlowContext::ExtractConflictPrompt { .. })
    ));
}

#[test]
fn test_extract_conflict_confirm_overwrites_destination() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("plain.zip");
    let dest_dir = temp.path().join("out");
    fs::create_dir_all(&dest_dir).unwrap();

    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("inside.txt", options).unwrap();
    writer.write_all(b"new-content").unwrap();
    writer.finish().unwrap();
    fs::write(dest_dir.join("inside.txt"), "old-content").unwrap();

    app.archive_flow = Some(ArchiveFlowContext::ExtractPending {
        archive_path: archive_path.clone(),
        format: ArchiveFormat::Zip,
    });
    app.dialog = Some(DialogKind::archive_extract_path_input(
        dest_dir.to_string_lossy().to_string(),
        temp.path().to_path_buf(),
    ));

    app.confirm_input_dialog(dest_dir.to_string_lossy().to_string());
    app.handle_conflict(ConflictResolution::Overwrite);
    run_archive_operation_until_done(&mut app);

    assert_eq!(
        fs::read_to_string(dest_dir.join("inside.txt")).unwrap(),
        "new-content"
    );
}

#[test]
fn test_auto_extract_multi_root_uses_archive_name_directory() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();

    let archive_path = base.join("multi.zip");
    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("a.txt", options).unwrap();
    writer.write_all(b"a").unwrap();
    writer.start_file("b.txt", options).unwrap();
    writer.write_all(b"b").unwrap();
    writer.finish().unwrap();

    app.go_to_mount_point(base.clone());
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == archive_path)
        .expect("archive entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;

    app.start_archive_extract_auto();
    run_archive_operation_until_done(&mut app);

    assert!(base.join("multi").join("a.txt").exists());
    assert!(base.join("multi").join("b.txt").exists());
    assert!(!base.join("a.txt").exists());
}

#[test]
fn test_auto_extract_single_root_extracts_into_root_directory() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();

    let archive_path = base.join("single.zip");
    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = ZipWriter::new(file);
    let options = ZipFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("docs/a.txt", options).unwrap();
    writer.write_all(b"a").unwrap();
    writer.finish().unwrap();

    app.go_to_mount_point(base.clone());
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == archive_path)
        .expect("archive entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;

    app.start_archive_extract_auto();
    run_archive_operation_until_done(&mut app);

    assert!(base.join("docs").join("a.txt").exists());
    assert!(!base.join("single").join("docs").join("a.txt").exists());
}

#[test]
fn test_history_back_forward_index_based_navigation() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let p1 = temp.path().join("p1");
    let p2 = temp.path().join("p2");
    let p3 = temp.path().join("p3");
    fs::create_dir_all(&p1).unwrap();
    fs::create_dir_all(&p2).unwrap();
    fs::create_dir_all(&p3).unwrap();

    app.go_to_mount_point(p1.clone());
    app.go_to_mount_point(p2.clone());
    app.go_to_mount_point(p3.clone());
    assert_eq!(app.active_panel_state().current_path, p3);

    app.history_back();
    assert_eq!(app.active_panel_state().current_path, p2);
    app.history_back();
    assert_eq!(app.active_panel_state().current_path, p1);
    app.history_forward();
    assert_eq!(app.active_panel_state().current_path, p2);
}

#[test]
fn test_history_list_default_selection_and_confirm() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let p1 = temp.path().join("p1");
    let p2 = temp.path().join("p2");
    let p3 = temp.path().join("p3");
    fs::create_dir_all(&p1).unwrap();
    fs::create_dir_all(&p2).unwrap();
    fs::create_dir_all(&p3).unwrap();

    app.go_to_mount_point(p1.clone());
    app.go_to_mount_point(p2.clone());
    app.go_to_mount_point(p3.clone());
    app.history_back();
    assert_eq!(app.active_panel_state().current_path, p2);

    app.show_history_list();
    if let Some(DialogKind::HistoryList {
        items,
        selected_index,
    }) = &app.dialog
    {
        assert_eq!(*selected_index, 1);
        assert!(items[*selected_index].2);
    } else {
        panic!("history list dialog not shown");
    }

    // 최신 항목(p3) 선택 후 이동
    app.history_list_move_up();
    app.history_list_confirm();
    assert_eq!(app.active_panel_state().current_path, p3);
    assert!(app.dialog.is_none());
}

#[test]
fn test_history_is_independent_per_tab() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    let c = temp.path().join("c");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    fs::create_dir_all(&c).unwrap();

    app.go_to_mount_point(a.clone());
    app.new_tab_active_panel();
    app.go_to_mount_point(b.clone());
    app.go_to_mount_point(c.clone());
    assert_eq!(app.active_panel_state().current_path, c);

    app.prev_tab_active_panel();
    assert_eq!(app.active_panel_state().current_path, a);
    app.history_back();
    assert_ne!(app.active_panel_state().current_path, b);

    app.next_tab_active_panel();
    assert_eq!(app.active_panel_state().current_path, c);
    app.history_back();
    assert_eq!(app.active_panel_state().current_path, b);
}

#[test]
fn test_history_list_clear_all_keeps_current_only() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let p1 = temp.path().join("p1");
    let p2 = temp.path().join("p2");
    fs::create_dir_all(&p1).unwrap();
    fs::create_dir_all(&p2).unwrap();

    app.go_to_mount_point(p1);
    app.go_to_mount_point(p2.clone());
    assert!(app.active_panel_state().history_entries.len() >= 2);

    app.show_history_list();
    app.history_list_clear_all();

    assert_eq!(app.active_panel_state().history_entries, vec![p2.clone()]);
    assert_eq!(app.active_panel_state().history_index, 0);
    if let Some(DialogKind::HistoryList {
        items,
        selected_index,
    }) = &app.dialog
    {
        assert_eq!(*selected_index, 0);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].1, p2);
        assert!(items[0].2);
    } else {
        panic!("history list dialog not shown");
    }
}

#[test]
fn test_dialog_input_completion_prefers_active_tab_history() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let history_dir = base.join("docs_history").join("nested");
    let fs_dir = base.join("docs_fs");
    fs::create_dir_all(&history_dir).unwrap();
    fs::create_dir_all(&fs_dir).unwrap();

    app.go_to_mount_point(base.clone());
    {
        let panel = app.active_panel_state_mut();
        panel.history_entries = vec![base.clone(), history_dir.clone()];
        panel.history_index = 1;
    }

    app.dialog = Some(DialogKind::operation_path_input(
        "Copy",
        "Copy to:",
        "doc",
        base.clone(),
    ));
    app.update_input_completion_state();

    if let Some(DialogKind::Input {
        completion_candidates,
        completion_index,
        ..
    }) = &app.dialog
    {
        assert_eq!(
            completion_candidates.first().map(String::as_str),
            Some("docs_history")
        );
        assert_eq!(
            completion_candidates.get(1).map(String::as_str),
            Some("docs_fs")
        );
        assert_eq!(*completion_index, Some(0));
    } else {
        panic!("input dialog not shown");
    }

    app.dialog_input_apply_selected_completion();
    if let Some(DialogKind::Input {
        value, cursor_pos, ..
    }) = &app.dialog
    {
        assert_eq!(value, "docs_history");
        assert_eq!(*cursor_pos, value.len());
    } else {
        panic!("input dialog not shown");
    }

    app.dialog_input_toggle_button();
    if let Some(DialogKind::Input {
        selected_button,
        value,
        ..
    }) = &app.dialog
    {
        assert_eq!(*selected_button, 1);
        assert_eq!(value, "docs_history");
    } else {
        panic!("input dialog not shown");
    }
}

#[test]
fn test_dialog_input_cycle_next_prev_applies_completion() {
    let mut app = make_test_app();
    app.dialog = Some(DialogKind::go_to_path_input("", PathBuf::from(".")));
    if let Some(DialogKind::Input {
        completion_candidates,
        completion_index,
        ..
    }) = &mut app.dialog
    {
        *completion_candidates = vec!["alpha".to_string(), "beta".to_string()];
        *completion_index = Some(0);
    }

    app.dialog_input_cycle_completion_next();
    assert_eq!(app.get_dialog_input_value().as_deref(), Some("beta"));

    app.dialog_input_cycle_completion_prev();
    assert_eq!(app.get_dialog_input_value().as_deref(), Some("alpha"));
}

#[test]
fn test_go_to_path_relative_success() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let child = base.join("child");
    fs::create_dir_all(&child).unwrap();

    app.go_to_mount_point(base.clone());
    app.start_go_to_path();
    app.confirm_input_dialog("child".to_string());

    assert_eq!(app.active_panel_state().current_path, child);
    assert!(app.dialog.is_none());
}

#[test]
fn test_go_to_path_fails_for_missing_directory() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();

    app.go_to_mount_point(base);
    app.start_go_to_path();
    app.confirm_input_dialog("missing_dir".to_string());

    assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
}

#[test]
fn test_go_to_path_fails_for_non_directory() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let file = base.join("file.txt");
    fs::create_dir_all(&base).unwrap();
    fs::write(&file, "data").unwrap();

    app.go_to_mount_point(base);
    app.start_go_to_path();
    app.confirm_input_dialog("file.txt".to_string());

    assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
}

#[test]
fn test_start_delete_sets_default_button_and_pending_delete() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let file = base.join("sample.txt");
    fs::create_dir_all(&base).unwrap();
    fs::write(&file, "payload").unwrap();

    app.go_to_mount_point(base.clone());
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == file)
        .expect("file entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;

    app.start_delete();

    match &app.dialog {
        Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) => {
            assert_eq!(*selected_button, 0);
        }
        other => panic!("expected delete confirm dialog, got {:?}", other),
    }
    let pending = app.pending_operation.as_ref().expect("pending operation");
    assert_eq!(pending.operation_type, OperationType::Delete);
}

#[test]
fn test_start_permanent_delete_sets_default_button_and_pending_delete() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let file = base.join("sample.txt");
    fs::create_dir_all(&base).unwrap();
    fs::write(&file, "payload").unwrap();

    app.go_to_mount_point(base.clone());
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == file)
        .expect("file entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;

    app.start_permanent_delete();

    match &app.dialog {
        Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) => {
            assert_eq!(*selected_button, 1);
        }
        other => panic!("expected delete confirm dialog, got {:?}", other),
    }
    let pending = app.pending_operation.as_ref().expect("pending operation");
    assert_eq!(pending.operation_type, OperationType::Delete);
}

#[test]
fn test_start_open_default_app_rejects_parent_entry() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();

    app.go_to_mount_point(base);
    app.active_panel_state_mut().selected_index = 0;
    app.start_open_default_app();

    assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
}

#[test]
fn test_start_open_default_app_rejects_directory() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let dir = base.join("docs");
    fs::create_dir_all(&dir).unwrap();

    app.go_to_mount_point(base.clone());
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == dir)
        .expect("directory entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;
    app.start_open_default_app();

    assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
}

#[test]
fn test_start_open_terminal_editor_rejects_parent_entry() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();

    app.go_to_mount_point(base);
    app.active_panel_state_mut().selected_index = 0;
    app.start_open_terminal_editor();

    assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
    assert!(app.take_pending_terminal_editor_request().is_none());
}

#[test]
fn test_start_open_terminal_editor_rejects_directory() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let dir = base.join("docs");
    fs::create_dir_all(&dir).unwrap();

    app.go_to_mount_point(base.clone());
    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == dir)
        .expect("directory entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;
    app.start_open_terminal_editor();

    assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
    assert!(app.take_pending_terminal_editor_request().is_none());
}

#[test]
fn test_start_open_terminal_editor_queues_request_for_file() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let file = base.join("notes.txt");
    fs::create_dir_all(&base).unwrap();
    fs::write(&file, "hello").unwrap();

    app.go_to_mount_point(base.clone());
    app.execute_action(Action::SetDefaultEditorVim);

    let has_parent = app.active_panel_state().current_path.parent().is_some();
    let offset = if has_parent { 1 } else { 0 };
    let entry_index = app
        .active_panel_state()
        .entries
        .iter()
        .position(|e| e.path == file)
        .expect("file entry should exist");
    app.active_panel_state_mut().selected_index = entry_index + offset;
    app.start_open_terminal_editor();

    let request = app
        .take_pending_terminal_editor_request()
        .expect("request should be queued");
    assert_eq!(request.editor_command, "vim");
    assert_eq!(request.target_path, file);
}

#[test]
fn test_apply_terminal_editor_result_sets_toast_on_success() {
    let mut app = make_test_app();
    let request = TerminalEditorRequest {
        editor_command: "vi".to_string(),
        target_path: PathBuf::from("/tmp/example.txt"),
    };

    app.apply_terminal_editor_result(&request, Ok(()));

    assert_eq!(app.toast_display(), Some("Edited: example.txt"));
    assert!(app.dialog.is_none());
}

#[test]
fn test_apply_terminal_editor_result_shows_error_on_failure() {
    let mut app = make_test_app();
    let request = TerminalEditorRequest {
        editor_command: "vi".to_string(),
        target_path: PathBuf::from("/tmp/example.txt"),
    };

    app.apply_terminal_editor_result(&request, Err("Failed to start 'vi': not found".to_string()));

    match &app.dialog {
        Some(DialogKind::Error { message, .. }) => {
            assert!(message.contains("Open in terminal editor failed."));
            assert!(message.contains("Failed to start 'vi'"));
        }
        other => panic!("expected error dialog, got {:?}", other),
    }
}

#[test]
fn test_editor_preset_actions_update_default_editor() {
    let mut app = make_test_app();

    app.execute_action(Action::SetDefaultEditorVim);
    assert_eq!(app.default_terminal_editor, "vim");
    assert_eq!(app.toast_display(), Some("Default editor: vim"));

    app.execute_action(Action::SetDefaultEditorNano);
    assert_eq!(app.default_terminal_editor, "nano");
    assert_eq!(app.toast_display(), Some("Default editor: nano"));

    app.execute_action(Action::SetDefaultEditorEmacs);
    assert_eq!(app.default_terminal_editor, "emacs");
    assert_eq!(app.toast_display(), Some("Default editor: emacs"));

    app.execute_action(Action::SetDefaultEditorVi);
    assert_eq!(app.default_terminal_editor, "vi");
    assert_eq!(app.toast_display(), Some("Default editor: vi"));
}

#[test]
fn test_about_action_opens_message_dialog() {
    let mut app = make_test_app();

    app.execute_action(Action::About);

    match &app.dialog {
        Some(DialogKind::Message { title, message }) => {
            assert_eq!(title, "About BokslDir");
            assert!(message.contains("BokslDir"));
        }
        other => panic!("expected about message dialog, got {:?}", other),
    }
}

#[test]
fn test_apply_open_default_app_result_sets_toast_on_success() {
    let mut app = make_test_app();
    let file_path = PathBuf::from("/tmp/example.txt");

    app.apply_open_default_app_result(&file_path, Ok(()));

    assert_eq!(app.toast_display(), Some("Opened: example.txt"));
    assert!(app.dialog.is_none());
}

#[test]
fn test_apply_open_default_app_result_shows_error_on_failure() {
    let mut app = make_test_app();
    let file_path = PathBuf::from("/tmp/example.txt");
    let error = BokslDirError::ExternalOpenFailed {
        path: file_path.clone(),
        reason: "mock failure".to_string(),
    };

    app.apply_open_default_app_result(&file_path, Err(error));

    match &app.dialog {
        Some(DialogKind::Error { message, .. }) => {
            assert!(message.contains("Open with default app failed."));
            assert!(message.contains("mock failure"));
        }
        other => panic!("expected error dialog, got {:?}", other),
    }
}

#[test]
fn test_operation_destination_accepts_relative_path_with_base() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let source = base.join("source.txt");
    let target = base.join("target");
    fs::create_dir_all(&target).unwrap();
    fs::write(&source, "payload").unwrap();

    app.pending_operation = Some(PendingOperation::new(
        OperationType::Copy,
        vec![source],
        base.clone(),
    ));
    app.dialog = Some(DialogKind::operation_path_input(
        "Copy",
        "Copy to:",
        "target",
        base.clone(),
    ));

    app.confirm_input_dialog("target".to_string());

    let pending = app.pending_operation.as_ref().expect("pending operation");
    assert_eq!(pending.dest_dir, target);
    assert!(matches!(app.dialog, Some(DialogKind::Progress { .. })));
}

#[test]
fn test_operation_destination_accepts_absolute_path() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    let source = base.join("source.txt");
    let target = temp.path().join("target_abs");
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(&source, "payload").unwrap();

    app.pending_operation = Some(PendingOperation::new(
        OperationType::Copy,
        vec![source],
        base.clone(),
    ));
    app.dialog = Some(DialogKind::operation_path_input(
        "Copy",
        "Copy to:",
        target.to_string_lossy(),
        base,
    ));

    app.confirm_input_dialog(target.to_string_lossy().to_string());

    let pending = app.pending_operation.as_ref().expect("pending operation");
    assert_eq!(pending.dest_dir, target);
    assert!(matches!(app.dialog, Some(DialogKind::Progress { .. })));
}

#[test]
fn test_app_state_encode_decode_roundtrip() {
    let mut app = make_test_app();
    app.bookmarks = vec![PersistedBookmark {
        name: "A".to_string(),
        path: PathBuf::from("/a"),
    }];
    app.left_tabs.active_mut().history_entries = vec![PathBuf::from("/l1"), PathBuf::from("/l2")];
    app.left_tabs.active_mut().history_index = 1;
    app.right_tabs.active_mut().history_entries = vec![PathBuf::from("/r1")];
    app.right_tabs.active_mut().history_index = 0;
    app.switch_theme_and_save("light");

    let text = app.encode_app_state().unwrap();
    let decoded = App::decode_app_state(&text).unwrap();
    assert_eq!(decoded.version, App::APP_STATE_VERSION);
    assert_eq!(decoded.theme, "light");
    assert_eq!(
        decoded.history.left.entries,
        vec![PathBuf::from("/l1"), PathBuf::from("/l2")]
    );
    assert_eq!(decoded.history.left.index, 1);
    assert_eq!(decoded.history.right.entries, vec![PathBuf::from("/r1")]);
    assert_eq!(decoded.history.right.index, 0);
    assert_eq!(decoded.bookmarks.len(), 1);
    assert_eq!(decoded.bookmarks[0].name, "A");
}

#[test]
fn test_apply_loaded_history_keeps_non_consecutive_duplicates() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    app.apply_loaded_history(
        ActivePanel::Left,
        vec![a.clone(), b.clone(), a.clone(), b.clone(), a.clone()],
        3,
    );

    let history = &app.left_active_panel_state().history_entries;
    assert_eq!(history.len(), 5);
    assert_eq!(history[0], a);
    assert_eq!(history[1], b);
    assert_eq!(history[2], a);
    assert_eq!(history[3], b);
    assert_eq!(history[4], a);
    assert_eq!(app.left_active_panel_state().history_index, 3);
    assert_eq!(app.left_active_panel_state().current_path, b);
}

#[test]
fn test_apply_loaded_history_clamps_index() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();

    app.apply_loaded_history(ActivePanel::Left, vec![a.clone(), b.clone()], 99);

    let panel = app.left_active_panel_state();
    assert_eq!(panel.history_entries, vec![a, b.clone()]);
    assert_eq!(panel.history_index, 1);
    assert_eq!(panel.current_path, b);
}

#[test]
fn test_add_bookmark_stores_current_path_and_prevents_duplicate_path() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let p1 = temp.path().join("p1");
    fs::create_dir_all(&p1).unwrap();

    app.go_to_mount_point(p1.clone());
    app.add_bookmark_current_dir();
    assert_eq!(app.bookmarks.len(), 1);
    assert_eq!(app.bookmarks[0].path, p1);

    app.add_bookmark_current_dir();
    assert_eq!(app.bookmarks.len(), 1);
    assert_eq!(app.toast_display(), Some("Bookmark already exists"));
}

#[test]
fn test_add_bookmark_assigns_unique_name_suffix() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let p1 = root.join("same");
    let p2 = root.join("x").join("same");
    fs::create_dir_all(&p1).unwrap();
    fs::create_dir_all(&p2).unwrap();

    app.go_to_mount_point(p1);
    app.add_bookmark_current_dir();
    app.go_to_mount_point(p2);
    app.add_bookmark_current_dir();

    assert_eq!(app.bookmarks.len(), 2);
    assert_eq!(app.bookmarks[0].name, "same");
    assert_eq!(app.bookmarks[1].name, "same (2)");
}

#[test]
fn test_bookmark_list_confirm_moves_to_selected_path() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let p1 = temp.path().join("p1");
    let p2 = temp.path().join("p2");
    fs::create_dir_all(&p1).unwrap();
    fs::create_dir_all(&p2).unwrap();

    app.go_to_mount_point(p1.clone());
    app.add_bookmark_current_dir();
    app.go_to_mount_point(p2.clone());
    app.add_bookmark_current_dir();

    app.go_to_mount_point(p1);
    app.show_bookmark_list();
    app.bookmark_list_move_down();
    app.bookmark_list_confirm();

    assert_eq!(app.active_panel_state().current_path, p2);
    assert!(app.dialog.is_none());
}

#[test]
fn test_bookmark_delete_reindexes_and_closes_on_last_delete() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let p1 = temp.path().join("p1");
    let p2 = temp.path().join("p2");
    fs::create_dir_all(&p1).unwrap();
    fs::create_dir_all(&p2).unwrap();

    app.go_to_mount_point(p1);
    app.add_bookmark_current_dir();
    app.go_to_mount_point(p2);
    app.add_bookmark_current_dir();

    app.show_bookmark_list();
    app.bookmark_list_move_down();
    app.bookmark_list_delete_selected();
    assert_eq!(app.bookmarks.len(), 1);
    if let Some(DialogKind::BookmarkList {
        items,
        selected_index,
    }) = &app.dialog
    {
        assert_eq!(items.len(), 1);
        assert_eq!(*selected_index, 0);
    } else {
        panic!("bookmark list dialog not shown");
    }

    app.bookmark_list_delete_selected();
    assert!(app.bookmarks.is_empty());
    assert!(app.dialog.is_none());
}

#[test]
fn test_bookmark_rename_validates_and_applies_unique_suffix() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let p1 = temp.path().join("p1");
    let p2 = temp.path().join("p2");
    fs::create_dir_all(&p1).unwrap();
    fs::create_dir_all(&p2).unwrap();

    app.bookmarks = vec![
        PersistedBookmark {
            name: "Work".to_string(),
            path: p1,
        },
        PersistedBookmark {
            name: "Notes".to_string(),
            path: p2,
        },
    ];

    app.confirm_bookmark_rename("   ".to_string(), 0);
    assert_eq!(app.bookmarks[0].name, "Work");
    assert_eq!(app.toast_display(), Some("Bookmark name cannot be empty"));

    app.confirm_bookmark_rename("Notes".to_string(), 0);
    assert_eq!(app.bookmarks[0].name, "Notes (2)");
    if let Some(DialogKind::BookmarkList { selected_index, .. }) = &app.dialog {
        assert_eq!(*selected_index, 0);
    } else {
        panic!("bookmark list dialog not shown");
    }
}

#[test]
fn test_save_persisted_state_writes_single_file() {
    let mut app = make_test_app();
    let state_path = app.state_store_override.clone().unwrap();
    app.bookmarks = vec![PersistedBookmark {
        name: "Temp".to_string(),
        path: PathBuf::from("/tmp"),
    }];
    app.switch_theme_and_save("light");
    app.save_persisted_state().unwrap();

    assert!(state_path.exists());
    let text = fs::read_to_string(&state_path).unwrap();
    assert!(text.contains("theme = \"light\""));
    assert!(text.contains("[history.left]"));
    assert!(text.contains("[history.right]"));
    assert!(text.contains("[[bookmarks]]"));
}

#[test]
fn test_load_persisted_state_restores_theme_history_bookmarks() {
    let mut app = make_test_app();
    let state_path = app.state_store_override.clone().unwrap();
    let temp = TempDir::new().unwrap();
    let left = temp.path().join("left");
    let right = temp.path().join("right");
    fs::create_dir_all(&left).unwrap();
    fs::create_dir_all(&right).unwrap();

    app.bookmarks = vec![PersistedBookmark {
        name: "Temp".to_string(),
        path: PathBuf::from("/tmp"),
    }];
    app.left_tabs.active_mut().history_entries = vec![left.clone()];
    app.left_tabs.active_mut().history_index = 0;
    app.right_tabs.active_mut().history_entries = vec![right.clone()];
    app.right_tabs.active_mut().history_index = 0;
    app.switch_theme_and_save("light");
    app.save_persisted_state().unwrap();

    let mut loaded = make_test_app();
    loaded.state_store_override = Some(state_path);
    loaded.bookmarks.clear();
    loaded.load_persisted_state();

    assert_eq!(
        loaded.theme_manager.current().bg_primary.to_color(),
        Color::Rgb(255, 255, 255)
    );
    assert_eq!(loaded.bookmarks.len(), 1);
    assert_eq!(loaded.bookmarks[0].name, "Temp");
    assert_eq!(loaded.left_tabs.active().history_entries, vec![left]);
    assert_eq!(loaded.right_tabs.active().history_entries, vec![right]);
}

#[test]
fn test_theme_switch_persists_via_unified_state() {
    let mut app = make_test_app();
    let state_path = app.state_store_override.clone().unwrap();
    app.switch_theme_and_save("light");

    let mut loaded = make_test_app();
    loaded.state_store_override = Some(state_path);
    loaded.load_persisted_state();
    assert_eq!(
        loaded.theme_manager.current().bg_primary.to_color(),
        Color::Rgb(255, 255, 255)
    );
}

#[test]
fn test_move_operation_removes_source_directories_when_successful() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let src_root = temp.path().join("src_root");
    let empty_dir = src_root.join("empty");
    let nested_dir = src_root.join("nested");
    let nested_file = nested_dir.join("data.txt");
    let dest_root = temp.path().join("dest_root");

    fs::create_dir_all(&empty_dir).unwrap();
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(&nested_file, "payload").unwrap();
    fs::create_dir_all(&dest_root).unwrap();

    let mut pending = PendingOperation::new(
        OperationType::Move,
        vec![src_root.clone()],
        dest_root.clone(),
    );
    app.prepare_and_start_operation(&mut pending, &dest_root);
    app.pending_operation = Some(pending);

    run_file_operation_until_done(&mut app);

    let moved_root = dest_root.join("src_root");
    assert!(!src_root.exists());
    assert!(moved_root.join("empty").is_dir());
    assert!(moved_root.join("nested").is_dir());
    assert_eq!(
        fs::read_to_string(moved_root.join("nested").join("data.txt")).unwrap(),
        "payload"
    );
}

#[test]
fn test_confirm_mkdir_uses_toast_and_focuses_new_directory() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(base.join("alpha")).unwrap();
    fs::create_dir_all(&base).unwrap();

    app.go_to_mount_point(base.clone());
    app.confirm_mkdir("new_dir".to_string(), base.clone());

    assert!(app.dialog.is_none());
    assert_eq!(app.toast_display(), Some("Directory 'new_dir' created."));
    assert_eq!(
        app.active_panel_state()
            .entries
            .get(app.active_panel_state().selected_index.saturating_sub(1))
            .map(|e| e.name.as_str()),
        Some("new_dir")
    );
}

#[test]
fn test_confirm_rename_uses_toast_and_focuses_new_name() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("base");
    fs::create_dir_all(&base).unwrap();
    let old = base.join("old_name");
    fs::write(&old, "x").unwrap();

    app.go_to_mount_point(base.clone());
    app.confirm_rename("new_name".to_string(), old);

    assert!(app.dialog.is_none());
    assert_eq!(app.toast_display(), Some("Rename completed"));
    assert_eq!(
        app.active_panel_state()
            .entries
            .get(app.active_panel_state().selected_index.saturating_sub(1))
            .map(|e| e.name.as_str()),
        Some("new_name")
    );
}

#[test]
fn test_cancel_operation_uses_toast() {
    let mut app = make_test_app();
    let mut pending = PendingOperation::new(OperationType::Copy, Vec::new(), PathBuf::new());
    pending.start_processing(0, 3);
    pending.progress.files_completed = 1;
    app.pending_operation = Some(pending);
    app.dialog = Some(DialogKind::progress(
        app.pending_operation
            .as_ref()
            .expect("pending set")
            .progress
            .clone(),
    ));

    app.cancel_operation();

    assert!(app.dialog.is_none());
    assert_eq!(app.toast_display(), Some("Copy cancelled (1/3)"));
}

#[cfg(unix)]
#[test]
fn test_copy_or_move_symlink_directory_fails_explicitly_and_continues() {
    let mut app = make_test_app();
    let temp = TempDir::new().unwrap();
    let src_root = temp.path().join("src_root");
    let target_dir = temp.path().join("target_dir");
    let dir_link = src_root.join("dir_link");
    let regular_file = src_root.join("regular.txt");
    let dest_root = temp.path().join("dest_root");

    fs::create_dir_all(&src_root).unwrap();
    fs::create_dir_all(&target_dir).unwrap();
    fs::write(target_dir.join("hidden.txt"), "target").unwrap();
    fs::write(&regular_file, "regular").unwrap();
    unix_fs::symlink(&target_dir, &dir_link).unwrap();
    fs::create_dir_all(&dest_root).unwrap();

    let mut pending = PendingOperation::new(
        OperationType::Copy,
        vec![dir_link.clone(), regular_file.clone()],
        dest_root.clone(),
    );
    app.prepare_and_start_operation(&mut pending, &dest_root);
    app.pending_operation = Some(pending);

    run_file_operation_until_done(&mut app);

    let dest_regular = dest_root.join("regular.txt");
    assert_eq!(fs::read_to_string(dest_regular).unwrap(), "regular");

    let error_text = match &app.dialog {
        Some(DialogKind::Error { message, .. }) => message.clone(),
        other => panic!("expected error dialog, got {:?}", other),
    };
    assert!(error_text.contains("Directory symlink is not supported"));
}
