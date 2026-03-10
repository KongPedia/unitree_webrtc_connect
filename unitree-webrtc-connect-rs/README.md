# unitree_webrtc_connect_rs

Python-to-Rust native binding for unitree WebRTC connection. This replaces the performance-heavy `aiortc` Python implementation with Zero-Copy Rust.

## 터미널(IDE)에서의 Conda 강제 주입 문제 원인 및 해결

`printenv | grep CONDA`나 `printenv | grep VIRTUAL_ENV` 시, 사용자의 `~/.zshrc`, `~/.zprofile`, `/etc/zshrc` 등 **모든 시스템 쉘 프로파일을 전부 뒤져본 결과, Conda 초기화 코드는 완전히 제거되었고 더 이상 존재하지 않습니다.**

그럼에도 불구하고 새 터미널을 열었을 때 `CONDA_*` 변수가 나오는 이유는 100% **현재 사용 중인 IDE(Cursor/VS Code) 자체의 설정(Extension) 캐싱 때문**입니다.

1. **원인 1**: 사용하는 에디터(Cursor) 내장 터미널 자체가 켜질 때 부모 프로세스(이전에 켜뒀던 에디터 메모리)의 `CONDA_PREFIX`를 계속 상속받아 복사하고 있습니다.
2. **원인 2**: 에디터의 `Python` Extension 설정 중 `python.terminal.activateEnvironment` 옵션이 켜져 있으면, 터미널을 열 때마다 백그라운드에서 강제로 Conda 환경변수를 주입합니다.

결론적으로 OS의 `bashrc`나 `zshrc` 파일에는 범인이 없으며, 에디터가 터미널을 열 때마다 매번 주입하는 것입니다.

### 영구 해결법 (명령어 사용)

IDE 설정을 끄지 않더라도 편하게 사용할 수 있도록 `.zshrc`에 임시 안전장치(Alias)를 방금 추가해두었습니다.
터미널에서 명령어 치듯이 아래와 같이 실행하시면, Conda의 간섭을 터미널에서 모두 끊어내고 `.venv` 가상환경만 클린하게 물고 빌드합니다.

```bash
# 앞으로 Rust 패키지를 테스트할 때는 이 명령어를 사용하세요.
test_cargo
```

이 `test_cargo` 명령어는 뒤에서 `conda deactivate 2>/dev/null; unset CONDA_PREFIX CONDA_DEFAULT_ENV; source .venv/bin/activate; cargo test`를 한 큐에 실행하도록 만들어 두었습니다.

### 수동으로 실행하고 싶을 때

```bash
# 1. 꼬인 Conda 환경 무시 (IDE가 주입한 것을 삭제)
conda deactivate
unset CONDA_PREFIX
unset CONDA_DEFAULT_ENV

# 2. 파이썬 가상환경 (.venv) 단독 활성화
source .venv/bin/activate

# 3. Rust 순수 모듈 테스트
cargo check
cargo test

# 4. 파이썬 C-extension(maturin) 통합 테스트
maturin develop
python tests/test_lidar.py
```
