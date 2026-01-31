# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

# 복슬Dir 개발 가이드

## 프로젝트 개요

터미널 환경에서 동작하는 듀얼 패널 파일 매니저. Total Commander/Midnight Commander에서 영감을 받아 Rust + TUI로 구현.

**현재 상태**: Phase 0 완료 (프로젝트 초기화)
**다음 단계**: Phase 1 - UX/UI 기반 구조 (레이아웃 시스템, 테마 시스템)

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
```

## 시스템 아키텍처

### 3계층 구조

이 프로젝트는 **엄격한 3계층 아키텍처**를 따릅니다:

```
UI Layer (src/ui/)
  ↓ 이벤트 → 액션 변환
Core Layer (src/core/)
  ↓ 비즈니스 로직 실행
System Layer (src/system/)
  ↓ OS/파일시스템 접근
```

**UI Layer** (`src/ui/`)
- `layout.rs`: 반응형 레이아웃 시스템 (터미널 크기별 조정)
- `theme.rs`: 색상 테마 시스템
- `components/`: 재사용 가능한 UI 위젯들
- **책임**: 렌더링, 이벤트 처리, 사용자 인터랙션

**Core Layer** (`src/core/`)
- `file_manager.rs`: 파일 작업 (복사/이동/삭제)
- `navigator.rs`: 네비게이션 (히스토리/북마크/탭)
- **책임**: 비즈니스 로직, 상태 관리, 순수 함수

**System Layer** (`src/system/`)
- `filesystem.rs`: 파일 시스템 추상화
- `config.rs`: 설정 파일 관리
- **책임**: OS 의존적 코드, 외부 프로세스 실행

### 주요 데이터 흐름

```
User Input → Event → App::handle_event() → State Update → UI Re-render
```

비동기 작업 (파일 복사/검색):
```
Action → FileManager (Tokio) → Progress Updates → UI Update
```

## Phase별 개발 전략

### Phase 1: UX/UI 기반 구조 (현재 작업 대상)

**우선순위**: 레이아웃과 테마가 모든 UI의 기초이므로 먼저 완성

1. **레이아웃 시스템** (`src/ui/layout.rs`)
   - 터미널 크기 감지 및 반응형 조정
   - 최소 80x24 이상에서 듀얼 패널
   - 40x24~79x24에서 싱글 패널 모드
   - ratatui의 `Layout::default()` 사용

2. **테마 시스템** (`src/ui/theme.rs`)
   - TOML 기반 색상 팔레트
   - Dark/Light/High Contrast 기본 테마
   - 런타임 테마 전환 지원

3. **UI 컴포넌트** (`src/ui/components/`)
   - 각 컴포넌트는 독립적으로 테스트 가능
   - 테마와 레이아웃에 의존

**완료 기준**:
- 터미널 크기 변경 시 자동 조정
- 최소 3개 테마 전환 가능
- Tab 키로 패널 전환 동작

### Phase 2: 파일 시스템 통합

실제 디렉토리 읽기 → 파일 리스트 렌더링 → 네비게이션

### Phase 3+

PRD.md 참조

## 중요한 설계 결정사항

### 비동기 처리 전략

- **UI 렌더링**: 동기 (메인 스레드)
- **파일 작업**: 비동기 (Tokio)
  - 대용량 파일 복사/이동
  - 재귀 디렉토리 검색
  - 압축/해제

이유: UI 블로킹을 방지하고 진행률을 실시간으로 표시하기 위함

### 반응형 UI 규칙

터미널 크기에 따른 레이아웃 변경 로직은 `src/ui/layout.rs`에 중앙화:

- **80+ cols**: 듀얼 패널 (50:50)
- **40-79 cols**: 싱글 패널 (Tab 전환)
- **<40 cols**: 경고 메시지

### 에러 처리

- `thiserror`로 계층적 에러 타입 정의 (`src/utils/error.rs`)
- 모든 I/O 작업은 `Result<T, BokslDirError>` 반환
- UI에서 다이얼로그로 에러 표시

### 설정 파일 우선순위

```
기본값 < ~/.config/boksldir/config.toml < 실행 시 인자
```

## Git 커밋 메시지 작성 규칙

- **한글로 작성** (전문용어는 영어 사용 가능)
- **형식**: `동사 + 명사 + 설명`

**예시**:
```
레이아웃 시스템 구현

- 터미널 크기에 따른 반응형 레이아웃 추가
- 듀얼/싱글 패널 모드 전환 로직 구현
- ratatui Layout API 통합
```

**좋은 예**:
- "Theme 시스템 구현"
- "파일 복사 기능 추가"
- "레이아웃 버그 수정"

**나쁜 예**:
- "Implement theme system" (영어 전체 문장)
- "작업 완료" (모호함)
- "Update files" (영어 + 구체적이지 않음)

## 관련 문서

- `docs/Requirements.md`: 추상적 요구사항 (그룹별 정리)
- `docs/PRD.md`: 상세 기능 명세 (Phase별 체크리스트)
- `docs/Architecture.md`: 시스템 설계 상세 (모듈별 API 포함)
