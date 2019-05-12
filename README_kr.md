# lc3dbg
## 설치
### 1. 컴파일된 실행 파일 사용
[Releases](https://github.com/cr0sh/lc3dbg/releases)에서 최신 릴리즈를 선택한 후, 아래의 빌드 아웃풋 목록에서 OS/아키텍쳐에 맞는 실행 파일을 다운로드하세요.

### 2. 직접 빌드

가장 좋은 방법은 컴파일러를 사용해 직접 빌드하는 것입니다.

#### 컴파일러 설치(`rustc`, `cargo`)
[Rust](https://www.rust-lang.org) 컴파일러가 필요합니다. [rustup](https://rustup.rs)를 사용하면 간단하게 컴파일러를 설치할 수 있습니다.
링크된 rustup 홈페이지에 들어가 자신의 OS에 맞는 인스톨러를 받으세요.

Linux/Mac: 다음 명령어를 터미널에 입력하세요.
```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Windows 64bit: 다음 파일을 실행하세요. https://win.rustup.rs/x86_64

Windows 32bit: 다음 파일을 실행하세요. https://win.rustup.rs/i686

세 개의 선택지를 묻는 창이 나온다면, 1을 입력해 기본값으로 설치하면 됩니다.

Linux에서는 이후 `source $HOME/.cargo/env` 를 입력하면, `cargo`와 `rustc` 명령어가 자동으로 추가됩니다.

컴파일러를 한 번 설치했다면 다음부터는 위의 과정을 반복할 필요가 없습니다.

#### `lc3dbg` 다운로드

터미널에 다음 명령어를 입력하면, 자동으로 필요한 라이브러리를 모두 다운로드하고 `lc3dbg`가 컴파일됩니다.

```shell
cargo install lc3dbg
```

이미 `lc3dbg`를 설치했고 새로운 버전으로 업데이트하려면, 
```shell
cargo install lc3dbg -f
```
명령어를 사용하면 됩니다.

## 사용법
```shell
lc3dbg file1.obj file2.obj (...)
```