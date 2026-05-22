use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub enum OrbId {
    Empty, // Placeholder for an empty orb slot
    Lightning,
    Dark,
    Frost,
    Plasma,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrbEntity {
    pub id: OrbId,
    pub base_passive_amount: i32,
    pub base_evoke_amount: i32,
    pub passive_amount: i32,
    pub evoke_amount: i32,
}

impl OrbEntity {
    pub fn new(id: OrbId) -> Self {
        match id {
            OrbId::Empty => OrbEntity {
                id,
                base_passive_amount: 0,
                base_evoke_amount: 0,
                passive_amount: 0,
                evoke_amount: 0,
            },
            OrbId::Lightning => OrbEntity {
                id,
                base_passive_amount: 3,
                base_evoke_amount: 8,
                passive_amount: 3,
                evoke_amount: 8,
            },
            OrbId::Dark => OrbEntity {
                id,
                base_passive_amount: 6,
                base_evoke_amount: 6,
                passive_amount: 6,
                evoke_amount: 6,
            },
            OrbId::Frost => OrbEntity {
                id,
                base_passive_amount: 2,
                base_evoke_amount: 5,
                passive_amount: 2,
                evoke_amount: 5,
            },
            OrbId::Plasma => OrbEntity {
                id,
                base_passive_amount: 1,
                base_evoke_amount: 2,
                passive_amount: 1,
                evoke_amount: 2,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub enum StanceId {
    Neutral,
    Wrath,
    Calm,
    Divinity,
}

impl StanceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            StanceId::Neutral => "Neutral",
            StanceId::Wrath => "Wrath",
            StanceId::Calm => "Calm",
            StanceId::Divinity => "Divinity",
        }
    }
}
