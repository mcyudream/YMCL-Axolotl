fn main() {
    let cubiomes_dir = std::path::Path::new("../../apps/app/vendor/cubiomes");
    assert!(
        cubiomes_dir.join("generator.c").is_file(),
        "Cubiomes submodule is missing. Run git submodule update --init --recursive."
    );

    let sources = [
        "biomenoise.c",
        "biomes.c",
        "finders.c",
        "generator.c",
        "layers.c",
        "noise.c",
        "quadbase.c",
        "util.c",
    ];
    cc::Build::new()
        .include(cubiomes_dir)
        .include(cubiomes_dir.join("tables"))
        .file("src/cubiomes_bridge.c")
        .files(sources.map(|source| cubiomes_dir.join(source)))
        .opt_level(3)
        .flag_if_supported("-fwrapv")
        .warnings(false)
        .compile("axolotl-cubiomes");

    println!("cargo:rerun-if-changed=src/cubiomes_bridge.c");
    println!("cargo:rerun-if-changed=src/cubiomes_bridge.h");
    println!("cargo:rerun-if-changed={}", cubiomes_dir.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        println!("cargo:rustc-link-lib=m");
    }
}
