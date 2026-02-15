---
name: bk-general-code-review
description: Perform pragmatic code review focused on correctness, regression risk, maintainability, and refactoring opportunities. Use when user asks for review/review comments or wants guidance on deduplication, replacing custom code with major libraries, simplifying complexity, strengthening tests, or checking architecture consistency.
allowed-tools: Bash(git status *), Bash(git diff *), Bash(rg *), Bash(cargo test *), Bash(cargo clippy *)
---

## 일반 코드리뷰 실행

요청이 "리뷰"이면 요약보다 이슈를 우선한다. 구현자 시점이 아니라 검토자 시점으로 본다.

### 1. 리뷰 범위 확정
- 우선순위: 사용자 요청 범위, staged diff, unstaged diff 순으로 기준을 명확히 한다.
- 범위가 불명확하면 변경 파일 목록과 핵심 수정 의도를 먼저 요약하고 그 기준으로 리뷰한다.

### 2. 필수 점검 항목 (10개)
- 정확성/회귀: 동작 변화, 엣지케이스 누락, 상태 전이 오류 가능성
- 아키텍처 일관성: 프로젝트 불변조건/레이어 경계 위반 여부
- 테스트 품질: 변경을 보호하는 테스트 존재 여부, 회귀 테스트 누락
- 성능/복잡도: 핫패스 비용, 불필요한 복사/할당, 비효율 반복
- 안전성: 에러 처리 일관성, panic 가능성, 자원/파일 처리 안전성
- 가독성/유지보수성: 함수 책임, 네이밍, 모듈 응집도, dead code
- UX/제품 일관성: 키바인딩/도움말/커맨드바 동기화, 사용자 흐름 일관성
- 국제화/문자 처리: UTF-8, 한글/IME, 바이트 인덱스 vs 문자 인덱스
- 의존성 정책: 자체 구현보다 메이저 라이브러리 사용 타당성, 라이선스/유지보수성
- 변경 범위 통제: 요청 범위 밖 리팩토링 혼입, 불필요한 구조 변경

### 3. 리팩토링 관점 필수 규칙
- 중복 제거: 유사 로직은 공통 함수/타입/모듈화 가능한지 검토한다.
- 라이브러리 우선: 검증된 메이저 라이브러리로 대체 가능한 자체 구현을 식별한다.
- 복잡도 단순화: 과도한 분기/중첩/긴 함수는 책임 분리와 데이터 흐름 단순화안을 제시한다.

### 4. 저장소 특화 체크포인트 (BokslDir)
- 액션/단축키 시스템은 `src/core/actions.rs` 단일 진실 원천을 유지하는지 확인한다.
- 패널 인덱스 모델(`selected_index`, `entries`, `selected_items`, `has_parent` 오프셋) 위반 여부를 확인한다.
- 레이아웃 규칙(DualPanel/TooSmall)과 현재 구현 제약을 깨지 않는지 확인한다.
- UTF-8/한글 처리와 `unicode-width` 기반 폭 계산을 깨지 않는지 확인한다.

### 5. 심각도 기준
- High: 잘못된 결과, 데이터 손실, 충돌, 명확한 회귀 위험
- Medium: 유지보수 비용 증가, 잠재 버그, 테스트 공백
- Low: 개선 권장 사항, 스타일/가독성 정리

### 6. 출력 형식
- Findings를 심각도 높은 순서로 제시한다.
- 각 Finding에 다음을 포함한다.
  - 위치: `path:line`
  - 문제: 무엇이 왜 문제인지
  - 영향: 사용자/동작/유지보수에 미치는 영향
  - 제안: 최소 변경으로 해결하는 구체적 수정안
- 이슈가 없으면 "No findings"를 명시하고, 잔여 리스크(테스트 미실행 등)를 남긴다.

### 7. 리뷰 중 금지
- 사용자 요청이 없으면 코드 수정/리팩토링을 수행하지 않는다.
- 근거 없는 취향성 지적보다 동작/리스크 중심 지적을 우선한다.
