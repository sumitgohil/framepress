//! Public traits. Engines and resolvers implement these; everything else in
//! `framepress-core` depends only on the trait API, not on concrete engines.

mod engine;
mod repository;
mod resolver;

pub use engine::CompressionEngine;
pub use repository::HistoryRepository;
pub use resolver::PresetResolver;
