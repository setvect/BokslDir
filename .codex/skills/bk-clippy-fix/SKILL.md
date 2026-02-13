---
name: bk-clippy-fix
description: Run clippy lint check and fix warnings
allowed-tools: Bash(cargo *), Bash(source *)
---

## Clippy 린트 검사 및 수정

1. Cargo 환경 설정 및 Clippy 실행:
   ```bash
   source "$HOME/.cargo/env" && cargo clippy
   ```

2. Clippy 경고 분석:
   - 각 경고의 원인과 권장 수정 방법 설명
   - 코드를 수정하여 경고 해결

3. 수정 후 다시 Clippy 실행하여 확인

4. 모든 경고가 해결될 때까지 반복

5. 최종적으로 코드 포맷팅:
   ```bash
   cargo fmt
   ```
