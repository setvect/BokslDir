---
name: bk-quality-check
description: Complete code quality check (build, clippy, fmt, test)
allowed-tools: Bash(cargo *), Bash(source *)
---

## 전체 코드 품질 검사

순서대로 실행하고 문제가 있으면 수정:

### 1. 빌드 검사
```bash
source "$HOME/.cargo/env" && cargo build
```
- 에러/경고 발견 시 수정

### 2. Clippy 린트 검사
```bash
cargo clippy
```
- 린트 경고 발견 시 수정

### 3. 코드 포맷팅
```bash
cargo fmt
```

### 4. 테스트 실행
```bash
cargo test
```
- 실패한 테스트가 있으면 원인 분석 및 수정

### 5. 최종 확인
모든 단계가 경고/에러 없이 통과하면 완료 보고
