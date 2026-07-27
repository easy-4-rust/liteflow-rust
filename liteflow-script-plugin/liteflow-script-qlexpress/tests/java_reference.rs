//! 使用 Java QLExpress 4.1.0 运行相同语义的差分参考测试。

use std::path::PathBuf;
use std::process::Command;

/// 定位 LiteFlow v2.16.0 使用的 QLExpress 4.1.0 JAR。
fn qlexpress_jar() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("LITEFLOW_QLEXPRESS4_JAR") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let path = PathBuf::from(std::env::var_os("HOME")?)
        .join(".m2/repository/com/alibaba/qlexpress4/4.1.0/qlexpress4-4.1.0.jar");
    path.is_file().then_some(path)
}

/// 当本机具备 JDK 和基线 JAR 时，真实运行 Java 4.1.0 参考程序并锁定结果。
#[test]
fn java_qlexpress_4_1_reference_matches_expected_semantics() {
    let Some(jar) = qlexpress_jar() else {
        eprintln!("跳过 Java QLExpress 差分参考：未找到 4.1.0 JAR；可设置 LITEFLOW_QLEXPRESS4_JAR");
        return;
    };
    if Command::new("javac").arg("-version").output().is_err()
        || Command::new("java").arg("-version").output().is_err()
    {
        eprintln!("跳过 Java QLExpress 差分参考：当前环境没有可执行的 javac/java");
        return;
    }

    let output_dir = tempfile::tempdir().expect("应创建 Java 差分测试临时目录");
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java/QlExpressReference.java");
    let compile = Command::new("javac")
        .arg("-cp")
        .arg(&jar)
        .arg("-d")
        .arg(output_dir.path())
        .arg(&source)
        .output()
        .expect("应启动 javac");
    assert!(
        compile.status.success(),
        "Java 参考程序编译失败：{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let classpath =
        std::env::join_paths([output_dir.path(), jar.as_path()]).expect("应构造 Java classpath");
    let execution = Command::new("java")
        .arg("-cp")
        .arg(classpath)
        .arg("QlExpressReference")
        .output()
        .expect("应启动 Java QLExpress 参考程序");
    assert!(
        execution.status.success(),
        "Java QLExpress 参考程序执行失败：{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout).replace("\r\n", "\n"),
        "score=90\ndecision=true\nroute=pass\ncount=3\norderType=6\n"
    );
}
