use std::path::PathBuf;
use std::process::{Command, exit};
use std::{env, fs};

/// Build-time opt-in to a private launcher data directory. Read by
/// `theseus::brand::app_data_dir_identifier`, which appends it to the directory
/// name.
const DATA_DIR_SUFFIX_VAR: &str = "AXOLOTL_DATA_DIR_SUFFIX";

/// Public service defaults for downstream builds that have no `.env`.
/// Private Modrinth services remain disabled by Axolotl capabilities at runtime.
const MODRINTH_ENV_DEFAULTS: &[(&str, &str)] = &[
    ("MODRINTH_API_BASE_URL", "https://api.modrinth.com"),
    ("MODRINTH_API_URL", "https://api.modrinth.com/v2/"),
    ("MODRINTH_API_URL_V3", "https://api.modrinth.com/v3/"),
    (
        "MODRINTH_LAUNCHER_META_URL",
        "https://launcher-meta.modrinth.com/",
    ),
    ("MODRINTH_URL", "https://modrinth.com/"),
    ("MODRINTH_SOCKET_URL", "wss://disabled.invalid/"),
];

fn main() {
    println!("cargo::rerun-if-changed=../../.nvmrc");
    println!(
        "cargo::rerun-if-changed=../../scripts/axolotl/prepare-build-resources.mjs"
    );
    println!("cargo::rerun-if-changed=java/gradlew");
    println!("cargo::rerun-if-changed=java/gradlew.bat");
    // Only watch .env when it exists. A missing rerun-if-changed path keeps
    // Cargo treating the crate as dirty on every invocation.
    if PathBuf::from(".env").exists() {
        println!("cargo::rerun-if-changed=.env");
    }
    println!("cargo::rerun-if-env-changed=CURSEFORGE_API_KEY");
    for (name, _) in MODRINTH_ENV_DEFAULTS {
        println!("cargo::rerun-if-env-changed={name}");
    }
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

    let dotenv_values: Vec<(String, String)> = dotenvy::dotenv_iter()
        .into_iter()
        .flatten()
        .flatten()
        .collect();

    for (var_name, var_value) in &dotenv_values {
        if var_name == "DATABASE_URL"
            || var_name == "CURSEFORGE_API_KEY"
            || var_name == DATA_DIR_SUFFIX_VAR
            || MODRINTH_ENV_DEFAULTS
                .iter()
                .any(|(name, _)| name == var_name)
        {
            // Handled explicitly below, where values are resolved with a
            // stable priority chain instead of being dumped as-is.
            continue;
        }

        println!("cargo::rustc-env={var_name}={var_value}");
    }

    // Single source of truth for env!() service URLs. Prefer local .env, then
    // an explicit process-env export, then public defaults. Always emit
    // rustc-env so Cargo does not mix process-env fingerprints with a
    // different baked value (which marked theseus dirty on every rebuild).
    for (name, default) in MODRINTH_ENV_DEFAULTS {
        let value = dotenv_values
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .or_else(|| env::var(name).ok().filter(|value| !value.is_empty()))
            .unwrap_or_else(|| (*default).to_string());
        println!("cargo::rustc-env={name}={value}");
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
    let status = Command::new("node")
        .args(["scripts/axolotl/prepare-build-resources.mjs", "java"])
        .current_dir("../..")
        .status()
        .expect("Node.js is required to prepare the Java agents");
    if !status.success() {
        println!("cargo::error=Java agent preparation failed with {status}");
        exit(status.code().unwrap_or(1));
    }

    let libs = dunce::canonicalize("java/build/libs")
        .expect("Java agent output directory is missing");
    println!("cargo::rustc-env=JAVA_JARS_DIR={}", libs.display());
    for jar in ["theseus.jar", "authlib-injector.jar"] {
        println!("cargo::rerun-if-changed=java/build/libs/{jar}");
    }
}
