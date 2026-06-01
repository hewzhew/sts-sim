use super::*;

impl RunState {
    /// Transforms a card: removes it from deck and replaces with a random card of the same color.
    /// Uses DeckManager properly so Omamori/Necronomicurse triggers fire correctly.
    /// `auto_upgrade` is true when transforming via Astrolabe.
    pub fn transform_card(&mut self, deck_index: usize, auto_upgrade: bool) {
        self.transform_card_with_source(deck_index, auto_upgrade, DomainEventSource::DeckMutation);
    }

    pub fn transform_card_uuid_with_source(
        &mut self,
        uuid: u32,
        auto_upgrade: bool,
        source: DomainEventSource,
    ) {
        if let Some(deck_index) = self.master_deck.iter().position(|card| card.uuid == uuid) {
            self.transform_card_with_source(deck_index, auto_upgrade, source);
        }
    }

    pub fn transform_card_uuids_with_source(
        &mut self,
        uuids: &[u32],
        auto_upgrade: bool,
        source: DomainEventSource,
    ) {
        for &uuid in uuids {
            self.transform_card_uuid_with_source(uuid, auto_upgrade, source);
        }
    }

    pub fn transform_card_uuids_after_removing_all_with_source(
        &mut self,
        uuids: &[u32],
        auto_upgrade: bool,
        source: DomainEventSource,
    ) {
        let removed = uuids
            .iter()
            .filter_map(|&uuid| self.remove_card_for_transform_with_source(uuid, source))
            .collect::<Vec<_>>();

        for before in removed {
            let new_id = self.transform_result_card_id(before.id, source);
            self.obtain_transformed_card(before, new_id, auto_upgrade, source);
        }
    }

    pub fn transform_card_uuids_deferred_obtain_with_source(
        &mut self,
        uuids: &[u32],
        auto_upgrade: bool,
        source: DomainEventSource,
    ) {
        let mut transformed = Vec::new();

        for &uuid in uuids {
            if let Some(before) = self.remove_card_for_transform_with_source(uuid, source) {
                let new_id = self.transform_result_card_id(before.id, source);
                transformed.push((before, new_id));
            }
        }

        for (before, new_id) in transformed {
            self.obtain_transformed_card(before, new_id, auto_upgrade, source);
        }
    }

    pub fn transform_card_with_source(
        &mut self,
        deck_index: usize,
        auto_upgrade: bool,
        source: DomainEventSource,
    ) {
        if deck_index >= self.master_deck.len() {
            return;
        }

        let old_card_uuid = self.master_deck[deck_index].uuid;
        if let Some(before) = self.remove_card_for_transform_with_source(old_card_uuid, source) {
            let new_id = self.transform_result_card_id(before.id, source);
            self.obtain_transformed_card(before, new_id, auto_upgrade, source);
        }
    }

    pub(super) fn remove_card_for_transform_with_source(
        &mut self,
        uuid: u32,
        source: DomainEventSource,
    ) -> Option<DomainCardSnapshot> {
        let pos = self.master_deck.iter().position(|card| card.uuid == uuid)?;
        let removed = self.master_deck.remove(pos);
        let before = Self::snapshot_card(&removed);
        let remove_result = crate::state::deck::manager::DeckManager::remove_card(removed.id);
        self.resolve_deck_actions(remove_result.actions, source);
        self.dispatch_on_master_deck_change();
        Some(before)
    }

    pub(super) fn transform_result_card_id(
        &mut self,
        old_card_id: crate::content::cards::CardId,
        source: DomainEventSource,
    ) -> crate::content::cards::CardId {
        use crate::content::cards::*;

        let def = crate::content::cards::get_card_definition(old_card_id);
        if def.card_type == CardType::Curse {
            let curse_pool = get_curse_pool();
            let filtered: Vec<CardId> = curse_pool
                .iter()
                .copied()
                .filter(|&c| c != old_card_id) // Java logic: CardLibrary.getCurse(c, rng)
                .collect();
            if filtered.is_empty() {
                CardId::Clumsy
            } else {
                let idx = self.transform_random_index(filtered.len(), source);
                filtered[idx]
            }
        } else if COLORLESS_UNCOMMON_POOL.contains(&old_card_id)
            || COLORLESS_RARE_POOL.contains(&old_card_id)
            || old_card_id == CardId::Madness
            || old_card_id == CardId::JAX
            || old_card_id == CardId::Apparition
            || old_card_id == CardId::Bite
            || old_card_id == CardId::RitualDagger
            || old_card_id == CardId::Shiv
            || old_card_id == CardId::Finesse
        {
            let pool = COLORLESS_UNCOMMON_POOL
                .iter()
                .chain(COLORLESS_RARE_POOL.iter())
                .copied()
                .filter(|&c| c != old_card_id)
                .collect::<Vec<_>>();
            if pool.is_empty() {
                old_card_id
            } else {
                let idx = self.transform_random_index(pool.len(), source);
                pool[idx]
            }
        } else {
            // Java: returnTrulyRandomCardFromAvailable(c, rng)
            let pool: Vec<CardId> = crate::engine::campfire_handler::card_pool_for_class(
                self.player_class,
                CardRarity::Common,
            )
            .iter()
            .chain(
                crate::engine::campfire_handler::card_pool_for_class(
                    self.player_class,
                    CardRarity::Uncommon,
                )
                .iter(),
            )
            .chain(
                crate::engine::campfire_handler::card_pool_for_class(
                    self.player_class,
                    CardRarity::Rare,
                )
                .iter(),
            )
            .copied()
            .filter(|&c| c != old_card_id)
            .collect();
            if pool.is_empty() {
                old_card_id
            } else {
                let idx = self.transform_random_index(pool.len(), source);
                pool[idx]
            }
        }
    }

    pub(super) fn obtain_transformed_card(
        &mut self,
        before: DomainCardSnapshot,
        new_id: crate::content::cards::CardId,
        auto_upgrade: bool,
        source: DomainEventSource,
    ) {
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid(); // This is just the base UUID, DeckManager will increment for actual insertions

        let pre_upgrades = if auto_upgrade {
            let transformed_card = crate::runtime::combat::CombatCard::new(new_id, target_uuid);
            u8::from(crate::content::cards::can_upgrade_card_once(
                &transformed_card,
            ))
        } else {
            0
        };

        let result = crate::state::deck::manager::DeckManager::obtain_card(
            &ctx,
            new_id,
            &mut target_uuid,
            pre_upgrades,
        );

        self.resolve_deck_actions(result.actions, source);

        // 3. Obtain
        let mut obtained_any = false;
        for card in result.final_cards {
            self.emit_event(DomainEvent::CardTransformed {
                before,
                after: Self::snapshot_card(&card),
                source,
            });
            self.master_deck.push(card);
            obtained_any = true;
        }
        if obtained_any {
            self.dispatch_on_master_deck_change();
        }
    }

    pub(super) fn transform_random_index(
        &mut self,
        len: usize,
        source: DomainEventSource,
    ) -> usize {
        if len == 0 {
            return 0;
        }
        let rng = if source == DomainEventSource::Event(crate::state::events::EventId::Neow) {
            &mut self.neow_rng
        } else {
            &mut self.rng_pool.misc_rng
        };
        rng.random_range(0, len as i32 - 1) as usize
    }
}
