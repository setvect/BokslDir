# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

# 복슬Dir 개발 가이드

## 프로젝트 개요

터미널 환경에서 동작하는 듀얼 패널 파일 매니저. Total Commander/Midnight Commander에서 영감을 받아 Rust + TUI로 구현.

**현재 상태**: Phase 3.1 완료 (파일 선택 시스템)
**다음 단계**: Phase 3.2 - 파일 복사/이동

## 개발 명령어

```bash
# 환경 설정 (필수 - 매 세션마다)
source "$HOME/.cargo/env"

# 개발 실행
cargo run

# 빌드
cargo build              # 개발 빌드
cargo build --release    # 릴리스 빌드

# 코드 품질
cargo fmt                # 포맷팅
cargo clippy             # 린팅
cargo test               # 테스트 실행
cargo test <test_name>   # 단일 테스트 실행
```

## 시스템 아키텍처

### 모듈 구조

```
src/
├── app.rs              # 앱 상태 및 비즈니스 로직 (App 구조체)
├── main.rs             # 이벤트 루프, 키 핸들링
├── ui/                 # UI Layer
│   ├── layout.rs       # 반응형 레이아웃 (LayoutManager)
│   ├── theme.rs        # 색상 테마 (ThemeManager)
│   ├── renderer.rs     # 전체 화면 렌더링
│   └── components/     # UI 위젯
│       ├── panel.rs        # 파일 패널 (Panel)
│       ├── menu_bar.rs     # 상단 메뉴바
│       ├── dropdown_menu.rs # 드롭다운 메뉴
│       ├── status_bar.rs   # 하단 상태바
│       └── command_bar.rs  # F키 단축키 바
├── models/             # 데이터 모델
│   ├── file_entry.rs   # 파일 정보 (FileEntry, FileType)
│   └── panel_state.rs  # 패널 상태 (PanelState)
├── system/             # System Layer
│   └── filesystem.rs   # 파일 시스템 추상화 (FileSystem)
└── utils/
    ├── error.rs        # 에러 타입 (BokslDirError)
    └── formatter.rs    # 포맷터 (크기, 날짜, 권한)
```

### 주요 데이터 흐름

```
키 입력 → main.rs (handle_normal_keys/handle_menu_keys)
       → App 메서드 호출 → 상태 업데이트
       → renderer.rs → 화면 렌더링
```

### 핵심 구조체

- **App** (`app.rs`): 전체 앱 상태 관리, 패널/메뉴/테마 상태 보유
- **PanelState** (`models/panel_state.rs`): 패널별 경로, 파일 목록, 커서 인덱스, 스크롤 오프셋, 다중 선택(selected_items)
- **FileEntry** (`models/file_entry.rs`): 파일 메타데이터 (이름, 크기, 날짜, 권한, 타입)
- **Panel** (`ui/components/panel.rs`): 파일 리스트 렌더링 위젯

## 구현된 기능 (Phase 1-2)

### Phase 1: UI 기반 구조
- 반응형 레이아웃 (듀얼 패널 80+cols, 싱글 패널 40-79cols)
- 색상 테마 (Dark/Light/High Contrast), 런타임 전환
- 드롭다운 메뉴 시스템 (F9로 활성화)
- 한글 문자 너비 처리 (unicode-width)

### Phase 2: 파일 시스템 통합
- 디렉토리 읽기, 파일 메타데이터 파싱
- 파일 리스트 렌더링 (아이콘, 색상 구분)
- 키보드 네비게이션 (↑↓, PageUp/PageDown)
- 디렉토리 진입 (Enter), 상위 이동 (..)
- 상위 이동 시 이전 디렉토리 포커스 유지
- 경로 표시 축약 (홈 디렉토리 ~, 중간 생략)

### Phase 3.1: 파일 선택 시스템
- 다중 선택: Space (토글 + 커서 아래로)
- 전체 선택: `*` 또는 Ctrl+A
- 선택 반전: `+`
- 전체 해제: Ctrl+D
- 선택 하이라이트: 골드색 + `*` 마커
- 상태바: 선택 개수/크기 표시
- ".." 항목 선택 불가, 디렉토리 변경 시 선택 초기화

## 중요한 설계 결정사항

### 인덱스 체계
- `selected_index`: ".." 항목 포함 UI 인덱스 (0 = "..", 1 = entries[0])
- `scroll_offset`: entries 배열 인덱스 (0 = entries[0])
- `selected_items`: entries 배열 인덱스 기반 HashSet (".." 제외)
- 변환 시 `has_parent` 여부 확인 필수

### 반응형 UI 규칙 (src/ui/layout.rs)
- **80+ cols**: 듀얼 패널 (50:50)
- **<80 cols**: 경고 메시지

### 에러 처리
- `thiserror`로 계층적 에러 타입 (`src/utils/error.rs`)
- 모든 I/O 작업은 `Result<T, BokslDirError>` 반환

## Git 커밋 메시지 작성 규칙

- **한글로 작성** (전문용어는 영어 사용 가능)
- **형식**: `동사 + 명사 + 설명`

**좋은 예**:
- "Theme 시스템 구현"
- "파일 복사 기능 추가"
- "레이아웃 버그 수정"

## 관련 문서

- `docs/PRD.md`: 상세 기능 명세 (Phase별 체크리스트)
- `docs/Requirements.md`: 추상적 요구사항
- `docs/Architecture.md`: 시스템 설계 상세
