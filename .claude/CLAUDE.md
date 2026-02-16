# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

# 복슬Dir 개발 가이드

## 프로젝트 개요

터미널 환경에서 동작하는 듀얼 패널 파일 매니저. Total Commander/Midnight Commander에서 영감을 받아 Rust + TUI로 구현.

**현재 상태**: Phase 6.3 완료 (북마크 시스템) + Phase 6.2 완료 (디렉토리 히스토리) + Phase 6.1 완료 (탭 시스템) + Phase 5.3 완료 (기타 탐색 기능) + Phase 5.2 완료 (검색 및 필터링) + Phase 5.1 완료 (파일 정렬) + Phase 4 완료 (Vim 스타일 단축키) + 액션 시스템 일원화 완료
**다음 단계**: Phase 6.4 - 경로 입력 및 자동완성

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
├── core/               # Core Layer
│   └── actions.rs      # 액션 시스템 (Action enum, 키바인딩 레지스트리)
├── ui/                 # UI Layer
│   ├── layout.rs       # 반응형 레이아웃 (LayoutManager)
│   ├── theme.rs        # 색상 테마 (ThemeManager)
│   ├── renderer.rs     # 전체 화면 렌더링
│   └── components/     # UI 위젯
│       ├── panel.rs        # 파일 패널 (Panel)
│       ├── menu_bar.rs     # 상단 메뉴바
│       ├── dropdown_menu.rs # 드롭다운 메뉴
│       ├── status_bar.rs   # 하단 상태바
│       └── command_bar.rs  # 단축키 바
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
키 입력 → main.rs (handle_normal_keys)
       → core/actions.rs (find_action) → Action enum
       → app.rs (execute_action) → 상태 업데이트
       → renderer.rs → 화면 렌더링
```

### 액션 시스템 (Single Source of Truth)

`src/core/actions.rs`가 모든 액션/단축키의 단일 진실 원천:
- **Action enum**: 모든 가능한 액션 열거
- **ACTION_DEFS**: 액션별 메타데이터 (id, label, category, shortcut, command_bar)
- **key_bindings()**: 키 → 액션 매핑
- **소비자**: main.rs, app.rs, command_bar.rs, dialog.rs, dropdown_menu.rs 모두 레지스트리 참조
- **새 기능 추가 시**: `actions.rs`에만 등록하면 모든 소비자에 자동 반영

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

### Phase 5.1: 파일 정렬
- 정렬 기준: 이름/크기/수정 날짜/확장자 (대소문자 무시)
- 디렉토리 우선 표시 (항상)
- 정렬 상태 표시: 패널 헤더 ▲/▼ 화살표 + 상태바 `[Name ▲]`
- 키 시퀀스: `sn`/`ss`/`sd`/`se` (정렬 기준), `sr` (순서 반전)
- 같은 기준 재선택 시 자동 순서 토글
- 메뉴 > 보기 > 정렬 기준/순서 지원
- 정렬 후 커서 위치 보존, 다중 선택 초기화

### Phase 5.2: 검색 및 필터링
- 빠른 필터: `/` → 패턴 입력 → 실시간 필터링 (라이브 업데이트)
- 글로브 패턴 지원: `*`, `?` 와일드카드 (예: `*.rs`, `test*`)
- 일반 패턴: contains 매칭 (대소문자 무시)
- 필터 하이라이트: contains=매칭 부분, glob=전체 이름 강조
- 상태바 필터 표시: `[Filter: *.rs]` (녹색)
- 보기 메뉴: 필터링/필터 해제 항목 추가

### Phase 5.3: 기타 탐색 기능
- 숨김 파일 토글: `.` 키, 양쪽 패널 동시 토글, 상태바 `[Hidden]` 인디케이터
- 마운트 포인트: `gm` 키 시퀀스, 선택형 다이얼로그 (j/k/Enter/Esc)
  - macOS: Home, Root, /Volumes/* 자동 탐지
  - Linux: Home, Root, /mnt/*, /media/* 자동 탐지
- 파일 크기 표시 형식: 보기 메뉴 > 크기 표시 형식 (자동 KB/MB/GB, 바이트)
  - App.size_format: SizeFormat enum (Auto, Bytes)
  - 패널 + 상태바 모두 반영

### Phase 6.1: 탭 시스템
- 패널별 독립 탭 상태 (`PanelTabs`)
- 탭 키 시퀀스: `tn`(새 탭), `tx`(닫기), `tt`(목록 모달)
- 활성 탭 목록 모달: `tt` (j/k/Enter/Esc로 이동/선택/닫기)
- 패널 타이틀에 탭 개수 표시 (`~/path [3]`)
- 탭별 경로/커서/스크롤/정렬/필터/선택/숨김 상태 독립 보존
- 마지막 탭 닫기 금지 (토스트 안내)

### Phase 6.2: 디렉토리 히스토리
- 탭별 독립 디렉토리 히스토리 (`PanelState.history_entries/history_index`)
- 히스토리 뒤로/앞으로: `Alt+←` / `Alt+→`
- 히스토리 목록 모달: `th` (최신순, 현재 위치 기본 선택)
- 히스토리 목록 모달에서 `D`로 현재 패널 히스토리 전체 삭제 (현재 경로만 유지)
- 프로그램 재시작 후 히스토리 자동 복원 (활성 탭 기준)
- 방문 기록 정책: 연속 중복 제거 + 최대 100개 유지

### Phase 6.3: 북마크 시스템
- 전역 북마크 목록 (`App.bookmarks`) 저장/복원
- 북마크 추가: `Ctrl+B` (현재 디렉토리)
- 북마크 목록 모달: `tb` (j/k/Enter/r/d/Esc)
- 북마크 이름 변경(`r`) / 삭제(`d`) 지원
- 북마크 설정 파일: `~/.boksldir/bookmarks.toml` (`BOKSLDIR_BOOKMARKS_FILE` override)

## 단축키 매핑 (Vim 스타일)

**Normal 모드 키바인딩** (F키 제거 완료, Vim only):

| 카테고리 | 키 | 동작 |
|---------|-----|------|
| 탐색 | `j`/`k` | 커서 아래/위 (↑/↓도 가능) |
| | `h`/`l` | 상위 디렉토리/진입 (←/Enter도 가능) |
| | `gg`/`G` | 맨 위/맨 아래 (Home/End도 가능) |
| | `Ctrl+U`/`Ctrl+D` | 반 페이지 위/아래 (PageUp/PageDown도 가능) |
| | `tn` | 새 탭 |
| | `tx` | 탭 닫기 (마지막 탭 제외) |
| | `tt` | 활성 패널 탭 목록 모달 |
| | `th` | 활성 패널 디렉토리 히스토리 모달 |
| | `tb` | 북마크 목록 모달 |
| | `Alt+←`/`Alt+→` | 히스토리 뒤로/앞으로 |
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
| 정렬 | `sn` | 이름순 정렬 |
| | `ss` | 크기순 정렬 |
| | `sd` | 날짜순 정렬 |
| | `se` | 확장자순 정렬 |
| | `sr` | 정렬 순서 반전 |
| 검색/필터 | `/` | 빠른 필터 (글로브 지원) |
| 보기 | `.` | 숨김 파일 토글 |
| | `gm` | 마운트 포인트 |
| | `Ctrl+B` | 현재 경로 북마크 추가 |
| 시스템 | `q` | 종료 |
| | `Tab` | 패널 전환 |
| | `F9` | 메뉴 |
| | `?` | 단축키 도움말 |
| | `Ctrl+R` | 새로고침 |

## 단축키 추가 규칙

**새로운 기능을 구현할 때, 해당 기능이 단축키로 접근 가능하다고 판단되면 반드시 단축키를 할당해야 한다.**

- Vim 스타일 니모닉 키를 우선 할당 (예: `e` = edit, `o` = open)
- **`src/core/actions.rs`에만 등록하면 모든 소비자에 자동 반영**:
  1. `Action` enum에 새 variant 추가
  2. `ACTION_DEFS`에 메타데이터 추가 (id, label, shortcut_display, command_bar)
  3. `key_bindings()`에 키 매핑 추가
  4. `app.rs`의 `execute_action()`에 매치 암 추가
- 기존 키와 충돌하지 않는지 반드시 확인
- 위 매핑표도 함께 업데이트

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
