---
name: bk-all
description: Run all quality checks (build, clippy, fmt, test) and fix issues
allowed-tools: Bash(cargo *), Bash(source *)
---

## 전체 품질 검사 및 수정

모든 검사를 순서대로 실행하고 문제 발견 시 수정합니다.

### 1. 빌드 검사 및 수정
```bash
source "$HOME/.cargo/env" && cargo build
```
- 에러가 있으면 원인 파악 후 수정
- 경고가 있으면 수정
- 수정 후 다시 빌드하여 확인

### 2. Clippy 린트 검사 및 수정
```bash
cargo clippy
```
- 린트 경고가 있으면 수정
- 수정 후 다시 검사하여 확인

### 3. 코드 포맷팅
```bash
cargo fmt
```

### 4. 테스트 실행
```bash
cargo test
```
- 실패한 테스트가 있으면 원인 분석 및 수정
- 수정 후 다시 테스트하여 확인

### 5. 최종 확인
모든 단계가 경고/에러 없이 통과할 때까지 반복합니다.

완료 시 결과 요약:
- 빌드: OK/수정됨
- Clippy: OK/수정됨
- 포맷팅: OK/수정됨
- 테스트: N개 통과
