use super::text_edit::TextBufferEdit;
use super::*;

impl App {
    // === 파일 복사/이동 관련 메서드 (Phase 3.2) ===

    /// 비활성 패널 상태 반환
    pub fn inactive_panel_state(&self) -> &PanelState {
        match self.layout.active_panel() {
            ActivePanel::Left => self.right_tabs.active(),
            ActivePanel::Right => self.left_tabs.active(),
        }
    }

    /// 재귀 복사/이동 검사 (복수 소스)
    ///
    /// 디렉토리를 자기 자신 내부로 복사/이동하려는 경우 에러 메시지 반환
    pub(super) fn check_recursive_operation(
        sources: &[PathBuf],
        operation_type: OperationType,
        dest_path: &std::path::Path,
    ) -> Option<String> {
        for source in sources {
            if !Self::is_recursive_path(source, dest_path) {
                continue;
            }
            let name = source.file_name().unwrap_or_default().to_string_lossy();
            return Some(format!(
                "Cannot {} '{}' into itself.\n\
                 The destination is inside the source directory.",
                operation_type.name().to_lowercase(),
                name
            ));
        }
        None
    }

    /// 재귀 경로 검사 (정적 메서드)
    pub(super) fn is_recursive_path(source: &std::path::Path, dest: &std::path::Path) -> bool {
        if !source.is_dir() {
            return false;
        }
        let Ok(canonical_source) = source.canonicalize() else {
            return false;
        };
        let Ok(canonical_dest) = dest.canonicalize() else {
            return false;
        };
        canonical_dest.starts_with(&canonical_source)
    }

    /// 작업 대상 파일 목록 가져오기
    ///
    /// 선택된 항목이 있으면 선택된 항목들, 없으면 커서 위치의 항목
    pub fn get_operation_sources(&self) -> Vec<PathBuf> {
        let panel = self.active_panel_state();

        // 선택된 항목이 있으면 그것들 반환
        if !panel.selected_items.is_empty() {
            return panel
                .selected_entries()
                .iter()
                .map(|e| e.path.clone())
                .collect();
        }

        // 선택된 항목이 없으면 커서 위치의 항목 반환
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        // ".." 항목이면 빈 벡터 반환
        if has_parent && selected_index == 0 {
            return Vec::new();
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        panel
            .entries
            .get(entry_index)
            .map(|e| vec![e.path.clone()])
            .unwrap_or_default()
    }

    /// 다이얼로그 활성 여부
    pub fn is_dialog_active(&self) -> bool {
        self.dialog.is_some()
    }

    /// 다이얼로그 닫기
    pub fn close_dialog(&mut self) {
        self.dialog = None;
        self.pending_operation = None;
        self.archive_flow = None;
    }

    /// 진행 중인 작업 취소
    pub fn cancel_operation(&mut self) {
        if let Some(worker) = &self.archive_worker {
            worker.cancel_flag.store(true, Ordering::Relaxed);
            self.set_toast("Archive cancel requested...");
            return;
        }

        if let Some(pending) = self.pending_operation.take() {
            // 패널 새로고침 (일부 복사된 파일 반영)
            self.refresh_both_panels();
            self.cleanup_archive_copy_temp_dir();

            // 취소 토스트 표시
            self.dialog = None;
            self.set_toast(&format!(
                "{} cancelled ({}/{})",
                pending.operation_type.name(),
                pending.progress.files_completed,
                pending.progress.total_files
            ));
        } else {
            self.close_dialog();
        }
    }

    /// 복사 시작 (y)
    pub fn start_copy(&mut self) {
        if self.is_active_panel_archive_view() {
            self.start_archive_copy_dialog();
            return;
        }
        self.start_file_operation(OperationType::Copy);
    }

    /// 이동 시작 (x)
    pub fn start_move(&mut self) {
        self.start_file_operation(OperationType::Move);
    }

    /// 압축 시작 (zc)
    pub fn start_archive_compress(&mut self) {
        let sources = self.get_operation_sources();
        if sources.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Information",
                "No files selected for archive.",
            ));
            return;
        }

        let base_path = self.active_panel_state().current_path.clone();
        let suggested = if sources.len() == 1 {
            let stem = sources[0]
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or("archive");
            format!("{}.zip", stem)
        } else {
            Self::default_multi_archive_name(&base_path)
        };
        let suggested_path = Self::next_unique_archive_path(&base_path, &suggested);
        let initial = suggested_path.to_string_lossy().to_string();
        self.dialog = Some(DialogKind::archive_create_options_input(initial, base_path));
        self.archive_flow = Some(ArchiveFlowContext::CreatePending { sources });
    }

    pub(super) fn default_multi_archive_name(current_dir: &Path) -> String {
        if current_dir.parent().is_none() {
            return "archive.zip".to_string();
        }
        let Some(name) = current_dir.file_name().and_then(OsStr::to_str) else {
            return "archive.zip".to_string();
        };
        if name.trim().is_empty() {
            "archive.zip".to_string()
        } else {
            format!("{}.zip", name)
        }
    }

    pub(super) fn next_unique_archive_path(base_dir: &Path, desired_filename: &str) -> PathBuf {
        let desired = Path::new(desired_filename);
        let stem = desired
            .file_stem()
            .and_then(OsStr::to_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("archive");
        let extension = desired.extension().and_then(OsStr::to_str);

        let make_name = |index: usize| -> String {
            let base = if index == 0 {
                stem.to_string()
            } else {
                format!("{}_({})", stem, index)
            };
            if let Some(ext) = extension {
                if ext.is_empty() {
                    base
                } else {
                    format!("{}.{}", base, ext)
                }
            } else {
                base
            }
        };

        let mut index = 0usize;
        loop {
            let candidate = base_dir.join(make_name(index));
            if !candidate.exists() {
                return candidate;
            }
            index += 1;
        }
    }

    /// 압축 해제 시작 (zx)
    pub fn start_archive_extract(&mut self) {
        let archive_path = match self.focused_open_target() {
            Ok(path) => path,
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error("Extract archive", None, &reason, ""),
                ));
                return;
            }
        };
        let Some(format) = detect_archive_format(&archive_path) else {
            self.dialog = Some(DialogKind::error(
                "Error",
                Self::format_user_error(
                    "Extract archive",
                    Some(&archive_path),
                    "Unsupported archive format",
                    "Supported: zip/tar/tar.gz/tar.zst/7z/jar/war",
                ),
            ));
            return;
        };

        let base_path = self.inactive_panel_state().current_path.clone();
        let initial = base_path.to_string_lossy().to_string();
        self.dialog = Some(DialogKind::archive_extract_path_input(initial, base_path));
        self.archive_flow = Some(ArchiveFlowContext::ExtractPending {
            archive_path,
            format,
        });
        self.update_input_completion_state();
    }

    pub(super) fn detect_single_root_dir(entries: &[ArchiveEntry]) -> Option<String> {
        use std::collections::BTreeSet;

        let mut top_levels = BTreeSet::new();
        for entry in entries {
            let normalized = Self::normalize_archive_entry_path(&entry.path);
            if normalized.is_empty() {
                continue;
            }
            if let Some(first) = normalized.split('/').next() {
                top_levels.insert(first.to_string());
            }
        }

        if top_levels.len() != 1 {
            return None;
        }
        let root = top_levels.into_iter().next()?;

        let has_nested = entries.iter().any(|entry| {
            let normalized = Self::normalize_archive_entry_path(&entry.path);
            normalized.starts_with(&format!("{}/", root))
        });
        let has_root_dir_entry = entries
            .iter()
            .any(|entry| entry.is_dir && Self::normalize_archive_entry_path(&entry.path).eq(&root));

        if has_nested || has_root_dir_entry {
            Some(root)
        } else {
            None
        }
    }

    pub(super) fn auto_extract_base_name(archive_path: &Path) -> String {
        let file_name = archive_path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("archive")
            .to_string();
        let lower = file_name.to_ascii_lowercase();

        let suffixes = [
            ".tar.gz", ".tar.zst", ".tgz", ".tzst", ".zip", ".7z", ".jar", ".war", ".tar",
        ];
        for suffix in suffixes {
            if lower.ends_with(suffix) && file_name.len() > suffix.len() {
                let base = &file_name[..file_name.len() - suffix.len()];
                if !base.trim().is_empty() {
                    return base.to_string();
                }
            }
        }

        archive_path
            .file_stem()
            .and_then(OsStr::to_str)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("archive")
            .to_string()
    }

    pub(super) fn next_unique_extract_dir(base_dir: &Path, desired_name: &str) -> PathBuf {
        let seed = desired_name.trim();
        let base_name = if seed.is_empty() { "archive" } else { seed };
        let mut index = 0usize;
        loop {
            let name = if index == 0 {
                base_name.to_string()
            } else {
                format!("{}_({})", base_name, index)
            };
            let candidate = base_dir.join(name);
            if !candidate.exists() {
                return candidate;
            }
            index += 1;
        }
    }

    pub(super) fn build_auto_extract_request(
        archive_path: &Path,
        base_dir: &Path,
        password: Option<&str>,
    ) -> Result<ArchiveExtractRequest> {
        if !base_dir.exists() || !base_dir.is_dir() {
            return Err(BokslDirError::ArchiveExtractFailed {
                path: archive_path.to_path_buf(),
                reason: format!(
                    "Destination directory does not exist: {}",
                    base_dir.display()
                ),
            });
        }

        let entries = list_entries(archive_path, password)?;
        let single_root_dir = Self::detect_single_root_dir(&entries);
        let dest_dir = if single_root_dir.is_some() {
            base_dir.to_path_buf()
        } else {
            let desired = Self::auto_extract_base_name(archive_path);
            let target_dir = Self::next_unique_extract_dir(base_dir, &desired);
            fs::create_dir_all(&target_dir).map_err(BokslDirError::Io)?;
            target_dir
        };

        Ok(ArchiveExtractRequest {
            archive_path: archive_path.to_path_buf(),
            dest_dir,
            password: password.map(|s| s.to_string()),
            overwrite_existing: false,
            overwrite_entries: Vec::new(),
            skip_existing_entries: Vec::new(),
            skip_all_existing: false,
        })
    }

    /// 압축 해제 시작 (za, 자동 대상 폴더)
    pub fn start_archive_extract_auto(&mut self) {
        let archive_path = match self.focused_open_target() {
            Ok(path) => path,
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error("Auto extract archive", None, &reason, ""),
                ));
                return;
            }
        };
        let Some(format) = detect_archive_format(&archive_path) else {
            self.dialog = Some(DialogKind::error(
                "Error",
                Self::format_user_error(
                    "Auto extract archive",
                    Some(&archive_path),
                    "Unsupported archive format",
                    "Supported: zip/tar/tar.gz/tar.zst/7z/jar/war",
                ),
            ));
            return;
        };

        let base_dir = self.active_panel_state().current_path.clone();
        match Self::build_auto_extract_request(&archive_path, &base_dir, None) {
            Ok(request) => {
                self.prepare_archive_extract_request(request);
            }
            Err(BokslDirError::ArchivePasswordRequired { .. }) if supports_password(format) => {
                self.archive_flow = Some(ArchiveFlowContext::ExtractAutoNeedsPassword {
                    archive_path,
                    base_dir,
                });
                self.dialog = Some(DialogKind::archive_password_input("Archive Password"));
            }
            Err(err) => {
                self.archive_flow = None;
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Auto extract archive",
                        Some(&archive_path),
                        &err.to_string(),
                        "",
                    ),
                ));
            }
        }
    }

    pub(super) fn show_archive_extract_conflict_dialog(
        &mut self,
        request: ArchiveExtractRequest,
        conflicts: Vec<String>,
        current_index: usize,
    ) {
        if let Some(source_rel_path) = conflicts.get(current_index) {
            self.archive_flow = Some(ArchiveFlowContext::ExtractConflictPrompt {
                request: request.clone(),
                conflicts: conflicts.clone(),
                current_index,
            });
            self.dialog = Some(DialogKind::conflict(
                PathBuf::from(source_rel_path),
                request.dest_dir.join(source_rel_path),
            ));
            return;
        }

        self.archive_flow = None;
        self.start_archive_extract_worker(request);
    }

    pub(super) fn prepare_archive_extract_request(&mut self, request: ArchiveExtractRequest) {
        match list_extract_conflicts(
            &request.archive_path,
            &request.dest_dir,
            request.password.as_deref(),
        ) {
            Ok(conflicts) if conflicts.is_empty() => {
                self.archive_flow = None;
                self.start_archive_extract_worker(request);
            }
            Ok(conflicts) => {
                self.show_archive_extract_conflict_dialog(request, conflicts, 0);
            }
            Err(err) => {
                self.archive_flow = None;
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error("Extract archive", None, &err.to_string(), ""),
                ));
            }
        }
    }

    /// 압축 파일 미리보기 시작 (포커스된 압축 파일에서 Enter)
    pub fn start_archive_preview(&mut self) {
        let archive_path = match self.focused_open_target() {
            Ok(path) => path,
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error("Preview archive", None, &reason, ""),
                ));
                return;
            }
        };
        let Some(format) = detect_archive_format(&archive_path) else {
            self.dialog = Some(DialogKind::error(
                "Error",
                Self::format_user_error(
                    "Preview archive",
                    Some(&archive_path),
                    "Unsupported archive format",
                    "Supported: zip/tar/tar.gz/tar.zst/7z/jar/war",
                ),
            ));
            return;
        };

        match self.enter_archive_panel_view(&archive_path, None) {
            Ok(()) => {}
            Err(err) => {
                if supports_password(format)
                    && matches!(err, BokslDirError::ArchivePasswordRequired { .. })
                {
                    self.archive_flow = Some(ArchiveFlowContext::PreviewNeedsPassword {
                        archive_path,
                        panel: self.active_panel(),
                    });
                    self.dialog = Some(DialogKind::archive_password_input("Archive Password"));
                } else {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        Self::format_user_error(
                            "Preview archive",
                            Some(&archive_path),
                            &err.to_string(),
                            "",
                        ),
                    ));
                }
            }
        }
    }

    pub(super) fn panel_state_by_slot_mut(&mut self, slot: PanelSlot) -> &mut PanelState {
        match slot {
            PanelSlot::Left => self.left_tabs.active_mut(),
            PanelSlot::Right => self.right_tabs.active_mut(),
        }
    }

    pub(super) fn panel_state_by_slot(&self, slot: PanelSlot) -> &PanelState {
        match slot {
            PanelSlot::Left => self.left_tabs.active(),
            PanelSlot::Right => self.right_tabs.active(),
        }
    }

    pub(super) fn is_active_panel_archive_view(&self) -> bool {
        self.archive_panel_view
            .as_ref()
            .is_some_and(|v| v.panel == PanelSlot::from(self.active_panel()))
    }

    pub(super) fn normalize_archive_entry_path(path: &str) -> String {
        let trimmed = path.replace('\\', "/");
        trimmed.trim_matches('/').to_string()
    }

    pub(super) fn build_archive_panel_entries(
        all_entries: &[ArchiveEntry],
        current_dir: &str,
    ) -> Vec<crate::models::file_entry::FileEntry> {
        use crate::models::file_entry::{FileEntry, FileType};
        use std::collections::BTreeMap;
        use std::time::SystemTime;

        let mut map: BTreeMap<String, (bool, u64)> = BTreeMap::new();
        let prefix = if current_dir.is_empty() {
            String::new()
        } else {
            format!("{}/", current_dir)
        };

        for entry in all_entries {
            let normalized = Self::normalize_archive_entry_path(&entry.path);
            if normalized.is_empty() {
                continue;
            }
            if !prefix.is_empty() && !normalized.starts_with(&prefix) {
                continue;
            }
            let rest = if prefix.is_empty() {
                normalized.as_str()
            } else {
                &normalized[prefix.len()..]
            };
            if rest.is_empty() {
                continue;
            }
            let mut split = rest.splitn(2, '/');
            let first = split.next().unwrap_or_default();
            let has_more = split.next().is_some();
            let full_rel = if current_dir.is_empty() {
                first.to_string()
            } else {
                format!("{}/{}", current_dir, first)
            };
            let is_dir = entry.is_dir || has_more;
            let size = if is_dir { 0 } else { entry.size };
            map.entry(full_rel)
                .and_modify(|v| {
                    v.0 = v.0 || is_dir;
                    if !v.0 {
                        v.1 = size;
                    }
                })
                .or_insert((is_dir, size));
        }

        let mut entries: Vec<FileEntry> = map
            .into_iter()
            .map(|(rel, (is_dir, size))| {
                let name = rel
                    .rsplit('/')
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| rel.clone());
                FileEntry::new(
                    name,
                    PathBuf::from(rel),
                    if is_dir {
                        FileType::Directory
                    } else {
                        FileType::File
                    },
                    size,
                    SystemTime::now(),
                    SystemTime::now(),
                    None,
                    false,
                )
            })
            .collect();

        entries.sort_by(|a, b| {
            let dir_cmp = b.is_directory().cmp(&a.is_directory());
            if dir_cmp != std::cmp::Ordering::Equal {
                return dir_cmp;
            }
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        });
        entries
    }

    pub(super) fn apply_archive_view_to_panel(&mut self, view: &ArchivePanelView) {
        let display_path = if view.current_dir.is_empty() {
            format!("{}::/", view.archive_path.display())
        } else {
            format!("{}::/{}", view.archive_path.display(), view.current_dir)
        };
        let entries = Self::build_archive_panel_entries(&view.all_entries, &view.current_dir);
        let panel = self.panel_state_by_slot_mut(view.panel);
        panel.current_path = PathBuf::from(display_path);
        panel.entries = entries;
        panel.selected_items.clear();
        panel.selected_index = 0;
        panel.scroll_offset = 0;
    }

    pub(super) fn enter_archive_panel_view(
        &mut self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        let entries = list_entries(archive_path, password)?;
        let panel = PanelSlot::from(self.active_panel());
        let base_dir = archive_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/"));
        let view = ArchivePanelView {
            panel,
            archive_path: archive_path.to_path_buf(),
            base_dir,
            current_dir: String::new(),
            all_entries: entries,
            password: password.map(|s| s.to_string()),
        };
        self.apply_archive_view_to_panel(&view);
        self.archive_panel_view = Some(view);
        self.dialog = None;
        Ok(())
    }

    pub(super) fn archive_view_go_parent(&mut self) -> bool {
        let active = PanelSlot::from(self.active_panel());
        let Some(mut view) = self.archive_panel_view.clone() else {
            return false;
        };
        if view.panel != active {
            return false;
        }

        if view.current_dir.is_empty() {
            let archive_name = view
                .archive_path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.to_string());
            let filesystem = FileSystem::new();
            let panel = self.panel_state_by_slot_mut(view.panel);
            if panel
                .change_directory(view.base_dir.clone(), &filesystem)
                .is_ok()
            {
                if let Some(name) = archive_name {
                    let has_parent = panel.current_path.parent().is_some();
                    let offset = if has_parent { 1 } else { 0 };
                    if let Some(idx) = panel.entries.iter().position(|e| e.name == name) {
                        panel.selected_index = idx + offset;
                    }
                }
            }
            self.archive_panel_view = None;
            return true;
        }

        if let Some(pos) = view.current_dir.rfind('/') {
            view.current_dir = view.current_dir[..pos].to_string();
        } else {
            view.current_dir.clear();
        }
        self.apply_archive_view_to_panel(&view);
        self.archive_panel_view = Some(view);
        true
    }

    pub(super) fn archive_view_enter_selected(&mut self) -> bool {
        let active = PanelSlot::from(self.active_panel());
        let Some(mut view) = self.archive_panel_view.clone() else {
            return false;
        };
        if view.panel != active {
            return false;
        }

        let panel = self.panel_state_by_slot(active);
        let has_parent = true;
        if panel.selected_index == 0 && has_parent {
            return self.archive_view_go_parent();
        }
        let entry_index = panel.selected_index.saturating_sub(1);
        let Some(entry) = panel.entries.get(entry_index) else {
            return true;
        };
        if entry.is_directory() {
            view.current_dir = entry.path.to_string_lossy().to_string();
            self.apply_archive_view_to_panel(&view);
            self.archive_panel_view = Some(view);
        }
        true
    }

    pub(super) fn start_archive_copy_dialog(&mut self) {
        let Some(view) = self.archive_panel_view.clone() else {
            return;
        };
        if view.panel != PanelSlot::from(self.active_panel()) {
            self.start_file_operation(OperationType::Copy);
            return;
        }
        let panel = self.panel_state_by_slot(view.panel);
        let has_parent = true;
        let selected_entries: Vec<FileEntry> = if !panel.selected_items.is_empty() {
            panel.selected_entries().into_iter().cloned().collect()
        } else {
            let idx = panel.selected_index;
            if has_parent && idx == 0 {
                Vec::new()
            } else {
                panel
                    .entries
                    .get(idx.saturating_sub(1))
                    .cloned()
                    .into_iter()
                    .collect()
            }
        };
        if selected_entries.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Information",
                "No archive entries selected for copy.",
            ));
            return;
        }
        let dest_dir = self.inactive_panel_state().current_path.clone();
        let dest_path = dest_dir.to_string_lossy().to_string();
        self.archive_flow = Some(ArchiveFlowContext::CopyFromPanel {
            view,
            selected_entries,
        });
        self.dialog = Some(DialogKind::operation_path_input(
            "Copy", "Copy to:", dest_path, dest_dir,
        ));
        self.update_input_completion_state();
    }

    pub(super) fn copy_from_archive_view_to_dest(
        &mut self,
        view: &ArchivePanelView,
        selected_entries: &[FileEntry],
        dest_dir: &Path,
    ) {
        let temp_root = std::env::temp_dir().join(format!(
            "boksldir-archive-copy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        if std::fs::create_dir_all(&temp_root).is_err() {
            self.dialog = Some(DialogKind::error(
                "Error",
                "Failed to prepare temporary extraction directory.",
            ));
            return;
        }

        let (tx, _rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let extract_result = extract_archive(
            &ArchiveExtractRequest {
                archive_path: view.archive_path.clone(),
                dest_dir: temp_root.clone(),
                password: view.password.clone(),
                overwrite_existing: false,
                overwrite_entries: Vec::new(),
                skip_existing_entries: Vec::new(),
                skip_all_existing: false,
            },
            tx,
            cancel,
        );
        if let Err(err) = extract_result {
            let _ = std::fs::remove_dir_all(&temp_root);
            self.dialog = Some(DialogKind::error(
                "Error",
                Self::format_user_error(
                    "Copy from archive",
                    Some(&view.archive_path),
                    &err.to_string(),
                    "",
                ),
            ));
            return;
        }

        let sources: Vec<PathBuf> = selected_entries
            .iter()
            .map(|entry| temp_root.join(&entry.path))
            .filter(|path| path.exists())
            .collect();
        if sources.is_empty() {
            let _ = std::fs::remove_dir_all(&temp_root);
            self.dialog = Some(DialogKind::error(
                "Error",
                "No extractable archive entries were found.",
            ));
            return;
        }

        self.archive_copy_temp_dir = Some(temp_root);
        let mut pending =
            PendingOperation::new(OperationType::Copy, sources, dest_dir.to_path_buf());
        self.prepare_and_start_operation(&mut pending, dest_dir);
        self.pending_operation = Some(pending);
    }

    pub(super) fn cleanup_archive_copy_temp_dir(&mut self) {
        if let Some(path) = self.archive_copy_temp_dir.take() {
            let _ = std::fs::remove_dir_all(path);
        }
    }

    pub(super) fn open_archive_preview_list(
        &mut self,
        archive_path: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        let mut entries = list_entries(archive_path, password)?;
        let truncated = entries.len() > 5000;
        if truncated {
            entries.truncate(5000);
        }
        let items: Vec<(String, String)> = entries
            .into_iter()
            .map(|e| {
                let size = if e.is_dir {
                    "<DIR>".to_string()
                } else {
                    crate::utils::formatter::format_file_size(e.size)
                };
                (e.path, size)
            })
            .collect();
        let archive_name = archive_path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("archive")
            .to_string();
        self.dialog = Some(DialogKind::archive_preview_list(
            archive_name,
            items,
            truncated,
        ));
        Ok(())
    }

    /// 경로 직접 이동 시작 (gp)
    pub fn start_go_to_path(&mut self) {
        let base_path = self.active_panel_state().current_path.clone();
        let initial = base_path.to_string_lossy().to_string();
        self.dialog = Some(DialogKind::go_to_path_input(initial, base_path));
        self.update_input_completion_state();
    }

    /// 파일 작업 시작 (공통)
    pub(super) fn start_file_operation(&mut self, operation_type: OperationType) {
        let sources = self.get_operation_sources();

        // 작업할 파일이 없으면 종료
        if sources.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Information",
                "No files selected for operation.",
            ));
            return;
        }

        // 반대 패널의 경로를 기본 대상으로
        let dest_dir = self.inactive_panel_state().current_path.clone();
        let dest_path = dest_dir.to_string_lossy().to_string();

        // 대기 작업 저장
        self.pending_operation = Some(PendingOperation::new(
            operation_type,
            sources,
            dest_dir.clone(),
        ));

        // 입력 다이얼로그 표시
        let title = operation_type.name();
        let prompt = format!("{} to:", title);
        self.dialog = Some(DialogKind::operation_path_input(
            title, prompt, dest_path, dest_dir,
        ));
        self.update_input_completion_state();
    }

    pub(super) fn home_dir() -> Option<PathBuf> {
        env::var_os("HOME").map(PathBuf::from)
    }

    pub(super) fn split_path_input(value: &str) -> (&str, &str) {
        if let Some((idx, _)) = value
            .char_indices()
            .rev()
            .find(|(_, c)| std::path::is_separator(*c))
        {
            (&value[..=idx], &value[idx + 1..])
        } else {
            ("", value)
        }
    }

    pub(super) fn input_parent_context(
        &self,
        value: &str,
        base_path: &Path,
    ) -> (PathBuf, String, String) {
        if value == "~" {
            if let Some(home) = Self::home_dir() {
                return (
                    home,
                    format!("~{}", std::path::MAIN_SEPARATOR),
                    String::new(),
                );
            }
        }

        let (raw_parent, raw_partial) = Self::split_path_input(value);
        if raw_parent.is_empty() {
            (
                base_path.to_path_buf(),
                String::new(),
                raw_partial.to_string(),
            )
        } else {
            (
                self.resolve_input_path(raw_parent, base_path),
                raw_parent.to_string(),
                raw_partial.to_string(),
            )
        }
    }

    pub(super) fn first_segment_candidate(
        path: &Path,
        parent_path: &Path,
        display_prefix: &str,
        partial: &str,
    ) -> Option<String> {
        if !path.is_dir() {
            return None;
        }
        let rest = path.strip_prefix(parent_path).ok()?;
        let first_segment = rest.components().find_map(|c| match c {
            Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })?;
        if !first_segment.starts_with(partial) {
            return None;
        }
        Some(format!("{}{}", display_prefix, first_segment))
    }

    pub(super) fn history_completion_candidates(
        &self,
        value: &str,
        base_path: &Path,
    ) -> Vec<String> {
        let (parent_path, display_prefix, partial) = self.input_parent_context(value, base_path);
        self.active_panel_state()
            .history_entries
            .iter()
            .rev()
            .filter_map(|path| {
                Self::first_segment_candidate(path, &parent_path, &display_prefix, &partial)
            })
            .collect()
    }

    pub(super) fn filesystem_completion_candidates(
        &self,
        value: &str,
        base_path: &Path,
    ) -> Vec<String> {
        let (dir_path, display_prefix, partial) = self.input_parent_context(value, base_path);

        let mut candidates: Vec<String> = fs::read_dir(dir_path)
            .ok()
            .into_iter()
            .flat_map(|iter| iter.flatten())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_dir() {
                    return None;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if !partial.is_empty() && !name.starts_with(&partial) {
                    return None;
                }
                Some(format!("{}{}", display_prefix, name))
            })
            .collect();
        candidates.sort_unstable();
        candidates
    }

    pub(super) fn collect_input_completion_candidates(
        &self,
        value: &str,
        base_path: &Path,
    ) -> Vec<String> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        for candidate in self
            .history_completion_candidates(value, base_path)
            .into_iter()
            .chain(self.filesystem_completion_candidates(value, base_path))
        {
            if seen.insert(candidate.clone()) {
                candidates.push(candidate);
            }
        }

        candidates
    }

    pub(super) fn selected_input_completion(&self) -> Option<String> {
        match &self.dialog {
            Some(DialogKind::Input {
                completion_candidates,
                completion_index: Some(idx),
                ..
            }) => completion_candidates.get(*idx).cloned(),
            _ => None,
        }
    }

    pub(super) fn update_input_completion_state(&mut self) {
        let (value, base_path, purpose, mask_input) = match &self.dialog {
            Some(DialogKind::Input {
                value,
                base_path,
                purpose,
                mask_input,
                ..
            }) => (value.clone(), base_path.clone(), *purpose, *mask_input),
            _ => return,
        };

        let use_completion = !mask_input && !matches!(purpose, InputPurpose::ArchivePassword);
        let completion_candidates = if use_completion {
            self.collect_input_completion_candidates(&value, &base_path)
        } else {
            Vec::new()
        };
        let completion_index = if completion_candidates.is_empty() {
            None
        } else {
            Some(0)
        };

        if let Some(DialogKind::Input {
            completion_candidates: candidates,
            completion_index: selected_idx,
            ..
        }) = &mut self.dialog
        {
            *candidates = completion_candidates;
            *selected_idx = completion_index;
        }
    }

    /// 경로 입력 다이얼로그: 현재 선택 추천 적용
    pub fn dialog_input_apply_selected_completion(&mut self) {
        let Some(candidate) = self.selected_input_completion() else {
            return;
        };

        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *value != candidate {
                *value = candidate;
                *cursor_pos = value.len();
            }
        }
        self.update_input_completion_state();
    }

    /// 경로 입력 다이얼로그: 다음 추천으로 순환 + 즉시 적용
    pub fn dialog_input_cycle_completion_next(&mut self) {
        let needs_seed = matches!(
            &self.dialog,
            Some(DialogKind::Input {
                completion_candidates,
                ..
            }) if completion_candidates.is_empty()
        );
        if needs_seed {
            self.update_input_completion_state();
        }

        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            value,
            cursor_pos,
            ..
        }) = &mut self.dialog
        {
            if completion_candidates.is_empty() {
                return;
            }
            let next = completion_index
                .map(|idx| (idx + 1) % completion_candidates.len())
                .unwrap_or(0);
            *completion_index = Some(next);
            *value = completion_candidates[next].clone();
            *cursor_pos = value.len();
        }
    }

    /// 경로 입력 다이얼로그: 이전 추천으로 순환 + 즉시 적용
    pub fn dialog_input_cycle_completion_prev(&mut self) {
        let needs_seed = matches!(
            &self.dialog,
            Some(DialogKind::Input {
                completion_candidates,
                ..
            }) if completion_candidates.is_empty()
        );
        if needs_seed {
            self.update_input_completion_state();
        }

        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            value,
            cursor_pos,
            ..
        }) = &mut self.dialog
        {
            if completion_candidates.is_empty() {
                return;
            }
            let prev = completion_index
                .map(|idx| {
                    if idx == 0 {
                        completion_candidates.len() - 1
                    } else {
                        idx - 1
                    }
                })
                .unwrap_or(0);
            *completion_index = Some(prev);
            *value = completion_candidates[prev].clone();
            *cursor_pos = value.len();
        }
    }

    pub(super) fn resolve_input_path(&self, input: &str, base_path: &Path) -> PathBuf {
        let expanded = if input == "~" {
            Self::home_dir().unwrap_or_else(|| PathBuf::from(input))
        } else if let Some(rest) = input.strip_prefix("~/") {
            if let Some(home) = Self::home_dir() {
                home.join(rest)
            } else {
                PathBuf::from(input)
            }
        } else if let Some(rest) = input.strip_prefix("~\\") {
            if let Some(home) = Self::home_dir() {
                home.join(rest)
            } else {
                PathBuf::from(input)
            }
        } else {
            PathBuf::from(input)
        };

        if expanded.is_absolute() {
            expanded
        } else {
            base_path.join(expanded)
        }
    }

    /// 기존 디렉토리 경로 여부 검증. 실패 시 표준 에러 메시지 반환.
    pub(in crate::app) fn validate_existing_directory_path(
        path: &Path,
        path_display: &str,
    ) -> std::result::Result<(), String> {
        if !path.exists() {
            return Err(format!(
                "Destination path does not exist:\n{}",
                path_display
            ));
        }
        if !path.is_dir() {
            return Err(format!("Destination is not a directory:\n{}", path_display));
        }
        Ok(())
    }

    /// 대상 경로 검증 (존재/디렉토리/재귀 검사). 실패 시 에러 메시지 반환.
    pub(super) fn validate_operation_destination(
        sources: &[PathBuf],
        operation_type: OperationType,
        dest_path: &std::path::Path,
        dest_path_str: &str,
    ) -> std::result::Result<(), String> {
        Self::validate_existing_directory_path(dest_path, dest_path_str)?;
        if let Some(error_msg) = Self::check_recursive_operation(sources, operation_type, dest_path)
        {
            return Err(error_msg);
        }
        Ok(())
    }

    /// 소스 평탄화 + 크기 계산 + processing 시작
    pub(super) fn prepare_and_start_operation(
        &mut self,
        pending: &mut PendingOperation,
        dest_path: &std::path::Path,
    ) {
        let flattened: Vec<FlattenedFile> =
            match self.filesystem.flatten_sources(&pending.sources, dest_path) {
                Ok(files) => files,
                Err(e) => {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        format!("Failed to scan files: {}", e),
                    ));
                    return;
                }
            };

        // 디렉토리는 size=0, 파일/링크는 size 누적
        let total_bytes: u64 = flattened.iter().map(|f| f.size).sum();
        let total_files = flattened.len();

        if pending.operation_type == OperationType::Move {
            pending.set_move_cleanup_dirs(self.filesystem.collect_move_cleanup_dirs(&flattened));
        } else {
            pending.set_move_cleanup_dirs(Vec::new());
        }

        pending.set_flattened_files(flattened);
        pending.start_processing(total_bytes, total_files);
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
    }

    pub(super) fn remove_existing_path(path: &std::path::Path) {
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }

    /// 단일 파일/디렉토리 엔트리 처리 + 결과 기록
    pub(super) fn execute_single_file_operation(
        &self,
        pending: &mut PendingOperation,
        file_entry: &FlattenedFile,
        file_name: &str,
    ) {
        if let Some(parent) = file_entry.dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let result = match file_entry.entry_kind {
            FlattenedEntryKind::Directory => std::fs::create_dir_all(&file_entry.dest)
                .map(|_| 0)
                .map_err(crate::utils::error::BokslDirError::Io),
            FlattenedEntryKind::File | FlattenedEntryKind::SymlinkFile => {
                match pending.operation_type {
                    OperationType::Copy => self
                        .filesystem
                        .copy_file(&file_entry.source, &file_entry.dest),
                    OperationType::Move => self
                        .filesystem
                        .move_file(&file_entry.source, &file_entry.dest),
                    OperationType::Delete => unreachable!("Delete uses process_next_delete"),
                    OperationType::ArchiveCompress | OperationType::ArchiveExtract => {
                        unreachable!("Archive uses process_next_archive")
                    }
                }
            }
            FlattenedEntryKind::SymlinkDirectory => {
                let message = "Directory symlink is not supported for copy/move";
                match pending.operation_type {
                    OperationType::Copy => Err(crate::utils::error::BokslDirError::CopyFailed {
                        src: file_entry.source.clone(),
                        dest: file_entry.dest.clone(),
                        reason: message.to_string(),
                    }),
                    OperationType::Move => Err(crate::utils::error::BokslDirError::MoveFailed {
                        src: file_entry.source.clone(),
                        dest: file_entry.dest.clone(),
                        reason: message.to_string(),
                    }),
                    OperationType::Delete => unreachable!("Delete uses process_next_delete"),
                    OperationType::ArchiveCompress | OperationType::ArchiveExtract => {
                        unreachable!("Archive uses process_next_archive")
                    }
                }
            }
        };

        match result {
            Ok(bytes) => pending.files_completed(bytes, 1),
            Err(e) => {
                pending.add_error(format!("{}: {}", file_name, e));
                pending.mark_item_failed();
                pending.file_skipped();
            }
        }

        pending.current_index += 1;
    }

    pub(super) fn resolve_conflict(
        &mut self,
        pending: &mut PendingOperation,
        source: &std::path::Path,
        dest_path: &std::path::Path,
    ) -> bool {
        let skip_all = pending
            .conflict_resolution
            .is_some_and(|r| r == ConflictResolution::SkipAll);
        let overwrite_all = pending
            .conflict_resolution
            .is_some_and(|r| r == ConflictResolution::OverwriteAll);

        if skip_all {
            pending.file_skipped();
            pending.current_index += 1;
            return false;
        }
        if !overwrite_all {
            pending.state = OperationState::WaitingConflict;
            self.dialog = Some(DialogKind::conflict(
                source.to_path_buf(),
                dest_path.to_path_buf(),
            ));
            return false;
        }
        // overwrite_all이면 기존 경로를 삭제
        Self::remove_existing_path(dest_path);
        true
    }

    pub(super) fn should_resolve_conflict(file_entry: &FlattenedFile) -> bool {
        match file_entry.entry_kind {
            FlattenedEntryKind::Directory => file_entry.dest.exists() && !file_entry.dest.is_dir(),
            FlattenedEntryKind::File
            | FlattenedEntryKind::SymlinkFile
            | FlattenedEntryKind::SymlinkDirectory => file_entry.dest.exists(),
        }
    }

    pub(super) fn cleanup_moved_directories(&self, pending: &mut PendingOperation) {
        if pending.operation_type != OperationType::Move {
            return;
        }

        for dir in pending.move_cleanup_dirs.clone() {
            if let Err(e) = std::fs::remove_dir(&dir) {
                use std::io::ErrorKind;
                if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty) {
                    continue;
                }
                pending.add_error(format!(
                    "Failed to cleanup source directory {}: {}",
                    dir.display(),
                    e
                ));
            }
        }
    }

    /// 다음 파일 처리 (메인 루프에서 호출)
    pub fn process_next_file(&mut self) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        if pending.state != OperationState::Processing {
            self.pending_operation = Some(pending);
            return;
        }

        if pending.is_all_processed() {
            self.finish_operation(pending);
            return;
        }

        let file_entry = pending.flattened_files[pending.current_index].clone();
        let source = file_entry.source.clone();
        let dest_path = file_entry.dest.clone();

        let file_name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        pending.set_current_file(&file_name);
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));

        if file_entry.entry_kind != FlattenedEntryKind::Directory && source == dest_path {
            pending.add_error(format!("Source and destination are the same: {:?}", source));
            pending.mark_item_failed();
            pending.file_skipped();
            pending.current_index += 1;
            self.pending_operation = Some(pending);
            return;
        }

        if Self::should_resolve_conflict(&file_entry)
            && !self.resolve_conflict(&mut pending, &source, &dest_path)
        {
            self.pending_operation = Some(pending);
            return;
        }

        self.execute_single_file_operation(&mut pending, &file_entry, &file_name);

        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
        self.pending_operation = Some(pending);
    }

    /// 입력 다이얼로그에서 확인 처리
    pub fn confirm_input_dialog(&mut self, dest_path_str: String) {
        let Some(DialogKind::Input {
            purpose, base_path, ..
        }) = &self.dialog
        else {
            self.close_dialog();
            return;
        };

        let purpose = *purpose;
        let base_path = base_path.clone();
        let resolved_path = self.resolve_input_path(&dest_path_str, &base_path);
        let resolved_path_str = resolved_path.to_string_lossy().to_string();

        match purpose {
            InputPurpose::OperationDestination => {
                if let Some(mut pending) = self.pending_operation.take() {
                    if let Err(error_msg) = Self::validate_operation_destination(
                        &pending.sources,
                        pending.operation_type,
                        &resolved_path,
                        &resolved_path_str,
                    ) {
                        self.dialog = Some(DialogKind::error("Error", error_msg));
                        self.pending_operation = Some(pending);
                        return;
                    }

                    pending.dest_dir = resolved_path.clone();
                    self.prepare_and_start_operation(&mut pending, &resolved_path);
                    self.pending_operation = Some(pending);
                    return;
                }

                if let Some(ArchiveFlowContext::CopyFromPanel {
                    view,
                    selected_entries,
                }) = self.archive_flow.clone()
                {
                    if let Err(error_msg) =
                        Self::validate_existing_directory_path(&resolved_path, &resolved_path_str)
                    {
                        self.dialog = Some(DialogKind::error("Error", error_msg));
                        return;
                    }

                    self.archive_flow = None;
                    self.dialog = None;
                    self.copy_from_archive_view_to_dest(&view, &selected_entries, &resolved_path);
                    return;
                }

                self.close_dialog();
            }
            InputPurpose::GoToPath => {
                if let Err(error_msg) =
                    Self::validate_existing_directory_path(&resolved_path, &resolved_path_str)
                {
                    self.dialog = Some(DialogKind::error("Error", error_msg));
                    return;
                }

                if self.change_active_dir(resolved_path, true, None) {
                    self.close_dialog();
                } else {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        format!("Failed to open path:\n{}", resolved_path_str),
                    ));
                }
            }
            InputPurpose::ArchiveCreatePath => {
                let Some(flow) = self.archive_flow.clone() else {
                    self.close_dialog();
                    return;
                };
                let ArchiveFlowContext::CreatePending { sources } = flow else {
                    self.close_dialog();
                    return;
                };

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

                let request = ArchiveCreateRequest {
                    sources,
                    output_path: resolved_path.clone(),
                    password: None,
                };
                if supports_password(format) {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        "Archive create path dialog is deprecated. Use Create Archive dialog.",
                    ));
                    return;
                }
                self.archive_flow = None;
                self.start_archive_create_worker(request);
            }
            InputPurpose::ArchiveExtractDestination => {
                let Some(flow) = self.archive_flow.clone() else {
                    self.close_dialog();
                    return;
                };
                let ArchiveFlowContext::ExtractPending {
                    archive_path,
                    format,
                } = flow
                else {
                    self.close_dialog();
                    return;
                };

                if let Err(error_msg) =
                    Self::validate_existing_directory_path(&resolved_path, &resolved_path_str)
                {
                    self.dialog = Some(DialogKind::error("Error", error_msg));
                    return;
                }

                let request = ArchiveExtractRequest {
                    archive_path,
                    dest_dir: resolved_path.clone(),
                    password: None,
                    overwrite_existing: false,
                    overwrite_entries: Vec::new(),
                    skip_existing_entries: Vec::new(),
                    skip_all_existing: false,
                };
                if supports_password(format) {
                    match list_entries(&request.archive_path, None) {
                        Ok(_) => {
                            self.prepare_archive_extract_request(request);
                        }
                        Err(BokslDirError::ArchivePasswordRequired { .. }) => {
                            self.archive_flow =
                                Some(ArchiveFlowContext::ExtractNeedsPassword { request });
                            self.dialog =
                                Some(DialogKind::archive_password_input("Archive Password"));
                        }
                        Err(err) => {
                            self.archive_flow = None;
                            self.dialog = Some(DialogKind::error(
                                "Error",
                                Self::format_user_error(
                                    "Extract archive",
                                    None,
                                    &err.to_string(),
                                    "",
                                ),
                            ));
                        }
                    }
                } else {
                    self.prepare_archive_extract_request(request);
                }
            }
            InputPurpose::ArchivePassword => {
                self.confirm_archive_password_input(dest_path_str);
            }
        }
    }

    pub(super) fn start_archive_create_worker(&mut self, request: ArchiveCreateRequest) {
        let (progress_tx, progress_rx) = mpsc::channel::<ArchiveProgressEvent>();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_for_worker = Arc::clone(&cancel_flag);
        let handle =
            std::thread::spawn(move || create_archive(&request, progress_tx, cancel_for_worker));

        let progress = OperationProgress::new(OperationType::ArchiveCompress, 0, 0);
        self.archive_worker = Some(ArchiveWorkerState {
            kind: ArchiveWorkerKind::Compress,
            progress_rx,
            join_handle: Some(handle),
            cancel_flag,
            progress: progress.clone(),
        });
        self.dialog = Some(DialogKind::progress(progress));
    }

    pub(super) fn start_archive_extract_worker(&mut self, request: ArchiveExtractRequest) {
        let (progress_tx, progress_rx) = mpsc::channel::<ArchiveProgressEvent>();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_for_worker = Arc::clone(&cancel_flag);
        let handle =
            std::thread::spawn(move || extract_archive(&request, progress_tx, cancel_for_worker));

        let progress = OperationProgress::new(OperationType::ArchiveExtract, 0, 0);
        self.archive_worker = Some(ArchiveWorkerState {
            kind: ArchiveWorkerKind::Extract,
            progress_rx,
            join_handle: Some(handle),
            cancel_flag,
            progress: progress.clone(),
        });
        self.dialog = Some(DialogKind::progress(progress));
    }

    pub fn confirm_archive_password_input(&mut self, password_input: String) {
        let password = if password_input.is_empty() {
            None
        } else {
            Some(password_input)
        };
        let Some(flow) = self.archive_flow.clone() else {
            self.close_dialog();
            return;
        };

        match flow {
            ArchiveFlowContext::ExtractNeedsPassword { mut request } => {
                request.password = password;
                self.prepare_archive_extract_request(request);
            }
            ArchiveFlowContext::ExtractConflictPrompt { mut request, .. } => {
                request.password = password;
                self.prepare_archive_extract_request(request);
            }
            ArchiveFlowContext::ExtractAutoNeedsPassword {
                archive_path,
                base_dir,
            } => match Self::build_auto_extract_request(
                &archive_path,
                &base_dir,
                password.as_deref(),
            ) {
                Ok(request) => self.prepare_archive_extract_request(request),
                Err(err) => {
                    self.archive_flow = None;
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        Self::format_user_error(
                            "Auto extract archive",
                            Some(&archive_path),
                            &err.to_string(),
                            "Check password and archive integrity.",
                        ),
                    ));
                }
            },
            ArchiveFlowContext::PreviewNeedsPassword {
                archive_path,
                panel,
            } => {
                self.archive_flow = None;
                if panel != self.active_panel() {
                    self.layout.set_active_panel(panel);
                }
                match self.enter_archive_panel_view(&archive_path, password.as_deref()) {
                    Ok(()) => {}
                    Err(err) => {
                        self.dialog = Some(DialogKind::error(
                            "Error",
                            Self::format_user_error(
                                "Preview archive",
                                Some(&archive_path),
                                &err.to_string(),
                                "Check password and archive integrity.",
                            ),
                        ));
                    }
                }
            }
            _ => {
                self.close_dialog();
            }
        }
    }

    /// 다음 압축 작업 진행 상태 반영 (메인 루프에서 호출)
    pub fn process_next_archive(&mut self) {
        let Some(worker) = &mut self.archive_worker else {
            return;
        };

        while let Ok(event) = worker.progress_rx.try_recv() {
            worker.progress.current_file = event.current_file;
            worker.progress.files_completed = event.files_completed;
            worker.progress.total_files = event.total_files;
            worker.progress.bytes_copied = event.bytes_processed;
            worker.progress.total_bytes = event.total_bytes;
            worker.progress.items_processed = event.items_processed;
            worker.progress.items_failed = event.items_failed;
            self.dialog = Some(DialogKind::progress(worker.progress.clone()));
        }

        let is_finished = worker
            .join_handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished);
        if !is_finished {
            return;
        }

        let mut worker = self.archive_worker.take().unwrap_or_else(|| unreachable!());
        let Some(handle) = worker.join_handle.take() else {
            return;
        };
        let kind = worker.kind;
        let result =
            handle.join().map_err(
                |_| crate::utils::error::BokslDirError::ArchiveCreateFailed {
                    path: PathBuf::from("archive"),
                    reason: "Archive worker thread panicked".to_string(),
                },
            );
        self.finish_archive_operation(kind, result);
    }

    pub(super) fn finish_archive_operation(
        &mut self,
        kind: ArchiveWorkerKind,
        join_result: std::result::Result<
            std::result::Result<ArchiveSummary, crate::utils::error::BokslDirError>,
            crate::utils::error::BokslDirError,
        >,
    ) {
        self.refresh_both_panels();
        self.active_panel_state_mut().deselect_all();
        self.dialog = None;

        let operation_name = match kind {
            ArchiveWorkerKind::Compress => "Archive create",
            ArchiveWorkerKind::Extract => "Archive extract",
        };

        match join_result {
            Ok(Ok(summary)) => {
                if summary.cancelled {
                    self.set_toast(&format!(
                        "{} cancelled ({}/{})",
                        operation_name, summary.items_processed, summary.total_files
                    ));
                    return;
                }
                if summary.errors.is_empty() {
                    self.set_toast(&format!(
                        "{} completed: {}",
                        operation_name,
                        crate::utils::formatter::pluralize(
                            summary.items_processed,
                            "item",
                            "items"
                        )
                    ));
                } else {
                    let preview: Vec<String> = summary.errors.iter().take(5).cloned().collect();
                    let detail = if summary.errors.len() > 5 {
                        format!(
                            "{}\n... and {} more errors",
                            preview.join("\n"),
                            summary.errors.len() - 5
                        )
                    } else {
                        preview.join("\n")
                    };
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        format!(
                            "{} completed with errors.\nSucceeded: {}\nFailed: {}\n\n{}",
                            operation_name,
                            summary.items_processed.saturating_sub(summary.items_failed),
                            summary.items_failed,
                            detail
                        ),
                    ));
                }
            }
            Ok(Err(err)) | Err(err) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(operation_name, None, &err.to_string(), ""),
                ));
            }
        }
    }

    /// 진행 중인 작업 여부 확인
    pub fn is_operation_processing(&self) -> bool {
        self.pending_operation
            .as_ref()
            .is_some_and(|p| p.state == OperationState::Processing)
            || self.archive_worker.is_some()
    }

    /// 작업 완료 처리
    pub(super) fn finish_operation(&mut self, mut pending: PendingOperation) {
        self.cleanup_moved_directories(&mut pending);
        self.cleanup_archive_copy_temp_dir();

        // 패널 새로고침
        self.refresh_both_panels();

        // 결과 표시
        if pending.errors.is_empty() {
            self.close_dialog();
            self.set_toast(&format!(
                "{} completed: {}",
                pending.operation_type.name(),
                crate::utils::formatter::pluralize(pending.completed_count, "file", "files")
            ));
        } else {
            let preview: Vec<String> = pending.errors.iter().take(5).cloned().collect();
            let detail = if pending.errors.len() > 5 {
                format!(
                    "{}\n... and {} more errors",
                    preview.join("\n"),
                    pending.errors.len() - 5
                )
            } else {
                preview.join("\n")
            };
            let error_msg = format!(
                "{} completed with errors.\nSucceeded: {}\nFailed: {}\n\n{}",
                pending.operation_type.name(),
                pending.completed_count,
                pending.errors.len(),
                detail
            );
            self.dialog = Some(DialogKind::error("Error", error_msg));
        }

        // 선택 상태 초기화
        self.active_panel_state_mut().deselect_all();
    }

    /// 파일 작업 실행 (레거시 호환용 - 충돌 해결 후 재개)
    pub fn execute_file_operation(&mut self) {
        if let Some(pending) = self.pending_operation.as_mut() {
            pending.state = OperationState::Processing;
        }
    }

    /// 대상 파일/디렉토리 삭제 (Overwrite/OverwriteAll 공용)
    pub(super) fn remove_existing_dest(&self) {
        if let Some(DialogKind::Conflict { dest_path, .. }) = &self.dialog {
            let dest = dest_path.clone();
            if dest.is_dir() {
                let _ = std::fs::remove_dir_all(&dest);
            } else {
                let _ = std::fs::remove_file(&dest);
            }
        }
    }

    /// 현재 파일 건너뛰기 + 인덱스 증가 (Skip/SkipAll 공용)
    pub(super) fn skip_current_file(&mut self) {
        if let Some(pending) = self.pending_operation.as_mut() {
            pending.file_skipped();
            pending.current_index += 1;
        }
    }

    /// 충돌 해결 처리
    pub fn handle_conflict(&mut self, resolution: ConflictResolution) {
        if let Some(ArchiveFlowContext::ExtractConflictPrompt {
            mut request,
            conflicts,
            current_index,
        }) = self.archive_flow.clone()
        {
            let current_path = conflicts.get(current_index).cloned();
            match resolution {
                ConflictResolution::Cancel => {
                    self.close_dialog();
                }
                ConflictResolution::Overwrite => {
                    if let Some(path) = current_path {
                        request.overwrite_entries.push(path);
                    }
                    self.show_archive_extract_conflict_dialog(
                        request,
                        conflicts,
                        current_index + 1,
                    );
                }
                ConflictResolution::Skip => {
                    if let Some(path) = current_path {
                        request.skip_existing_entries.push(path);
                    }
                    self.show_archive_extract_conflict_dialog(
                        request,
                        conflicts,
                        current_index + 1,
                    );
                }
                ConflictResolution::OverwriteAll => {
                    request.overwrite_existing = true;
                    self.archive_flow = None;
                    self.start_archive_extract_worker(request);
                }
                ConflictResolution::SkipAll => {
                    request.skip_all_existing = true;
                    self.archive_flow = None;
                    self.start_archive_extract_worker(request);
                }
            }
            return;
        }

        match resolution {
            ConflictResolution::Cancel => {
                if let Some(pending) = self.pending_operation.take() {
                    self.finish_operation(pending);
                } else {
                    self.close_dialog();
                }
            }
            ConflictResolution::Overwrite => {
                self.remove_existing_dest();
                self.execute_file_operation();
            }
            ConflictResolution::Skip => {
                self.skip_current_file();
                self.execute_file_operation();
            }
            ConflictResolution::OverwriteAll => {
                self.remove_existing_dest();
                if let Some(pending) = self.pending_operation.as_mut() {
                    pending.conflict_resolution = Some(ConflictResolution::OverwriteAll);
                }
                self.execute_file_operation();
            }
            ConflictResolution::SkipAll => {
                self.skip_current_file();
                if let Some(pending) = self.pending_operation.as_mut() {
                    pending.conflict_resolution = Some(ConflictResolution::SkipAll);
                }
                self.execute_file_operation();
            }
        }
    }

    // === 파일 삭제 관련 메서드 (Phase 3.3) ===

    pub(in crate::app) fn prepare_delete_pending_dialog(&mut self, selected_button: usize) {
        let sources = self.get_operation_sources();

        if sources.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Information",
                "No files selected for deletion.",
            ));
            return;
        }

        // 파일명 목록 생성
        let items: Vec<String> = sources
            .iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if p.is_dir() {
                    format!("{}/", name)
                } else {
                    name
                }
            })
            .collect();

        // 총 크기 계산
        let (total_bytes, total_files) = self
            .filesystem
            .calculate_total_size(&sources)
            .unwrap_or((0, 0));
        let total_size = format!(
            "{}, {}",
            crate::utils::formatter::pluralize(total_files, "file", "files"),
            crate::utils::formatter::format_file_size(total_bytes)
        );

        // 대기 작업 저장
        let mut pending = PendingOperation::new(OperationType::Delete, sources, PathBuf::new());
        pending.progress.total_bytes = total_bytes;
        pending.progress.total_files = total_files;
        self.pending_operation = Some(pending);

        self.dialog = Some(DialogKind::DeleteConfirm {
            items,
            total_size,
            selected_button,
        });
    }

    /// 삭제 시작 (d)
    pub fn start_delete(&mut self) {
        self.prepare_delete_pending_dialog(0);
    }

    /// 삭제 확인 처리
    pub fn confirm_delete(&mut self, use_trash: bool) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        if use_trash {
            // 휴지통으로 이동: 한 번에 처리
            match self.filesystem.trash_items(&pending.sources) {
                Ok(()) => {
                    self.refresh_both_panels();
                    self.active_panel_state_mut().deselect_all();
                    self.dialog = None;
                    self.set_toast(&format!(
                        "Moved {} to trash.",
                        crate::utils::formatter::pluralize(pending.sources.len(), "item", "items")
                    ));
                }
                Err(e) => {
                    self.refresh_both_panels();
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        Self::format_user_error(
                            "Move to trash",
                            pending.sources.first().map(|p| p.as_path()),
                            &e.to_string(),
                            "Check permissions and available disk space.",
                        ),
                    ));
                }
            }
        } else {
            // 영구 삭제: Progress 다이얼로그 표시 + Processing 시작
            let total_bytes = pending.progress.total_bytes;
            let total_files = pending.sources.len();
            pending.start_processing(total_bytes, total_files);
            self.dialog = Some(DialogKind::progress(pending.progress.clone()));
            self.pending_operation = Some(pending);
        }
    }

    /// 파일/디렉토리 삭제 실행 + 결과 기록
    pub(super) fn execute_single_delete(
        &self,
        pending: &mut PendingOperation,
        source: &std::path::Path,
        file_name: &str,
    ) {
        let result = if source.is_dir() {
            self.filesystem.delete_directory(source)
        } else {
            self.filesystem.delete_file(source)
        };

        match result {
            Ok(bytes) => pending.files_completed(bytes, 1),
            Err(e) => {
                pending.add_error(format!("{}: {}", file_name, e));
                pending.mark_item_failed();
                pending.file_skipped();
            }
        }

        pending.current_index += 1;
    }

    /// 다음 삭제 항목 처리 (메인 루프에서 호출)
    pub fn process_next_delete(&mut self) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        if pending.state != OperationState::Processing {
            self.pending_operation = Some(pending);
            return;
        }

        if pending.current_index >= pending.sources.len() {
            self.finish_operation(pending);
            return;
        }

        let source = pending.sources[pending.current_index].clone();
        let file_name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        pending.set_current_file(&file_name);
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));

        self.execute_single_delete(&mut pending, &source, &file_name);

        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
        self.pending_operation = Some(pending);
    }

    /// Delete 작업 여부 확인
    pub fn is_delete_operation(&self) -> bool {
        self.pending_operation
            .as_ref()
            .is_some_and(|p| p.operation_type == OperationType::Delete)
    }

    /// Archive 작업 여부 확인
    pub fn is_archive_operation(&self) -> bool {
        self.archive_worker.is_some()
    }

    // === DeleteConfirm 다이얼로그 입력 처리 ===

    /// 삭제 확인 다이얼로그: 버튼 이동 (다음)
    pub fn dialog_delete_confirm_next(&mut self) {
        if let Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = (*selected_button + 1) % 3;
        }
    }

    /// 삭제 확인 다이얼로그: 버튼 이동 (이전)
    pub fn dialog_delete_confirm_prev(&mut self) {
        if let Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 {
                2
            } else {
                *selected_button - 1
            };
        }
    }

    /// 삭제 확인 다이얼로그: 선택된 버튼 반환
    pub fn get_delete_confirm_button(&self) -> Option<usize> {
        if let Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    // === Phase 3.4: 기타 파일 작업 ===

    /// 새 디렉토리 생성 시작 (a)
    pub fn start_mkdir(&mut self) {
        let parent_path = self.active_panel_state().current_path.clone();
        self.dialog = Some(DialogKind::mkdir_input(parent_path));
    }

    /// 새 디렉토리 생성 확인
    pub fn confirm_mkdir(&mut self, dir_name: String, parent_path: PathBuf) {
        let dir_name = dir_name.trim().to_string();

        if dir_name.is_empty() {
            self.dialog = Some(DialogKind::error(
                "Error",
                "Create directory failed.\nReason: Name cannot be empty.\nHint: Enter at least one character.",
            ));
            return;
        }

        let new_path = parent_path.join(&dir_name);

        match self.filesystem.create_directory(&new_path) {
            Ok(()) => {
                self.refresh_both_panels();
                self.focus_active_entry_by_name(&dir_name);
                self.dialog = None;
                self.set_toast(&format!("Directory '{}' created.", dir_name));
            }
            Err(e) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Create directory",
                        Some(&new_path),
                        &e.to_string(),
                        "Use a valid name and check write permission.",
                    ),
                ));
            }
        }
    }

    /// 이름 변경 시작 (r)
    pub fn start_rename(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        // ".." 선택 시 무시
        if has_parent && selected_index == 0 {
            return;
        }

        // 커서 위치의 항목 이름 변경
        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        if let Some(entry) = panel.entries.get(entry_index) {
            let original_path = entry.path.clone();
            let current_name = entry.name.clone();
            self.dialog = Some(DialogKind::rename_input(original_path, current_name));
        }
    }

    pub(super) fn focused_open_target(&self) -> std::result::Result<PathBuf, String> {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        if has_parent && selected_index == 0 {
            return Err("Cannot open parent entry ('..').".to_string());
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        let Some(entry) = panel.entries.get(entry_index) else {
            return Err("No file selected.".to_string());
        };

        if entry.is_directory() || entry.path.is_dir() {
            return Err("Only files can be opened in Phase 7.1.".to_string());
        }

        Ok(entry.path.clone())
    }

    pub(super) fn apply_open_default_app_result(
        &mut self,
        target_path: &Path,
        result: crate::utils::error::Result<()>,
    ) {
        match result {
            Ok(()) => {
                let display_name = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| target_path.to_string_lossy().to_string());
                self.set_toast(&format!("Opened: {}", display_name));
            }
            Err(e) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open with default app",
                        Some(target_path),
                        &e.to_string(),
                        "Check file path and OS application association.",
                    ),
                ));
            }
        }
    }

    /// 기본 연결 앱으로 파일 열기 (o)
    pub fn start_open_default_app(&mut self) {
        let target_path = match self.focused_open_target() {
            Ok(path) => path,
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open with default app",
                        None,
                        &reason,
                        "Select a regular file and try again.",
                    ),
                ));
                return;
            }
        };

        let result = self.filesystem.open_with_default_app(&target_path);
        self.apply_open_default_app_result(&target_path, result);
    }

    pub(super) fn focused_terminal_editor_target(&self) -> std::result::Result<PathBuf, String> {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        if has_parent && selected_index == 0 {
            return Err("Cannot edit parent entry ('..').".to_string());
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        let Some(entry) = panel.entries.get(entry_index) else {
            return Err("No file selected.".to_string());
        };

        if entry.is_directory() || entry.path.is_dir() {
            return Err("Only files can be edited in Phase 7.2.".to_string());
        }

        Ok(entry.path.clone())
    }

    /// 터미널 에디터로 파일 열기 (e) - 실행 자체는 main 루프에서 처리
    pub fn start_open_terminal_editor(&mut self) {
        let target_path = match self.focused_terminal_editor_target() {
            Ok(path) => path,
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open in terminal editor",
                        None,
                        &reason,
                        "Select a regular file and try again.",
                    ),
                ));
                return;
            }
        };

        self.pending_terminal_editor_request = Some(TerminalEditorRequest {
            editor_command: self.default_terminal_editor.clone(),
            target_path,
        });
    }

    pub fn take_pending_terminal_editor_request(&mut self) -> Option<TerminalEditorRequest> {
        self.pending_terminal_editor_request.take()
    }

    pub fn apply_terminal_editor_result(
        &mut self,
        request: &TerminalEditorRequest,
        result: std::result::Result<(), String>,
    ) {
        match result {
            Ok(()) => {
                let display_name = request
                    .target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| request.target_path.to_string_lossy().to_string());
                self.set_toast(&format!("Edited: {}", display_name));
            }
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open in terminal editor",
                        Some(&request.target_path),
                        &reason,
                        "Check editor command and file path.",
                    ),
                ));
            }
        }
    }

    pub(super) fn set_default_terminal_editor(&mut self, editor: &str) {
        self.default_terminal_editor = editor.to_string();
        self.set_toast(&format!("Default editor: {}", editor));
    }

    pub fn set_default_editor_vi(&mut self) {
        self.set_default_terminal_editor("vi");
    }

    pub fn set_default_editor_vim(&mut self) {
        self.set_default_terminal_editor("vim");
    }

    pub fn set_default_editor_nano(&mut self) {
        self.set_default_terminal_editor("nano");
    }

    pub fn set_default_editor_emacs(&mut self) {
        self.set_default_terminal_editor("emacs");
    }

    /// 이름 변경 확인
    pub fn confirm_rename(&mut self, new_name: String, original_path: PathBuf) {
        let new_name = new_name.trim().to_string();

        if new_name.is_empty() {
            self.dialog = Some(DialogKind::error(
                "Error",
                "Rename failed.\nReason: Name cannot be empty.\nHint: Enter at least one character.",
            ));
            return;
        }

        let new_path = original_path
            .parent()
            .map(|p| p.join(&new_name))
            .unwrap_or_else(|| PathBuf::from(&new_name));

        match self.filesystem.rename_path(&original_path, &new_path) {
            Ok(()) => {
                self.refresh_both_panels();
                self.focus_active_entry_by_name(&new_name);
                self.dialog = None;
                self.set_toast("Rename completed");
            }
            Err(e) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Rename",
                        Some(&original_path),
                        &e.to_string(),
                        "Check duplicate names and write permission.",
                    ),
                ));
            }
        }
    }

    /// 디렉토리/파일 크기 문자열 생성
    pub(super) fn format_size_display(
        &self,
        entry: &crate::models::file_entry::FileEntry,
    ) -> String {
        if entry.is_directory() {
            match self
                .filesystem
                .calculate_total_size(std::slice::from_ref(&entry.path))
            {
                Ok((bytes, files)) => format!(
                    "{} ({} bytes, {})",
                    crate::utils::formatter::format_file_size(bytes),
                    crate::utils::formatter::format_number_with_commas(bytes),
                    crate::utils::formatter::pluralize(files, "file", "files")
                ),
                Err(_) => "Unknown".to_string(),
            }
        } else {
            format!(
                "{} ({} bytes)",
                crate::utils::formatter::format_file_size(entry.size),
                crate::utils::formatter::format_number_with_commas(entry.size)
            )
        }
    }

    /// 하위 항목 개수 문자열 생성
    pub(super) fn format_children_info(
        &self,
        entry: &crate::models::file_entry::FileEntry,
    ) -> Option<String> {
        if !entry.is_directory() {
            return None;
        }
        match self.filesystem.read_directory(&entry.path) {
            Ok(entries) => {
                let dirs = entries.iter().filter(|e| e.is_directory()).count();
                let files = entries.len() - dirs;
                Some(format!(
                    "{}, {}",
                    crate::utils::formatter::pluralize(files, "file", "files"),
                    crate::utils::formatter::pluralize(dirs, "dir", "dirs")
                ))
            }
            Err(_) => None,
        }
    }

    /// 파일 속성 보기 (Alt+Enter)
    pub fn show_properties(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        if has_parent && selected_index == 0 {
            return;
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        if let Some(entry) = panel.entries.get(entry_index).cloned() {
            let file_type_str = match entry.file_type {
                crate::models::file_entry::FileType::Directory => "Directory",
                crate::models::file_entry::FileType::File => "File",
                crate::models::file_entry::FileType::Symlink => "Symbolic Link",
                crate::models::file_entry::FileType::Executable => "Executable",
            };

            let size_str = self.format_size_display(&entry);
            let modified_str = crate::utils::formatter::format_date_full(entry.modified);
            let permissions_str =
                crate::utils::formatter::format_permissions(entry.permissions.as_ref());
            let children_info = self.format_children_info(&entry);

            self.dialog = Some(DialogKind::properties(
                &entry.name,
                entry.path.to_string_lossy(),
                file_type_str,
                &size_str,
                &modified_str,
                &permissions_str,
                children_info,
            ));
        }
    }

    // === MkdirInput 다이얼로그 입력 처리 ===

    pub fn dialog_mkdir_input_char(&mut self, c: char) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::insert_char(value, cursor_pos, c);
        }
    }

    pub fn dialog_mkdir_input_backspace(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::backspace(value, cursor_pos);
        }
    }

    pub fn dialog_mkdir_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete_prev_word(value, cursor_pos);
        }
    }

    pub fn dialog_mkdir_input_delete(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete(value, cursor_pos);
        }
    }

    pub fn dialog_mkdir_input_left(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::left(value, cursor_pos);
        }
    }

    pub fn dialog_mkdir_input_right(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::right(value, cursor_pos);
        }
    }

    pub fn dialog_mkdir_input_home(&mut self) {
        if let Some(DialogKind::MkdirInput { cursor_pos, .. }) = &mut self.dialog {
            TextBufferEdit::home(cursor_pos);
        }
    }

    pub fn dialog_mkdir_input_end(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::end(value, cursor_pos);
        }
    }

    pub fn dialog_mkdir_toggle_button(&mut self) {
        if let Some(DialogKind::MkdirInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_mkdir_input_value(&self) -> Option<(String, PathBuf)> {
        if let Some(DialogKind::MkdirInput {
            value, parent_path, ..
        }) = &self.dialog
        {
            Some((value.clone(), parent_path.clone()))
        } else {
            None
        }
    }

    pub fn get_mkdir_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::MkdirInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    // === RenameInput 다이얼로그 입력 처리 ===

    pub fn dialog_rename_input_char(&mut self, c: char) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::insert_char(value, cursor_pos, c);
        }
    }

    pub fn dialog_rename_input_backspace(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::backspace(value, cursor_pos);
        }
    }

    pub fn dialog_rename_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete_prev_word(value, cursor_pos);
        }
    }

    pub fn dialog_rename_input_delete(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete(value, cursor_pos);
        }
    }

    pub fn dialog_rename_input_left(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::left(value, cursor_pos);
        }
    }

    pub fn dialog_rename_input_right(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::right(value, cursor_pos);
        }
    }

    pub fn dialog_rename_input_home(&mut self) {
        if let Some(DialogKind::RenameInput { cursor_pos, .. }) = &mut self.dialog {
            TextBufferEdit::home(cursor_pos);
        }
    }

    pub fn dialog_rename_input_end(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::end(value, cursor_pos);
        }
    }

    pub fn dialog_rename_toggle_button(&mut self) {
        if let Some(DialogKind::RenameInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_rename_input_value(&self) -> Option<(String, PathBuf)> {
        if let Some(DialogKind::RenameInput {
            value,
            original_path,
            ..
        }) = &self.dialog
        {
            Some((value.clone(), original_path.clone()))
        } else {
            None
        }
    }

    pub fn get_rename_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::RenameInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    pub fn dialog_bookmark_rename_input_char(&mut self, c: char) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::insert_char(value, cursor_pos, c);
        }
    }

    pub fn dialog_bookmark_rename_input_backspace(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::backspace(value, cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete_prev_word(value, cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_input_delete(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete(value, cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_input_left(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::left(value, cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_input_right(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::right(value, cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_input_home(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput { cursor_pos, .. }) = &mut self.dialog {
            TextBufferEdit::home(cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_input_end(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::end(value, cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_toggle_button(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_bookmark_rename_input_value(&self) -> Option<(String, usize)> {
        if let Some(DialogKind::BookmarkRenameInput {
            value,
            bookmark_index,
            ..
        }) = &self.dialog
        {
            Some((value.clone(), *bookmark_index))
        } else {
            None
        }
    }

    pub fn get_bookmark_rename_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::BookmarkRenameInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    // === 숨김 파일 토글 (Phase 5.3) ===

    /// 숨김 파일 표시/숨김 토글 (양쪽 패널 동시)
    pub fn toggle_hidden(&mut self) {
        let new_val = !self.left_active_panel_state().show_hidden;
        self.left_active_panel_state_mut().show_hidden = new_val;
        self.right_active_panel_state_mut().show_hidden = new_val;
        let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
        let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
        self.set_toast(if new_val {
            "Hidden files shown"
        } else {
            "Hidden files hidden"
        });
    }

    /// 마운트 포인트 다이얼로그 표시
    pub fn show_mount_points(&mut self) {
        let points = self.filesystem.list_mount_points();
        let items: Vec<(String, std::path::PathBuf)> =
            points.into_iter().map(|mp| (mp.name, mp.path)).collect();
        if items.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Mount Points",
                "No mount points found.",
            ));
        } else {
            self.dialog = Some(DialogKind::mount_points(items));
        }
    }

    /// 활성 패널 탭 목록 다이얼로그 표시
    pub fn show_tab_list(&mut self) {
        let active_panel = self.active_panel();
        let items = self.panel_tab_titles(active_panel);
        let selected_index = self.panel_active_tab_index(active_panel);
        self.dialog = Some(DialogKind::tab_list(items, selected_index));
    }

    /// 활성 패널 디렉토리 히스토리 목록 표시 (최신순)
    pub fn show_history_list(&mut self) {
        let items = self.active_panel_state().history_items_latest_first();
        if items.is_empty() {
            self.dialog = Some(DialogKind::message("History", "No history entries."));
            return;
        }
        let selected_index = items
            .iter()
            .position(|(_, _, is_current)| *is_current)
            .unwrap_or(0);
        self.dialog = Some(DialogKind::history_list(items, selected_index));
    }

    pub(super) fn make_unique_bookmark_name(
        &self,
        desired_name: &str,
        exclude_index: Option<usize>,
    ) -> String {
        let desired = desired_name.trim();
        let base = if desired.is_empty() {
            "bookmark"
        } else {
            desired
        };
        if !self
            .bookmarks
            .iter()
            .enumerate()
            .any(|(idx, b)| Some(idx) != exclude_index && b.name.eq_ignore_ascii_case(base))
        {
            return base.to_string();
        }

        for n in 2.. {
            let candidate = format!("{} ({})", base, n);
            if !self.bookmarks.iter().enumerate().any(|(idx, b)| {
                Some(idx) != exclude_index && b.name.eq_ignore_ascii_case(&candidate)
            }) {
                return candidate;
            }
        }

        base.to_string()
    }

    pub(super) fn bookmark_items(&self) -> Vec<(String, PathBuf)> {
        self.bookmarks
            .iter()
            .map(|b| (b.name.clone(), b.path.clone()))
            .collect()
    }

    pub fn add_bookmark_current_dir(&mut self) {
        let current_path = self.active_panel_state().current_path.clone();
        if self.bookmarks.iter().any(|b| b.path == current_path) {
            self.set_toast("Bookmark already exists");
            return;
        }

        let default_name = current_path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if current_path.parent().is_none() {
                    "/".to_string()
                } else {
                    current_path.to_string_lossy().to_string()
                }
            });
        let name = self.make_unique_bookmark_name(&default_name, None);
        self.bookmarks.push(PersistedBookmark {
            name: name.clone(),
            path: current_path,
        });
        let _ = self.save_persisted_state();
        self.set_toast(&format!("Bookmark added: {}", name));
    }

    pub fn show_bookmark_list(&mut self) {
        if self.bookmarks.is_empty() {
            self.dialog = Some(DialogKind::message("Bookmarks", "No bookmarks."));
            return;
        }

        let current_path = self.active_panel_state().current_path.clone();
        let items = self.bookmark_items();
        let selected_index = items
            .iter()
            .position(|(_, path)| *path == current_path)
            .unwrap_or(0);
        self.dialog = Some(DialogKind::bookmark_list(items, selected_index));
    }

    pub fn bookmark_list_move_down(&mut self) {
        if let Some(DialogKind::BookmarkList {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    pub fn bookmark_list_move_up(&mut self) {
        if let Some(DialogKind::BookmarkList { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    pub fn bookmark_list_confirm(&mut self) {
        let (selected_index, item_len) = if let Some(DialogKind::BookmarkList {
            items,
            selected_index,
        }) = &self.dialog
        {
            (*selected_index, items.len())
        } else {
            return;
        };

        if item_len == 0 || selected_index >= item_len {
            return;
        }

        let Some(bookmark) = self.bookmarks.get(selected_index).cloned() else {
            return;
        };

        if self.change_active_dir(bookmark.path, true, None) {
            self.dialog = None;
        } else {
            self.set_toast("Failed to open bookmark path");
        }
    }

    pub fn bookmark_list_delete_selected(&mut self) {
        let selected_index =
            if let Some(DialogKind::BookmarkList { selected_index, .. }) = &self.dialog {
                *selected_index
            } else {
                return;
            };

        if selected_index >= self.bookmarks.len() {
            return;
        }

        self.bookmarks.remove(selected_index);
        let _ = self.save_persisted_state();

        if self.bookmarks.is_empty() {
            self.dialog = None;
            self.set_toast("Bookmark deleted");
            return;
        }

        let new_index = selected_index.min(self.bookmarks.len().saturating_sub(1));
        self.dialog = Some(DialogKind::bookmark_list(self.bookmark_items(), new_index));
        self.set_toast("Bookmark deleted");
    }

    pub fn start_bookmark_rename_selected(&mut self) {
        let (selected_index, item_name) = if let Some(DialogKind::BookmarkList {
            items,
            selected_index,
        }) = &self.dialog
        {
            if items.is_empty() || *selected_index >= items.len() {
                return;
            }
            (*selected_index, items[*selected_index].0.clone())
        } else {
            return;
        };

        self.dialog = Some(DialogKind::bookmark_rename_input(item_name, selected_index));
    }

    pub fn confirm_bookmark_rename(&mut self, new_name: String, bookmark_index: usize) {
        if bookmark_index >= self.bookmarks.len() {
            self.dialog = None;
            return;
        }

        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            self.set_toast("Bookmark name cannot be empty");
            return;
        }

        let unique = self.make_unique_bookmark_name(trimmed, Some(bookmark_index));
        self.bookmarks[bookmark_index].name = unique;
        let _ = self.save_persisted_state();
        self.dialog = Some(DialogKind::bookmark_list(
            self.bookmark_items(),
            bookmark_index.min(self.bookmarks.len().saturating_sub(1)),
        ));
        self.set_toast("Bookmark renamed");
    }

    /// 탭 목록 다이얼로그에서 선택 이동 (아래)
    pub fn tab_list_move_down(&mut self) {
        if let Some(DialogKind::TabList {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    /// 탭 목록 다이얼로그에서 선택 이동 (위)
    pub fn tab_list_move_up(&mut self) {
        if let Some(DialogKind::TabList { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    /// 탭 목록 다이얼로그에서 선택 확인
    pub fn tab_list_confirm(&mut self) {
        let index = if let Some(DialogKind::TabList { selected_index, .. }) = &self.dialog {
            Some(*selected_index)
        } else {
            None
        };
        if let Some(index) = index {
            self.switch_tab_active_panel(index);
            self.dialog = None;
        }
    }

    /// 히스토리 목록 다이얼로그에서 선택 이동 (아래)
    pub fn history_list_move_down(&mut self) {
        if let Some(DialogKind::HistoryList {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    /// 히스토리 목록 다이얼로그에서 선택 이동 (위)
    pub fn history_list_move_up(&mut self) {
        if let Some(DialogKind::HistoryList { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    /// 히스토리 목록 다이얼로그에서 선택 확인
    pub fn history_list_confirm(&mut self) {
        let (selected_index, item_len) = if let Some(DialogKind::HistoryList {
            items,
            selected_index,
        }) = &self.dialog
        {
            (*selected_index, items.len())
        } else {
            return;
        };

        if item_len == 0 || selected_index >= item_len {
            return;
        }

        let target_index = item_len - 1 - selected_index;
        let (target_path, old_index) = {
            let panel = self.active_panel_state_mut();
            let old = panel.history_index;
            (panel.history_jump_to(target_index), old)
        };

        if let Some(path) = target_path {
            if self.change_active_dir(path, false, None) {
                self.dialog = None;
            } else {
                self.active_panel_state_mut().history_index = old_index;
                self.set_toast("Failed to open history path");
            }
        }
    }

    /// 현재 패널 히스토리 전체 삭제 (현재 경로만 유지)
    pub fn history_list_clear_all(&mut self) {
        self.active_panel_state_mut().clear_history_to_current();
        let items = self.active_panel_state().history_items_latest_first();
        self.dialog = Some(DialogKind::history_list(items, 0));
        let _ = self.save_persisted_state();
        self.set_toast("History cleared");
    }

    pub(super) fn archive_preview_adjust_scroll(
        selected: usize,
        scroll: &mut usize,
        visible_height: usize,
    ) {
        if visible_height == 0 {
            *scroll = 0;
            return;
        }
        if selected < *scroll {
            *scroll = selected;
        } else if selected >= *scroll + visible_height {
            *scroll = selected + 1 - visible_height;
        }
    }

    pub fn archive_preview_move_down(&mut self) {
        if let Some(DialogKind::ArchivePreviewList {
            items,
            selected_index,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
                Self::archive_preview_adjust_scroll(*selected_index, scroll_offset, 12);
            }
        }
    }

    pub fn archive_preview_move_up(&mut self) {
        if let Some(DialogKind::ArchivePreviewList {
            selected_index,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            if *selected_index > 0 {
                *selected_index -= 1;
                Self::archive_preview_adjust_scroll(*selected_index, scroll_offset, 12);
            }
        }
    }

    pub fn archive_preview_page_down(&mut self) {
        if let Some(DialogKind::ArchivePreviewList {
            items,
            selected_index,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            if items.is_empty() {
                return;
            }
            *selected_index = (*selected_index + 12).min(items.len().saturating_sub(1));
            Self::archive_preview_adjust_scroll(*selected_index, scroll_offset, 12);
        }
    }

    pub fn archive_preview_page_up(&mut self) {
        if let Some(DialogKind::ArchivePreviewList {
            selected_index,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            *selected_index = selected_index.saturating_sub(12);
            Self::archive_preview_adjust_scroll(*selected_index, scroll_offset, 12);
        }
    }

    pub fn archive_preview_go_top(&mut self) {
        if let Some(DialogKind::ArchivePreviewList {
            selected_index,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            *selected_index = 0;
            *scroll_offset = 0;
        }
    }

    pub fn archive_preview_go_bottom(&mut self) {
        if let Some(DialogKind::ArchivePreviewList {
            items,
            selected_index,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            if items.is_empty() {
                *selected_index = 0;
                *scroll_offset = 0;
                return;
            }
            *selected_index = items.len() - 1;
            Self::archive_preview_adjust_scroll(*selected_index, scroll_offset, 12);
        }
    }

    /// 히스토리 뒤로 이동 (Alt+Left)
    pub fn history_back(&mut self) {
        let (target_path, old_index) = {
            let panel = self.active_panel_state_mut();
            let old = panel.history_index;
            (panel.history_back_target(), old)
        };

        if let Some(path) = target_path {
            if !self.change_active_dir(path, false, None) {
                self.active_panel_state_mut().history_index = old_index;
                self.set_toast("History back failed");
            }
        } else {
            self.set_toast("No back history");
        }
    }

    /// 히스토리 앞으로 이동 (Alt+Right)
    pub fn history_forward(&mut self) {
        let (target_path, old_index) = {
            let panel = self.active_panel_state_mut();
            let old = panel.history_index;
            (panel.history_forward_target(), old)
        };

        if let Some(path) = target_path {
            if !self.change_active_dir(path, false, None) {
                self.active_panel_state_mut().history_index = old_index;
                self.set_toast("History forward failed");
            }
        } else {
            self.set_toast("No forward history");
        }
    }

    /// 마운트 포인트로 이동
    pub fn go_to_mount_point(&mut self, path: std::path::PathBuf) {
        if self.change_active_dir(path, true, None) {
            self.dialog = None;
        }
    }

    /// 마운트 포인트 다이얼로그에서 선택 이동 (아래)
    pub fn mount_points_move_down(&mut self) {
        if let Some(DialogKind::MountPoints {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    /// 마운트 포인트 다이얼로그에서 선택 이동 (위)
    pub fn mount_points_move_up(&mut self) {
        if let Some(DialogKind::MountPoints { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    /// 마운트 포인트 다이얼로그에서 선택 확인
    pub fn mount_points_confirm(&mut self) {
        let path = if let Some(DialogKind::MountPoints {
            items,
            selected_index,
        }) = &self.dialog
        {
            items.get(*selected_index).map(|(_, p)| p.clone())
        } else {
            None
        };
        if let Some(path) = path {
            self.go_to_mount_point(path);
        }
    }

    // === 필터/검색 관련 메서드 (Phase 5.2) ===

    /// 필터 시작 (/)
    pub fn start_filter(&mut self) {
        let initial = self.active_panel_state().filter.clone();
        // 다이얼로그 취소 시 복원하기 위해 현재 필터 저장
        self.dialog = Some(DialogKind::filter_input(initial.as_deref()));
    }

    /// 필터 해제
    pub fn clear_filter(&mut self) {
        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut().set_filter(None);
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut().set_filter(None);
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
        self.set_toast("Filter cleared");
    }

    /// 필터 확인 적용
    pub fn confirm_filter(&mut self, pattern: String) {
        let pattern = pattern.trim().to_string();
        if pattern.is_empty() {
            self.clear_filter();
            self.dialog = None;
            return;
        }

        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut()
                    .set_filter(Some(pattern.clone()));
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut()
                    .set_filter(Some(pattern.clone()));
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
        self.dialog = None;
        self.set_toast(&format!("Filter: {}", pattern));
    }

    /// 라이브 필터 업데이트 (다이얼로그 입력 중 실시간 반영)
    pub fn apply_live_filter(&mut self, pattern: &str) {
        let filter = if pattern.is_empty() {
            None
        } else {
            Some(pattern.to_string())
        };
        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut().set_filter(filter);
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut().set_filter(filter);
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
    }

    /// 필터 취소 (다이얼로그 ESC — 필터 해제하고 다이얼로그 닫기)
    pub fn cancel_filter(&mut self) {
        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut().set_filter(None);
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut().set_filter(None);
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
        self.dialog = None;
    }

    // === FilterInput 다이얼로그 입력 처리 ===

    pub fn dialog_filter_input_char(&mut self, c: char) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::insert_char(value, cursor_pos, c);
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_backspace(&mut self) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::backspace(value, cursor_pos);
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_delete_prev_word(&mut self) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete_prev_word(value, cursor_pos);
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_delete(&mut self) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::delete(value, cursor_pos);
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_left(&mut self) {
        if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::left(value, cursor_pos);
        }
    }

    pub fn dialog_filter_input_right(&mut self) {
        if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::right(value, cursor_pos);
        }
    }

    pub fn dialog_filter_input_home(&mut self) {
        if let Some(DialogKind::FilterInput { cursor_pos, .. }) = &mut self.dialog {
            TextBufferEdit::home(cursor_pos);
        }
    }

    pub fn dialog_filter_input_end(&mut self) {
        if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            TextBufferEdit::end(value, cursor_pos);
        }
    }

    pub fn dialog_filter_toggle_button(&mut self) {
        if let Some(DialogKind::FilterInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_filter_input_value(&self) -> Option<String> {
        if let Some(DialogKind::FilterInput { value, .. }) = &self.dialog {
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn get_filter_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::FilterInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    /// 양쪽 패널 새로고침
    pub fn refresh_both_panels(&mut self) {
        let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
        let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
    }
}
