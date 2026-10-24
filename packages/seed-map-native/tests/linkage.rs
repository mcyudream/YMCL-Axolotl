use axolotl_seed_map_native::axolotl_seed_map_java_version;

#[test]
fn downstream_crates_link_the_native_archive() {
    assert!(unsafe { axolotl_seed_map_java_version(12103) } > 0);
}
