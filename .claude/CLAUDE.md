# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

# 복슬Dir 개발 가이드

## 프로젝트 개요

터미널 환경에서 동작하는 듀얼 패널 파일 매니저. Total Commander/Midnight Commander에서 영감을 받아 Rust + TUI로 구현.

**현재 상태**: Phase 4 완료 (Vim 스타일 단축키)
**다음 단계**: Phase 5 - 파일 정렬 및 필터링

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

## 구현된 기능 (Phase 1-4)

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

### Phase 3.2: 파일 복사/이동
- F5 복사, F6 이동: 입력 다이얼로그 → Progress → 완료
- 충돌 처리 (Overwrite/Skip/OverwriteAll/SkipAll/Cancel)
- 진행률 표시, ESC 취소
- 재귀 복사/이동 방지

### Phase 3.3: 파일 삭제
- F8 삭제: DeleteConfirm 다이얼로그 (휴지통/영구삭제/취소)
- 휴지통: `trash` crate 사용, 즉시 처리
- 영구 삭제: Progress 다이얼로그, 파일별 순차 처리
- 다중 선택 삭제, 재귀 디렉토리 삭제

### Phase 3.4: 기타 파일 작업
- F7 새 디렉토리, F2 이름 변경, Alt+Enter 파일 속성
- 모든 입력 다이얼로그 UTF-8 커서 처리 완료

### Phase 4: 단축키 시스템 리팩토링
- Vim 스타일 키바인딩 (j/k/h/l, y/x/d/D/a/r/i)
- 키 시퀀스 시스템 (gg, 500ms 타임아웃, 상태바 표시)
- 단축키 도움말 팝업 (`?`, j/k 스크롤)
- 커맨드 바 Vim 스타일 업데이트
- Ctrl+R 새로고침, D 영구삭제

## 단축키 매핑 (Vim 스타일)

**Normal 모드 키바인딩** (F키 제거 완료, Vim only):

| 카테고리 | 키 | 동작 |
|---------|-----|------|
| 탐색 | `j`/`k` | 커서 아래/위 (↑/↓도 가능) |
| | `h`/`l` | 상위 디렉토리/진입 (←/Enter도 가능) |
| | `gg`/`G` | 맨 위/맨 아래 (Home/End도 가능) |
| | `Ctrl+U`/`Ctrl+D` | 반 페이지 위/아래 (PageUp/PageDown도 가능) |
| 파일 조작 | `y` | 복사 |
| | `x` | 이동 |
| | `d` | 삭제(휴지통) |
| | `D` | 영구 삭제 |
| | `a` | 새 디렉토리 |
| | `r` | 이름 변경 |
| | `i` | 파일 속성 |
| 선택 | `Space` | 선택 토글 |
| | `v` | 선택 반전 |
| | `Ctrl+A` | 전체 선택 |
| | `u` | 전체 해제 |
| 시스템 | `q` | 종료 |
| | `Tab` | 패널 전환 |
| | `F9` | 메뉴 |
| | `?` | 단축키 도움말 |
| | `Ctrl+R` | 새로고침 |

## 단축키 추가 규칙

**새로운 기능을 구현할 때, 해당 기능이 단축키로 접근 가능하다고 판단되면 반드시 단축키를 할당해야 한다.**

- Vim 스타일 니모닉 키를 우선 할당 (예: `e` = edit, `o` = open)
- 위 매핑표에 새 단축키를 추가하고, PRD Phase 4의 키바인딩 매핑표도 함께 업데이트
- 기존 키와 충돌하지 않는지 반드시 확인
- `main.rs`의 `handle_normal_keys()` 함수에 키 핸들러 추가
- 도움말 팝업 (`?`) 내용에도 새 단축키 반영
- 커맨드 바 `command_bar.rs`의 `default_commands()`에도 반영

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
