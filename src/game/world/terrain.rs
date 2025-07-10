pub struct Terrain {
    pub max_height: f32,
    pub size: u32,
    pub tile_size: u32,
    pub tiles: Vec<TerrainTile>,
}
impl Terrain {
    pub(crate) fn generate(terrain_size: u32, tile_size: u32, max_height: f32) -> Terrain {
        let mut tiles = Vec::with_capacity((terrain_size * terrain_size) as _);

        for z in 0..terrain_size {
            for x in 0..terrain_size {
                tiles.push(TerrainTile {
                    id: (x, z),
                    height_map: vec![0.0; (tile_size * tile_size) as _],
                });
            }
        }

        Terrain {
            max_height,
            size: terrain_size,
            tile_size,
            tiles,
        }
    }
}

pub struct TerrainTile {
    pub height_map: Vec<f32>,
    pub id: (u32, u32),
}
