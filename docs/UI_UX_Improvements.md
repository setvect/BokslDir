# UI/UX 개선 계획

## 현재 상태
- Phase 4 완료 (Vim 스타일 단축키 + 액션 시스템 일원화)
- 이 문서는 기존 기능에 대한 UI/UX 개선 사항을 정리한 것

---

## 1. 파일 패널 스크롤바 추가

**현재 문제**: 파일 목록이 한 화면에 다 표시되지 않으면 스크롤 오프셋으로 내부 처리는 하지만, 사용자에게 현재 위치를 알려주는 시각적 피드백이 없음.

**개선 내용**:
- 패널 우측 테두리에 스크롤바 표시 (전체 항목 대비 현재 보이는 영역 비율)
- 모든 항목이 화면에 표시되면 스크롤바 숨김
- ratatui의 `Scrollbar` 위젯 활용 가능

**관련 코드**: `src/ui/components/panel.rs` - `Widget::render()` (526-575줄)

---

## 2. 모달창 크기 터미널 반응형 최적화

**현재 문제**: 모든 다이얼로그가 고정 크기로 정의되어 있음.

```rust
// src/ui/components/dialog.rs:294-315
DialogKind::Help { .. } => (60, 22),           // 고정 60x22
DialogKind::Properties { .. } => (50, base),    // 고정 너비 50
DialogKind::Input { .. } => (50, 7),             // 고정 50x7
```

**개선 내용**:

### 2-1. Help(단축키 도움말) 다이얼로그
- 고정 높이 22 대신 터미널 높이에 비례하여 조정 (예: `screen.height - 6`)
- 최소 높이만 보장하고 최대한 많은 내용 표시
- 내용이 넘치면 스크롤바 표시 (현재 j/k 스크롤 기능은 있으나 시각적 표시 없음)

### 2-2. Properties 다이얼로그
- 고정 너비 50 대신 터미널 너비에 비례 (예: `min(screen.width - 8, 80)`)
- Path 정보가 잘리는 문제 해결 (현재 `truncate_middle`로 경로가 생략됨)
- 긴 파일명이나 경로도 충분히 표시 가능

### 2-3. 기타 다이얼로그
- Input/Confirm/Conflict/Progress 다이얼로그도 터미널 크기에 비례하여 최소/최대 범위 내에서 동적 조정

**관련 코드**: `src/ui/components/dialog.rs` - `calculate_area()` (294-329줄)

---

## 3. 커맨드바 레이블 명확화

**현재 문제**: `?:Help`라고 표시되어 있지만, 이 기능은 일반적인 "도움말"이 아니라 "단축키 안내(Keyboard Shortcuts)"임.

```rust
// src/core/actions.rs
command_bar: Some(CommandBarEntry {
    key: "?",
    label: "Help",    // 모호한 레이블
    priority: 20,
}),
```

**개선 내용**:
- `"Help"` → `"Keys"` 또는 `"Shortcuts"` 로 변경
- 커맨드바는 공간이 제한적이므로 짧은 레이블 권장

**관련 코드**: `src/core/actions.rs` - ACTION_DEFS 내 `ShowHelp` 항목

---

## 4. Properties 다이얼로그 표시 개선

### 4-1. 복수형 (s) 처리

**현재 문제**: `file(s)`, `dir(s)`, `item(s)` 형태로 항상 `(s)`가 붙어 있음.

```rust
// src/app.rs
format!("{} file(s), {}", total_files, ...)              // 411줄
format!("{} completed: {} file(s)", ...)                  // 1024줄
format!("Moved {} item(s) to trash.", ...)                // 1173줄
format!("{} ({} file(s))", ...)                           // 1396줄
format!("{} file(s), {} dir(s)", files, dirs)             // 1416줄

// src/ui/components/dialog.rs
format!("Delete {} item(s)? ({})", items.len(), ...)      // 700줄
```

**개선 내용**: 프로그래밍으로 단/복수형 판단
```
1 file  →  "1 file"
5 files →  "5 files"
1 item  →  "1 item"
3 items →  "3 items"
```

헬퍼 함수 추가:
```rust
fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 { format!("{} {}", count, singular) }
    else { format!("{} {}", count, plural) }
}
```

### 4-2. 천단위 콤마 적용

**현재 문제**: 숫자에 천단위 구분자가 없어 큰 숫자가 읽기 어려움.

```
현재:  1234567 → "1234567"
개선:  1234567 → "1,234,567"
```

**적용 대상** (전체):
- 파일 크기 바이트 표시 (Properties의 원본 크기 등)
- 파일/디렉토리 개수 (상태바, Properties)
- 진행률 다이얼로그의 파일 카운트

**관련 코드**: `src/utils/formatter.rs`에 천단위 포맷 함수 추가

### 4-3. Modified 날짜에 년월일 시분초 모두 표시

**현재 문제**: Properties에서도 `format_date()`를 사용하여 오늘이면 시간만, 과거면 날짜만 표시.

```rust
// src/utils/formatter.rs:53-65
if time_date == today {
    datetime.format("%H:%M").to_string()      // "14:30"
} else {
    datetime.format("%Y-%m-%d").to_string()    // "2026-01-30"
}
```

**개선 내용**:
- Properties 다이얼로그에서는 **전체 날짜/시간** 표시: `"2026-01-30 14:30:25"`
- 기존 패널 목록용 `format_date()`는 그대로 유지
- 별도 함수 `format_date_full()` 추가

**관련 코드**: `src/utils/formatter.rs`, `src/app.rs:1447` (Properties 생성 시)

---

## 5. 한글 입력 상태 표시

**현재 문제**: 한글 입력 모드에서는 Vim 스타일 단축키가 동작하지 않음 (OS IME가 키 이벤트를 가로챔). 사용자가 한글 모드인지 인지하기 어려움.

**현실적 한계**: 터미널 앱에서 OS의 IME 상태를 직접 감지하는 것은 불가능에 가까움.

**대안적 개선 방안**:
- 상태바 또는 패널 하단에 **안내 메시지** 상시 표시: `"IME: 영문 모드에서 단축키 사용"` 또는 `"[EN mode for shortcuts]"`
- 또는 Help 다이얼로그 상단에 안내 문구 추가
- 키 입력 시 일정 시간 내 매칭되는 액션이 없으면 "한영키를 확인하세요" 같은 토스트 메시지 표시 (입력한 문자가 한글 범위 `\uAC00-\uD7A3`인지 감지하여 판단 가능)

**관련 코드**: `src/main.rs` - `handle_normal_keys()`, `src/ui/components/status_bar.rs`

---

## 6. 파일명 잘림 시 확장자 보존

**현재 문제**: 긴 파일명이 끝에서 잘려 확장자가 보이지 않음.

```rust
// src/ui/components/panel.rs:580-599
// 현재: "very_long_filename_that..."
// 확장자가 잘려서 파일 타입 파악 불가
```

**개선 내용**: 중간 생략 방식으로 변경하여 확장자 보존
```
현재: "very_long_filename_that_should_be_tr..."
개선: "very_long_fi...uncated.txt"
```

**관련 코드**: `src/ui/components/panel.rs` - `truncate_name()` (580-599줄)

---

## 7. 패널 날짜 포맷 정렬 불일치

**현재 문제**: 같은 컬럼에서 오늘 파일은 `"14:30"` (5자), 과거 파일은 `"2026-01-30"` (10자)로 너비가 다름. short 포맷에서도 `"HH:MM"` vs `"MM-DD"`로 혼재.

```rust
// src/ui/components/panel.rs:489-498
let date_str = if layout.date_format == "long" {
    format_date(entry.modified)      // "14:30" 또는 "2026-01-30"
} else {
    // short: "14:30" 또는 "01-30"
};
```

**개선 내용**:
- 날짜 컬럼 너비를 항상 일정하게 유지
- long 포맷: `"2026-01-30"` / `"Today 14:30"` (모두 10~11자)
- 또는 항상 `"YYYY-MM-DD HH:MM"` 형태로 통일 (16자)
- short 포맷도 통일된 너비

**관련 코드**: `src/utils/formatter.rs` - `format_date()`, `src/ui/components/panel.rs` - 날짜 렌더링

---

## 8. 파일 크기 단위 너비 불일치

**현재 문제**: 크기 컬럼에서 `"512B"` vs `"1.5KB"` vs `"1.0MB"` 등 단위 문자열 길이가 달라 정렬이 어색할 수 있음.

```rust
// src/utils/formatter.rs:18-36
// "0B", "512B", "1.5KB", "1.0MB", "2.0GB"
```

**개선 내용**:
- 단위를 고정 너비로 맞춤: `" 512 B"`, `"1.5 KB"`, `"1.0 MB"`
- 또는 우측 정렬 시 패딩 적용으로 시각적 정렬 보장
- 현재 패널에서는 `format!("{:>9}", size_str)` (484줄)로 9자 우측정렬하고 있으나, 단위와 숫자 사이에 공백이 없어 읽기 어려울 수 있음

**관련 코드**: `src/utils/formatter.rs` - `format_file_size()`, `src/ui/components/panel.rs:484`

---

## 9. 도움말 다이얼로그 스크롤바 표시

**현재 문제**: Help 다이얼로그에서 j/k로 스크롤 가능하지만, 현재 위치를 알 수 있는 시각적 표시가 없음.

**개선 내용**:
- 우측에 간단한 스크롤 인디케이터 추가
- 또는 하단 힌트 영역에 `"1/5"` 같은 페이지 번호 표시
- 스크롤 가능할 때만 ▲/▼ 화살표 표시

**관련 코드**: `src/ui/components/dialog.rs` - `render_help()` (846-916줄)

---

## 10. 빈 디렉토리 안내 메시지 개선

**현재 문제**: 빈 디렉토리에서 `" <empty>"` 만 표시.

```rust
// src/ui/components/panel.rs:517-523
let empty_text = " <empty>";
```

**개선 내용**:
- 더 유용한 안내: `"Empty directory  (a: new dir)"` 등
- 또는 최소한 `"(No files)"` 같이 좀 더 명확한 표현

**관련 코드**: `src/ui/components/panel.rs` - `render_empty_state()` (517-523줄)

---

## 11. 삭제 다이얼로그 버튼 레이블 일관성

**현재 문제**: DeleteConfirm 다이얼로그만 한글 버튼 (`"휴지통"`, `"영구 삭제"`, `"취소"`)이고, 나머지 다이얼로그는 영어 버튼 (`"OK"`, `"Cancel"`)임.

```rust
// src/ui/components/dialog.rs:729-733
self.render_button(buf, x, button_y, "휴지통", ...);
self.render_button(buf, x, button_y, "영구 삭제", ...);
self.render_button(buf, x, button_y, "취소", ...);

// 다른 다이얼로그:
self.render_button(buf, ..., "OK", ...);
self.render_button(buf, ..., "Cancel", ...);
```

**개선 내용**:
- 모든 다이얼로그를 한글 또는 영어로 통일
- 권장: 영어 통일 (`"Trash"`, `"Delete"`, `"Cancel"` 등) - 영문 모드에서 사용하는 프로그램 특성상
- 또는 한글 통일 (`"확인"`, `"취소"` 등)

**관련 코드**: `src/ui/components/dialog.rs` - 각 `render_*` 함수

---

## 12. 진행률 다이얼로그 정보 보강

**현재 문제**: 복사/이동 진행률에 현재 파일명과 바이트/파일 카운트만 표시.

**개선 내용**:
- 전송 속도 표시 (MB/s)
- 예상 남은 시간 (ETA) 표시
- 경과 시간 표시

**관련 코드**: `src/ui/components/dialog.rs` - `render_progress()` (603-668줄), `src/models/operation.rs`

---

## 13. 이모지 아이콘 터미널 호환성

**현재 문제**: 파일 타입 아이콘으로 이모지(📁📄🔧🔗) 사용. 일부 터미널/폰트에서 깨지거나 너비가 맞지 않을 수 있음.

```rust
// src/ui/components/panel.rs:207-214
FileType::Directory => "📁",
FileType::File => "📄",
FileType::Executable => "🔧",
FileType::Symlink => "🔗",
```

**개선 내용**:
- 설정으로 아이콘 모드 선택 가능: emoji / ascii / nerd-font
- ASCII 대체: `"D "`, `"F "`, `"X "`, `"L "` 또는 `"/"`, `" "`, `"*"`, `"@"`
- 또는 Nerd Font 아이콘 활용

**관련 코드**: `src/ui/components/panel.rs` - `file_icon()` (207-214줄)

---

## 14. 상태바 정보 밀도 개선

**현재 문제**: 상태바에 `" 10 files, 5 dirs | 1.2GB"` 형태로 표시. 좁은 화면에서 잘릴 수 있음.

```rust
// src/ui/components/status_bar.rs:126-129
let left_info = format!(
    " {} files, {} dirs | {}",
    self.file_count, self.dir_count, self.total_size
);
```

**개선 내용**:
- 좁은 화면 대응: `"10f 5d | 1.2GB"` 같은 축약 모드
- 현재 선택된 파일의 정보 (이름, 크기) 표시 추가 고려
- 화면 너비에 따라 정보량 동적 조절

**관련 코드**: `src/ui/components/status_bar.rs` - `Widget::render()` (120-169줄)

---

## 15. 다이얼로그 내부 여백 불일치

**현재 문제**: 다이얼로그마다 inner 영역 계산 시 패딩이 다름.

```rust
// input/confirm/delete_confirm 등:
let inner = Rect {
    x: area.x + 2,           // 좌우 패딩 2
    width: area.width.saturating_sub(4),
    ...
};
```

대부분 `+2`/`sub(4)` 패턴이지만, 향후 추가되는 다이얼로그에서 불일치 발생 가능.

**개선 내용**:
- 다이얼로그 inner margin을 상수로 통일 정의
- `const DIALOG_PADDING: u16 = 2;`

**관련 코드**: `src/ui/components/dialog.rs` - 각 `render_*` 함수

---

## 16. 단축키 표시 형식 통일

**현재 문제**: 커맨드바와 도움말에서 단축키 표기가 혼재.

```
"j/k"          → 슬래시 구분
"gg/G"         → 시퀀스와 단일키 혼합
"Ctrl+U/D"     → 한쪽만 Ctrl 표기
"Ctrl+A"       → 전체 표기
```

**개선 내용**:
- 통일된 표기법 정의
  - 수식키: `^A` (Ctrl+A), `M-Enter` (Alt+Enter)
  - 복합: `^U / ^D` (각각 독립 표기)
  - 또는 현재 형식 유지하되 `Ctrl+U/D` → `Ctrl+U/Ctrl+D`로 명확화

**관련 코드**: `src/core/actions.rs` - ACTION_DEFS의 `shortcut_display`

---

## 우선순위 정리

### P0 (즉시 개선 - 사용성에 직접 영향)
| # | 항목 | 난이도 |
|---|------|--------|
| 1 | 파일 패널 스크롤바 | 중 |
| 2 | 모달창 반응형 크기 | 중 |
| 4 | Properties 표시 개선 (복수형, 콤마, 날짜) | 하 |
| 6 | 파일명 잘림 시 확장자 보존 | 하 |

### P1 (빠른 개선 - 쉽게 수정 가능)
| # | 항목 | 난이도 |
|---|------|--------|
| 3 | 커맨드바 `?:Help` → `?:Keys` | 최하 |
| 7 | 패널 날짜 포맷 정렬 | 하 |
| 8 | 파일 크기 단위 정렬 | 하 |
| 10 | 빈 디렉토리 안내 | 최하 |
| 11 | 다이얼로그 버튼 언어 통일 | 하 |

### P2 (후순위 개선)
| # | 항목 | 난이도 |
|---|------|--------|
| 5 | 한글 입력 상태 감지/안내 | 중 |
| 9 | 도움말 스크롤바 | 하 |
| 12 | 진행률 속도/ETA | 중 |
| 13 | 아이콘 호환성 | 중 |
| 14 | 상태바 반응형 | 하 |
| 15 | 다이얼로그 여백 상수화 | 최하 |
| 16 | 단축키 표기 통일 | 하 |
