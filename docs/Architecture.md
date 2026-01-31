# 복슬Dir (Boksl Dir) - Architecture

## 문서 개요

본 문서는 복슬Dir의 시스템 아키텍처, 모듈 구조, 데이터 흐름을 정의합니다.

**관련 문서**: [Requirements.md](Requirements.md) | [PRD.md](PRD.md)

---

## 1. 시스템 아키텍처 개요

### 1.1 전체 구조

```
┌─────────────────────────────────────────────────────────────┐
│                        Application                          │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                    Event Loop                          │ │
│  │  - Keyboard Events                                     │ │
│  │  - Mouse Events                                        │ │
│  │  - Terminal Resize Events                              │ │
│  └────────────────────────────────────────────────────────┘ │
│                            │                                │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                  State Manager                         │ │
│  │  - App State                                           │ │
│  │  - UI State                                            │ │
│  │  - Panel States                                        │ │
│  └────────────────────────────────────────────────────────┘ │
│                            │                                │
│  ┌─────────────────┬──────────────────┬───────────────────┐ │
│  │   UI Layer      │   Core Layer     │   System Layer    │ │
│  │                 │                  │                   │ │
│  │  - Layout       │  - FileManager   │  - FileSystem     │ │
│  │  - Components   │  - Navigator     │  - ProcessRunner  │ │
│  │  - Theme        │  - Searcher      │  - Compressor     │ │
│  │  - Renderer     │  - Clipboard     │  - Config         │ │
│  └─────────────────┴──────────────────┴───────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 레이어 구조

**Presentation Layer (UI Layer)**
- 사용자 인터페이스 렌더링
- 이벤트 → 액션 변환
- 테마, 레이아웃 관리

**Business Logic Layer (Core Layer)**
- 파일 관리 로직
- 검색, 정렬, 필터링
- 내비게이션, 북마크

**System Layer**
- OS 파일 시스템 접근
- 외부 프로그램 실행
- 설정 파일 관리

---

## 2. 디렉토리 구조

```
boksldir/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── .gitignore
│
├── docs/                    # 문서
│   ├── Requirements.md
│   ├── PRD.md
│   └── Architecture.md
│
├── src/
│   ├── main.rs              # 애플리케이션 진입점
│   │
│   ├── app.rs               # 애플리케이션 메인 로직
│   ├── event.rs             # 이벤트 타입 정의 및 처리
│   ├── state.rs             # 전역 상태 관리
│   │
│   ├── ui/                  # UI Layer
│   │   ├── mod.rs
│   │   ├── layout.rs        # 레이아웃 시스템
│   │   ├── theme.rs         # 테마 시스템
│   │   ├── renderer.rs      # 메인 렌더러
│   │   │
│   │   └── components/      # UI 컴포넌트
│   │       ├── mod.rs
│   │       ├── panel.rs     # 파일 패널
│   │       ├── menu_bar.rs  # 상단 메뉴바
│   │       ├── command_bar.rs   # 하단 커맨드 바
│   │       ├── status_bar.rs    # 상태바
│   │       ├── dialog.rs    # 다이얼로그 시스템
│   │       ├── file_list.rs # 파일 리스트 위젯
│   │       └── input.rs     # 입력 위젯
│   │
│   ├── core/                # Core Layer
│   │   ├── mod.rs
│   │   ├── file_manager.rs  # 파일 작업 (복사, 이동, 삭제)
│   │   ├── navigator.rs     # 네비게이션 (히스토리, 북마크)
│   │   ├── searcher.rs      # 파일 검색
│   │   ├── sorter.rs        # 정렬 로직
│   │   ├── filter.rs        # 필터링 로직
│   │   ├── selector.rs      # 파일 선택 관리
│   │   └── clipboard.rs     # 클립보드 관리
│   │
│   ├── system/              # System Layer
│   │   ├── mod.rs
│   │   ├── filesystem.rs    # 파일 시스템 추상화
│   │   ├── process.rs       # 외부 프로그램 실행
│   │   ├── compressor.rs    # 압축/해제
│   │   ├── config.rs        # 설정 관리
│   │   └── platform.rs      # 플랫폼별 로직 (macOS, Linux, Windows)
│   │
│   ├── models/              # 데이터 모델
│   │   ├── mod.rs
│   │   ├── file_entry.rs    # 파일/디렉토리 엔트리
│   │   ├── panel_state.rs   # 패널 상태
│   │   ├── app_config.rs    # 앱 설정
│   │   ├── theme_config.rs  # 테마 설정
│   │   └── keybindings.rs   # 키바인딩 설정
│   │
│   └── utils/               # 유틸리티
│       ├── mod.rs
│       ├── formatter.rs     # 파일 크기, 날짜 포맷팅
│       ├── icon.rs          # 파일 타입별 아이콘
│       └── error.rs         # 에러 타입 정의
│
├── config/                  # 기본 설정 파일 (템플릿)
│   ├── default.toml
│   ├── themes/
│   │   ├── dark.toml
│   │   ├── light.toml
│   │   └── high_contrast.toml
│   └── keybindings.toml
│
└── tests/                   # 테스트
    ├── integration/
    └── unit/
```

---

## 3. 핵심 모듈 상세 설계

### 3.1 main.rs (진입점)

**책임**:
- 애플리케이션 초기화
- 터미널 설정
- 이벤트 루프 시작
- 정리 작업 (cleanup)

**주요 로직**:
```rust
fn main() -> Result<()> {
    // 1. 설정 로드
    let config = Config::load()?;

    // 2. 터미널 초기화
    let mut terminal = setup_terminal()?;

    // 3. 애플리케이션 상태 생성
    let mut app = App::new(config)?;

    // 4. 이벤트 루프 실행
    run_event_loop(&mut terminal, &mut app)?;

    // 5. 정리
    cleanup_terminal(terminal)?;

    Ok(())
}
```

---

### 3.2 app.rs (애플리케이션 메인 로직)

**책임**:
- 전체 애플리케이션 상태 관리
- 이벤트 라우팅
- UI 업데이트 조율

**구조**:
```rust
pub struct App {
    // 상태
    state: AppState,

    // UI 상태
    left_panel: PanelState,
    right_panel: PanelState,
    active_panel: PanelSide,

    // 시스템
    file_manager: FileManager,
    navigator: Navigator,
    config: AppConfig,
    theme: Theme,

    // 플래그
    should_quit: bool,
    current_dialog: Option<Dialog>,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self>;
    pub fn handle_event(&mut self, event: Event) -> Result<()>;
    pub fn render(&self, frame: &mut Frame);
    pub fn should_quit(&self) -> bool;
}
```

---

### 3.3 event.rs (이벤트 시스템)

**책임**:
- 모든 이벤트 타입 정의
- 이벤트 → 액션 변환

**이벤트 타입**:
```rust
pub enum Event {
    // 키보드
    Key(KeyEvent),

    // 마우스
    Mouse(MouseEvent),

    // 터미널
    Resize(u16, u16),

    // 앱 내부
    FileOperation(FileOperationEvent),
    NavigationChange(NavigationEvent),
}

pub enum KeyEvent {
    Char(char),
    Up, Down, Left, Right,
    Enter, Esc, Tab,
    Function(u8),  // F1-F12
    Ctrl(char),
    Alt(char),
}

pub enum Action {
    Quit,
    ChangePanel,
    NavigateUp,
    NavigateDown,
    EnterDirectory,
    GoBack,
    CopyFile,
    MoveFile,
    DeleteFile,
    // ... 더 많은 액션
}
```

**이벤트 핸들러**:
```rust
impl App {
    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => self.handle_key_event(key),
            Event::Mouse(mouse) => self.handle_mouse_event(mouse),
            Event::Resize(w, h) => self.handle_resize(w, h),
            Event::FileOperation(op) => self.handle_file_operation(op),
            Event::NavigationChange(nav) => self.handle_navigation(nav),
        }
    }
}
```

---

### 3.4 state.rs (상태 관리)

**책임**:
- 전역 애플리케이션 상태
- 상태 전이 관리

**상태 구조**:
```rust
pub enum AppState {
    Normal,           // 일반 모드
    Input(InputMode), // 입력 모드
    Dialog(DialogType), // 다이얼로그 표시
    FileOperation(OperationType), // 파일 작업 중
}

pub enum InputMode {
    PathInput,        // 경로 입력
    Search,           // 검색
    Rename,           // 이름 변경
}

pub enum DialogType {
    Confirmation(ConfirmDialog),
    Selection(SelectionDialog),
    Progress(ProgressDialog),
}
```

---

### 3.5 ui/layout.rs (레이아웃 시스템)

**책임**:
- 반응형 레이아웃 계산
- 터미널 크기에 따른 레이아웃 조정

**구조**:
```rust
pub struct LayoutManager {
    terminal_size: (u16, u16),
    mode: LayoutMode,
}

pub enum LayoutMode {
    Dual,     // 듀얼 패널
    Single,   // 싱글 패널
    Minimal,  // 최소 UI
}

impl LayoutManager {
    pub fn calculate_layout(&self) -> Layout {
        let (width, height) = self.terminal_size;

        match (width, height) {
            (w, h) if w >= 80 && h >= 24 => self.dual_panel_layout(),
            (w, h) if w >= 40 && h >= 24 => self.single_panel_layout(),
            _ => self.minimal_layout(),
        }
    }

    fn dual_panel_layout(&self) -> Layout;
    fn single_panel_layout(&self) -> Layout;
    fn minimal_layout(&self) -> Layout;
}

pub struct Layout {
    pub menu_bar: Rect,
    pub left_panel: Rect,
    pub right_panel: Rect,
    pub status_bar: Rect,
    pub command_bar: Rect,
}
```

---

### 3.6 ui/theme.rs (테마 시스템)

**책임**:
- 색상 팔레트 관리
- 테마 로드/저장
- 런타임 테마 전환

**구조**:
```rust
pub struct Theme {
    pub name: String,
    pub colors: ColorPalette,
}

pub struct ColorPalette {
    // 배경/전경
    pub bg_primary: Color,
    pub fg_primary: Color,

    // 패널
    pub panel_active_border: Color,
    pub panel_inactive_border: Color,
    pub panel_bg: Color,

    // 파일 리스트
    pub file_normal: Color,
    pub file_selected: Color,
    pub file_selected_bg: Color,
    pub directory: Color,
    pub executable: Color,
    pub symlink: Color,

    // UI 컴포넌트
    pub menu_bar_bg: Color,
    pub menu_bar_fg: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub command_bar_bg: Color,
    pub command_bar_fg: Color,

    // 강조
    pub accent: Color,
    pub warning: Color,
    pub error: Color,
    pub success: Color,
}

impl Theme {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn save(&self, path: &Path) -> Result<()>;
    pub fn default_dark() -> Self;
    pub fn default_light() -> Self;
    pub fn high_contrast() -> Self;
}
```

---

### 3.7 ui/components/panel.rs (파일 패널)

**책임**:
- 파일 패널 렌더링
- 패널 상태 관리
- 파일 리스트 표시

**구조**:
```rust
pub struct Panel {
    state: PanelState,
    is_active: bool,
}

pub struct PanelState {
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
    pub show_hidden: bool,
    pub filter: Option<String>,
}

impl Panel {
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme);
    pub fn navigate_up(&mut self);
    pub fn navigate_down(&mut self);
    pub fn enter_directory(&mut self) -> Result<()>;
    pub fn go_to_parent(&mut self) -> Result<()>;
    pub fn refresh(&mut self) -> Result<()>;
}
```

---

### 3.8 core/file_manager.rs (파일 관리자)

**책임**:
- 파일 작업 (복사, 이동, 삭제)
- 작업 진행률 추적
- 에러 처리

**구조**:
```rust
pub struct FileManager {
    filesystem: FileSystem,
}

impl FileManager {
    pub async fn copy_files(
        &self,
        sources: &[PathBuf],
        destination: &Path,
        progress: &mut ProgressTracker,
    ) -> Result<()>;

    pub async fn move_files(
        &self,
        sources: &[PathBuf],
        destination: &Path,
        progress: &mut ProgressTracker,
    ) -> Result<()>;

    pub async fn delete_files(
        &self,
        paths: &[PathBuf],
        progress: &mut ProgressTracker,
    ) -> Result<()>;

    pub fn create_directory(&self, path: &Path) -> Result<()>;
    pub fn rename(&self, old: &Path, new: &Path) -> Result<()>;
}

pub struct ProgressTracker {
    pub total_files: usize,
    pub processed_files: usize,
    pub total_bytes: u64,
    pub processed_bytes: u64,
    pub current_file: Option<PathBuf>,
}
```

---

### 3.9 core/navigator.rs (네비게이션)

**책임**:
- 디렉토리 히스토리 관리
- 북마크 관리
- 탭 관리

**구조**:
```rust
pub struct Navigator {
    history: History,
    bookmarks: Bookmarks,
    tabs: Vec<Tab>,
    active_tab_index: usize,
}

pub struct History {
    entries: Vec<PathBuf>,
    current_index: usize,
}

impl History {
    pub fn push(&mut self, path: PathBuf);
    pub fn go_back(&mut self) -> Option<&PathBuf>;
    pub fn go_forward(&mut self) -> Option<&PathBuf>;
}

pub struct Bookmarks {
    items: Vec<Bookmark>,
}

pub struct Bookmark {
    pub name: String,
    pub path: PathBuf,
}

impl Bookmarks {
    pub fn add(&mut self, name: String, path: PathBuf);
    pub fn remove(&mut self, index: usize);
    pub fn load(path: &Path) -> Result<Self>;
    pub fn save(&self, path: &Path) -> Result<()>;
}
```

---

### 3.10 system/filesystem.rs (파일 시스템)

**책임**:
- OS 파일 시스템 추상화
- 파일/디렉토리 읽기
- 메타데이터 파싱

**구조**:
```rust
pub struct FileSystem;

impl FileSystem {
    pub fn read_directory(&self, path: &Path) -> Result<Vec<FileEntry>>;
    pub fn get_metadata(&self, path: &Path) -> Result<FileMetadata>;
    pub fn exists(&self, path: &Path) -> bool;
    pub fn is_directory(&self, path: &Path) -> bool;
    pub fn get_file_type(&self, path: &Path) -> FileType;
}

pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub modified: SystemTime,
    pub permissions: Option<Permissions>,
}

pub enum FileType {
    Directory,
    File,
    Symlink,
    Executable,
}
```

---

### 3.11 models/panel_state.rs (패널 상태)

**책임**:
- 패널 상태 데이터 구조
- 선택 항목 관리

**구조**:
```rust
pub struct PanelState {
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub selected_indices: HashSet<usize>, // 다중 선택
    pub scroll_offset: usize,
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
    pub show_hidden: bool,
    pub filter: Option<String>,
}

pub enum SortBy {
    Name,
    Size,
    Modified,
    Extension,
}

pub enum SortOrder {
    Ascending,
    Descending,
}

impl PanelState {
    pub fn new(path: PathBuf) -> Self;
    pub fn select_next(&mut self);
    pub fn select_previous(&mut self);
    pub fn toggle_selection(&mut self);
    pub fn clear_selection(&mut self);
    pub fn get_selected_entries(&self) -> Vec<&FileEntry>;
}
```

---

## 4. 데이터 흐름

### 4.1 이벤트 처리 흐름

```
User Input (Keyboard/Mouse)
    │
    ↓
Crossterm Event
    │
    ↓
Event Loop (main.rs)
    │
    ↓
App::handle_event()
    │
    ↓
┌───────────────────────────────────┐
│  Event Type에 따라 분기           │
├───────────────────────────────────┤
│  - Key Event → handle_key_event() │
│  - Resize → handle_resize()       │
│  - Mouse → handle_mouse_event()   │
└───────────────────────────────────┘
    │
    ↓
State Update (state.rs)
    │
    ↓
UI Re-render (ui/renderer.rs)
    │
    ↓
Terminal Display (ratatui)
```

### 4.2 파일 작업 흐름

```
User Action (F5: Copy)
    │
    ↓
App::handle_key_event(KeyEvent::Function(5))
    │
    ↓
Open Copy Dialog
    │
    ↓
User Confirms Destination
    │
    ↓
FileManager::copy_files()
    │
    ↓
┌─────────────────────────────────┐
│  Async Task                     │
│  - Read source files            │
│  - Copy to destination          │
│  - Update ProgressTracker       │
└─────────────────────────────────┘
    │
    ↓
Progress Dialog Update (렌더링 중)
    │
    ↓
Operation Complete
    │
    ↓
Refresh Panel
    │
    ↓
Close Dialog
```

### 4.3 테마 전환 흐름

```
User Input (Ctrl+T)
    │
    ↓
App::handle_key_event()
    │
    ↓
Theme Selection Dialog
    │
    ↓
User Selects Theme
    │
    ↓
Theme::load("path/to/theme.toml")
    │
    ↓
App.theme = new_theme
    │
    ↓
Full UI Re-render with New Colors
```

---

## 5. 비동기 처리 전략

### 5.1 Tokio 런타임 사용

**긴 작업에 대한 비동기 처리**:
- 파일 복사/이동 (대용량)
- 파일 검색 (재귀)
- 압축/해제

**동기 작업**:
- UI 렌더링
- 디렉토리 읽기 (일반 크기)
- 키보드 입력 처리

**구조**:
```rust
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // 런타임 생성
    let runtime = tokio::runtime::Runtime::new()?;

    // UI는 메인 스레드에서 동기 실행
    // 긴 작업은 runtime.spawn()으로 비동기 실행
}
```

### 5.2 Progress 업데이트

```rust
pub async fn copy_files(
    sources: &[PathBuf],
    destination: &Path,
) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    // 백그라운드 작업
    tokio::spawn(async move {
        for source in sources {
            // 복사 작업
            // 진행률 전송
            tx.send(Progress { ... }).await;
        }
    });

    // UI 스레드에서 진행률 수신 및 렌더링
    while let Some(progress) = rx.recv().await {
        // UI 업데이트
    }
}
```

---

## 6. 설정 파일 구조

### 6.1 메인 설정 파일 (~/.config/boksldir/config.toml)

```toml
[general]
theme = "dark"
default_editor = "vim"
show_hidden_files = false

[ui]
dual_panel_ratio = [50, 50]
show_icons = true
show_permissions = true

[behavior]
confirm_delete = true
confirm_overwrite = true
follow_symlinks = false

[keybindings]
quit = "q"
panel_switch = "Tab"
copy = "F5"
move = "F6"
delete = "F8"
# ... more keybindings

[file_associations]
"rs" = "code"
"md" = "typora"
"txt" = "vim"
```

### 6.2 테마 파일 (~/.config/boksldir/themes/dark.toml)

```toml
[theme]
name = "Dark"

[colors]
bg_primary = "#1e1e1e"
fg_primary = "#d4d4d4"
panel_active_border = "#0078d4"
panel_inactive_border = "#3c3c3c"
# ... more colors
```

---

## 7. 에러 처리 전략

### 7.1 에러 타입 계층

```rust
// utils/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BokslDirError {
    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Theme error: {0}")]
    Theme(String),

    #[error("Operation cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, BokslDirError>;
```

### 7.2 에러 표시

```rust
impl App {
    fn show_error(&mut self, error: BokslDirError) {
        self.current_dialog = Some(Dialog::Error {
            title: "Error".to_string(),
            message: error.to_string(),
        });
    }
}
```

---

## 8. 테스트 전략

### 8.1 단위 테스트

- 각 모듈별 핵심 로직 테스트
- 파일 작업 로직 (모킹)
- 정렬, 필터링 알고리즘
- 상태 전이

### 8.2 통합 테스트

- 이벤트 처리 플로우
- 파일 작업 전체 플로우
- 설정 로드/저장

### 8.3 테스트 헬퍼

```rust
// tests/helpers/mod.rs
pub fn create_test_app() -> App { ... }
pub fn create_temp_directory() -> TempDir { ... }
pub fn create_test_files(dir: &Path, count: usize) { ... }
```

---

## 9. 성능 최적화 고려사항

### 9.1 대용량 디렉토리 처리

- **Lazy Loading**: 보이는 항목만 렌더링
- **Virtual Scrolling**: 스크롤 영역만 계산
- **파일 목록 캐싱**: 변경 감지 시에만 재로드

### 9.2 렌더링 최적화

- **Dirty Flag Pattern**: 변경된 부분만 다시 그리기
- **Debouncing**: 빠른 키 입력 시 렌더링 스킵
- **Frame Rate Limiting**: 60fps 제한

### 9.3 메모리 관리

- **Arc/Rc 사용**: 큰 데이터 구조 공유
- **String Interning**: 반복되는 문자열 재사용
- **Drop 구현**: 리소스 정리 명시적 관리

---

## 10. 플랫폼별 처리

### 10.1 파일 시스템 차이

```rust
// system/platform.rs
#[cfg(target_os = "macos")]
pub fn open_with_default(path: &Path) -> Result<()> {
    Command::new("open").arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn open_with_default(path: &Path) -> Result<()> {
    Command::new("xdg-open").arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn open_with_default(path: &Path) -> Result<()> {
    Command::new("cmd")
        .args(&["/C", "start", path.to_str().unwrap()])
        .spawn()?;
    Ok(())
}
```

### 10.2 경로 구분자

- Unix: `/`
- Windows: `\`
- `std::path::PathBuf` 사용으로 자동 처리

---

## 11. 확장성 고려사항

### 11.1 플러그인 시스템 (미래)

- WebAssembly 플러그인 지원
- 플러그인 API 정의
- 샌드박싱

### 11.2 원격 파일 시스템 (미래)

- SFTP/FTP 지원
- 클라우드 스토리지 (S3, Google Drive)
- 추상화된 FileSystem trait

---

## 12. 개발 우선순위

### Phase 1: 핵심 구조
1. 프로젝트 스캐폴딩
2. 이벤트 루프
3. 레이아웃 시스템
4. 테마 시스템

### Phase 2: 기본 기능
1. 파일 시스템 읽기
2. 패널 렌더링
3. 네비게이션

### Phase 3: 파일 작업
1. 복사/이동/삭제
2. 진행률 표시

---

**문서 버전**: 1.0.0
**최종 수정일**: 2026-01-31
**다음 리뷰**: Phase 0 완료 후
