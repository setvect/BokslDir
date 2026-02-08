# BokslDir 리팩토링 가이드

## 0. 목적과 범위

이 문서는 **BokslDir 프로젝트의 구조 개선**을 위한 실용적 가이드입니다.
모든 리팩토링은 **기능을 유지**하면서 다음을 달성합니다:

* 가독성 향상
* 유지보수 편의성
* 버그 발생 가능성 감소
* 일관된 코드 스타일

---

## 1. BokslDir 프로젝트의 핵심 원칙

### 1.1 액션 시스템이 단일 진실 원천이다

**현재 상태**: `src/core/actions.rs`가 모든 액션의 중앙 레지스트리 역할
- `Action` enum: 모든 가능한 액션
- `ACTION_DEFS`: 메타데이터 (라벨, 단축키, 카테고리)
- `key_bindings()`: 키 → 액션 매핑

**리팩토링 원칙**:
- 새 기능 추가 시 반드시 `actions.rs`에 먼저 등록
- UI 컴포넌트는 레지스트리를 참조만 할 것 (하드코딩 금지)
- 액션 로직은 `app.rs`의 `execute_action()`에 집중

---

### 1.2 명확한 이름 사용

* 이름은 **의도**를 설명해야 한다.
* 구현 세부사항보다 역할을 드러낸다.

❌ `handle()` → ✅ `handle_user_input()`
❌ `data` → ✅ `selected_entry`
❌ `process()` → ✅ `execute_action()`

**BokslDir 네이밍 컨벤션**:
- 패널 관련: `panel_`, `active_panel`, `inactive_panel`
- 파일 작업: `copy_`, `move_`, `delete_`
- UI 상태: `dialog_kind`, `menu_state`

---

### 1.3 함수 크기는 실용적으로

* 목표: **50줄 이하** (엄격하지 않음, 가독성이 우선)
* 중첩 depth **3 초과** 시 분리 고려
* 복잡한 match 표현식은 helper 함수로 추출

**리팩토링 우선순위**:
1. 조기 return으로 중첩 제거
2. match arm이 긴 경우 함수 추출
3. 반복 패턴은 helper로

---

## 2. Rust + TUI 프로젝트 특화 원칙

### 2.1 소유권 문제는 구조 설계의 신호다

* lifetime 에러가 자주 나면 **데이터 소유 위치 재검토**
* BokslDir의 상태는 `App`이 소유하고, UI는 참조만 받는다

**현재 패턴**:
```rust
// App이 모든 상태 소유
pub struct App {
    pub left_panel: PanelState,
    pub right_panel: PanelState,
    pub theme: ThemeManager,
    // ...
}

// UI 컴포넌트는 참조만
impl Panel {
    pub fn render(&self, f: &mut Frame, area: Rect, state: &PanelState)
}
```

**원칙**: 참조보다 값 소유를 우선하되, UI 렌더링은 불변 참조로

---

### 2.2 enum으로 상태 모델링

* boolean 조합 대신 `enum` 사용
* `match`로 모든 케이스 강제 처리

**BokslDir 예시**:
```rust
// ✅ 현재: 명확한 상태
pub enum DialogKind {
    Input { prompt: String, value: String, /* ... */ },
    DeleteConfirm { /* ... */ },
    Progress { /* ... */ },
    // ...
}

// ❌ 나쁜 예
struct Dialog {
    is_input: bool,
    is_confirm: bool,
    is_progress: bool,  // 상태 충돌 가능
}
```

---

### 2.3 모듈 구조 (BokslDir 현재 상태 유지)

**현재 구조**는 잘 분리되어 있음:
```
src/
 ├─ core/       # 액션 시스템
 ├─ ui/         # UI 레이어
 ├─ models/     # 데이터 모델
 ├─ system/     # 파일 시스템
 └─ utils/      # 에러, 포매터 (허용됨)
```

**원칙**: `utils`는 2-3개 범용 모듈만 허용 (error, formatter 등)

---

### 2.4 Result / Option 처리

* `unwrap()`은 명백히 안전한 경우만 (예: 하드코딩 인덱스)
* `?` 연산자로 에러 전파
* UI 경계에서 에러를 메시지로 변환

**BokslDir 패턴**:
```rust
// ✅ 파일 작업은 Result 반환
pub fn copy_file(&self, src: &Path, dst: &Path) -> Result<(), BokslDirError>

// ✅ UI에서 처리
match filesystem.copy_file(src, dst) {
    Ok(_) => { /* 성공 처리 */ }
    Err(e) => {
        self.show_error_dialog(format!("복사 실패: {}", e));
    }
}
```

---

## 3. BokslDir의 TUI 아키텍처 원칙

### 3.1 Rendering은 상태를 변경하지 않는다

**엄격한 원칙**:
* 모든 `render()` 함수는 `&self` 또는 `&State` 받기
* 렌더링 중 `App` 상태 변경 금지

**BokslDir 패턴**:
```rust
// ✅ 올바른 렌더링
impl Panel {
    pub fn render(&self, f: &mut Frame, area: Rect, state: &PanelState) {
        // 읽기만 수행
    }
}

// ❌ 나쁜 예
impl Panel {
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.state.scroll_offset += 1; // NO!
    }
}
```

---

### 3.2 Input → Action → State → Render 흐름 유지

**BokslDir의 이벤트 루프**:
```
1. main.rs: 키 입력 → Action 변환
2. app.rs: execute_action() → 상태 업데이트
3. renderer.rs: render() → 화면 그리기
```

**원칙**:
- 키 입력 처리는 `main.rs`에만
- 비즈니스 로직은 `app.rs`에만
- 렌더링은 `ui/` 모듈에만

---

### 3.3 상태 분리 전략

**현재 상태** (적절함):
```rust
pub struct App {
    // 패널 상태
    pub left_panel: PanelState,
    pub right_panel: PanelState,
    pub active_panel: PanelSide,

    // UI 상태
    pub dialog: Option<DialogKind>,
    pub menu_state: Option<MenuState>,

    // 글로벌 설정
    pub theme: ThemeManager,
    pub layout: LayoutManager,
}
```

**원칙**:
- 패널 상태는 `PanelState`로 분리 ✅
- 다이얼로그는 `Option<DialogKind>` ✅
- 비대해지면 서브 struct 추출 (예: `UIState`, `FileOpState`)

---

## 4. 실용적 코드 품질 원칙

### 4.1 테스트는 중요한 로직만

* 모든 코드에 테스트를 강요하지 않음
* 우선순위: 파일 작업, 경로 처리, 인덱스 계산 로직
* UI 렌더링은 수동 테스트로 충분

**테스트가 필요한 영역**:
- `system/filesystem.rs`: 파일 복사/이동/삭제
- `models/panel_state.rs`: 인덱스 계산, 선택 로직
- `utils/formatter.rs`: 크기/날짜 포맷팅

**테스트 불필요**:
- UI 컴포넌트 렌더링
- 간단한 getter/setter

---

### 4.2 에러 처리는 경계에서만

* 파일 작업은 `Result` 반환
* UI 레이어에서 에러를 사용자 메시지로 변환
* `unwrap()`은 명백히 안전한 곳만

---

### 4.3 UTF-8 안전성 (중요!)

**교훈**: `String::insert()`는 바이트 인덱스를 받는다
- 한글 등 멀티바이트 문자는 `char.len_utf8()` 사용
- 커서 이동은 `char_indices()` 사용
- 화면 너비는 `unicode_width` 사용

**BokslDir에서 지켜진 패턴**:
```rust
// ✅ UTF-8 안전한 커서 처리
let byte_pos = dialog_state.value[..dialog_state.cursor_pos]
    .char_indices()
    .last()
    .map(|(i, _)| i)
    .unwrap_or(0);
```

---

## 5. 리팩토링 실행 규칙

### 5.1 변경 범위 제한

* **한 번에 하나의 목적만**: 이름 변경 OR 로직 수정, 동시에 하지 않기
* **기능 추가 금지**: 리팩토링은 구조 개선만
* **컴파일 가능 상태 유지**: 커밋마다 빌드 성공해야 함

---

### 5.2 BokslDir 리팩토링 순서

1. **이름 개선**: 함수/변수명을 명확하게
2. **함수 추출**: 긴 함수를 작은 함수로 분리
3. **구조 정리**: 모듈 경계 명확히, 중복 제거
4. **성능 개선**: 프로파일링 후 병목 제거

**각 단계마다 커밋 + 테스트**

---

### 5.3 빌드 체크 (필수)

매 리팩토링 후 실행:
```bash
cargo build && cargo clippy && cargo fmt --check && cargo test
```

또는 스킬 사용:
```
/bk-quality-check
```

---

## 6. BokslDir 리팩토링 체크리스트

**리팩토링 전**:
* [ ] 변경 이유가 명확한가? (성능/가독성/버그 수정)
* [ ] 기능 추가가 아닌 구조 개선인가?
* [ ] 영향 범위를 파악했는가?

**리팩토링 중**:
* [ ] 이름이 의도를 드러내는가?
* [ ] 책임이 적절히 분리되었는가?
* [ ] enum으로 표현 가능한 boolean 조합은 없는가?
* [ ] 렌더링 함수가 상태를 변경하지 않는가?
* [ ] UTF-8 처리가 안전한가?

**리팩토링 후**:
* [ ] 빌드가 성공하는가?
* [ ] Clippy 경고가 없는가?
* [ ] 기존 기능이 정상 동작하는가?
* [ ] 커밋 메시지가 명확한가?

---

## 7. BokslDir 최종 원칙

> **액션 시스템이 단일 진실 원천이다**
> 모든 키바인딩과 액션은 `core/actions.rs`에서 관리

> **Rendering은 상태를 읽기만 한다**
> `render()` 함수는 `&self` 또는 `&State`만 받음

> **Input → Action → State → Render**
> 이벤트 흐름을 엄격히 분리

> **실용성이 순수성보다 우선**
> 이론보다 동작하는 코드, 과도한 추상화 금지

> **UTF-8 안전성은 타협하지 않는다**
> 모든 문자열 처리는 `char_indices()` 기반

---

## 8. 참고: BokslDir 주요 패턴

### 액션 추가 패턴
```rust
// 1. core/actions.rs에 Action variant 추가
pub enum Action {
    NewFeature,
}

// 2. ACTION_DEFS에 메타데이터 추가
("new_feature", ActionDef { /* ... */ }),

// 3. key_bindings()에 키 매핑 추가
('n', Action::NewFeature),

// 4. app.rs의 execute_action()에 로직 추가
Action::NewFeature => {
    // 구현
}
```

### 다이얼로그 추가 패턴
```rust
// 1. DialogKind에 variant 추가
pub enum DialogKind {
    NewDialog { /* fields */ },
}

// 2. dialog.rs에 렌더링 추가
DialogKind::NewDialog { /* ... */ } => {
    // 렌더링 로직
}

// 3. main.rs에 입력 처리 추가
if let Some(DialogKind::NewDialog { /* ... */ }) = &app.dialog {
    // 키 입력 처리
}
```
