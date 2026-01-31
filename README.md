# 복슬Dir (Boksl Dir)

Rust로 작성된 터미널 기반 듀얼 패널 파일 매니저

## 프로젝트 상태

🚧 **현재 Phase 1 개발 중**

이 프로젝트는 개발 초기 단계입니다. Mdir과 Total Commander에 영감을 받아 제작 중입니다.

## 주요 기능 (계획)

- 듀얼 패널 인터페이스
- 반응형 레이아웃
- 테마 지원
- 파일 작업 (복사, 이동, 삭제)
- 빠른 탐색 및 검색

## 요구사항

- Rust 1.93+ (2021 edition)
- Unicode 및 컬러를 지원하는 터미널

## 설치 및 실행

```bash
# 저장소 클론
git clone https://github.com/setvect/BokslDir.git
cd boksldir

# 개발 모드 실행
cargo run

# 릴리스 빌드
cargo build --release
./target/release/boksldir
```

## 개발

### 코드 품질 도구

```bash
cargo fmt       # 코드 포맷팅
cargo clippy    # 린트 검사 (ESLint와 유사)
cargo check     # 빠른 컴파일 검사
cargo test      # 테스트 실행
```

자세한 개발 가이드는 다음 문서를 참고하세요:
- [Requirements](docs/Requirements.md) - 요구사항
- [PRD](docs/PRD.md) - 기능 명세
- [Architecture](docs/Architecture.md) - 시스템 아키텍처

## 라이센스

MIT
