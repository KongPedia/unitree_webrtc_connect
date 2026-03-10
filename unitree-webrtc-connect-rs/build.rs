fn main() {
    // pyo3의 기본 빌드 스크립트 실행
    pyo3_build_config::use_pyo3_cfgs();

    // macOS 환경의 경우 rpath (동적 라이브러리 경로)를 런타임에 바이너리 안에 밀어넣습니다.
    if cfg!(target_os = "macos") {
        // 활성화된 파이썬(가령 uv나 conda)의 구성 정보 파악
        if let Some(lib_dir) = pyo3_build_config::get().lib_dir.as_ref() {
            // 이 마법의 한 줄이 바이너리(테스트 실행 파일) 자체에 동적 라이브러리 경로를 영숫자로 각인시킵니다.
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir);
        }
    }
}
