#![allow(dead_code)]

use crate::models::file_entry::FileEntry;
use crate::system::filesystem::FileSystem;
use crate::utils::error::Result;
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
    /// 선택된 항목 인덱스
    pub selected_index: usize,
    /// 스크롤 오프셋
    pub scroll_offset: usize,
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
        self.refresh(filesystem)
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
}

impl Default for PanelState {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
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
}
