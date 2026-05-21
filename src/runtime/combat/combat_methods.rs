use super::*;

// Derived combat runtime state that is recomputed from other sources.
impl CombatState {
    pub fn recompute_turn_start_draw_modifier(&mut self) {
        let mut modifier = 0;
        if self
            .entities
            .player
            .has_relic(crate::content::relics::RelicId::RingOfTheSerpent)
        {
            modifier += 1;
        }
        if let Some(powers) = crate::content::powers::store::powers_for(self, 0) {
            for power in powers {
                match power.power_type {
                    PowerId::Draw => modifier += power.amount,
                    PowerId::DrawReduction => modifier -= power.amount,
                    _ => {}
                }
            }
        }
        self.turn.turn_start_draw_modifier = modifier;
    }
}

// Card-zone utilities used by action handlers to reconcile card movement.
impl CombatState {
    pub fn next_card_uuid(&mut self) -> u32 {
        self.zones.card_uuid_counter += 1;
        self.zones.card_uuid_counter
    }

    pub fn next_power_instance_id(&mut self) -> u32 {
        self.runtime.power_instance_counter += 1;
        self.runtime.power_instance_counter
    }

    /// Java `MonsterGroup.areMonstersBasicallyDead()` only skips monsters that
    /// are `isDying` or `isEscaping`. It does not check current HP and does not
    /// treat `halfDead` as basically dead by itself.
    pub fn are_monsters_basically_dead_java(&self) -> bool {
        self.entities
            .monsters
            .iter()
            .all(|m| m.is_dying || m.is_escaped)
    }

    /// Java `MonsterGroup.haveMonstersEscaped()` returns true only when every
    /// monster has its `escaped` flag set. Dying/dead monsters do not count.
    pub fn have_monsters_escaped_java(&self) -> bool {
        self.entities.monsters.iter().all(|m| m.is_escaped)
    }

    pub fn add_card_to_draw_pile_top(&mut self, card: CombatCard) {
        self.zones.add_to_draw_pile_top(card);
    }

    pub fn add_card_to_draw_pile_bottom(&mut self, card: CombatCard) {
        self.zones.add_to_draw_pile_bottom(card);
    }

    pub fn add_card_to_draw_pile_random_spot(&mut self, card: CombatCard) {
        let java_insert_index = if self.zones.draw_pile.is_empty() {
            0
        } else {
            self.rng
                .card_random_rng
                .random(self.zones.draw_pile.len() as i32 - 1) as usize
        };
        self.zones
            .add_to_draw_pile_random_spot_from_java_index(card, java_insert_index);
    }

    pub fn draw_top_card(&mut self) -> Option<CombatCard> {
        self.zones.draw_top_card()
    }

    pub fn add_card_to_discard_pile_top(&mut self, card: CombatCard) {
        self.zones.add_to_discard_pile_top(card);
    }

    pub fn add_card_to_exhaust_pile_top(&mut self, card: CombatCard) {
        self.zones.add_to_exhaust_pile_top(card);
    }

    pub fn shuffle_discard_pile_into_draw_pile(&mut self) {
        self.zones.draw_pile.append(&mut self.zones.discard_pile);
        crate::runtime::rng::shuffle_with_random_long(
            &mut self.zones.draw_pile,
            &mut self.rng.shuffle_rng,
        );
        // Java draw-pile top is the end of CardGroup.group. Rust draw-pile top
        // is index 0, so reverse only after preserving Java shuffle order.
        self.zones.draw_pile.reverse();
    }

    pub fn apply_java_initialize_deck_order_after_shuffle(&mut self) {
        let bottled_uuids: Vec<u32> = self
            .entities
            .player
            .relics
            .iter()
            .filter_map(|relic| match relic.id {
                crate::content::relics::RelicId::BottledFlame
                | crate::content::relics::RelicId::BottledLightning
                | crate::content::relics::RelicId::BottledTornado
                    if relic.amount > 0 =>
                {
                    Some(relic.amount as u32)
                }
                _ => None,
            })
            .collect();
        let mut java_group_order = Vec::new();
        let mut place_on_top = Vec::new();
        for card in std::mem::take(&mut self.zones.draw_pile) {
            if crate::content::cards::is_innate_card(&card) || bottled_uuids.contains(&card.uuid) {
                place_on_top.push(card);
            } else {
                java_group_order.push(card);
            }
        }
        java_group_order.extend(place_on_top);
        java_group_order.reverse();
        self.zones.draw_pile = java_group_order;
    }

    /// Helper to find a card by UUID in a specific slice and remove it. Returns the removed card.
    pub fn remove_card_by_uuid(pile: &mut Vec<CombatCard>, uuid: u32) -> Option<CombatCard> {
        if let Some(index) = pile.iter().position(|c| c.uuid == uuid) {
            Some(pile.remove(index))
        } else {
            None
        }
    }

    /// Looks everywhere for a card and removes it. Useful for UseCard when we don't know exactly where the card went.
    pub fn take_card_from_anywhere(&mut self, uuid: u32) -> Option<CombatCard> {
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.hand, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.limbo, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.draw_pile, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.discard_pile, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.exhaust_pile, uuid) {
            return Some(c);
        }
        None
    }
}

// Lightweight read helpers over combat-owned runtime state.
impl CombatState {
    /// Gets the current stack amount of a specific power on an entity
    pub fn get_power(&self, target: EntityId, power_id: PowerId) -> i32 {
        crate::content::powers::store::power_amount(self, target, power_id)
    }
}

// Queue-sensitive runtime helpers for Java cardQueue approximations.
impl CombatState {
    /// Best-effort approximation of Java's cardQueue membership for effects that
    /// should avoid already-queued cards (for example Mummified Hand).
    ///
    /// We do not model AbstractDungeon.actionManager.cardQueue explicitly, but cards
    /// already in limbo or already wrapped in queued play actions should not be treated
    /// as normal in-hand candidates.
    pub fn reserved_card_uuids_for_queue_sensitive_effects(&self) -> HashSet<u32> {
        let mut reserved = HashSet::new();
        for card in &self.zones.limbo {
            reserved.insert(card.uuid);
        }
        for queued in &self.zones.queued_cards {
            reserved.insert(queued.card.uuid);
        }
        for queued in &self.runtime.card_queue {
            reserved.insert(queued.card_uuid);
        }
        for action in &self.engine.action_queue {
            match action {
                Action::EnqueueCardPlay { item, .. } => {
                    reserved.insert(item.card.uuid);
                }
                Action::PlayCardDirect { card, .. } => {
                    reserved.insert(card.uuid);
                }
                Action::UseCard { uuid, .. } => {
                    reserved.insert(*uuid);
                }
                _ => {}
            }
        }
        reserved
    }

    pub fn enqueue_card_play(&mut self, item: QueuedCardPlay, in_front: bool) {
        let was_empty = self.zones.queued_cards.is_empty();
        if in_front {
            self.zones.queued_cards.push_front(item);
        } else {
            self.zones.queued_cards.push_back(item);
        }
        if was_empty {
            self.queue_action_back(Action::FlushNextQueuedCard);
        }
    }

    pub fn colorless_combat_pool(&self) -> Vec<CardId> {
        if !self.runtime.colorless_combat_pool.is_empty() {
            self.runtime.colorless_combat_pool.clone()
        } else {
            crate::content::cards::random_colorless_in_combat_pool()
        }
    }
}

// Hand re-evaluation helpers used after state-changing effects.
impl CombatState {
    /// Reparses all cards in the hand to dynamically calculate damage, block, and magic numbers.
    /// Clones the hand to satisfy borrow-checker while allowing `PerfectedStrike` to read `&self.hand`.
    pub fn update_hand_cards(&mut self) {
        let mut new_hand = self.zones.hand.clone();
        for card in &mut new_hand {
            crate::content::cards::evaluate_card(card, self, None);
        }
        self.zones.hand = new_hand;
    }
}
