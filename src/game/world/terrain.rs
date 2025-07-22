use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Terrain {
    pub mountain_height: f32,
    pub dune_height: f32,
    pub spire_height: f32,
    pub size: u32,
    pub tile_size: u32,
    pub tiles: Vec<TerrainTile>,
}

impl Terrain {
    pub(crate) fn generate(
        terrain_size: u32,
        tile_size: u32,
        mountain_height: f32,
        dune_height: f32,
        spire_height: f32,
    ) -> Terrain {
        let mut tiles = Vec::with_capacity((terrain_size * terrain_size) as _);

        for z in 0..terrain_size {
            for x in 0..terrain_size {
                tiles.push(TerrainTile {
                    id: (x, z),
                    // height_map: vec![0.0; (tile_size * tile_size) as _],
                });
            }
        }

        Terrain {
            mountain_height,
            dune_height,
            spire_height,
            size: terrain_size,
            tile_size,
            tiles,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainTile {
    // pub height_map: Vec<f32>,
    pub id: (u32, u32),
}
