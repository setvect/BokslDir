#![allow(dead_code)]

use crate::models::file_entry::FileEntry;
use crate::system::filesystem::FileSystem;
use crate::utils::error::Result;
use crate::utils::glob;
use std::cmp::Ordering;
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

        // 필터 적용 (글로브 패턴 또는 부분 문자열 매칭)
        if let Some(ref filter) = self.filter {
            if !filter.is_empty() {
                if glob::is_glob_pattern(filter) {
                    entries.retain(|entry| glob::glob_match(filter, &entry.name));
                } else {
                    let filter_lower = filter.to_lowercase();
                    entries.retain(|entry| entry.name.to_lowercase().contains(&filter_lower));
                }
            }
        }

        self.entries = entries;
        self.sort_entries();

        // 디렉토리가 변경되면 선택 상태 초기화
        self.selected_items.clear();

        // 선택 인덱스가 범위를 벗어나면 조정
        // selected_index는 ".." 항목 포함 UI 인덱스
        // has_parent일 때: 최대 유효 인덱스 = entries.len() (0 = "..", 1..=len = 파일들)
        // !has_parent일 때: 최대 유효 인덱스 = entries.len() - 1
        let has_parent = self.current_path.parent().is_some();
        let max_index = if has_parent {
            self.entries.len()
        } else {
            self.entries.len().saturating_sub(1)
        };

        if self.selected_index > max_index {
            self.selected_index = max_index;
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

    // === 정렬 관련 메서드 (Phase 5.1) ===

    /// 엔트리 정렬: 디렉토리 우선, 그 다음 기준별 정렬
    pub(crate) fn sort_entries(&mut self) {
        let sort_by = self.sort_by;
        let sort_order = self.sort_order;

        self.entries.sort_by(|a, b| {
            // 디렉토리 우선 (항상)
            let dir_cmp = b.is_directory().cmp(&a.is_directory());
            if dir_cmp != Ordering::Equal {
                return dir_cmp;
            }

            // 기준별 비교
            let cmp = match sort_by {
                SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::Size => a.size.cmp(&b.size),
                SortBy::Modified => a.modified.cmp(&b.modified),
                SortBy::Extension => {
                    let ext_a = extract_extension(&a.name);
                    let ext_b = extract_extension(&b.name);
                    let ext_cmp = ext_a.cmp(&ext_b);
                    if ext_cmp == Ordering::Equal {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        ext_cmp
                    }
                }
            };

            // 정렬 순서 적용
            match sort_order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });
    }

    /// 정렬 기준 설정 (같은 기준이면 순서 토글, 다르면 Ascending으로 리셋)
    pub fn set_sort(&mut self, sort_by: SortBy) {
        if self.sort_by == sort_by {
            self.sort_order = match self.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            self.sort_by = sort_by;
            self.sort_order = SortOrder::Ascending;
        }
    }

    /// 정렬 순서 명시적 설정
    pub fn set_sort_order(&mut self, order: SortOrder) {
        self.sort_order = order;
    }

    /// 정렬 상태 표시 문자열 (상태바용)
    pub fn sort_indicator(&self) -> String {
        let name = match self.sort_by {
            SortBy::Name => "Name",
            SortBy::Size => "Size",
            SortBy::Modified => "Date",
            SortBy::Extension => "Ext",
        };
        let arrow = match self.sort_order {
            SortOrder::Ascending => "▲",
            SortOrder::Descending => "▼",
        };
        format!("{} {}", name, arrow)
    }

    // === 필터 관련 메서드 (Phase 5.2) ===

    /// 필터 설정
    pub fn set_filter(&mut self, pattern: Option<String>) {
        self.filter = pattern;
    }

    /// 필터 상태 표시 문자열 (상태바용)
    pub fn filter_indicator(&self) -> Option<String> {
        self.filter
            .as_ref()
            .filter(|f| !f.is_empty())
            .map(|f| format!("Filter: {}", f))
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

/// 파일명에서 확장자 추출 (소문자)
/// `.bashrc` → "" (숨김파일은 확장자 없음), `file.tar.gz` → "gz"
fn extract_extension(name: &str) -> String {
    let search = name.strip_prefix('.').unwrap_or(name);
    match search.rfind('.') {
        Some(pos) => search[pos + 1..].to_lowercase(),
        None => String::new(),
    }
}

/// 탭 상태 (패널 내 탭 하나의 상태)
#[derive(Debug, Clone)]
pub struct TabState {
    /// 탭 ID (1-based, 표시용)
    pub id: usize,
    /// 패널 상태 (디렉토리, 파일 목록, 커서, 정렬 등)
    pub panel: PanelState,
}

impl TabState {
    pub fn new(id: usize, panel: PanelState) -> Self {
        Self { id, panel }
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

    fn create_test_entry_with_size(name: &str, size: u64) -> FileEntry {
        use crate::models::file_entry::FileType;
        use std::time::SystemTime;

        FileEntry::new(
            name.to_string(),
            PathBuf::from(format!("/tmp/{}", name)),
            FileType::File,
            size,
            SystemTime::now(),
            None,
            false,
        )
    }

    fn create_test_dir(name: &str) -> FileEntry {
        use crate::models::file_entry::FileType;
        use std::time::SystemTime;

        FileEntry::new(
            name.to_string(),
            PathBuf::from(format!("/tmp/{}", name)),
            FileType::Directory,
            0,
            SystemTime::now(),
            None,
            false,
        )
    }

    fn create_test_entry_with_time(name: &str, secs_offset: u64) -> FileEntry {
        use crate::models::file_entry::FileType;
        use std::time::{Duration, SystemTime};

        FileEntry::new(
            name.to_string(),
            PathBuf::from(format!("/tmp/{}", name)),
            FileType::File,
            100,
            SystemTime::UNIX_EPOCH + Duration::from_secs(secs_offset),
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

    // === 정렬 테스트 (Phase 5.1) ===

    #[test]
    fn test_sort_directories_first() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("banana.txt"),
            create_test_dir("alpha_dir"),
            create_test_entry("apple.txt"),
            create_test_dir("zebra_dir"),
        ];

        state.sort_entries();

        // 디렉토리가 파일보다 앞에 와야 함
        assert!(state.entries[0].is_directory());
        assert!(state.entries[1].is_directory());
        assert!(!state.entries[2].is_directory());
        assert!(!state.entries[3].is_directory());
        // 디렉토리 내에서 이름순
        assert_eq!(state.entries[0].name, "alpha_dir");
        assert_eq!(state.entries[1].name, "zebra_dir");
    }

    #[test]
    fn test_sort_by_name_case_insensitive() {
        let mut state = PanelState::default();
        state.sort_by = SortBy::Name;
        state.entries = vec![
            create_test_entry("Banana.txt"),
            create_test_entry("apple.txt"),
            create_test_entry("Cherry.txt"),
        ];

        state.sort_entries();

        assert_eq!(state.entries[0].name, "apple.txt");
        assert_eq!(state.entries[1].name, "Banana.txt");
        assert_eq!(state.entries[2].name, "Cherry.txt");
    }

    #[test]
    fn test_sort_by_size() {
        let mut state = PanelState::default();
        state.sort_by = SortBy::Size;
        state.entries = vec![
            create_test_entry_with_size("big.txt", 1000),
            create_test_entry_with_size("small.txt", 10),
            create_test_entry_with_size("medium.txt", 500),
        ];

        state.sort_entries();

        assert_eq!(state.entries[0].name, "small.txt");
        assert_eq!(state.entries[1].name, "medium.txt");
        assert_eq!(state.entries[2].name, "big.txt");
    }

    #[test]
    fn test_sort_by_date() {
        let mut state = PanelState::default();
        state.sort_by = SortBy::Modified;
        state.entries = vec![
            create_test_entry_with_time("newest.txt", 3000),
            create_test_entry_with_time("oldest.txt", 1000),
            create_test_entry_with_time("middle.txt", 2000),
        ];

        state.sort_entries();

        assert_eq!(state.entries[0].name, "oldest.txt");
        assert_eq!(state.entries[1].name, "middle.txt");
        assert_eq!(state.entries[2].name, "newest.txt");
    }

    #[test]
    fn test_sort_by_extension() {
        let mut state = PanelState::default();
        state.sort_by = SortBy::Extension;
        state.entries = vec![
            create_test_entry("file.txt"),
            create_test_entry("image.png"),
            create_test_entry("data.csv"),
            create_test_entry("readme"),
        ];

        state.sort_entries();

        // 확장자 없음 → csv → png → txt
        assert_eq!(state.entries[0].name, "readme");
        assert_eq!(state.entries[1].name, "data.csv");
        assert_eq!(state.entries[2].name, "image.png");
        assert_eq!(state.entries[3].name, "file.txt");
    }

    #[test]
    fn test_sort_descending() {
        let mut state = PanelState::default();
        state.sort_by = SortBy::Name;
        state.sort_order = SortOrder::Descending;
        state.entries = vec![
            create_test_entry("apple.txt"),
            create_test_entry("cherry.txt"),
            create_test_entry("banana.txt"),
        ];

        state.sort_entries();

        assert_eq!(state.entries[0].name, "cherry.txt");
        assert_eq!(state.entries[1].name, "banana.txt");
        assert_eq!(state.entries[2].name, "apple.txt");
    }

    #[test]
    fn test_sort_descending_dirs_still_first() {
        let mut state = PanelState::default();
        state.sort_by = SortBy::Name;
        state.sort_order = SortOrder::Descending;
        state.entries = vec![
            create_test_entry("file_a.txt"),
            create_test_dir("dir_z"),
            create_test_dir("dir_a"),
            create_test_entry("file_z.txt"),
        ];

        state.sort_entries();

        // 디렉토리 우선, 내림차순
        assert_eq!(state.entries[0].name, "dir_z");
        assert_eq!(state.entries[1].name, "dir_a");
        assert_eq!(state.entries[2].name, "file_z.txt");
        assert_eq!(state.entries[3].name, "file_a.txt");
    }

    #[test]
    fn test_set_sort_toggle() {
        let mut state = PanelState::default();
        assert_eq!(state.sort_by, SortBy::Name);
        assert_eq!(state.sort_order, SortOrder::Ascending);

        // 같은 기준 → 순서 토글
        state.set_sort(SortBy::Name);
        assert_eq!(state.sort_by, SortBy::Name);
        assert_eq!(state.sort_order, SortOrder::Descending);

        // 다시 토글
        state.set_sort(SortBy::Name);
        assert_eq!(state.sort_order, SortOrder::Ascending);

        // 다른 기준 → Ascending으로 리셋
        state.set_sort(SortBy::Size);
        assert_eq!(state.sort_by, SortBy::Size);
        assert_eq!(state.sort_order, SortOrder::Ascending);
    }

    #[test]
    fn test_sort_indicator() {
        let mut state = PanelState::default();
        assert_eq!(state.sort_indicator(), "Name ▲");

        state.sort_by = SortBy::Size;
        state.sort_order = SortOrder::Descending;
        assert_eq!(state.sort_indicator(), "Size ▼");

        state.sort_by = SortBy::Modified;
        state.sort_order = SortOrder::Ascending;
        assert_eq!(state.sort_indicator(), "Date ▲");

        state.sort_by = SortBy::Extension;
        assert_eq!(state.sort_indicator(), "Ext ▲");
    }

    // === 필터 테스트 (Phase 5.2) ===

    #[test]
    fn test_filter_contains() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("main.rs"),
            create_test_entry("test.rs"),
            create_test_entry("readme.md"),
        ];
        state.filter = Some("main".to_string());

        // contains 필터: "main" 포함 항목만 남기기
        let filter = state.filter.clone().unwrap();
        let filter_lower = filter.to_lowercase();
        state
            .entries
            .retain(|e| e.name.to_lowercase().contains(&filter_lower));
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].name, "main.rs");
    }

    #[test]
    fn test_filter_glob_star() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("main.rs"),
            create_test_entry("test.rs"),
            create_test_entry("readme.md"),
        ];

        // glob 필터: *.rs
        state
            .entries
            .retain(|e| crate::utils::glob::glob_match("*.rs", &e.name));
        assert_eq!(state.entries.len(), 2);
        assert_eq!(state.entries[0].name, "main.rs");
        assert_eq!(state.entries[1].name, "test.rs");
    }

    #[test]
    fn test_filter_glob_question() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("a.rs"),
            create_test_entry("ab.rs"),
            create_test_entry("abc.rs"),
        ];

        state
            .entries
            .retain(|e| crate::utils::glob::glob_match("??.rs", &e.name));
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.entries[0].name, "ab.rs");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let mut state = PanelState::default();
        state.entries = vec![
            create_test_entry("README.md"),
            create_test_entry("readme.txt"),
            create_test_entry("other.rs"),
        ];

        let filter_lower = "readme".to_lowercase();
        state
            .entries
            .retain(|e| e.name.to_lowercase().contains(&filter_lower));
        assert_eq!(state.entries.len(), 2);
    }

    #[test]
    fn test_filter_indicator() {
        let mut state = PanelState::default();
        assert!(state.filter_indicator().is_none());

        state.filter = Some("*.rs".to_string());
        assert_eq!(state.filter_indicator(), Some("Filter: *.rs".to_string()));

        state.filter = Some(String::new());
        assert!(state.filter_indicator().is_none());
    }

    #[test]
    fn test_extract_extension() {
        assert_eq!(super::extract_extension("file.txt"), "txt");
        assert_eq!(super::extract_extension("file.tar.gz"), "gz");
        assert_eq!(super::extract_extension("README"), "");
        assert_eq!(super::extract_extension(".bashrc"), "");
        assert_eq!(super::extract_extension(".config.json"), "json");
        assert_eq!(super::extract_extension("photo.JPG"), "jpg");
    }
}
