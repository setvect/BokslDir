#![allow(dead_code)]

use crate::models::panel_state::PanelState;
use std::path::Path;

/// 패널별 탭 상태
#[derive(Debug, Clone)]
pub struct PanelTabs {
    tabs: Vec<PanelState>,
    active_index: usize,
}

impl PanelTabs {
    /// 초기 탭 1개로 생성
    pub fn new(initial: PanelState) -> Self {
        Self {
            tabs: vec![initial],
            active_index: 0,
        }
    }

    /// 활성 탭 상태 반환
    pub fn active(&self) -> &PanelState {
        &self.tabs[self.active_index]
    }

    /// 활성 탭 상태 반환 (mutable)
    pub fn active_mut(&mut self) -> &mut PanelState {
        &mut self.tabs[self.active_index]
    }

    /// 탭 개수
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// 활성 탭 인덱스
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// 현재 상태를 복제해 새 탭 생성 + 활성화
    pub fn create_tab(&mut self, from: &PanelState) -> usize {
        self.tabs.push(from.clone());
        self.active_index = self.tabs.len() - 1;
        self.active_index
    }

    /// 활성 탭 닫기. 마지막 탭이면 false 반환
    pub fn close_active_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        self.tabs.remove(self.active_index);
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        }
        true
    }

    /// 다음 탭
    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    /// 이전 탭
    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    /// 특정 탭으로 전환
    pub fn switch_to(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }
        self.active_index = index;
        true
    }

    /// 탭 타이틀 목록
    pub fn titles(&self) -> Vec<String> {
        self.tabs
            .iter()
            .map(|panel| title_from_path(&panel.current_path))
            .collect()
    }
}

fn title_from_path(path: &Path) -> String {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if !name.is_empty() {
            return name.to_string();
        }
    }
    if path.parent().is_none() {
        "/".to_string()
    } else {
        path.to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::panel_state::{SortBy, SortOrder};
    use std::path::PathBuf;

    fn panel(path: &str) -> PanelState {
        PanelState::new(PathBuf::from(path))
    }

    #[test]
    fn test_panel_tabs_initial_state() {
        let tabs = PanelTabs::new(panel("/tmp"));
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs.active_index(), 0);
        assert_eq!(tabs.active().current_path, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_create_tab_activates_new_tab() {
        let mut tabs = PanelTabs::new(panel("/tmp"));
        let from = tabs.active().clone();
        let new_index = tabs.create_tab(&from);

        assert_eq!(tabs.len(), 2);
        assert_eq!(new_index, 1);
        assert_eq!(tabs.active_index(), 1);
        assert_eq!(tabs.active().current_path, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_close_active_tab_keeps_one_minimum() {
        let mut tabs = PanelTabs::new(panel("/tmp"));
        assert!(!tabs.close_active_tab());

        let from = tabs.active().clone();
        tabs.create_tab(&from);
        assert!(tabs.close_active_tab());
        assert_eq!(tabs.len(), 1);
    }

    #[test]
    fn test_next_prev_and_switch() {
        let mut tabs = PanelTabs::new(panel("/tmp"));
        let base = tabs.active().clone();
        tabs.create_tab(&base);
        tabs.create_tab(&base);
        assert_eq!(tabs.active_index(), 2);

        tabs.next_tab();
        assert_eq!(tabs.active_index(), 0);
        tabs.prev_tab();
        assert_eq!(tabs.active_index(), 2);

        assert!(tabs.switch_to(1));
        assert_eq!(tabs.active_index(), 1);
        assert!(!tabs.switch_to(5));
    }

    #[test]
    fn test_tab_state_is_independent() {
        let mut tabs = PanelTabs::new(panel("/tmp"));
        {
            let p = tabs.active_mut();
            p.set_filter(Some("foo".to_string()));
            p.set_sort(SortBy::Size);
            p.set_sort_order(SortOrder::Descending);
        }
        let from = tabs.active().clone();
        tabs.create_tab(&from);

        {
            let p = tabs.active_mut();
            p.set_filter(Some("bar".to_string()));
        }
        tabs.switch_to(0);
        assert_eq!(tabs.active().filter.as_deref(), Some("foo"));
        assert_eq!(tabs.active().sort_by, SortBy::Size);
        assert_eq!(tabs.active().sort_order, SortOrder::Descending);
    }
}
