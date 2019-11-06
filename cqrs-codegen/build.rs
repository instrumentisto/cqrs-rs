use std::{
    env,
    error::Error,
    io::Write as _,
    fs::{self, OpenOptions},
    path::Path,
    process,
};

fn main() -> Result<(), Box<dyn Error>> {
    let watt = env::var("CARGO_FEATURE_WATT");
    let no_watt = env::var("CARGO_FEATURE_NO_WATT");

    if watt.is_ok() && no_watt.is_ok() {
        panic!(
            "Invalid configuration: both 'watt' and 'no-watt' features specified; \
            exactly one of the two features have to be specified"
        );
    } else if watt.is_err() && no_watt.is_err() {
        panic!(
            "Invalid configuration: neither 'watt', nor 'no-watt' feature specified; \
            exactly one of the two features have to be specified"
        );
    }

    if no_watt.is_ok() {
        return Ok(());
    }

    let codegen_wasm_exists = Path::new("./src/codegen.wasm").is_file();
    let impl_exists = Path::new("./impl").is_dir();

    if !codegen_wasm_exists && !impl_exists {
        panic!(
            "Neither './src/codegen.wasm' file, nor './impl/' directory exist, \
            so it's impossible to build cqrs-codegen with 'watt' feature"
        );
    }

    let watch = Path::new("./.watch-cqrs-codegen-impl").exists();

    if watch {
        if impl_exists {
            rerun_if_changed_recursive_with_exceptions("./src", &["codegen.wasm"])?;
            // rerun_if_changed("./Cargo.lock");
            // rerun_if_changed("./Cargo.toml");

            rerun_if_changed_recursive("./impl/src")?;
            // rerun_if_changed("./impl/Cargo.lock");
            // rerun_if_changed("./impl/Cargo.toml");
        } else {
            println!(
                "cargo:warning='./.watch-cqrs-codegen-impl' file exists, but \
                './impl/' directory doesn't; './src/codegen.wasm' won't be rebuilt"
            );
        }
    }

    if (!codegen_wasm_exists || watch) && impl_exists {
        let root = env::current_dir()?;

        env::set_current_dir("./impl")?;

        fs::copy("./Cargo.toml", "./Cargo.toml~")?;
        writeln!(OpenOptions::new().append(true).open("./Cargo.toml")?, "[workspace]")?;

        let status = process::Command::new(env::var("CARGO")?)
            .args(vec![
                "build",
                "--release",
                "--target", "wasm32-unknown-unknown",
                "--features", "watt",
                "--target-dir", "target",
            ])
            .status()?;

        if !status.success() {
            panic!("cargo-build for cqrs-codegen-impl returned non-zero status code");
        }

        fs::copy("./Cargo.toml~", "./Cargo.toml")?;
        drop(fs::remove_file("./Cargo.toml~")); // result is explicitly ignored

        env::set_current_dir(root)?;

        fs::copy(
            "./impl/target/wasm32-unknown-unknown/release/cqrs_codegen_impl.wasm",
            "./src/codegen.wasm",
        )?;
    }

    Ok(())
}

fn rerun_if_changed(path: &str) {
    println!("cargo:rerun-if-changed={}", path);
}

fn rerun_if_changed_recursive<P>(path: P) -> Result<(), Box<dyn Error>>
where P: AsRef<Path>
{
    rerun_if_changed_recursive_with_exceptions(path, &[])
}

fn rerun_if_changed_recursive_with_exceptions<P>(
    path: P,
    exceptions: &[&str]
) -> Result<(), Box<dyn Error>>
where P: AsRef<Path>
{
    for path in fs::read_dir(path)? {
        let path = path?;

        if exceptions.iter().any(|&exception| path.file_name() == exception) {
            continue;
        }

        let path_type = path.file_type()?;

        let path = path.path();

        if path_type.is_dir() {
            rerun_if_changed_recursive(path)?;
        } else if path_type.is_file() {
            rerun_if_changed(path.to_str().ok_or("Failed to convert PathBuf to &str")?);
        }
    }

    Ok(())
}
