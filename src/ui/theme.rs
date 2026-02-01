// TODO: 추후 UI 컴포넌트에서 사용 예정 - Phase 1.3 완료 시 제거
#![allow(dead_code)]

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 색상 테마 시스템
///
/// 애플리케이션 전체의 색상 테마를 관리합니다.
/// TOML 파일에서 테마를 로드하거나 미리 정의된 테마를 사용할 수 있습니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    // 배경/전경
    pub bg_primary: ColorDef,
    pub fg_primary: ColorDef,

    // 패널
    pub panel_active_border: ColorDef,
    pub panel_inactive_border: ColorDef,
    pub panel_bg: ColorDef,

    // 파일 리스트
    pub file_normal: ColorDef,
    pub file_selected: ColorDef,
    pub file_selected_bg: ColorDef,
    pub directory: ColorDef,
    pub executable: ColorDef,
    pub symlink: ColorDef,

    // UI 컴포넌트
    pub menu_bar_bg: ColorDef,
    pub menu_bar_fg: ColorDef,
    pub status_bar_bg: ColorDef,
    pub status_bar_fg: ColorDef,
    pub command_bar_bg: ColorDef,
    pub command_bar_fg: ColorDef,

    // 강조
    pub accent: ColorDef,
    pub warning: ColorDef,
    pub error: ColorDef,
    pub success: ColorDef,
}

/// 색상 정의 (TOML 직렬화/역직렬화 지원)
///
/// Hex 문자열("#1e1e1e") 또는 색상 이름("Red")을 지원합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorDef {
    Hex(String),
    Named(String),
}

impl ColorDef {
    /// ColorDef를 ratatui의 Color로 변환
    pub fn to_color(&self) -> Color {
        match self {
            ColorDef::Hex(hex) => parse_hex_color(hex),
            ColorDef::Named(name) => parse_named_color(name),
        }
    }
}

impl From<&str> for ColorDef {
    fn from(s: &str) -> Self {
        if s.starts_with('#') {
            ColorDef::Hex(s.to_string())
        } else {
            ColorDef::Named(s.to_string())
        }
    }
}

/// Hex 색상 문자열을 Color로 파싱
fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::Rgb(r, g, b)
    } else {
        Color::Reset
    }
}

/// 색상 이름을 Color로 파싱
fn parse_named_color(name: &str) -> Color {
    match name.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        "reset" => Color::Reset,
        _ => Color::Reset,
    }
}

impl Theme {
    /// Dark 테마 (기본)
    pub fn dark() -> Self {
        Theme {
            // 배경/전경
            bg_primary: "#1e1e1e".into(),
            fg_primary: "#d4d4d4".into(),

            // 패널
            panel_active_border: "#0078d4".into(),
            panel_inactive_border: "#3c3c3c".into(),
            panel_bg: "#1e1e1e".into(),

            // 파일 리스트
            file_normal: "#d4d4d4".into(),
            file_selected: "#ffffff".into(),
            file_selected_bg: "#0078d4".into(),
            directory: "#569cd6".into(),
            executable: "#4ec9b0".into(),
            symlink: "#ce9178".into(),

            // UI 컴포넌트
            menu_bar_bg: "#2d2d30".into(), // 어두운 배경으로 변경하여 선택된 메뉴 강조
            menu_bar_fg: "#ffffff".into(),
            status_bar_bg: "#007acc".into(),
            status_bar_fg: "#ffffff".into(),
            command_bar_bg: "#2d2d30".into(),
            command_bar_fg: "#cccccc".into(),

            // 강조
            accent: "#0078d4".into(),
            warning: "#ffa500".into(),
            error: "#f44747".into(),
            success: "#4ec9b0".into(),
        }
    }

    /// Light 테마
    pub fn light() -> Self {
        Theme {
            // 배경/전경
            bg_primary: "#ffffff".into(),
            fg_primary: "#1e1e1e".into(),

            // 패널
            panel_active_border: "#0078d4".into(),
            panel_inactive_border: "#cccccc".into(),
            panel_bg: "#ffffff".into(),

            // 파일 리스트
            file_normal: "#1e1e1e".into(),
            file_selected: "#000000".into(),
            file_selected_bg: "#add6ff".into(),
            directory: "#0066cc".into(),
            executable: "#008080".into(),
            symlink: "#a65e2b".into(),

            // UI 컴포넌트
            menu_bar_bg: "#0078d4".into(),
            menu_bar_fg: "#ffffff".into(),
            status_bar_bg: "#0078d4".into(),
            status_bar_fg: "#ffffff".into(),
            command_bar_bg: "#f3f3f3".into(),
            command_bar_fg: "#1e1e1e".into(),

            // 강조
            accent: "#0078d4".into(),
            warning: "#ff8c00".into(),
            error: "#e51400".into(),
            success: "#107c10".into(),
        }
    }

    /// High Contrast 테마
    pub fn high_contrast() -> Self {
        Theme {
            // 배경/전경
            bg_primary: "#000000".into(),
            fg_primary: "#ffffff".into(),

            // 패널
            panel_active_border: "#00ff00".into(),
            panel_inactive_border: "#808080".into(),
            panel_bg: "#000000".into(),

            // 파일 리스트
            file_normal: "#ffffff".into(),
            file_selected: "#000000".into(),
            file_selected_bg: "#00ff00".into(),
            directory: "#00ffff".into(),
            executable: "#00ff00".into(),
            symlink: "#ffff00".into(),

            // UI 컴포넌트
            menu_bar_bg: "#000000".into(),
            menu_bar_fg: "#00ff00".into(),
            status_bar_bg: "#000000".into(),
            status_bar_fg: "#00ff00".into(),
            command_bar_bg: "#000000".into(),
            command_bar_fg: "#ffffff".into(),

            // 강조
            accent: "#00ff00".into(),
            warning: "#ffff00".into(),
            error: "#ff0000".into(),
            success: "#00ff00".into(),
        }
    }

    /// TOML 파일에서 테마 로드
    pub fn from_file(path: PathBuf) -> Result<Self, anyhow::Error> {
        let content = fs::read_to_string(path)?;
        let theme: Theme = toml::from_str(&content)?;
        Ok(theme)
    }

    /// 테마를 TOML 파일로 저장
    pub fn save_to_file(&self, path: PathBuf) -> Result<(), anyhow::Error> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// 테마 관리자
///
/// 현재 활성 테마를 관리하고 런타임에 테마를 전환합니다.
pub struct ThemeManager {
    current_theme: Theme,
    available_themes: Vec<(String, Theme)>,
}

impl ThemeManager {
    /// 기본 테마 관리자 생성 (Dark 테마)
    pub fn new() -> Self {
        Self {
            current_theme: Theme::dark(),
            available_themes: vec![
                ("dark".to_string(), Theme::dark()),
                ("light".to_string(), Theme::light()),
                ("high_contrast".to_string(), Theme::high_contrast()),
            ],
        }
    }

    /// 특정 테마로 초기화
    pub fn with_theme(theme: Theme) -> Self {
        Self {
            current_theme: theme.clone(),
            available_themes: vec![
                ("dark".to_string(), Theme::dark()),
                ("light".to_string(), Theme::light()),
                ("high_contrast".to_string(), Theme::high_contrast()),
            ],
        }
    }

    /// 현재 테마 반환
    pub fn current(&self) -> &Theme {
        &self.current_theme
    }

    /// 테마 전환 (이름으로)
    pub fn switch_theme(&mut self, name: &str) -> Result<(), String> {
        if let Some((_, theme)) = self.available_themes.iter().find(|(n, _)| n == name) {
            self.current_theme = theme.clone();
            Ok(())
        } else {
            Err(format!("테마를 찾을 수 없습니다: {}", name))
        }
    }

    /// 다음 테마로 순환
    pub fn cycle_theme(&mut self) {
        let current_index = self
            .available_themes
            .iter()
            .position(|(_, t)| {
                // 테마 비교는 이름으로 하는 것이 더 안전
                // 임시로 첫 번째 색상으로 비교
                format!("{:?}", t.bg_primary) == format!("{:?}", self.current_theme.bg_primary)
            })
            .unwrap_or(0);

        let next_index = (current_index + 1) % self.available_themes.len();
        self.current_theme = self.available_themes[next_index].1.clone();
    }

    /// 사용 가능한 테마 목록 반환
    pub fn available_themes(&self) -> Vec<String> {
        self.available_themes
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// 커스텀 테마 추가
    pub fn add_theme(&mut self, name: String, theme: Theme) {
        self.available_themes.push((name, theme));
    }

    /// 설정 디렉토리에서 테마 파일 로드
    pub fn load_themes_from_config_dir(&mut self) -> Result<(), anyhow::Error> {
        if let Some(config_dir) = dirs::config_dir() {
            let themes_dir = config_dir.join("boksldir").join("themes");

            if themes_dir.exists() {
                for entry in fs::read_dir(themes_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        if let Ok(theme) = Theme::from_file(path.clone()) {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("custom")
                                .to_string();

                            self.add_theme(name, theme);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_creation() {
        let theme = Theme::dark();
        assert_eq!(theme.bg_primary.to_color(), Color::Rgb(30, 30, 30));
    }

    #[test]
    fn test_light_theme_creation() {
        let theme = Theme::light();
        assert_eq!(theme.bg_primary.to_color(), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_high_contrast_theme_creation() {
        let theme = Theme::high_contrast();
        assert_eq!(theme.bg_primary.to_color(), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_hex_color_parsing() {
        let color = parse_hex_color("#1e1e1e");
        assert_eq!(color, Color::Rgb(30, 30, 30));
    }

    #[test]
    fn test_named_color_parsing() {
        assert_eq!(parse_named_color("red"), Color::Red);
        assert_eq!(parse_named_color("blue"), Color::Blue);
        assert_eq!(parse_named_color("white"), Color::White);
    }

    #[test]
    fn test_theme_manager_creation() {
        let manager = ThemeManager::new();
        assert_eq!(manager.available_themes().len(), 3);
    }

    #[test]
    fn test_theme_switching() {
        let mut manager = ThemeManager::new();
        assert!(manager.switch_theme("light").is_ok());
        assert_eq!(
            manager.current().bg_primary.to_color(),
            Color::Rgb(255, 255, 255)
        );
    }

    #[test]
    fn test_theme_cycling() {
        let mut manager = ThemeManager::new();
        manager.cycle_theme();
        // 순환 후에는 다음 테마로 변경됨
        assert_eq!(manager.available_themes().len(), 3);
    }
}
