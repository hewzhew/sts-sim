/// A read-only snapshot constructed by `RunState` before submitting deck changes.
#[derive(Debug, Clone)]
pub struct DeckContext {
    pub has_hoarder_mod: bool,

    // Omamori interception
    pub has_omamori: bool,
    pub omamori_charges: i32,

    // Side effects
    pub has_ceramic_fish: bool,
    pub has_darkstone_periapt: bool,

    // Egg upgrades
    pub has_molten_egg: bool,
    pub has_toxic_egg: bool,
    pub has_frozen_egg: bool,

    // Necronomicurse triggers
    pub has_necronomicon: bool,
}
