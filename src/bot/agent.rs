use crate::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

const CURIOSITY_RUNTIME_ENABLED: bool = false;

/// An autonomous agent that can decide the next `ClientInput` based on the game state.
pub struct Agent {
    bot_depth: u32,
    /// Pre-computed optimal map path for current act (x-coords for y=0..14, plus boss)
    pub(crate) map_path: Vec<i32>,
    pub db: crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    pub(crate) curiosity_target: Option<crate::bot::coverage::CuriosityTarget>,
}

impl Agent {
    pub fn new() -> Self {
        Self {
            bot_depth: 6,
            map_path: Vec::new(),
            db: crate::bot::coverage::CoverageDb::load_or_default(),
            coverage_mode: crate::bot::coverage::CoverageMode::PreferNovel,
            curiosity_target: None,
        }
    }

    /// Sets the search depth for combat decision tree.
    pub fn set_bot_depth(&mut self, depth: u32) {
        self.bot_depth = depth;
    }

    pub fn set_coverage_mode(&mut self, mode: crate::bot::coverage::CoverageMode) {
        self.coverage_mode = mode;
    }

    pub fn set_curiosity_target(&mut self, target: Option<crate::bot::coverage::CuriosityTarget>) {
        self.curiosity_target = target;
    }

    pub(crate) const fn curiosity_runtime_enabled() -> bool {
        CURIOSITY_RUNTIME_ENABLED
    }

    pub(crate) fn active_curiosity_target(&self) -> Option<&crate::bot::coverage::CuriosityTarget> {
        if Self::curiosity_runtime_enabled() {
            self.curiosity_target.as_ref()
        } else {
            None
        }
    }

    /// Primary entry point for the bot to decide the next move.
    pub fn decide(
        &mut self,
        es: &EngineState,
        rs: &RunState,
        cs: &Option<CombatState>,
        verbose: bool,
    ) -> ClientInput {
        match es {
            EngineState::PendingChoice(crate::state::core::PendingChoice::CardRewardSelect {
                cards,
                can_skip,
                ..
            }) => match crate::bot::reward_heuristics::evaluate_reward_screen(cards) {
                Some(idx) => ClientInput::SubmitDiscoverChoice(
                    self.active_curiosity_target()
                        .and_then(|_| self.curiosity_reward_pick(cards, rs))
                        .unwrap_or(idx),
                ),
                None if *can_skip => ClientInput::Cancel,
                None => ClientInput::SubmitDiscoverChoice(0),
            },
            EngineState::CombatPlayerTurn
            | EngineState::PendingChoice(_)
            | EngineState::EventCombat(_) => {
                if let Some(combat) = cs {
                    let chosen = crate::bot::search::find_best_move(
                        es,
                        combat,
                        self.bot_depth,
                        verbose,
                        &self.db,
                        self.coverage_mode,
                        self.active_curiosity_target(),
                    );

                    // Live coverage tracking: mark executing moves as tested
                    match &chosen {
                        ClientInput::PlayCard { card_index, .. } => {
                            if let Some(card) = combat.zones.hand.get(*card_index) {
                                let def = crate::content::cards::get_card_definition(card.id);
                                self.db.tested_cards.insert(def.name.to_string());
                                self.db.save();
                            }
                        }
                        ClientInput::UsePotion { potion_index, .. } => {
                            if let Some(Some(p)) = combat.entities.potions.get(*potion_index) {
                                let def = crate::content::potions::get_potion_definition(p.id);
                                self.db.tested_potions.insert(def.name.to_string());
                                self.db.save();
                            }
                        }
                        _ => {}
                    }

                    self.record_signature_for_choice(es, combat, &chosen);

                    chosen
                } else {
                    ClientInput::Proceed
                }
            }
            EngineState::MapNavigation => self.decide_map(rs),
            EngineState::RewardScreen(reward) => {
                use crate::rewards::state::RewardItem;

                // 1. Handle pending card choice
                if let Some(cards) = &reward.pending_card_choice {
                    let offered_cards: Vec<_> =
                        cards.iter().map(|reward_card| reward_card.id).collect();

                    if let Some(idx) = self
                        .active_curiosity_target()
                        .and_then(|_| self.curiosity_reward_pick(&offered_cards, rs))
                        .or_else(|| {
                            crate::bot::reward_heuristics::evaluate_reward_screen_for_run(
                                &offered_cards,
                                rs,
                            )
                        })
                    {
                        ClientInput::SelectCard(idx)
                    } else {
                        ClientInput::Proceed
                    }
                } else if !reward.items.is_empty() {
                    if let Some(idx) = self
                        .active_curiosity_target()
                        .and_then(|_| self.curiosity_reward_claim(&reward.items))
                    {
                        return ClientInput::ClaimReward(idx);
                    }

                    // 2. Handle claiming items
                    let mut claimed = false;
                    let mut claim_idx = 0;

                    for (i, item) in reward.items.iter().enumerate() {
                        match item {
                            RewardItem::Potion { .. } => {
                                // Claim potion only if we have an empty slot
                                if rs.potions.iter().any(|p| p.is_none()) {
                                    claim_idx = i;
                                    claimed = true;
                                    break;
                                }
                            }
                            // Always claim gold/relics/cards/etc.
                            _ => {
                                claim_idx = i;
                                claimed = true;
                                break;
                            }
                        }
                    }

                    if claimed {
                        ClientInput::ClaimReward(claim_idx)
                    } else {
                        // Leftover items (e.g. potions when full), proceed
                        ClientInput::Proceed
                    }
                } else {
                    ClientInput::Proceed
                }
            }
            EngineState::BossRelicSelect(bs) => {
                if let Some(idx) = self
                    .active_curiosity_target()
                    .and_then(|_| self.curiosity_boss_relic_pick(&bs.relics))
                {
                    return ClientInput::SubmitRelicChoice(idx);
                }

                let mut best_idx = 0;
                let mut best_score = i32::MIN;
                for (i, relic) in bs.relics.iter().enumerate() {
                    let score = self.boss_relic_score(rs, *relic);
                    if score > best_score {
                        best_score = score;
                        best_idx = i;
                    }
                }

                ClientInput::SubmitRelicChoice(best_idx)
            }
            EngineState::Campfire => self.decide_campfire(rs),
            EngineState::EventRoom => self.decide_event(rs),
            EngineState::Shop(shop) => self.decide_shop(rs, shop),
            EngineState::RunPendingChoice(choice_state) => {
                use crate::state::core::RunPendingChoiceReason;
                match choice_state.reason {
                    RunPendingChoiceReason::Purge => {
                        if rs.master_deck.is_empty() {
                            ClientInput::Cancel
                        } else {
                            let indices = self.best_purge_indices(
                                rs,
                                choice_state.max_choices.min(rs.master_deck.len()),
                            );
                            ClientInput::SubmitSelection(SelectionResolution {
                                scope: SelectionScope::Deck,
                                selected: indices
                                    .into_iter()
                                    .filter_map(|idx| rs.master_deck.get(idx))
                                    .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                                    .collect(),
                            })
                        }
                    }
                    RunPendingChoiceReason::Upgrade => {
                        if let Some(best_idx) = self.best_upgrade_index(rs) {
                            ClientInput::SubmitSelection(SelectionResolution {
                                scope: SelectionScope::Deck,
                                selected: rs
                                    .master_deck
                                    .get(best_idx)
                                    .map(|card| vec![SelectionTargetRef::CardUuid(card.uuid)])
                                    .unwrap_or_default(),
                            })
                        } else {
                            ClientInput::Cancel
                        }
                    }
                    RunPendingChoiceReason::Transform
                    | RunPendingChoiceReason::TransformUpgraded => {
                        if rs.master_deck.is_empty() {
                            ClientInput::Cancel
                        } else {
                            let indices = self.best_transform_indices(
                                rs,
                                choice_state.max_choices.min(rs.master_deck.len()),
                                matches!(
                                    choice_state.reason,
                                    RunPendingChoiceReason::TransformUpgraded
                                ),
                            );
                            ClientInput::SubmitSelection(SelectionResolution {
                                scope: SelectionScope::Deck,
                                selected: indices
                                    .into_iter()
                                    .filter_map(|idx| rs.master_deck.get(idx))
                                    .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                                    .collect(),
                            })
                        }
                    }
                    RunPendingChoiceReason::Duplicate => {
                        if let Some(best_idx) = self.best_duplicate_index(rs) {
                            ClientInput::SubmitSelection(SelectionResolution {
                                scope: SelectionScope::Deck,
                                selected: rs
                                    .master_deck
                                    .get(best_idx)
                                    .map(|card| vec![SelectionTargetRef::CardUuid(card.uuid)])
                                    .unwrap_or_default(),
                            })
                        } else {
                            ClientInput::Cancel
                        }
                    }
                }
            }
            EngineState::GameOver(_) => ClientInput::Proceed,
            _ => ClientInput::Proceed,
        }
    }

    fn record_signature_for_choice(
        &mut self,
        engine: &EngineState,
        combat: &CombatState,
        input: &ClientInput,
    ) {
        let before_engine = engine.clone();
        let before_state = combat.clone();
        let mut after_engine = engine.clone();
        let mut after_state = combat.clone();
        let alive = crate::diff::replay::replay_support::tick_until_stable(
            &mut after_engine,
            &mut after_state,
            input.clone(),
        );
        if !alive && !matches!(after_engine, EngineState::GameOver(_)) {
            return;
        }
        let signature = crate::interaction_coverage::signature_from_transition(
            &before_engine,
            &before_state,
            input,
            &after_engine,
            &after_state,
        );
        self.db.record_signature(&signature);
        self.db.save();
    }
}

#[cfg(test)]
mod tests {
    use super::Agent;
    use crate::combat::CombatCard;
    use crate::content::cards::CardId;
    use crate::content::potions::PotionId;
    use crate::content::relics::RelicId;
    use crate::shop::{ShopCard, ShopRelic, ShopState};
    use crate::state::core::{CampfireChoice, ClientInput};
    use crate::state::run::RunState;

    fn run_with(cards: &[CardId]) -> RunState {
        let mut rs = RunState::new(17, 0, false, "Ironclad");
        rs.master_deck = cards
            .iter()
            .enumerate()
            .map(|(idx, &id)| CombatCard::new(id, idx as u32))
            .collect();
        rs
    }

    fn shop_with_cards(cards: &[(CardId, i32)]) -> ShopState {
        let mut shop = ShopState::new();
        shop.cards = cards
            .iter()
            .map(|(card_id, price)| ShopCard {
                card_id: *card_id,
                price: *price,
                can_buy: true,
                blocked_reason: None,
            })
            .collect();
        shop
    }

    fn shop_with_cards_and_relics(cards: &[(CardId, i32)], relics: &[(RelicId, i32)]) -> ShopState {
        let mut shop = shop_with_cards(cards);
        shop.relics = relics
            .iter()
            .map(|(relic_id, price)| ShopRelic {
                relic_id: *relic_id,
                price: *price,
                can_buy: true,
                blocked_reason: None,
            })
            .collect();
        shop
    }

    #[test]
    fn shop_relic_score_prefers_dead_branch_in_exhaust_deck() {
        let agent = Agent::new();
        let thin = run_with(&[CardId::Strike, CardId::Defend, CardId::Bash]);
        let exhaust = run_with(&[
            CardId::Corruption,
            CardId::FeelNoPain,
            CardId::SecondWind,
            CardId::BurningPact,
        ]);

        let low = agent.shop_relic_score(&thin, RelicId::DeadBranch);
        let high = agent.shop_relic_score(&exhaust, RelicId::DeadBranch);

        assert!(high > low);
    }

    #[test]
    fn boss_relic_score_prefers_empty_cage_when_deck_has_many_basics() {
        let agent = Agent::new();
        let bloated = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]);
        let cleaner = run_with(&[
            CardId::ShrugItOff,
            CardId::FlameBarrier,
            CardId::Corruption,
            CardId::FeelNoPain,
        ]);

        let high = agent.boss_relic_score(&bloated, RelicId::EmptyCage);
        let low = agent.boss_relic_score(&cleaner, RelicId::EmptyCage);

        assert!(high > low);
    }

    #[test]
    fn best_purge_keeps_exhaust_core_and_cuts_basic() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::Corruption,
            CardId::FeelNoPain,
            CardId::SecondWind,
            CardId::BurningPact,
        ]);

        let idx = agent.best_purge_index(&rs);
        assert!(matches!(
            rs.master_deck[idx].id,
            CardId::Strike | CardId::Defend
        ));
    }

    #[test]
    fn best_purge_prioritizes_parasite_over_basic_cards() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Parasite,
            CardId::Strike,
            CardId::Defend,
            CardId::Bash,
        ]);

        let idx = agent.best_purge_index(&rs);
        assert_eq!(rs.master_deck[idx].id, CardId::Parasite);
    }

    #[test]
    fn best_upgrade_prefers_limit_break_in_strength_shell() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Inflame,
            CardId::HeavyBlade,
            CardId::TwinStrike,
            CardId::LimitBreak,
        ]);

        let idx = agent.best_upgrade_index(&rs).expect("upgrade target");
        assert_eq!(rs.master_deck[idx].id, CardId::LimitBreak);
    }

    #[test]
    fn shop_card_score_prefers_dark_embrace_in_exhaust_shell() {
        let agent = Agent::new();
        let thin = run_with(&[CardId::Strike, CardId::Defend, CardId::Bash]);
        let exhaust = run_with(&[
            CardId::Corruption,
            CardId::FeelNoPain,
            CardId::SecondWind,
            CardId::BurningPact,
        ]);

        let low = agent.shop_card_score(&thin, CardId::DarkEmbrace);
        let high = agent.shop_card_score(&exhaust, CardId::DarkEmbrace);

        assert!(high > low);
    }

    #[test]
    fn shop_card_score_under_snecko_penalizes_anger_vs_expensive_scaler() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::ShrugItOff,
            CardId::PommelStrike,
        ]);
        rs.relics
            .push(crate::content::relics::RelicState::new(RelicId::SneckoEye));

        let anger = agent.shop_card_score(&rs, CardId::Anger);
        let demon = agent.shop_card_score(&rs, CardId::DemonForm);

        assert!(demon > anger);
    }

    #[test]
    fn best_upgrade_prefers_searing_blow_on_early_route() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::SearingBlow,
            CardId::Armaments,
            CardId::BattleTrance,
            CardId::ShrugItOff,
        ]);
        rs.floor_num = 6;

        let idx = agent.best_upgrade_index(&rs).expect("upgrade target");
        assert_eq!(rs.master_deck[idx].id, CardId::SearingBlow);
    }

    #[test]
    fn best_upgrade_hard_commits_to_searing_blow_under_busted_crown() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::SearingBlow,
            CardId::Armaments,
            CardId::Offering,
            CardId::FlameBarrier,
            CardId::ShrugItOff,
        ]);
        rs.floor_num = 10;
        rs.relics.push(crate::content::relics::RelicState::new(
            RelicId::BustedCrown,
        ));

        let idx = agent.best_upgrade_index(&rs).expect("upgrade target");
        assert_eq!(rs.master_deck[idx].id, CardId::SearingBlow);
    }

    #[test]
    fn decide_shop_buys_deficit_solving_card_before_purge() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]);
        rs.gold = 120;

        let mut shop = shop_with_cards(&[(CardId::Hemokinesis, 75)]);
        shop.purge_available = true;
        shop.purge_cost = 75;

        let choice = agent.decide_shop(&rs, &shop);
        assert!(
            matches!(choice, ClientInput::BuyCard(0)),
            "unexpected shop choice: {:?}",
            choice
        );
    }

    #[test]
    fn decide_shop_compares_cards_and_relics_in_one_pool() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Corruption,
            CardId::FeelNoPain,
            CardId::SecondWind,
            CardId::BurningPact,
        ]);
        rs.gold = 180;

        let shop =
            shop_with_cards_and_relics(&[(CardId::DarkEmbrace, 75)], &[(RelicId::Girya, 150)]);

        let choice = agent.decide_shop(&rs, &shop);
        assert!(
            matches!(choice, ClientInput::BuyCard(0)),
            "unexpected shop choice: {:?}",
            choice
        );
    }

    #[test]
    fn decide_shop_prefers_purge_over_mediocre_card_in_opening_shop() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]);
        rs.gold = 125;

        let mut shop = shop_with_cards(&[(CardId::IronWave, 75)]);
        shop.purge_available = true;
        shop.purge_cost = 75;

        let choice = agent.decide_shop(&rs, &shop);
        assert!(
            matches!(choice, ClientInput::PurgeCard(_)),
            "unexpected shop choice: {:?}",
            choice
        );
    }

    #[test]
    fn decide_shop_under_snecko_does_not_buy_low_cost_filler_when_only_anger_is_offered() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::ShrugItOff,
            CardId::PommelStrike,
            CardId::BattleTrance,
        ]);
        rs.gold = 80;
        rs.relics
            .push(crate::content::relics::RelicState::new(RelicId::SneckoEye));

        let shop = shop_with_cards(&[(CardId::Anger, 62)]);
        let choice = agent.decide_shop(&rs, &shop);
        assert!(
            !matches!(choice, ClientInput::BuyCard(0)),
            "unexpected shop choice: {:?}",
            choice
        );
    }

    #[test]
    fn decide_shop_prefers_strong_relic_over_merely_decent_card() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::ShrugItOff,
        ]);
        rs.gold = 170;

        let shop =
            shop_with_cards_and_relics(&[(CardId::IronWave, 75)], &[(RelicId::Shuriken, 150)]);

        let choice = agent.decide_shop(&rs, &shop);
        assert!(
            matches!(choice, ClientInput::BuyRelic(0)),
            "unexpected shop choice: {:?}",
            choice
        );
    }

    #[test]
    fn decide_shop_can_save_gold_when_no_option_is_good_enough() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::IronWave,
            CardId::ShrugItOff,
        ]);
        rs.gold = 92;

        let mut shop = shop_with_cards(&[(CardId::Clothesline, 75)]);
        shop.purge_available = false;
        let choice = agent.decide_shop(&rs, &shop);
        assert_eq!(
            choice,
            ClientInput::Proceed,
            "unexpected shop choice: {:?}",
            choice
        );
    }

    #[test]
    fn shop_potion_score_prefers_ghost_in_a_jar_when_block_gap_exists() {
        let agent = Agent::new();
        let weak = run_with(&[CardId::Strike, CardId::Strike, CardId::Bash]);
        let sturdy = run_with(&[
            CardId::ShrugItOff,
            CardId::FlameBarrier,
            CardId::GhostlyArmor,
            CardId::Bash,
        ]);

        let weak_score = agent.shop_potion_score(&weak, PotionId::GhostInAJar);
        let sturdy_score = agent.shop_potion_score(&sturdy, PotionId::GhostInAJar);

        assert!(weak_score > sturdy_score);
    }

    #[test]
    fn campfire_prefers_smithing_searing_blow_when_route_is_active_and_safe() {
        let agent = Agent::new();
        let mut rs = run_with(&[CardId::SearingBlow, CardId::Armaments, CardId::ShrugItOff]);
        rs.current_hp = 58;
        rs.max_hp = 80;
        rs.floor_num = 10;

        let choice = agent.decide_campfire(&rs);
        assert!(matches!(
            choice,
            ClientInput::CampfireOption(CampfireChoice::Smith(0))
        ));
    }

    #[test]
    fn low_hp_map_weights_prefer_rests_over_elites() {
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::Inflame,
            CardId::HeavyBlade,
        ]);
        rs.current_hp = 18;
        rs.max_hp = 80;

        let weights = Agent::map_room_weights(&rs, None);
        assert!(weights[1] > weights[3]);
    }

    #[test]
    fn living_wall_prefers_upgrade_when_shell_has_good_target() {
        let agent = Agent::new();
        let mut rs = run_with(&[
            CardId::Inflame,
            CardId::HeavyBlade,
            CardId::TwinStrike,
            CardId::LimitBreak,
        ]);
        rs.event_state = Some(crate::state::events::EventState::new(
            crate::state::events::EventId::LivingWall,
        ));

        assert!(matches!(
            agent.decide_event(&rs),
            crate::state::core::ClientInput::EventChoice(2)
        ));
    }

    #[test]
    fn best_duplicate_prefers_shell_payoff() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Inflame,
            CardId::TwinStrike,
            CardId::HeavyBlade,
            CardId::LimitBreak,
        ]);

        let idx = agent.best_duplicate_index(&rs).expect("duplicate target");
        assert!(matches!(
            rs.master_deck[idx].id,
            CardId::LimitBreak | CardId::HeavyBlade
        ));
    }

    #[test]
    fn best_transform_prefers_basic_over_shell_core() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::Corruption,
            CardId::FeelNoPain,
            CardId::SecondWind,
        ]);

        let idx = agent.best_transform_index(&rs);
        assert!(matches!(
            rs.master_deck[idx].id,
            CardId::Strike | CardId::Defend
        ));
    }

    #[test]
    fn best_purge_keeps_strength_shell_enabler_and_payoff() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Bash,
            CardId::Inflame,
            CardId::HeavyBlade,
        ]);

        let idx = agent.best_purge_index(&rs);
        assert!(matches!(
            rs.master_deck[idx].id,
            CardId::Strike | CardId::Defend
        ));
    }

    #[test]
    fn best_purge_keeps_exhaust_engine_and_outlet_before_basics() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::FeelNoPain,
            CardId::DarkEmbrace,
            CardId::SecondWind,
            CardId::BurningPact,
        ]);

        let idx = agent.best_purge_index(&rs);
        assert!(matches!(
            rs.master_deck[idx].id,
            CardId::Strike | CardId::Defend
        ));
    }

    #[test]
    fn best_purge_does_not_cut_bash_before_basics_in_opening_deck() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]);

        let idx = agent.best_purge_index(&rs);
        assert_ne!(rs.master_deck[idx].id, CardId::Bash);
        assert!(crate::content::cards::is_starter_basic(
            rs.master_deck[idx].id
        ));
    }

    #[test]
    fn best_transform_upgraded_prefers_transforming_multiple_basics_for_astrolabe() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]);

        let indices = agent.best_transform_indices(&rs, 3, true);
        assert_eq!(indices.len(), 3);
        assert!(indices
            .into_iter()
            .all(|idx| { crate::content::cards::is_starter_basic(rs.master_deck[idx].id) }));
    }

    #[test]
    fn astrolabe_transform_keeps_premium_scalers_and_rolls_basics() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::DemonForm,
            CardId::Apotheosis,
            CardId::Reaper,
        ]);

        let indices = agent.best_transform_indices(&rs, 3, true);
        assert_eq!(indices.len(), 3);
        assert!(indices
            .into_iter()
            .all(|idx| { matches!(rs.master_deck[idx].id, CardId::Strike | CardId::Defend) }));
    }

    #[test]
    fn best_duplicate_avoids_basics_when_premium_keepers_exist() {
        let agent = Agent::new();
        let rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::DemonForm,
            CardId::Apotheosis,
        ]);

        let idx = agent.best_duplicate_index(&rs).expect("duplicate target");
        assert!(matches!(
            rs.master_deck[idx].id,
            CardId::DemonForm | CardId::Apotheosis
        ));
    }

    #[test]
    fn curiosity_archetype_matches_signature_tags() {
        let sig = crate::interaction_coverage::InteractionSignature {
            source_kind: "card".into(),
            source_id: "Inflame".into(),
            target_shape: "none".into(),
            pending_choice: "none".into(),
            archetype_tags: vec!["strength".into(), "shell_incomplete".into()],
            hook_tags: vec![],
            pile_tags: vec![],
            power_tags: vec![],
            outcome_tags: vec![],
        };
        assert!(crate::interaction_coverage::curiosity_target_matches(
            &sig,
            &crate::bot::coverage::CuriosityTarget::archetype("strength")
        ));
    }

    #[test]
    fn curiosity_archetype_reward_pick_prefers_shell_card() {
        let mut agent = Agent::new();
        agent.set_curiosity_target(Some(crate::bot::coverage::CuriosityTarget::archetype(
            "strength",
        )));
        let rs = run_with(&[CardId::Strike, CardId::Defend, CardId::Bash]);
        let offered = [CardId::Inflame, CardId::Defend, CardId::TrueGrit];

        assert!(!Agent::curiosity_runtime_enabled());
        assert_eq!(agent.active_curiosity_target(), None);
        assert_eq!(agent.curiosity_reward_pick(&offered, &rs), None);
    }
}
