pub struct Terrain {
    pub max_height: f32,
    pub size: u32,
    pub tile_size: u32,
    pub tiles: Vec<TerrainTile>,
}
impl Terrain {
    pub(crate) fn generate(terrain_size: u32, tile_size: u32, max_height: f32) -> Terrain {
        todo!()
    }
}

pub struct TerrainTile {
    pub height_map: Vec<f32>,
}