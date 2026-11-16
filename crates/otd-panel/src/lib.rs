//! Panel COMPs — widgets, fractional layout, and default interiors.

pub mod layout;
pub mod panel;
pub mod params;
pub mod templates;

pub use layout::{apply_container_layout, participates_in_layout, resolve_panel_rects};
pub use panel::*;
pub use params::*;
pub use templates::{
    ensure_button_interior, ensure_field_interior, ensure_slider_interior, install_if_panel,
    install_text_comp_interior, upgrade_panel_internals,
};