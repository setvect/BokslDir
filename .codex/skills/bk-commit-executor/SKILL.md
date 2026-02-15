---
name: bk-commit-executor
description: Execute safe git commits end-to-end including staging, pre-checks (status, staged scope, basic quality gates), and non-interactive commit commands. Use when user asks to actually run commit (e.g. "지금 커밋해줘", "변경사항 스테이징하고 커밋해줘").
allowed-tools: Bash(git status *), Bash(git diff *), Bash(git add *), Bash(git commit *)
---

## 안전 커밋 실행

의도한 변경만 커밋되도록 확인 후 비대화형으로 커밋한다.

### 1. 변경사항 확인
```bash
git status --short
git diff --stat
```
- 변경 파일이 없으면 작업을 종료한다.
- 의도와 무관한 파일이 보이면 스테이징 전에 사용자 확인을 받는다.

### 2. 스테이징
기본은 전체 변경을 스테이징한다.
```bash
git add -A
```
- 특정 파일만 커밋 요청이 있으면 해당 파일만 `git add <path>`로 스테이징한다.

### 3. 스테이징 검증
```bash
git status --short
git diff --cached --stat
```
- staged가 비어 있으면 커밋을 중단하고 사용자에게 알린다.
- 의도와 무관한 파일이 staged면 커밋을 멈추고 확인한다.

### 4. 품질 게이트(프로젝트 맞춤)
기본 순서:
```bash
source "$HOME/.cargo/env" && cargo test
```
- 요청이 있거나 변경 영향이 클 때 추가 실행:
```bash
cargo fmt
cargo clippy --all-targets --all-features
```

### 5. 커밋 실행
- 커밋 메시지는 `commit-message` 스킬 결과를 우선 사용한다.
- 커밋은 항상 비대화형으로 실행한다.
```bash
git commit -m "<type>(<scope>): <subject>"
```

### 6. 결과 보고
- 커밋 해시(짧은 SHA), 제목, 포함 파일 수를 요약한다.
- 품질 게이트를 생략했다면 무엇을 생략했는지 명시한다.
