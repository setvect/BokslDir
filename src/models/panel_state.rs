#![allow(dead_code)]

use crate::models::file_entry::FileEntry;
use crate::system::filesystem::FileSystem;
use crate::utils::error::Result;
use std::collections::HashSet;
use std::path::PathBuf;

/// 정렬 기준
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// 이름
    Name,
    /// 크기
    Size,
    /// 수정 날짜
    Modified,
    /// 확장자
    Extension,
}

/// 정렬 순서
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// 오름차순
    Ascending,
    /// 내림차순
    Descending,
}

/// 패널 상태
#[derive(Debug, Clone)]
pub struct PanelState {
    /// 현재 경로
    pub current_path: PathBuf,
    /// 파일 목록
    pub entries: Vec<FileEntry>,
    /// 선택된 항목 인덱스 (커서 위치, ".." 포함)
    pub selected_index: usize,
    /// 스크롤 오프셋 (entries 배열 인덱스)
    pub scroll_offset: usize,
    /// 다중 선택된 항목 (entries 배열 인덱스 기반, ".." 제외)
    pub selected_items: HashSet<usize>,
    /// 정렬 기준
    pub sort_by: SortBy,
    /// 정렬 순서
    pub sort_order: SortOrder,
    /// 숨김 파일 표시 여부
    pub show_hidden: bool,
    /// 필터 패턴
    pub filter: Option<String>,
}

impl PanelState {
    /// 새 패널 상태 생성
    pub fn new(path: PathBuf) -> Self {
        Self {
            current_path: path,
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            selected_items: HashSet::new(),
            sort_by: SortBy::Name,
            sort_order: SortOrder::Ascending,
            show_hidden: false,
            filter: None,
        }
    }

    /// 파일 목록 새로고침
    ///
    /// 현재 경로의 파일 목록을 다시 읽어옵니다.
    pub fn refresh(&mut self, filesystem: &FileSystem) -> Result<()> {
        // 파일 목록 읽기
        let mut entries = filesystem.read_directory(&self.current_path)?;

        // 숨김 파일 필터링
        if !self.show_hidden {
            entries.retain(|entry| !entry.is_hidden);
        }

        // 필터 적용
        if let Some(ref filter) = self.filter {
            entries.retain(|entry| entry.name.to_lowercase().contains(&filter.to_lowercase()));
        }

        // TODO: Phase 4에서 정렬 구현
        // 현재는 정렬 없이 파일 시스템이 반환한 순서대로 사용

        self.entries = entries;

        // 디렉토리가 변경되면 선택 상태 초기화
        self.selected_items.clear();

        // 선택 인덱스가 범위를 벗어나면 조정
        if self.selected_index >= self.entries.len() && !self.entries.is_empty() {
            self.selected_index = self.entries.len() - 1;
        }

        Ok(())
    }

    /// 경로 변경
    pub fn change_directory(&mut self, path: PathBuf, filesystem: &FileSystem) -> Result<()> {
        self.current_path = path;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.selected_items.clear();
        self.refresh(filesystem)
    }

    /// 경로 변경 후 특정 항목에 포커스
    pub fn change_directory_and_focus(
        &mut self,
        path: PathBuf,
        focus_name: Option<&str>,
        filesystem: &FileSystem,
    ) -> Result<()> {
        self.current_path = path;
        self.scroll_offset = 0;
        self.selected_items.clear();
        self.refresh(filesystem)?;

        // 포커스할 항목 찾기
        if let Some(name) = focus_name {
            let has_parent = self.current_path.parent().is_some();
            let offset = if has_parent { 1 } else { 0 }; // ".." 항목 고려

            if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                self.selected_index = idx + offset;
            } else {
                self.selected_index = 0;
            }
        } else {
            self.selected_index = 0;
        }

        Ok(())
    }

    /// 선택된 항목 반환
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    /// 파일 개수 반환
    pub fn file_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_file()).count()
    }

    /// 디렉토리 개수 반환
    pub fn dir_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_directory()).count()
    }

    /// 전체 크기 반환 (바이트)
    pub fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.size).sum()
    }

    // === 다중 선택 관련 메서드 (Phase 3.1) ===

    /// 항목 선택 토글
    ///
    /// entry_index는 entries 배열의 인덱스 (0부터 시작, ".." 제외)
    pub fn toggle_selection(&mut self, entry_index: usize) {
        if entry_index >= self.entries.len() {
            return;
        }

        if self.selected_items.contains(&entry_index) {
            self.selected_items.remove(&entry_index);
        } else {
            self.selected_items.insert(entry_index);
        }
    }

    /// 전체 선택
    pub fn select_all(&mut self) {
        self.selected_items.clear();
        for i in 0..self.entries.len() {
            self.selected_items.insert(i);
        }
    }

    /// 선택 반전
    pub fn invert_selection(&mut self) {
        let mut new_selection = HashSet::new();
        for i in 0..self.entries.len() {
            if !self.selected_items.contains(&i) {
                new_selection.insert(i);
            }
        }
        self.selected_items = new_selection;
    }

    /// 전체 해제
    pub fn deselect_all(&mut self) {
        self.selected_items.clear();
    }

    /// 선택 여부 확인
    ///
    /// entry_index는 entries 배열의 인덱스
    pub fn is_selected(&self, entry_index: usize) -> bool {
        self.selected_items.contains(&entry_index)
    }

    /// 선택된 항목 개수
    pub fn selected_count(&self) -> usize {
        self.selected_items.len()
    }

    /// 선택된 항목들의 FileEntry 목록 반환
    pub fn selected_entries(&self) -> Vec<&FileEntry> {
        self.selected_items
            .iter()
            .filter_map(|&idx| self.entries.get(idx))
            .collect()
    }

    /// 선택된 항목들의 총 크기 (바이트)
    pub fn selected_size(&self) -> u64 {
        self.selected_items
            .iter()
            .filter_map(|&idx| self.entries.get(idx))
            .map(|e| e.size)
            .sum()
    }
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            current_path: PathBuf::from("."),
            entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            selected_items: HashSet::new(),
            sort_by: SortBy::Name,
            sort_order: SortOrder::Ascending,
            show_hidden: false,
            filter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_state_creation() {
        let path = PathBuf::from("/tmp");
        let state = PanelState::new(path.clone());

        assert_eq!(state.current_path, path);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.sort_by, SortBy::Name);
        assert_eq!(state.sort_order, SortOrder::Ascending);
        assert!(!state.show_hidden);
        assert!(state.selected_items.is_empty());
    }

    #[test]
    fn test_refresh() {
        let fs = FileSystem::new();
        let current_dir = std::env::current_dir().unwrap();
        let mut state = PanelState::new(current_dir);

        let result = state.refresh(&fs);
        assert!(result.is_ok());
        assert!(!state.entries.is_empty());
    }

    #[test]
    fn test_hidden_files_filtering() {
        let fs = FileSystem::new();
        let current_dir = std::env::current_dir().unwrap();

        // 숨김 파일 표시 안 함
        let mut state = PanelState::new(current_dir.clone());
        state.show_hidden = false;
        let _ = state.refresh(&fs);
        let visible_count = state.entries.len();

        // 숨김 파일 표시
        let mut state2 = PanelState::new(current_dir);
        state2.show_hidden = true;
        let _ = state2.refresh(&fs);
        let total_count = state2.entries.len();

        // 숨김 파일 표시할 때 더 많은 파일이 있어야 함 (보통의 경우)
        assert!(total_count >= visible_count);
    }

    fn create_test_entry(name: &str) -> FileEntry {
        use crate::models::file_entry::FileType;
        use std::time::SystemTime;

        FileEntry::new(
            name.to_string(),
            PathBuf::from(format!("/tmp/{}", name)),
            FileType::File,
            100,
            SystemTime::now(),
            None,
            false,
        )
    }

    #[test]
    fn test_toggle_selection() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("file1.txt"),
            create_test_entry("file2.txt"),
            create_test_entry("file3.txt"),
        ];

        // 선택
        state.toggle_selection(0);
        assert!(state.is_selected(0));
        assert!(!state.is_selected(1));
        assert_eq!(state.selected_count(), 1);

        // 해제
        state.toggle_selection(0);
        assert!(!state.is_selected(0));
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn test_select_all() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("file1.txt"),
            create_test_entry("file2.txt"),
            create_test_entry("file3.txt"),
        ];

        state.select_all();
        assert_eq!(state.selected_count(), 3);
        assert!(state.is_selected(0));
        assert!(state.is_selected(1));
        assert!(state.is_selected(2));
    }

    #[test]
    fn test_invert_selection() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("file1.txt"),
            create_test_entry("file2.txt"),
            create_test_entry("file3.txt"),
        ];

        // 첫 번째 항목만 선택
        state.toggle_selection(0);
        assert_eq!(state.selected_count(), 1);

        // 반전
        state.invert_selection();
        assert_eq!(state.selected_count(), 2);
        assert!(!state.is_selected(0));
        assert!(state.is_selected(1));
        assert!(state.is_selected(2));
    }

    #[test]
    fn test_deselect_all() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("file1.txt"),
            create_test_entry("file2.txt"),
        ];

        state.select_all();
        assert_eq!(state.selected_count(), 2);

        state.deselect_all();
        assert_eq!(state.selected_count(), 0);
    }
}
