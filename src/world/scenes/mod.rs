mod controller;
mod hardcoded;
mod lod;

pub use controller::{FrameParams, SceneBuffers, SceneController, SceneOutput};
pub use hardcoded::hardcoded_scenes;
pub use lod::SceneStats;
