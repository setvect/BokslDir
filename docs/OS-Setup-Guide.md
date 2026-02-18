# 복슬Dir OS별 셋업 가이드

이 문서는 **`boksldir` 실행 파일이 이미 있는 상태**를 기준으로 설명합니다.

## 1) PATH 추가 방법

### macOS (zsh)

```bash
# 예시: 실행 파일을 ~/bin 에 둔 경우
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

확인:

```bash
which boksldir
```

### Linux (bash/zsh)

```bash
# 예시: 실행 파일을 ~/.local/bin 에 둔 경우
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

zsh 사용 시:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

확인:

```bash
which boksldir
```

### Windows (PowerShell)

현재 사용자 PATH에 실행 파일 폴더 추가:

```powershell
$bin = "$env:USERPROFILE\bin"
[Environment]::SetEnvironmentVariable(
  "Path",
  $env:Path + ";" + $bin,
  "User"
)
```

새 터미널에서 확인:

```powershell
Get-Command boksldir.exe
```

## 2) 실행 아규먼트 활용 방법

복슬Dir은 **첫 번째 인자 1개만 사용**합니다.

```bash
boksldir
boksldir .
boksldir /tmp
```

동작 규칙:
- 인자 없음: 이전 종료 시점 히스토리 경로 복원
- 인자가 유효한 디렉토리: 좌/우 패널 모두 해당 경로로 시작
- 인자가 유효하지 않음: 인자 없음과 동일하게 동작
- 두 번째 이후 인자: 무시됨

## 3) 환경설정 파일 저장 경로

기본 저장 경로:
- macOS/Linux: `~/.boksldir/settings.toml`
- Windows: `%USERPROFILE%\.boksldir\settings.toml` (HOME 기준)

설정 파일 경로를 직접 지정하려면 환경변수 `BOKSLDIR_SETTINGS_FILE` 사용:

### macOS/Linux

```bash
export BOKSLDIR_SETTINGS_FILE="$HOME/.config/boksldir/settings.toml"
boksldir
```

### Windows (PowerShell)

```powershell
$env:BOKSLDIR_SETTINGS_FILE="$env:USERPROFILE\.config\boksldir\settings.toml"
boksldir.exe
```

