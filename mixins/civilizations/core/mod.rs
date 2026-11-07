// Civilization domain model. Deliberately independent from Minecraft/Horizon APIs.
// Authored as a normal Rust module tree; tools/mixin_sources.py flattens it into
// the single self-contained payload the Horizon mixin tool requires.

mod bakery;
mod barrel;
mod buildings;
mod chunks;
mod civic;
mod constants;
mod dialogue;
mod farm;
mod gather;
mod goods;
mod housing;
mod jobs;
mod people;
mod settlement;
mod signs;
mod world;

#[cfg(test)]
mod tests;

pub use bakery::*;
pub use barrel::*;
pub use buildings::*;
pub use chunks::*;
pub use civic::*;
pub use constants::*;
pub use dialogue::*;
pub use farm::*;
pub use gather::*;
pub use goods::*;
pub use housing::*;
pub use jobs::*;
pub use people::*;
pub use settlement::*;
pub use signs::*;
pub use world::*;
