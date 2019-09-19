use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Find a binary in `target/debug` or `target/release`.
fn get_bin(name: &str) -> PathBuf {
    let dir = {
        let mut test_executable = env::current_exe().unwrap();
        test_executable.pop();
        if test_executable.ends_with("deps") {
            test_executable.pop();
        }
        test_executable
    };

    dir.join(format!("{}{}", name, env::consts::EXE_SUFFIX))
}

#[test]
fn cli_smoke_test() {
    let status = Command::new(get_bin("xain-coordinator"))
        .args(&["fashion_mnist_100p_IID_balanced", "--clients=20", "--rounds=50"])
        .stdout(Stdio::null())
        .status()
        .unwrap();

    assert!(status.success());
}
