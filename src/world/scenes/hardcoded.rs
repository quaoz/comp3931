use crate::{
    settings::{PlantType, SceneData, WorldDate},
    util::rng,
    world::plants::PlantInstance,
};

pub fn hardcoded_scenes() -> Vec<SceneData> {
    vec![
        garden_scene(),
        meadow_scene(),
        botanical_scene(),
        forest_scene(),
    ]
}

fn garden_scene() -> SceneData {
    SceneData {
        name: "Garden".to_string(),
        plants: vec![
            PlantInstance::new(PlantType::Tree, 0.5).with_transform([0.0, 0.0, 0.0], 10.0, 0.0),
            PlantInstance::new(PlantType::Bush, 0.4).with_transform([15.0, 0.0, 10.0], 8.0, 45.0),
            PlantInstance::new(PlantType::Fern, 0.4).with_transform([-12.0, 0.0, 5.0], 6.0, 120.0),
            PlantInstance::new(PlantType::Mint, 0.4).with_transform([8.0, 0.0, -10.0], 7.0, 200.0),
            PlantInstance::new(PlantType::Tree, 0.4).with_transform(
                [-8.0, 0.0, -15.0],
                12.0,
                270.0,
            ),
            PlantInstance::new(PlantType::Wildflower, 0.5).with_transform(
                [5.0, 0.0, -5.0],
                5.0,
                90.0,
            ),
            PlantInstance::new(PlantType::Capsella, 0.3).with_transform(
                [18.0, 0.0, -5.0],
                5.0,
                160.0,
            ),
            PlantInstance::new(PlantType::Fern, 0.3).with_transform([-18.0, 0.0, -8.0], 5.0, 300.0),
            PlantInstance::new(PlantType::Bush, 0.3).with_transform([-5.0, 0.0, 18.0], 7.0, 215.0),
            PlantInstance::new(PlantType::Wildflower, 0.4).with_transform(
                [20.0, 0.0, -18.0],
                4.0,
                330.0,
            ),
        ],
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 7,
        generation: 0,
    }
}

fn meadow_scene() -> SceneData {
    SceneData {
        name: "Meadow".to_string(),
        plants: vec![
            PlantInstance::new(PlantType::Wildflower, 0.5).with_transform(
                [0.0, 0.0, 0.0],
                5.0,
                0.0,
            ),
            PlantInstance::new(PlantType::Wildflower, 0.4).with_transform(
                [-8.0, 0.0, 6.0],
                6.0,
                45.0,
            ),
            PlantInstance::new(PlantType::Wildflower, 0.6).with_transform(
                [10.0, 0.0, -4.0],
                4.0,
                90.0,
            ),
            PlantInstance::new(PlantType::Lychnis, 0.4).with_transform(
                [-5.0, 0.0, -10.0],
                8.0,
                135.0,
            ),
            PlantInstance::new(PlantType::Lychnis, 0.3).with_transform([7.0, 0.0, 8.0], 7.0, 180.0),
            PlantInstance::new(PlantType::Carrot, 0.4).with_transform(
                [-12.0, 0.0, -3.0],
                6.0,
                225.0,
            ),
            PlantInstance::new(PlantType::Carrot, 0.3).with_transform([3.0, 0.0, 12.0], 7.0, 270.0),
            PlantInstance::new(PlantType::Capsella, 0.4).with_transform(
                [15.0, 0.0, 5.0],
                5.0,
                315.0,
            ),
            PlantInstance::new(PlantType::Carrot, 0.4).with_transform(
                [-18.0, 0.0, 12.0],
                6.0,
                20.0,
            ),
            PlantInstance::new(PlantType::Wildflower, 0.3).with_transform(
                [18.0, 0.0, 10.0],
                4.0,
                250.0,
            ),
            PlantInstance::new(PlantType::Capsella, 0.3).with_transform(
                [5.0, 0.0, -18.0],
                5.0,
                80.0,
            ),
            PlantInstance::new(PlantType::Lychnis, 0.5).with_transform(
                [-14.0, 0.0, -15.0],
                7.0,
                195.0,
            ),
        ],
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 13,
        generation: 0,
    }
}

fn botanical_scene() -> SceneData {
    SceneData {
        name: "Botanical".to_string(),
        plants: vec![
            PlantInstance::new(PlantType::Capsella, 0.5).with_transform(
                [-20.0, 0.0, 0.0],
                8.0,
                0.0,
            ),
            PlantInstance::new(PlantType::Mint, 0.5).with_transform([-10.0, 0.0, 0.0], 7.0, 30.0),
            PlantInstance::new(PlantType::Lychnis, 0.4).with_transform([0.0, 0.0, 0.0], 9.0, 60.0),
            PlantInstance::new(PlantType::Lychnis, 0.5).with_transform([10.0, 0.0, 0.0], 8.0, 90.0),
            PlantInstance::new(PlantType::Carrot, 0.4).with_transform([20.0, 0.0, 0.0], 7.0, 120.0),
            PlantInstance::new(PlantType::Fern, 0.5).with_transform([-5.0, 0.0, -15.0], 8.0, 150.0),
            PlantInstance::new(PlantType::Bush, 0.4).with_transform([5.0, 0.0, -15.0], 7.0, 180.0),
            PlantInstance::new(PlantType::Tree, 0.4).with_transform(
                [-25.0, 0.0, -20.0],
                10.0,
                230.0,
            ),
            PlantInstance::new(PlantType::Wildflower, 0.4).with_transform(
                [25.0, 0.0, -10.0],
                5.0,
                310.0,
            ),
            PlantInstance::new(PlantType::Mint, 0.3).with_transform([0.0, 0.0, 18.0], 6.0, 45.0),
            PlantInstance::new(PlantType::Carrot, 0.3).with_transform(
                [-15.0, 0.0, 15.0],
                6.0,
                270.0,
            ),
        ],
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 22,
        generation: 0,
    }
}

fn forest_scene() -> SceneData {
    SceneData {
        name: "Forest".to_string(),
        plants: generate_forest_plants(0),
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 0,
        generation: 0,
    }
}

/// Generate forest plants from a seed so the layout changes with the scene seed.
fn generate_forest_plants(seed: u64) -> Vec<PlantInstance> {
    rng::seed(seed);
    let types = [
        PlantType::Tree,
        PlantType::Tree,
        PlantType::Fern,
        PlantType::Bush,
    ];
    (0..12)
        .map(|_| {
            let x = rng::random_range(-40.0, 40.0);
            let z = rng::random_range(-40.0, 40.0);
            let age = rng::random_range(6.0, 6.5);
            let scale = rng::random_range(8.0, 16.0);
            let rotation = rng::random_range(0.0, 360.0);
            let idx = (rng::random_range(0.0, types.len() as f32) as usize).min(types.len() - 1);
            PlantInstance::new(types[idx], -age).with_transform([x, 0.0, z], scale, rotation)
        })
        .collect()
}
