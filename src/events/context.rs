use crate::content::relics::RelicId;

#[derive(Debug, Clone)]
pub struct EventContext {
    pub act_num: u8,
    pub ascension_level: u8,
    pub floor_num: i32,
    pub gold: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    
    // Conditions used for specific events (e.g., Fountain of Curse Cleansing requires a curse)
    pub has_curses: bool,
    
    pub tiny_chest_counter: i32,
    
    // Used by Moai Head
    pub has_golden_idol: bool,
    
    // Used by Juzu Bracelet (overrides monster roll inside event chance calculation)
    pub has_juzu_bracelet: bool,
    
    // Used by Nloth (requires at least 2 relics)
    pub relic_count: usize,
}
