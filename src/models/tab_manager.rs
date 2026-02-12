use crate::models::panel_state::{PanelState, TabState};
use std::path::PathBuf;

/// 패널당 최대 탭 수
pub const MAX_TABS: usize = 9;

/// 패널별 탭 관리자
#[derive(Debug, Clone)]
pub struct TabManager {
    /// 탭 목록 (최소 1개, 최대 MAX_TABS)
    pub tabs: Vec<TabState>,
    /// 활성 탭 인덱스 (0-based)
    pub active_tab: usize,
    /// 다음 탭 ID (자동 증가)
    next_id: usize,
}

impl TabManager {
    /// 초기 경로로 탭 관리자 생성 (탭 1개)
    pub fn new(initial_path: PathBuf) -> Self {
        Self {
            tabs: vec![TabState::new(1, PanelState::new(initial_path))],
            active_tab: 0,
            next_id: 2,
        }
    }

    /// 활성 탭의 PanelState 참조
    pub fn active_panel(&self) -> &PanelState {
        &self.tabs[self.active_tab].panel
    }

    /// 활성 탭의 PanelState 가변 참조
    pub fn active_panel_mut(&mut self) -> &mut PanelState {
        &mut self.tabs[self.active_tab].panel
    }

    /// 새 탭 생성 (현재 탭의 경로를 복제, 현재 탭 뒤에 삽입)
    ///
    /// 성공 시 true, 최대 탭 수 초과 시 false
    pub fn new_tab(&mut self) -> bool {
        if self.tabs.len() >= MAX_TABS {
            return false;
        }

        let current = &self.tabs[self.active_tab].panel;
        let mut new_panel = PanelState::new(current.current_path.clone());
        new_panel.sort_by = current.sort_by;
        new_panel.sort_order = current.sort_order;
        new_panel.show_hidden = current.show_hidden;
        // 필터는 상속하지 않음 (깨끗하게 시작)

        let id = self.next_id;
        self.next_id += 1;

        let insert_pos = self.active_tab + 1;
        self.tabs.insert(insert_pos, TabState::new(id, new_panel));
        self.active_tab = insert_pos;
        true
    }

    /// 현재 탭 닫기
    ///
    /// 성공 시 true, 마지막 탭이면 false
    pub fn close_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        true
    }

    /// 탭 번호로 전환 (1-based)
    ///
    /// 성공 시 true, 해당 탭이 없으면 false
    pub fn switch_to(&mut self, number: usize) -> bool {
        let index = number.saturating_sub(1);
        if index < self.tabs.len() {
            self.active_tab = index;
            true
        } else {
            false
        }
    }

    /// 다음 탭 (순환)
    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }

    /// 이전 탭 (순환)
    pub fn prev_tab(&mut self) {
        if self.active_tab == 0 {
            self.active_tab = self.tabs.len() - 1;
        } else {
            self.active_tab -= 1;
        }
    }

    /// 탭 개수
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tab_manager() {
        let mgr = TabManager::new(PathBuf::from("/tmp"));
        assert_eq!(mgr.tab_count(), 1);
        assert_eq!(mgr.active_tab, 0);
        assert_eq!(mgr.active_panel().current_path, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_new_tab() {
        let mut mgr = TabManager::new(PathBuf::from("/tmp"));
        assert!(mgr.new_tab());
        assert_eq!(mgr.tab_count(), 2);
        assert_eq!(mgr.active_tab, 1); // 새 탭이 활성화됨
        assert_eq!(mgr.active_panel().current_path, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_max_tabs() {
        let mut mgr = TabManager::new(PathBuf::from("/tmp"));
        for _ in 0..8 {
            assert!(mgr.new_tab());
        }
        assert_eq!(mgr.tab_count(), MAX_TABS);
        assert!(!mgr.new_tab()); // 9개 초과 불가
    }

    #[test]
    fn test_close_tab() {
        let mut mgr = TabManager::new(PathBuf::from("/tmp"));
        mgr.new_tab();
        mgr.new_tab();
        assert_eq!(mgr.tab_count(), 3);

        assert!(mgr.close_tab());
        assert_eq!(mgr.tab_count(), 2);

        assert!(mgr.close_tab());
        assert_eq!(mgr.tab_count(), 1);

        assert!(!mgr.close_tab()); // 마지막 탭은 닫을 수 없음
    }

    #[test]
    fn test_close_last_active_adjusts_index() {
        let mut mgr = TabManager::new(PathBuf::from("/tmp"));
        mgr.new_tab();
        mgr.new_tab();
        // active_tab = 2 (마지막)
        assert_eq!(mgr.active_tab, 2);
        mgr.close_tab();
        // 마지막 탭 닫으면 이전 탭으로 이동
        assert_eq!(mgr.active_tab, 1);
    }

    #[test]
    fn test_switch_to() {
        let mut mgr = TabManager::new(PathBuf::from("/tmp"));
        mgr.new_tab();
        mgr.new_tab();

        assert!(mgr.switch_to(1)); // 1번 탭 (0-based: 0)
        assert_eq!(mgr.active_tab, 0);

        assert!(mgr.switch_to(3)); // 3번 탭 (0-based: 2)
        assert_eq!(mgr.active_tab, 2);

        assert!(!mgr.switch_to(4)); // 4번 탭 없음
        assert_eq!(mgr.active_tab, 2); // 변경 안 됨
    }

    #[test]
    fn test_next_prev_tab() {
        let mut mgr = TabManager::new(PathBuf::from("/tmp"));
        mgr.new_tab();
        mgr.new_tab();
        mgr.switch_to(1); // 0-based: 0

        mgr.next_tab();
        assert_eq!(mgr.active_tab, 1);

        mgr.next_tab();
        assert_eq!(mgr.active_tab, 2);

        mgr.next_tab(); // 순환
        assert_eq!(mgr.active_tab, 0);

        mgr.prev_tab(); // 순환
        assert_eq!(mgr.active_tab, 2);

        mgr.prev_tab();
        assert_eq!(mgr.active_tab, 1);
    }

    #[test]
    fn test_new_tab_inherits_settings() {
        let mut mgr = TabManager::new(PathBuf::from("/home"));
        {
            let panel = mgr.active_panel_mut();
            panel.sort_by = crate::models::panel_state::SortBy::Size;
            panel.sort_order = crate::models::panel_state::SortOrder::Descending;
            panel.show_hidden = true;
            panel.filter = Some("*.rs".to_string());
        }

        mgr.new_tab();
        let new_panel = mgr.active_panel();
        assert_eq!(new_panel.current_path, PathBuf::from("/home"));
        assert_eq!(new_panel.sort_by, crate::models::panel_state::SortBy::Size);
        assert_eq!(
            new_panel.sort_order,
            crate::models::panel_state::SortOrder::Descending
        );
        assert!(new_panel.show_hidden);
        assert!(new_panel.filter.is_none()); // 필터는 상속 안 됨
    }
}
