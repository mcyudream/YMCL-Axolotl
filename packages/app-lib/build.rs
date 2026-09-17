use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::{env, fs};

/// Build-time opt-in to a private launcher data directory. Read by
/// `theseus::brand::app_data_dir_identifier`, which appends it to the directory
/// name.
const DATA_DIR_SUFFIX_VAR: &str = "AXOLOTL_DATA_DIR_SUFFIX";

fn main() {
    println!("cargo::rerun-if-changed=.env");
    println!("cargo::rerun-if-env-changed=CURSEFORGE_API_KEY");
    println!("cargo::rerun-if-changed=java/gradle");
    println!("cargo::rerun-if-changed=java/src");
    println!("cargo::rerun-if-changed=java/build.gradle.kts");
    println!("cargo::rerun-if-changed=java/settings.gradle.kts");
    println!("cargo::rerun-if-changed=java/gradle.properties");

    #[cfg(target_os = "windows")]
    if env::var_os("CARGO_FEATURE_TAURI").is_some() {
        println!(
            "cargo::rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }

    set_env();
    build_java_jars();
}

fn set_env() {
    let curseforge_api_key = env::var("CURSEFORGE_API_KEY")
        .ok()
        .or_else(|| read_dotenv_literal("CURSEFORGE_API_KEY"));

    for (var_name, var_value) in
        dotenvy::dotenv_iter().into_iter().flatten().flatten()
    {
        if var_name == "DATABASE_URL"
            || var_name == "CURSEFORGE_API_KEY"
            || var_name == DATA_DIR_SUFFIX_VAR
        {
            // Handled explicitly below, where an empty value can be rejected
            // instead of baked into the crate.
            continue;
        }

        println!("cargo::rustc-env={var_name}={var_value}");

        // Cargo's dep-info env check compares the value rustc baked in
        // against the value visible in this process (including .cargo's
        // [env] section). When the two differ, every build re-runs this
        // crate from scratch, so surface the conflict instead of silently
        // losing the cache.
        match env::var_os(&var_name) {
            Some(current) if current != OsString::from(&var_value) => {
                println!(
                    "cargo::warning={var_name} differs between .env and the process/`[env]` value; cargo will consider theseus dirty on every build"
                );
            }
            _ => {}
        }
    }

    if let Some(curseforge_api_key) = curseforge_api_key {
        println!("cargo::rustc-env=CURSEFORGE_API_KEY={curseforge_api_key}");
    }

    // Lets a local or test build keep its own data directory, so it cannot write
    // the database an installed launcher is using. Releases leave it unset and
    // resolve to the plain identifier.
    println!("cargo::rerun-if-env-changed={DATA_DIR_SUFFIX_VAR}");
    let data_dir_suffix = env::var(DATA_DIR_SUFFIX_VAR)
        .ok()
        .filter(|suffix| !suffix.is_empty())
        .or_else(|| {
            read_dotenv_literal(DATA_DIR_SUFFIX_VAR)
                .filter(|suffix| !suffix.is_empty())
        });

    if let Some(data_dir_suffix) = data_dir_suffix {
        // brand::data_dir_identifier drops every character that is not ASCII
        // alphanumeric or `-_.` and then trims dots and dashes off both ends, so
        // a suffix holding none of what survives sanitizes to nothing and the
        // build would fall back to the installed launcher's own data directory -
        // exactly the state this variable exists to avoid. Stop the build
        // instead of letting that happen silently.
        if !data_dir_suffix.chars().any(|character| {
            character.is_ascii_alphanumeric() || character == '_'
        }) {
            println!(
                "cargo::error={DATA_DIR_SUFFIX_VAR} leaves no usable directory name, so this build would use the installed launcher's data directory"
            );
            exit(1);
        }

        println!("cargo::rustc-env={DATA_DIR_SUFFIX_VAR}={data_dir_suffix}");
    }
}

fn read_dotenv_literal(name: &str) -> Option<String> {
    let contents = fs::read_to_string(".env").ok()?;

    contents.lines().find_map(|line| {
        let line = line.trim_start().strip_prefix("export ").unwrap_or(line);
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim() != name {
            return None;
        }

        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('\'') && value.ends_with('\''))
                || (value.starts_with('"') && value.ends_with('"')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };

        (!value.is_empty()).then(|| value.to_string())
    })
}

fn build_java_jars() {
    let out_dir =
        dunce::canonicalize(PathBuf::from(env::var_os("OUT_DIR").unwrap()))
            .unwrap();

    println!(
        "cargo::rustc-env=JAVA_JARS_DIR={}",
        out_dir.join("java/libs").display()
    );

    // Gradle dominates build-script reruns that have nothing to do with Java
    // (e.g. a .env edit), so keep the existing jars when every watched Java
    // input is older than what OUT_DIR already holds.
    if java_jars_up_to_date(&out_dir.join("java/libs")) {
        return;
    }

    let gradle_path = fs::canonicalize(
        #[cfg(target_os = "windows")]
        "java\\gradlew.bat",
        #[cfg(not(target_os = "windows"))]
        "java/gradlew",
    )
    .unwrap();

    let mut build_dir_str = OsString::from("-Dorg.gradle.project.buildDir=");
    build_dir_str.push(out_dir.join("java"));
    let exit_status = Command::new(gradle_path)
        .arg(build_dir_str)
        .arg("build")
        .arg("--no-daemon")
        .arg("--console=rich")
        .current_dir(dunce::canonicalize("java").unwrap())
        .status()
        .expect("Failed to wait on Gradle build");

    if !exit_status.success() {
        println!("cargo::error=Gradle build failed with {exit_status}");
        exit(exit_status.code().unwrap_or(1));
    }
}

fn java_jars_up_to_date(libs_dir: &Path) -> bool {
    const JAVA_INPUTS: &[&str] = &[
        "java/src",
        "java/gradle",
        "java/build.gradle.kts",
        "java/settings.gradle.kts",
        "java/gradle.properties",
    ];

    let mut newest_input: Option<std::time::SystemTime> = None;
    for input in JAVA_INPUTS {
        // An unreadable input falls back to running Gradle rather than
        // silently reusing stale jars.
        let Some(modified) = newest_mtime(Path::new(input)) else {
            return false;
        };
        newest_input = Some(match newest_input {
            Some(current) => current.max(modified),
            None => modified,
        });
    }
    let newest_input = newest_input.unwrap();

    let Ok(entries) = fs::read_dir(libs_dir) else {
        return false;
    };
    let mut jar_count = 0;
    for entry in entries.flatten() {
        if entry.path().extension().is_some_and(|ext| ext == "jar") {
            jar_count += 1;
            let modified =
                match entry.metadata().and_then(|meta| meta.modified()) {
                    Ok(modified) => modified,
                    Err(_) => return false,
                };
            if modified < newest_input {
                return false;
            }
        }
    }
    jar_count > 0
}

fn newest_mtime(path: &Path) -> Option<std::time::SystemTime> {
    let meta = fs::metadata(path).ok()?;
    let mut newest = meta.modified().ok()?;
    if meta.is_dir() {
        for entry in fs::read_dir(path).ok()? {
            newest = newest.max(newest_mtime(&entry.ok()?.path())?);
        }
    }
    Some(newest)
}
