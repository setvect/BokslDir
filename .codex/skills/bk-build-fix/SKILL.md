---
name: bk-build-fix
description: Build project and fix any warnings or errors
allowed-tools: Bash(cargo *), Bash(source *)
---

## Rust 빌드 후 경고/에러 수정

1. Cargo 환경 설정 및 빌드 실행:
   ```bash
   source "$HOME/.cargo/env" && cargo build
   ```

2. 빌드 결과 분석:
   - 에러가 있으면 원인을 파악하고 수정
   - 경고가 있으면 각 경고의 원인을 설명하고 수정

3. 수정 후 다시 빌드하여 문제가 해결되었는지 확인

4. 모든 경고와 에러가 해결될 때까지 반복
