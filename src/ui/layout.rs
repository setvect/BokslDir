#![allow(dead_code)]
// Layout system - 반응형 레이아웃 시스템
//
// 터미널 크기에 따른 레이아웃 모드:
// - 80+ cols: 듀얼 패널 모드
// - 40-79 cols: 싱글 패널 모드 (Tab으로 전환)
// - <40 cols: 경고 메시지 표시

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// 최소 터미널 크기 상수
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 15;
pub const DUAL_PANEL_MIN_WIDTH: u16 = 80;
pub const STANDARD_HEIGHT: u16 = 24;

/// 레이아웃 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// 듀얼 패널 모드 (80+ cols)
    DualPanel,
    /// 싱글 패널 모드 (40-79 cols)
    SinglePanel,
    /// 경고 모드 (터미널이 너무 작음)
    TooSmall,
}

/// 활성 패널
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePanel {
    #[default]
    Left,
    Right,
}

impl ActivePanel {
    /// 패널 전환
    pub fn toggle(&mut self) {
        *self = match self {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }
}

/// 패널 비율 설정
#[derive(Debug, Clone, Copy)]
pub struct PanelRatio {
    pub left: u16,
    pub right: u16,
}

impl Default for PanelRatio {
    fn default() -> Self {
        Self {
            left: 50,
            right: 50,
        }
    }
}

impl PanelRatio {
    pub fn new(left: u16, right: u16) -> Self {
        Self { left, right }
    }

    /// 70:30 비율
    pub fn wide_left() -> Self {
        Self {
            left: 70,
            right: 30,
        }
    }

    /// 30:70 비율
    pub fn wide_right() -> Self {
        Self {
            left: 30,
            right: 70,
        }
    }
}

/// 레이아웃 영역
#[derive(Debug, Clone, Default)]
pub struct LayoutAreas {
    /// 상단 메뉴바 영역
    pub menu_bar: Rect,
    /// 좌측 패널 영역
    pub left_panel: Rect,
    /// 우측 패널 영역
    pub right_panel: Rect,
    /// 상태바 영역
    pub status_bar: Rect,
    /// 하단 커맨드 바 영역
    pub command_bar: Rect,
    /// 경고 메시지 영역 (TooSmall 모드에서 사용)
    pub warning: Rect,
}

/// 레이아웃 상태
#[derive(Debug, Clone)]
pub struct LayoutState {
    /// 현재 레이아웃 모드
    pub mode: LayoutMode,
    /// 활성 패널
    pub active_panel: ActivePanel,
    /// 패널 비율
    pub panel_ratio: PanelRatio,
    /// 터미널 크기
    pub terminal_size: (u16, u16),
    /// 계산된 레이아웃 영역
    pub areas: LayoutAreas,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            mode: LayoutMode::DualPanel,
            active_panel: ActivePanel::default(),
            panel_ratio: PanelRatio::default(),
            terminal_size: (80, 24),
            areas: LayoutAreas::default(),
        }
    }
}

/// 레이아웃 매니저
#[derive(Debug)]
pub struct LayoutManager {
    state: LayoutState,
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            state: LayoutState::default(),
        }
    }

    /// 터미널 크기에 따라 레이아웃 모드 결정
    fn determine_mode(width: u16, height: u16) -> LayoutMode {
        if width < MIN_WIDTH || height < MIN_HEIGHT {
            LayoutMode::TooSmall
        } else if width < DUAL_PANEL_MIN_WIDTH {
            LayoutMode::SinglePanel
        } else {
            LayoutMode::DualPanel
        }
    }

    /// 터미널 크기 업데이트 및 레이아웃 재계산
    pub fn update(&mut self, area: Rect) {
        let width = area.width;
        let height = area.height;

        self.state.terminal_size = (width, height);
        self.state.mode = Self::determine_mode(width, height);
        self.state.areas = self.calculate_areas(area);
    }

    /// 레이아웃 영역 계산
    fn calculate_areas(&self, area: Rect) -> LayoutAreas {
        match self.state.mode {
            LayoutMode::TooSmall => LayoutAreas {
                warning: area,
                ..Default::default()
            },
            LayoutMode::SinglePanel => self.calculate_single_panel_areas(area),
            LayoutMode::DualPanel => self.calculate_dual_panel_areas(area),
        }
    }

    /// 듀얼 패널 레이아웃 계산
    fn calculate_dual_panel_areas(&self, area: Rect) -> LayoutAreas {
        // 메인 수직 레이아웃: 메뉴바 | 패널 | 상태바 | 커맨드바
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // 메뉴바
                Constraint::Min(3),    // 패널 영역
                Constraint::Length(1), // 상태바
                Constraint::Length(1), // 커맨드바
            ])
            .split(area);

        // 패널 영역을 좌우로 분할
        let panel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(self.state.panel_ratio.left),
                Constraint::Percentage(self.state.panel_ratio.right),
            ])
            .split(vertical_chunks[1]);

        LayoutAreas {
            menu_bar: vertical_chunks[0],
            left_panel: panel_chunks[0],
            right_panel: panel_chunks[1],
            status_bar: vertical_chunks[2],
            command_bar: vertical_chunks[3],
            warning: Rect::default(),
        }
    }

    /// 싱글 패널 레이아웃 계산
    fn calculate_single_panel_areas(&self, area: Rect) -> LayoutAreas {
        // 메인 수직 레이아웃
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // 메뉴바
                Constraint::Min(3),    // 패널 영역
                Constraint::Length(1), // 상태바
                Constraint::Length(1), // 커맨드바
            ])
            .split(area);

        // 싱글 패널 모드에서는 활성 패널만 전체 너비 사용
        let panel_area = vertical_chunks[1];

        let (left_panel, right_panel) = match self.state.active_panel {
            ActivePanel::Left => (panel_area, Rect::default()),
            ActivePanel::Right => (Rect::default(), panel_area),
        };

        LayoutAreas {
            menu_bar: vertical_chunks[0],
            left_panel,
            right_panel,
            status_bar: vertical_chunks[2],
            command_bar: vertical_chunks[3],
            warning: Rect::default(),
        }
    }

    /// 현재 레이아웃 모드 반환
    pub fn mode(&self) -> LayoutMode {
        self.state.mode
    }

    /// 현재 레이아웃 상태 반환
    pub fn state(&self) -> &LayoutState {
        &self.state
    }

    /// 레이아웃 영역 반환
    pub fn areas(&self) -> &LayoutAreas {
        &self.state.areas
    }

    /// 활성 패널 반환
    pub fn active_panel(&self) -> ActivePanel {
        self.state.active_panel
    }

    /// 패널 전환
    pub fn toggle_panel(&mut self) {
        self.state.active_panel.toggle();
    }

    /// 활성 패널 설정
    pub fn set_active_panel(&mut self, panel: ActivePanel) {
        self.state.active_panel = panel;
    }

    /// 패널 비율 설정
    pub fn set_panel_ratio(&mut self, ratio: PanelRatio) {
        self.state.panel_ratio = ratio;
    }

    /// 터미널 크기 반환
    pub fn terminal_size(&self) -> (u16, u16) {
        self.state.terminal_size
    }

    /// 듀얼 패널 모드인지 확인
    pub fn is_dual_panel(&self) -> bool {
        matches!(self.state.mode, LayoutMode::DualPanel)
    }

    /// 싱글 패널 모드인지 확인
    pub fn is_single_panel(&self) -> bool {
        matches!(self.state.mode, LayoutMode::SinglePanel)
    }

    /// 터미널이 너무 작은지 확인
    pub fn is_too_small(&self) -> bool {
        matches!(self.state.mode, LayoutMode::TooSmall)
    }

    /// 활성 패널의 영역 반환
    pub fn active_panel_area(&self) -> Rect {
        match self.state.active_panel {
            ActivePanel::Left => self.state.areas.left_panel,
            ActivePanel::Right => self.state.areas.right_panel,
        }
    }

    /// 비활성 패널의 영역 반환
    pub fn inactive_panel_area(&self) -> Rect {
        match self.state.active_panel {
            ActivePanel::Left => self.state.areas.right_panel,
            ActivePanel::Right => self.state.areas.left_panel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_mode() {
        // 듀얼 패널 모드 (80+ cols)
        assert_eq!(LayoutManager::determine_mode(80, 24), LayoutMode::DualPanel);
        assert_eq!(
            LayoutManager::determine_mode(120, 30),
            LayoutMode::DualPanel
        );

        // 싱글 패널 모드 (40-79 cols)
        assert_eq!(
            LayoutManager::determine_mode(79, 24),
            LayoutMode::SinglePanel
        );
        assert_eq!(
            LayoutManager::determine_mode(40, 24),
            LayoutMode::SinglePanel
        );

        // TooSmall 모드
        assert_eq!(LayoutManager::determine_mode(39, 24), LayoutMode::TooSmall);
        assert_eq!(LayoutManager::determine_mode(80, 14), LayoutMode::TooSmall);
    }

    #[test]
    fn test_toggle_panel() {
        let mut manager = LayoutManager::new();
        assert_eq!(manager.active_panel(), ActivePanel::Left);

        manager.toggle_panel();
        assert_eq!(manager.active_panel(), ActivePanel::Right);

        manager.toggle_panel();
        assert_eq!(manager.active_panel(), ActivePanel::Left);
    }

    #[test]
    fn test_panel_ratio() {
        let default = PanelRatio::default();
        assert_eq!(default.left, 50);
        assert_eq!(default.right, 50);

        let wide_left = PanelRatio::wide_left();
        assert_eq!(wide_left.left, 70);
        assert_eq!(wide_left.right, 30);
    }
}
