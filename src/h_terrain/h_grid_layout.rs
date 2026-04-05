//! Re-exports [`hex_grid::HGridLayout`] and bridges from h_terrain's [`HGridSettings`].

use super::HGridSettings;

pub use hex_grid::HGridLayout;

impl HGridSettings {
    /// Build an [`HGridLayout`] from these settings.
    pub fn build_layout(&self) -> HGridLayout {
        HGridLayout::from_settings(&hex_grid::HGridSettings::from(self))
    }
}
