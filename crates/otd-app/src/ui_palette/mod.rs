//! Editor color palettes — network chrome and egui chrome, not project media.



mod chrome;
mod helpers;
mod network;
mod palettes;
mod theme;

pub use chrome::apply;
pub use network::{apply_network_appearance, network, network_for_appearance};
pub use theme::NetworkTheme;

