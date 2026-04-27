//! Re-exports [`apicult_desigual::HGridLayout`] and bridges from h_terrain's [`HGridSettings`].

use super::HGridSettings;

pub use apicult_desigual::HGridLayout;

impl HGridSettings {
    /// Build an [`HGridLayout`] from these settings.
    pub fn build_layout(&self) -> HGridLayout {
        HGridLayout::from_settings(&apicult_desigual::HGridSettings::from(self))
    }
}
