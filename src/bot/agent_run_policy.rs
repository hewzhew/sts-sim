use crate::bot::agent::Agent;

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
}
