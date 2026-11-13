//! cm-core: lõi mô phỏng tất định của Chín Mạng. KHÔNG phụ thuộc Bevy / I/O. Xem SPEC.md §3.
pub mod api;
mod commands;
mod data;
pub mod fmt;
pub mod game;
mod rng;
mod rules;
mod save;
mod sim;
mod state;

pub use api::*;
pub use fmt::fmt_num;
pub use game::Game;

#[cfg(test)]
mod tests;
