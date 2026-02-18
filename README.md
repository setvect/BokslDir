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

# 시작 경로 지정 (첫 번째 인자만 사용)
./target/release/boksldir .
./target/release/boksldir /tmp
```

시작 인자 동작:
- 인자 없음: 이전 종료 시점 히스토리 경로 복원
- `.` 또는 유효한 디렉토리: 좌/우 패널 모두 해당 경로로 시작
- 유효하지 않은 경로: 인자 없음과 동일하게 동작

## 배포/패키징 (OS별)

공통: 먼저 릴리스 바이너리를 생성합니다.

```bash
cargo build --release
```

빌드 결과물:
- `target/release/boksldir` (Linux/macOS)
- `target/release/boksldir.exe` (Windows)

### macOS

가장 단순한 배포 형태는 `.tar.gz`입니다.

```bash
mkdir -p dist/boksldir-macos
cp target/release/boksldir dist/boksldir-macos/
tar -czf dist/boksldir-macos.tar.gz -C dist boksldir-macos
```

배포 정책이 필요한 경우:
- Apple Developer 인증서로 `codesign`
- 외부 배포 시 `notarize`/`staple` 적용

### Linux

기본 배포는 `.tar.gz` 또는 `.zip`을 권장합니다.

```bash
mkdir -p dist/boksldir-linux
cp target/release/boksldir dist/boksldir-linux/
tar -czf dist/boksldir-linux.tar.gz -C dist boksldir-linux
```

배포판 패키지 생성 예시:
- Debian/Ubuntu: `cargo-deb`
- RHEL/Fedora: `cargo-generate-rpm`

```bash
cargo install cargo-deb cargo-generate-rpm
cargo deb
cargo generate-rpm
```

### Windows

기본 배포는 `.zip`이 가장 간단합니다.

```powershell
New-Item -ItemType Directory -Force dist\boksldir-windows | Out-Null
Copy-Item target\release\boksldir.exe dist\boksldir-windows\
Compress-Archive -Path dist\boksldir-windows\* -DestinationPath dist\boksldir-windows.zip -Force
```

설치형 배포판(MSI) 예시:
- WiX Toolset + `cargo-wix` 사용

```powershell
cargo install cargo-wix
cargo wix
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
