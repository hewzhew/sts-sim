use crate::bot::agent::Agent;
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn is_high_value_tactical_card(
        &self,
        card_id: crate::content::cards::CardId,
    ) -> bool {
        use crate::content::cards::CardId;
        matches!(
            card_id,
            CardId::Apotheosis
                | CardId::Panacea
                | CardId::Blind
                | CardId::DarkShackles
                | CardId::Trip
                | CardId::GoodInstincts
                | CardId::Finesse
                | CardId::FlashOfSteel
                | CardId::MasterOfStrategy
                | CardId::Corruption
                | CardId::FeelNoPain
                | CardId::DarkEmbrace
                | CardId::Shockwave
        )
    }

    pub(crate) fn shop_needs_frontload_damage(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> bool {
        let has_premium_damage = rs.master_deck.iter().any(|c| {
            matches!(
                c.id,
                crate::content::cards::CardId::SearingBlow
                    | crate::content::cards::CardId::Hemokinesis
                    | crate::content::cards::CardId::Carnage
                    | crate::content::cards::CardId::Immolate
                    | crate::content::cards::CardId::Whirlwind
                    | crate::content::cards::CardId::Pummel
                    | crate::content::cards::CardId::Bludgeon
            )
        });
        !has_premium_damage || (profile.attack_count <= 6 && profile.strength_payoffs == 0)
    }

    pub(crate) fn shop_needs_reliable_block(
        &self,
        rs: &RunState,
        profile: &crate::bot::evaluator::DeckProfile,
    ) -> bool {
        let has_anchor_defense = rs.master_deck.iter().any(|c| {
            matches!(
                c.id,
                crate::content::cards::CardId::ShrugItOff
                    | crate::content::cards::CardId::FlameBarrier
                    | crate::content::cards::CardId::GhostlyArmor
                    | crate::content::cards::CardId::Impervious
                    | crate::content::cards::CardId::PowerThrough
            )
        });
        profile.block_core < 2 || !has_anchor_defense
    }

    pub(crate) fn shop_needs_damage_control(&self, rs: &RunState) -> bool {
        !rs.master_deck.iter().any(|c| {
            matches!(
                c.id,
                crate::content::cards::CardId::Disarm
                    | crate::content::cards::CardId::Shockwave
                    | crate::content::cards::CardId::Uppercut
                    | crate::content::cards::CardId::Clothesline
            )
        })
    }
}
