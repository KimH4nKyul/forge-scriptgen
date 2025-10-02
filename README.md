# forge-scriptgen

Solidity 컨트랙트 배포를 위한 Foundry 스크립트(`*.s.sol`)를 자동으로 생성하는 CLI 도구입니다. 프로젝트의 `src` 디렉터리에 존재하는 배포 가능한 컨트랙트를 탐색하고, 선택한 컨트랙트의 생성자 인자를 바탕으로 스크립트 파일을 만들어 줍니다.

## 기능 요구사항

- **컨트랙트 탐색**: Foundry 프로젝트 구조(`src/**/*.sol`)를 순회하며 배포 가능한 컨트랙트를 식별합니다.
- **컨트랙트 선택**: 컨트랙트명, 상대 경로, 파일명을 기준으로 원하는 컨트랙트를 지정할 수 있습니다.
- **생성자 인자 입력**:
  - `--args` 옵션에 JSON 배열을 전달해서 비대화식으로 입력
  - 옵션이 없으면 생성자 시그니처를 안내하고 터미널에서 직접 값을 입력받음
- **프라이빗 키 지정**: `--private-key` 옵션으로 프라이빗 키 리터럴을 제공하거나, 옵션이 없을 경우 CLI가 안전하게 입력을 요구합니다.
- **스크립트 생성**: `script/<ContractName>.s.sol` 형태의 파일을 생성하고, 기존 파일이 있을 경우 `--force` 옵션으로 덮어쓸 수 있습니다.
- **헬프 및 리스트 출력**: `--help`, `--list` 등의 옵션을 제공하여 사용 가능한 컨트랙트와 명령을 확인할 수 있습니다.

## 비기능 요구사항

- **로컬 실행**: macOS 및 Linux의 표준 Rust toolchain 위에서 동작합니다.
- **Foundry 친화성**: 생성된 스크립트는 `forge-std/Script.sol`을 기반으로 하며, 기존 Foundry 워크플로와 호환됩니다.
- **안전한 파일 처리**: 생성 대상 스크립트가 이미 존재하면 기본적으로 덮어쓰지 않으며, 명시적으로 `--force`를 준 경우에만 overwrite 합니다.
- **사용성**: CLI 사용법을 `--help`로 확인할 수 있고, 생성자 인자 및 프라이빗 키 입력 시 친절한 프롬프트를 제공합니다.
- **보안 유의**: 프라이빗 키는 생성된 스크립트에 그대로 기록되므로 VCS에 커밋하지 않도록 주의합니다.

## 설치 방법 (macOS / Linux)

### 1. Rust toolchain 설치

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. 저장소 준비

```bash
git clone https://github.com/kimh4nkyul/forge-scriptgen.git
cd forge-scriptgen
```

### 3. 바이너리 설치

- 전역 설치: 현재 디렉터리에서 바로 설치하여 `$HOME/.cargo/bin`에 바이너리를 추가합니다.

  ```bash
  cargo install --path .
  ```

- 또는 프로젝트 로컬에서 실행하려면 아래와 같이 릴리즈 바이너리를 빌드한 뒤 `target/release/forge-scriptgen`을 사용합니다.

  ```bash
  cargo build --release
  ./target/release/forge-scriptgen --help
  ```

## 사용 방법

프로젝트 루트(Foundry 프로젝트)에서 실행하면 현재 디렉터리를 기준으로 컨트랙트를 탐색합니다.

### 헬프 출력

```bash
forge-scriptgen --help
```

### 컨트랙트 목록 확인

```bash
forge-scriptgen --list
```

### 생성자 인자를 JSON으로 지정하여 스크립트 생성

```bash
forge-scriptgen --args '["0xDeAd", 42]' --private-key 0xabc123 Counter
```

### 인터랙티브 모드로 스크립트 생성

```bash
forge-scriptgen Counter
# 출력된 프롬프트에 따라 생성자 인자와 프라이빗 키 입력
```

### 프라이빗 키만 별도로 지정

```bash
forge-scriptgen --private-key 0xfeedface Counter
```

### 기존 스크립트 덮어쓰기

```bash
forge-scriptgen --force Counter
```

생성된 스크립트는 기본적으로 `script/<ContractName>.s.sol` 에 저장되며, `--output-dir` 옵션으로 경로를 변경할 수 있습니다. import 구문은 스크립트와 컨트랙트 간의 상대 경로를 자동으로 계산하여 작성됩니다.

## 개발 및 테스트

- 포맷팅: `cargo fmt`
- 단위/통합 테스트: `cargo test`

Foundry 프로젝트와 함께 사용할 때는 `forge test` 혹은 `forge script` 등의 표준 명령으로 이어서 배포 작업을 진행할 수 있습니다.
