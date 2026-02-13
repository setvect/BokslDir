# AGENTS.md

이 문서는 Codex(OpenAI coding agent)가 이 저장소에서 일관되게 작업하기 위한 운영 가이드다.

## 1. 프로젝트 요약

- 프로젝트: 복슬Dir(Boksl Dir), Rust 기반 TUI 듀얼 패널 파일 매니저
- 핵심 스택: `ratatui`, `crossterm`, `thiserror`, `trash`, `unicode-width`
- 진입점/중심 로직: `src/main.rs`, `src/app.rs`
- 액션 단일 진실 원천: `src/core/actions.rs`

## 2. 문서 신뢰도 우선순위

문서 간 상태가 일부 어긋나므로 아래 우선순위를 따른다.

1. 실제 코드 (`src/**`)
2. `.claude/CLAUDE.md`
3. `docs/PRD.md` (로드맵/체크리스트 참고용)
4. `README.md` (진행 상태가 오래되었을 수 있음)

주의:
- `docs/PRD.md` 상단 Quick Status, `README.md`의 Phase 표기는 현재 구현과 다를 수 있다.
- 동작 판단은 반드시 코드 기준으로 한다.

## 3. 개발/검증 명령어

```bash
# (세션 시작 시 필요할 수 있음)
source "$HOME/.cargo/env"

# 실행/빌드
cargo run
cargo build
cargo build --release

# 품질
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

참고:
- 현재 `cargo test`는 통과한다.
- `cargo clippy --all-targets --all-features`는 테스트 코드(`panel_state.rs`의 default 후 재할당 패턴) 경고가 일부 발생할 수 있다.

## 4. 실제 구현 구조 (중요)

- `src/main.rs`
  - 이벤트 루프, 모드별 키 처리(normal/menu/dialog), 렌더링 조립
  - 긴 파일 작업 중 non-blocking 처리(`process_next_file`, `process_next_delete`)
- `src/app.rs`
  - 앱 상태, 파일 작업(복사/이동/삭제), 다이얼로그 전이, 액션 실행
- `src/core/actions.rs`
  - `Action` enum, 액션 메타데이터, 키 바인딩, 커맨드바/도움말 데이터 생성
- `src/models/panel_state.rs`
  - 패널 상태, 정렬/필터/숨김 파일/선택 로직
- `src/system/filesystem.rs`
  - 파일 시스템 접근, 복사/이동/삭제, 마운트 포인트 수집
- `src/system/ime.rs`
  - IME 상태 감지(macOS 중심)
- `src/ui/components/*`
  - 패널/메뉴/상태바/다이얼로그/커맨드바 렌더링

현재 스텁(미구현 placeholder):
- `src/core/file_manager.rs`
- `src/core/navigator.rs`
- `src/system/config.rs`
- `src/ui/renderer.rs`

새 기능은 우선 기존 동작 경로(`main.rs` + `app.rs` + `actions.rs` + `models/system/ui/components`)에 붙이고,
명시적 리팩토링 요청이 있을 때만 큰 구조 이동을 한다.

## 5. 핵심 불변조건

### 5.1 액션/단축키 시스템

- 단축키/도움말/커맨드바/메뉴는 `src/core/actions.rs` 레지스트리를 기준으로 유지한다.
- 새 기능이 키보드 접근 가능해야 한다면:
  1. `Action` enum 추가
  2. `ACTION_DEFS` 메타데이터 추가
  3. `key_bindings()` 추가
  4. `app.rs`의 `execute_action()` 매핑 추가
- 시퀀스 키(`gg`, `gm`, `s*`)는 `main.rs`의 pending-key 처리도 함께 수정한다.

### 5.2 패널 인덱스 모델

- `selected_index`: UI 인덱스 (`..` 포함 가능)
- `entries`: 실제 엔트리 배열 (`..` 미포함)
- `selected_items`: `entries` 인덱스 기준
- 부모 디렉토리 존재 여부(`has_parent`)에 따른 오프셋 보정이 필수다.

### 5.3 레이아웃 규칙 (코드 기준)

- 현재 구현은 사실상 2모드다.
  - `DualPanel`: 폭/높이 조건 충족
  - `TooSmall`: 폭 `<80` 또는 높이 `<24`
- 싱글 패널 모드는 현재 비활성(호환용 메서드만 남아 있음).

### 5.4 UTF-8/한글 처리

- 입력 커서/문자 편집은 바이트 인덱스와 문자 인덱스를 혼동하지 않는다.
- UI 너비 계산은 `unicode-width` 기준을 유지한다.
- IME 상태/한영 전환 안내 관련 동작을 깨지 않도록 주의한다.

## 6. 변경 시 체크리스트

- 기능 추가/수정 시:
  - 관련 테스트 추가 또는 기존 테스트 보강
  - `cargo test` 실행
  - 필요 시 `cargo fmt`, `cargo clippy --all-targets --all-features` 실행
- 사용자-visible 단축키/동작을 바꿨다면:
  - `.claude/CLAUDE.md`의 단축키/기능 설명 업데이트
  - `docs/PRD.md` 체크리스트 또는 상태 문구 정합성 점검
- 큰 구조 변경 시:
  - `docs/Architecture.md`와 실제 파일 구조 불일치가 커지지 않게 최소한의 정리 반영

## 7. 코드 스타일 가이드

- 에러는 `BokslDirError`/`Result<T, BokslDirError>` 패턴을 따른다.
- 렌더링 로직은 상태 변경 없이 읽기 중심으로 유지한다.
- 과도한 추상화보다 현재 구조와 일관성을 우선한다.
- 주석/문구는 기존 코드 톤(한국어 중심, 기술 용어 영어 허용)에 맞춘다.

