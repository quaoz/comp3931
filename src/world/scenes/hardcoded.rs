use crate::settings::{EcosystemKernel, EcosystemSettings, PlantType, SceneData, WorldDate};

pub fn hardcoded_scenes() -> Vec<SceneData> {
    vec![
        garden_scene(),
        meadow_scene(),
        botanical_scene(),
        forest_scene(),
    ]
}

fn garden_scene() -> SceneData {
    use PlantType::*;
    SceneData {
        name: "Garden".to_string(),
        plants: Vec::new(),
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 7,
        ecosystem: EcosystemSettings {
            area: 80.0,
            num_plants: 400,
            kernel: EcosystemKernel::Mixed,
            kernel_radius: 7.0,
            use_self_thinning: true,
            thinning_radius: 4.0,
            use_succession: false,
            succession_steps: 10,
            species: vec![
                (Tree, 1.0),
                (Bush, 2.0),
                (Fern, 1.5),
                (Mint, 1.5),
                (Wildflower, 2.0),
                (Capsella, 1.0),
                (Lychnis, 1.0),
            ],
            generation: 0,
        },
        generation: 0,
    }
}

fn meadow_scene() -> SceneData {
    use PlantType::*;
    SceneData {
        name: "Meadow".to_string(),
        plants: Vec::new(),
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 13,
        ecosystem: EcosystemSettings {
            area: 140.0,
            num_plants: 1000,
            kernel: EcosystemKernel::Inhibitory,
            kernel_radius: 6.0,
            use_self_thinning: false,
            thinning_radius: 4.0,
            use_succession: true,
            succession_steps: 6,
            species: vec![
                (Wildflower, 5.0),
                (Lychnis, 2.0),
                (Carrot, 2.0),
                (Capsella, 1.5),
                (Mint, 1.0),
            ],
            generation: 0,
        },
        generation: 0,
    }
}

fn botanical_scene() -> SceneData {
    use PlantType::*;
    SceneData {
        name: "Botanical".to_string(),
        plants: Vec::new(),
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 22,
        ecosystem: EcosystemSettings {
            area: 100.0,
            num_plants: 90,
            kernel: EcosystemKernel::Neutral,
            kernel_radius: 9.0,
            use_self_thinning: true,
            thinning_radius: 6.0,
            use_succession: false,
            succession_steps: 10,
            // One of each species at equal weight — good for grammar comparison
            species: vec![
                (Capsella, 1.0),
                (Mint, 1.0),
                (Lychnis, 1.0),
                (Carrot, 1.0),
                (Fern, 1.0),
                (Bush, 1.0),
                (Tree, 1.0),
                (Wildflower, 1.0),
                (Mycelis, 1.0),
            ],
            generation: 0,
        },
        generation: 0,
    }
}

fn forest_scene() -> SceneData {
    use PlantType::*;
    SceneData {
        name: "Forest".to_string(),
        plants: Vec::new(),
        global_scale: 1.0,
        date: WorldDate::default(),
        seed: 0,
        ecosystem: EcosystemSettings {
            area: 160.0,
            num_plants: 500,
            kernel: EcosystemKernel::Inhibitory,
            kernel_radius: 12.0,
            use_self_thinning: true,
            thinning_radius: 8.0,
            use_succession: true,
            succession_steps: 12,
            species: vec![(Tree, 3.5), (Bush, 2.0), (Fern, 3.0), (Mycelis, 1.5)],
            generation: 0,
        },
        generation: 0,
    }
}
