---
name: bk-release-notes-updater
description: Update RELEASE_NOTES.md for this project from current branch or commit range. Use when user asks to write, revise, simplify, or restructure release notes (for example 기능/버그 수정 중심으로 정리).
---

## 릴리즈 노트 업데이트

다음 순서로 `RELEASE_NOTES.md`를 갱신한다.

### 1) 기준 범위 확인
- 사용자 요청의 기준 버전/브랜치를 우선 사용한다.
- 기준이 없으면 최신 태그부터 `HEAD`까지를 기본 범위로 사용한다.
- 확인 명령:
```bash
git branch --show-current
git tag --sort=-creatordate | head -n 20
git log --oneline <base>..HEAD
git diff --name-only <base>..HEAD
```

### 2) 변경 분류
- 사용자 관점으로 짧게 정리한다.
- 기본 섹션은 `기능`, `버그 수정`을 사용한다.
- `기타`/`문서`/`개선`은 사용자가 요청할 때만 추가한다.
- 하나의 기능 흐름은 bullet 1개로 묶는다.

### 3) 문구 스타일
- bullet은 간결하게 한 줄 중심으로 작성한다.
- 내부 구현 세부사항(타입명, 내부 리팩토링 과정)은 생략한다.
- 요청이 "간단히"인 경우 결과 중심 문장으로 작성한다.
- 예시:
  - `:` 단축키로 명령 실행 팝업을 열고, 활성 패널 경로 기준으로 쉘 명령 실행 지원

### 4) 파일 반영
- 기존 `RELEASE_NOTES.md` 형식을 유지한다.
- 최신 버전 섹션을 상단에 추가하거나 기존 섹션을 수정한다.
- 과거 버전 내용은 요청이 없으면 수정하지 않는다.

### 5) 검증
- 반영 후 상단을 확인한다.
```bash
sed -n '1,220p' RELEASE_NOTES.md
```
- 사용자 요청 톤(간단/상세), 섹션 구조, bullet 개수를 점검한다.
