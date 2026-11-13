//! cm-core: lõi mô phỏng tất định của Chín Mạng. KHÔNG phụ thuộc Bevy / I/O. Xem SPEC.md §3.
pub mod api;
pub mod fmt;
pub mod game;

pub use api::*;
pub use fmt::fmt_num;
pub use game::Game;
