---
name: bk-commit-message
description: Generate conventional commit messages from staged or unstaged git changes. Use when user asks to create/improve commit text (e.g. "커밋 메시지 만들어줘", "컨벤셔널 커밋으로 써줘").
allowed-tools: Bash(git status *), Bash(git diff *)
---

## 커밋 메시지 생성

Git 변경사항을 분석해 Conventional Commits 형식의 메시지를 만든다.

### 1. 변경사항 수집
```bash
git status --short
git diff --cached
```
- staged diff가 비어 있으면 `git diff`를 본다.
- staged와 unstaged가 섞여 있으면 기준(커밋 대상)을 먼저 명확히 안내한다.

### 2. 타입/스코프 결정
- 타입은 `feat|fix|refactor|docs|test|chore` 중 하나를 선택한다.
- 스코프는 경로/모듈이 명확할 때만 사용한다. 애매하면 생략한다.

### 3. 메시지 작성 규칙
- 제목(subject)은 72자 이내, 명령형, 마침표 없이 작성한다.
- 기본 형식: `<type>(<scope>): <subject>`
- 필요할 때만 본문 bullet을 추가한다:
  - 왜 바꿨는지
  - 사용자 영향/리스크
  - 마이그레이션 필요 여부

### 4. 결과 제시 형식
- 기본 1개를 제안하고, 애매하면 대안 1개를 추가한다.
- 변경이 여러 의도로 섞였으면 "커밋 분리"를 먼저 제안한다.

