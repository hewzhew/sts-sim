use super::*;

impl RunState {
    /// Adds a card to the master deck using DeckManager pipeline.
    /// Handles Omamori negation, CeramicFish gold, Elite Eggs upgrades, etc.
    /// Returns true if the card was actually added (false if Omamori blocked it).
    pub fn add_card_to_deck(&mut self, card_id: crate::content::cards::CardId) -> bool {
        self.add_card_to_deck_with_upgrades_from(card_id, 0, DomainEventSource::RewardScreen)
    }

    /// Adds a card with an explicit pre-upgrade count.
    pub fn add_card_to_deck_with_upgrades(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
    ) -> bool {
        self.add_card_to_deck_with_upgrades_from(
            card_id,
            pre_upgrades,
            DomainEventSource::DeckMutation,
        )
    }

    pub fn add_card_to_deck_with_upgrades_from(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
    ) -> bool {
        let ctx = self.build_deck_context();
        self.add_card_to_deck_with_context(card_id, pre_upgrades, source, ctx)
    }

    pub fn add_card_to_deck_with_omamori_snapshot_from(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
        has_omamori: bool,
        omamori_charges: i32,
    ) -> bool {
        let mut ctx = self.build_deck_context();
        ctx.has_omamori = has_omamori;
        ctx.omamori_charges = omamori_charges;
        self.add_card_to_deck_with_context(card_id, pre_upgrades, source, ctx)
    }

    pub(super) fn add_card_to_deck_with_context(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
        ctx: crate::state::deck::context::DeckContext,
    ) -> bool {
        let mut target_uuid = self.next_card_uuid();

        let result = crate::state::deck::manager::DeckManager::obtain_card(
            &ctx,
            card_id,
            &mut target_uuid,
            pre_upgrades,
        );
        let mut was_added = false;

        self.resolve_deck_actions(result.actions, source);

        if !result.final_cards.is_empty() {
            was_added = true;
            for card in result.final_cards {
                self.emit_event(DomainEvent::CardObtained {
                    card: Self::snapshot_card(&card),
                    source,
                });
                self.master_deck.push(card);
            }
            self.dispatch_on_master_deck_change();
        }

        was_added
    }

    pub fn add_card_to_deck_without_interception_from(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
    ) -> bool {
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid();

        let result = crate::state::deck::manager::DeckManager::obtain_card_without_interception(
            &ctx,
            card_id,
            &mut target_uuid,
            pre_upgrades,
        );
        let mut was_added = false;

        self.resolve_deck_actions(result.actions, source);

        if !result.final_cards.is_empty() {
            was_added = true;
            for card in result.final_cards {
                self.emit_event(DomainEvent::CardObtained {
                    card: Self::snapshot_card(&card),
                    source,
                });
                self.master_deck.push(card);
            }
            self.dispatch_on_master_deck_change();
        }

        was_added
    }

    /// Adds a stat-equivalent copy of an existing master-deck card.
    ///
    /// This is the headless equivalent of Java's `makeStatEquivalentCopy()`
    /// followed by `ShowCardAndObtainEffect`: obtain hooks still run, but
    /// persistent per-card state such as `misc`, cost flags, and base-stat
    /// mutations is preserved on the obtained copy. Transient rendered
    /// damage/block/magic values are not copied. Bottle ownership is
    /// represented by relic UUIDs in Rust, so the new UUID naturally clears
    /// bottle attachment.
    pub fn add_card_instance_copy_to_deck_from(
        &mut self,
        template: &crate::runtime::combat::CombatCard,
        source: DomainEventSource,
    ) -> bool {
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid();

        let result = crate::state::deck::manager::DeckManager::obtain_card(
            &ctx,
            template.id,
            &mut target_uuid,
            template.upgrades,
        );
        let mut was_added = false;

        self.resolve_deck_actions(result.actions, source);

        if !result.final_cards.is_empty() {
            was_added = true;
            for mut card in result.final_cards {
                card.misc_value = template.misc_value;
                card.base_damage_override = template.base_damage_override;
                card.base_block_override = template.base_block_override;
                card.cost_modifier = template.cost_modifier;
                card.cost_for_turn = template.cost_for_turn;
                card.free_to_play_once = template.free_to_play_once;
                self.emit_event(DomainEvent::CardObtained {
                    card: Self::snapshot_card(&card),
                    source,
                });
                self.master_deck.push(card);
            }
            self.dispatch_on_master_deck_change();
        }

        was_added
    }

    /// Removes a specific card instance from the master deck.
    /// Handles Parasite triggers, Necronomicurse regeneration.
    pub fn remove_card_from_deck(&mut self, uuid: u32) {
        self.remove_card_from_deck_with_source(uuid, DomainEventSource::DeckMutation);
    }

    pub fn remove_card_from_deck_with_source(&mut self, uuid: u32, source: DomainEventSource) {
        if let Some(removed) =
            self.remove_card_from_deck_without_removal_hooks_with_source(uuid, source)
        {
            let result = crate::state::deck::manager::DeckManager::remove_card(removed.id);
            self.resolve_deck_actions(result.actions, source);
            self.dispatch_on_master_deck_change();
        }
    }

    pub fn remove_card_from_deck_without_removal_hooks_with_source(
        &mut self,
        uuid: u32,
        source: DomainEventSource,
    ) -> Option<DomainCardSnapshot> {
        let pos = self.master_deck.iter().position(|c| c.uuid == uuid)?;
        let removed = self.master_deck.remove(pos);
        let snapshot = Self::snapshot_card(&removed);
        self.emit_event(DomainEvent::CardRemoved {
            card: snapshot,
            source,
        });
        Some(snapshot)
    }

    pub(super) fn build_deck_context(&self) -> crate::state::deck::context::DeckContext {
        use crate::content::relics::RelicId;
        let mut omamori_charges = 0;
        let mut has_omamori = false;

        for relic in &self.relics {
            if relic.id == RelicId::Omamori {
                has_omamori = true;
                omamori_charges = relic.counter;
            }
        }

        crate::state::deck::context::DeckContext {
            has_hoarder_mod: false,
            has_omamori,
            omamori_charges,
            has_ceramic_fish: self.relics.iter().any(|r| r.id == RelicId::CeramicFish),
            has_darkstone_periapt: self
                .relics
                .iter()
                .any(|r| r.id == RelicId::DarkstonePeriapt),
            has_molten_egg: self.relics.iter().any(|r| r.id == RelicId::MoltenEgg),
            has_toxic_egg: self.relics.iter().any(|r| r.id == RelicId::ToxicEgg),
            has_frozen_egg: self.relics.iter().any(|r| r.id == RelicId::FrozenEgg),
        }
    }

    pub fn preview_obtain_card_upgrades(
        &self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
    ) -> u8 {
        let ctx = self.build_deck_context();
        crate::state::deck::manager::DeckManager::preview_obtain_upgrades(
            &ctx,
            card_id,
            pre_upgrades,
        )
    }

    pub(super) fn resolve_deck_actions(
        &mut self,
        actions: Vec<crate::state::deck::manager::DeckAction>,
        source: DomainEventSource,
    ) {
        use crate::state::deck::manager::DeckAction;
        for action in actions {
            match action {
                DeckAction::PreventObtain => { /* Handled structurally */ }
                DeckAction::GainGold(amount) => {
                    self.change_gold_with_source(amount, source);
                }
                DeckAction::GainMaxHp(amount) => {
                    self.gain_max_hp_with_source(amount, amount, source);
                }
                DeckAction::LoseMaxHp(amount) => {
                    self.lose_max_hp_with_source(amount, source);
                }
                DeckAction::UpdateRelicCounter(relic_id, counter) => {
                    if let Some(relic) = self.relics.iter_mut().find(|r| r.id == relic_id) {
                        relic.counter = counter;
                        if counter == 0 && relic_id == crate::content::relics::RelicId::Omamori {
                            relic.used_up = true;
                        }
                    }
                }
                DeckAction::ReaddCardToMasterDeck(card_id) => {
                    self.readd_card_to_master_deck_without_obtain_hooks(card_id, source);
                }
            }
        }
    }

    pub(super) fn readd_card_to_master_deck_without_obtain_hooks(
        &mut self,
        card_id: crate::content::cards::CardId,
        source: DomainEventSource,
    ) {
        let card = crate::runtime::combat::CombatCard::new(card_id, self.next_card_uuid());
        self.emit_event(DomainEvent::CardObtained {
            card: Self::snapshot_card(&card),
            source,
        });
        self.master_deck.push(card);
    }

    /// Triggers AbstractRelic.onMasterDeckChange for all relics
    pub fn dispatch_on_master_deck_change(&mut self) {
        crate::content::relics::du_vu_doll::refresh_counters_from_deck(
            &self.master_deck,
            &mut self.relics,
        );
    }

    /// Returns a deterministic fresh-ish UUID for new master-deck cards.
    ///
    /// Starter cards use small UUIDs. Obtained cards use the 10000+ range, and
    /// this must stay above existing obtained-card UUIDs after removals so a
    /// remove-then-obtain path does not collide with an existing card instance.
    pub fn next_card_uuid(&self) -> u32 {
        self.master_deck
            .iter()
            .map(|card| card.uuid)
            .max()
            .map_or(10000, |uuid| (uuid + 1).max(10000))
    }

    /// Shuffles upgradable cards in the master deck and upgrades up to `count`.
    /// Mirrors Java ShiningLight's upgrade logic using miscRng for shuffling.
    pub fn upgrade_random_cards(&mut self, count: usize) {
        self.upgrade_random_cards_with_source(count, DomainEventSource::DeckMutation);
    }

    pub fn upgrade_random_cards_with_source(&mut self, count: usize, source: DomainEventSource) {
        // Collect indices of upgradable cards
        let mut upgradable_indices: Vec<usize> = self
            .master_deck
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                let def = crate::content::cards::get_card_definition(c.id);
                // A card can be upgraded if it hasn't been upgraded yet (for most cards)
                // Searing Blow can be upgraded infinitely, so always qualifies
                c.id == crate::content::cards::CardId::SearingBlow
                    || c.upgrades == 0
                        && def.card_type != crate::content::cards::CardType::Status
                        && def.card_type != crate::content::cards::CardType::Curse
            })
            .map(|(i, _)| i)
            .collect();

        // Shuffle using miscRng.randomLong() seed (mirrors Java's Collections.shuffle)
        crate::runtime::rng::shuffle_with_random_long(
            &mut upgradable_indices,
            &mut self.rng_pool.misc_rng,
        );

        // Upgrade up to `count` cards
        for &idx in upgradable_indices.iter().take(count) {
            let uuid = self.master_deck[idx].uuid;
            self.upgrade_card_with_source(uuid, source);
        }
    }

    /// Upgrades a specific card in the master deck by its UUID.
    pub fn upgrade_card(&mut self, uuid: u32) {
        self.upgrade_card_with_source(uuid, DomainEventSource::DeckMutation);
    }

    pub fn upgrade_card_with_source(&mut self, uuid: u32, source: DomainEventSource) {
        if let Some(card) = self.master_deck.iter_mut().find(|c| c.uuid == uuid) {
            let before = Self::snapshot_card(card);
            if crate::content::cards::upgrade_card_once_java(card) {
                let after = Self::snapshot_card(card);
                self.emit_event(DomainEvent::CardUpgraded {
                    before,
                    after,
                    source,
                });
            }
        }
    }

    pub fn modify_card_misc_value(&mut self, uuid: u32, amount: i32) {
        if let Some(card) = self.master_deck.iter_mut().find(|c| c.uuid == uuid) {
            card.misc_value += amount;
        }
    }
}
