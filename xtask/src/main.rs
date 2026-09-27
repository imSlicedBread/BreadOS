use std::{
    env,
    path::PathBuf,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let command = env::args().nth(1).unwrap_or_else(|| "help".into());
    let result = match command.as_str() {
        "check-env" => check_env(&root),
        "build" => build(&root),
        "image" => image(&root),
        "smoke" => script(&root, "smoke.ps1"),
        "run" => script(&root, "run.ps1"),
        "help" | "--help" | "-h" => {
            help();
            Ok(())
        }
        _ => {
            help();
            Err(format!("unknown command: {command}"))
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("BreadOS xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn help() {
    println!(
        "BreadOS v001 commands:\n  cargo xtask check-env\n  cargo xtask build\n  cargo xtask image\n  cargo xtask smoke\n  cargo xtask run"
    );
}

fn check_env(root: &PathBuf) -> Result<(), String> {
    println!("workspace: {}", root.display());
    println!("required Rust targets: x86_64-unknown-uefi, x86_64-unknown-none");
    let status = Command::new("rustup")
        .args([
            "target",
            "list",
            "--installed",
            "--toolchain",
            "1.95.0-x86_64-pc-windows-msvc",
        ])
        .status()
        .map_err(|e| format!("rustup unavailable: {e}"))?;
    if !status.success() {
        return Err("rustup target query failed".into());
    }
    report_version("python", "python");
    if let Some(qemu) = qemu_path() {
        report_version("qemu-system-x86_64", qemu.to_string_lossy().as_ref());
    } else {
        println!(
            "qemu-system-x86_64: not found (build/image can still work; guest run needs QEMU)"
        );
    }
    Ok(())
}

fn report_version(label: &str, program: &str) {
    match Command::new(program).arg("--version").output() {
        Ok(o) if o.status.success() => println!(
            "{label}: {}",
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("available")
        ),
        _ => println!("{label}: not found"),
    }
}

fn qemu_path() -> Option<PathBuf> {
    if let Some(configured) = env::var_os("BREADOS_QEMU") {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }
    for directory in env::split_paths(&env::var_os("PATH")?) {
        let path = directory.join("qemu-system-x86_64.exe");
        if path.is_file() {
            return Some(path);
        }
    }
    let installed = PathBuf::from(r"C:\Program Files\qemu\qemu-system-x86_64.exe");
    installed.is_file().then_some(installed)
}

fn build(root: &PathBuf) -> Result<(), String> {
    for (package, target) in [
        ("breados-loader", "x86_64-unknown-uefi"),
        ("breados-kernel", "x86_64-unknown-none"),
    ] {
        run(
            root,
            "cargo",
            &["build", "--release", "-p", package, "--target", target],
        )?;
    }
    Ok(())
}

fn image(root: &PathBuf) -> Result<(), String> {
    build(root)?;
    let efi = root.join("target/x86_64-unknown-uefi/release/breados-loader.efi");
    let kernel = root.join("target/x86_64-unknown-none/release/breados-kernel");
    let output = root.join("out/BreadOS-v001.img");
    run(
        root,
        "python",
        &[
            "tools/mkimage.py",
            "--efi",
            path_arg(&efi),
            "--kernel",
            path_arg(&kernel),
            "--output",
            path_arg(&output),
        ],
    )
}

fn script(root: &PathBuf, file: &str) -> Result<(), String> {
    let path = root.join("scripts").join(file);
    run(
        root,
        "powershell",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            path_arg(&path),
        ],
    )
}

fn path_arg(path: &PathBuf) -> &str {
    path.to_str().expect("UTF-8 Windows path")
}

fn run(root: &PathBuf, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| format!("could not start {program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}
