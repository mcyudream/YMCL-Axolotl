//! Native seed-map bridge, built independently of Tauri assets and configuration.

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BridgeFeature {
    pub x: i32,
    pub z: i32,
    pub kind: u32,
    pub approximate: u8,
    pub end_ship: i8,
}

unsafe extern "C" {
    pub fn axolotl_seed_map_java_version(version: i32) -> i32;
    pub fn axolotl_seed_map_render(
        seed: u64,
        minecraft_version: i32,
        generator_flags: i32,
        dimension: i32,
        x: i32,
        z: i32,
        scale: i32,
        width: i32,
        height: i32,
        elevation: i32,
        terrain: i32,
        contours: i32,
        highlight_mask: *const u8,
        rgb: *mut u8,
        rgb_len: usize,
    ) -> i32;
    pub fn axolotl_seed_map_find_features(
        seed: u64,
        minecraft_version: i32,
        generator_flags: i32,
        dimension: i32,
        min_x: i32,
        min_z: i32,
        max_x: i32,
        max_z: i32,
        feature_mask: u32,
        out: *mut BridgeFeature,
        out_len: usize,
    ) -> usize;
    pub fn axolotl_seed_map_get_spawn(
        seed: u64,
        minecraft_version: i32,
        generator_flags: i32,
        x: *mut i32,
        z: *mut i32,
    ) -> i32;
    pub fn axolotl_seed_map_biome_at(
        seed: u64,
        minecraft_version: i32,
        generator_flags: i32,
        dimension: i32,
        x: i32,
        y: i32,
        z: i32,
    ) -> i32;
    pub fn axolotl_seed_map_surface_heights(
        seed: u64,
        minecraft_version: i32,
        generator_flags: i32,
        x: i32,
        z: i32,
        width: i32,
        height: i32,
        out: *mut f32,
    ) -> i32;
    pub fn axolotl_seed_map_scan_vein(
        seed: u64,
        chunk_x: i32,
        chunk_z: i32,
        vein_kind: i32,
        out_xyz: *mut i32,
        out_cap: usize,
    ) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_layout_matches_the_c_bridge() {
        assert_eq!(std::mem::size_of::<BridgeFeature>(), 16);
        assert_eq!(std::mem::offset_of!(BridgeFeature, x), 0);
        assert_eq!(std::mem::offset_of!(BridgeFeature, z), 4);
        assert_eq!(std::mem::offset_of!(BridgeFeature, kind), 8);
        assert_eq!(std::mem::offset_of!(BridgeFeature, approximate), 12);
        assert_eq!(std::mem::offset_of!(BridgeFeature, end_ship), 13);
    }

    #[test]
    fn native_library_links_and_resolves_versions() {
        assert!(unsafe { axolotl_seed_map_java_version(12103) } > 0);
        assert_eq!(unsafe { axolotl_seed_map_java_version(-1) }, 0);
    }
}
