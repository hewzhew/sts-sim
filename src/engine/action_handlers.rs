use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatState, Intent};
use crate::content::cards::CardId;
use crate::content::powers::PowerId;

/// Executes a single atomic Action off the queue.
pub fn execute_action(action: Action, state: &mut CombatState) {
    match action {
        Action::Damage(info) => {
            let target_id = info.target;
            let source_id = info.source;
            
            // Apply power modifiers if not already applied (is_modified == false)
            // For monster attacks: use entity.intent_dmg (synced from Java's move_adjusted_damage)
            // which already includes Strength, Weak, Vulnerable, etc.
            // IMPORTANT: Only use intent_dmg for Normal damage (actual attacks).
            // THORNS/HpLoss damage from powers (SharpHide, etc.) should use the raw output.
            let calculated_output = if !info.is_modified && source_id != 0 
                && info.damage_type == crate::action::DamageType::Normal {
                // Look up source monster's pre-calculated damage
                if let Some(monster) = state.monsters.iter().find(|m| m.id == source_id) {
                    if monster.intent_dmg > 0 {
                        monster.intent_dmg
                    } else {
                        info.output.max(0) // Fallback to raw output
                    }
                } else {
                    info.output.max(0)
                }
            } else {
                info.output.max(0)
            };
            
            let mut final_damage = calculated_output;
            let target_is_player = target_id == 0;

            // 1. Final Receive / Intangible Pre-Check
            // Java AST: IF (hasPower(\"Intangible\")) -> cap damage to 1 BEFORE block logic
            if let Some(target_powers) = state.power_db.get(&target_id).cloned() {
                for power in &target_powers {
                    final_damage = crate::content::powers::resolve_power_at_damage_final_receive(
                        power.power_type,
                        final_damage,
                        power.amount,
                        info.damage_type,
                    );
                }
            }

            if target_is_player {
                // 2. Block Deduction
                let _had_block = state.player.block > 0;
                if state.player.block > 0 {
                    if final_damage >= state.player.block {
                        final_damage -= state.player.block;
                        state.player.block = 0;
                    } else {
                        state.player.block -= final_damage;
                        final_damage = 0;
                    }
                }

                // 3. onAttackedToChangeDamage (Relics then Powers)
                // Java L1392-1397: Relics first, then Powers
                final_damage = crate::content::relics::hooks::on_attacked_to_change_damage(state, final_damage, &info);
                if let Some(powers) = state.power_db.get(&0).cloned() {
                    for power in &powers {
                        final_damage = crate::content::powers::resolve_power_on_attacked_to_change_damage(
                            power.power_type, state, &info, final_damage, power.amount
                        );
                    }
                }

                // 4. on_attacked (Target Powers + Relics) — guarded by owner != null
                // Java L1403: if (info.owner != null) { ... onAttacked ... } else { skip }
                if source_id != 0 || info.damage_type == crate::action::DamageType::Normal {
                    // owner is non-null equivalent
                    if let Some(powers) = state.power_db.get(&0).cloned() {
                        for power in &powers {
                            let hook_actions = crate::content::powers::resolve_power_on_attacked(
                                power.power_type, state, 0, final_damage, source_id, power.amount
                            );
                            for a in hook_actions.into_iter().rev() {
                                state.action_queue.push_front(a);
                            }
                        }
                    }
                }

                // 5. onLoseHpLast (Tungsten Rod) — Java L1416: runs UNCONDITIONALLY
                // Not guarded by damageAmount > 0
                final_damage = crate::content::relics::hooks::on_lose_hp_last(state, final_damage);

                if final_damage > 0 {
                    // 6. Power onLoseHp (modifies damageAmount) + Relic onLoseHp (notification)
                    // Java L1421-1426: powers modify, relics notify — ALL damage types
                    let lose_hp_actions = crate::content::relics::hooks::on_lose_hp(state, final_damage);
                    crate::engine::core::queue_actions(&mut state.action_queue, lose_hp_actions);

                    state.player.current_hp = (state.player.current_hp - final_damage).max(0);
                    state.counters.times_damaged_this_combat += 1;
                    
                    // 7. Death Check — Java L1465-1481: MarkOfBloom → Fairy → LizardTail
                    if state.player.current_hp <= 0 {
                        let has_mark_of_bloom = state.player.has_relic(crate::content::relics::RelicId::MarkOfTheBloom);
                        if !has_mark_of_bloom {
                            // Try Fairy first
                            if let Some(fairy_slot) = state.potions.iter().position(|p| {
                                p.as_ref().map_or(false, |pot| pot.id == crate::content::potions::PotionId::FairyPotion)
                            }) {
                                let max_hp = state.player.max_hp as f32;
                                let mut heal_pct = 0.3_f32;
                                if state.player.has_relic(crate::content::relics::RelicId::SacredBark) {
                                    heal_pct *= 2.0;
                                }
                                let heal_amount = (max_hp * heal_pct) as i32;
                                state.player.current_hp = heal_amount.max(1);
                                state.potions[fairy_slot] = None;
                            } else if state.player.has_relic(crate::content::relics::RelicId::LizardTail) {
                                // Java L1476: LizardTail — counter == -1 means unused
                                let lizard_unused = state.player.relics.iter()
                                    .find(|r| r.id == crate::content::relics::RelicId::LizardTail)
                                    .map_or(false, |r| !r.used_up);
                                if lizard_unused {
                                    state.player.current_hp = 0;
                                    let heal_amount = std::cmp::max(1, state.player.max_hp / 2);
                                    state.action_queue.push_front(Action::Heal { target: 0, amount: heal_amount });
                                    // Mark LizardTail as used
                                    if let Some(lt) = state.player.relics.iter_mut()
                                        .find(|r| r.id == crate::content::relics::RelicId::LizardTail) {
                                        lt.used_up = true;
                                        lt.counter = -2;
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let Some(mut m) = state.monsters.iter().find(|m| m.id == target_id).cloned() {
                // Java L614-616: Skip damage to dying/escaping monsters
                if m.is_dying {
                    return;
                }

                // Damage to monster
                let had_block = m.block > 0;
                if m.block > 0 {
                    if final_damage >= m.block {
                        final_damage -= m.block;
                        m.block = 0;
                    } else {
                        m.block -= final_damage;
                        final_damage = 0;
                    }
                }

                // Java L626-630: onAttackToChangeDamage (player relics — Boot)
                if source_id == 0
                    && info.damage_type == crate::action::DamageType::Normal
                    && final_damage > 0 && final_damage < 5
                    && state.player.has_relic(crate::content::relics::RelicId::Boot)
                {
                    final_damage = 5;
                }

                // Java L636-638: Monster powers onAttackedToChangeDamage
                if let Some(powers) = state.power_db.get(&target_id).cloned() {
                    for power in &powers {
                        final_damage = crate::content::powers::resolve_power_on_attacked_to_change_damage(
                            power.power_type, state, &info, final_damage, power.amount
                        );
                    }
                }

                // Write back block to real monster and apply HP loss
                if let Some(real_m) = state.monsters.iter_mut().find(|monster| monster.id == target_id) {
                    real_m.block = m.block;
                    if final_damage > 0 {
                        real_m.current_hp = (real_m.current_hp - final_damage).max(0);
                    }
                }

                // HandDrill: if block broke (was >0, now 0), apply 2 Vulnerable
                if had_block && m.block == 0 && state.player.has_relic(crate::content::relics::RelicId::HandDrill) {
                    let hand_drill_actions = crate::content::relics::hand_drill::on_break_block(state, target_id);
                    crate::engine::core::queue_actions(&mut state.action_queue, hand_drill_actions);
                }

                // Java L644-646: wasHPLost fires BEFORE onAttacked for monsters!
                // on_hp_lost power hooks (ModeShift, Split, etc.)
                if final_damage > 0 {
                    if let Some(powers) = state.power_db.get(&target_id).cloned() {
                        for power in &powers {
                            let hook_actions = crate::content::powers::resolve_power_on_hp_lost(
                                power.power_type, state, target_id, final_damage
                            );
                            for a in hook_actions {
                                state.action_queue.push_front(a);
                            }
                        }
                    }
                }

                // Java L652-654: Monster onAttacked (Thorns, CurlUp, Angry, etc.)
                let should_fire_monster_on_attacked = info.damage_type != crate::action::DamageType::Thorns
                    && info.damage_type != crate::action::DamageType::HpLoss;
                if should_fire_monster_on_attacked {
                    if let Some(powers) = state.power_db.get(&target_id).cloned() {
                        for power in &powers {
                            let hook_actions = crate::content::powers::resolve_power_on_attacked(
                                power.power_type, state, target_id, final_damage, source_id, power.amount
                            );
                            if power.power_type == PowerId::Malleable {
                                for a in hook_actions {
                                    state.action_queue.push_back(a);
                                }
                                if let Some(powers_mut) = state.power_db.get_mut(&target_id) {
                                    if let Some(mal) = powers_mut.iter_mut().find(|p| p.power_type == PowerId::Malleable) {
                                        if final_damage > 0 {
                                            mal.amount += 1;
                                        }
                                    }
                                }
                            } else {
                                for a in hook_actions {
                                    state.action_queue.push_front(a);
                                }
                            }
                        }
                    }
                }
                // CurlUp: zero amount after dispatch to prevent re-trigger on multi-hit
                if let Some(powers) = state.power_db.get_mut(&target_id) {
                    if let Some(curl) = powers.iter_mut().find(|p| p.power_type == PowerId::CurlUp) {
                        if curl.amount > 0 && final_damage > 0 {
                            curl.amount = 0;
                        }
                    }
                }

                // Monster death check — fire on_monster_death relic hooks (GremlinHorn, TheSpecimen)
                // Java: AbstractCreature.damage() → this.isDying = true, then relic hooks fire
                let mut is_darkling_or_awakened = false;
                let mut is_gremlin_leader = false;
                let mut triggered_death = false;
                let mut dying_monster_type: Option<crate::content::monsters::EnemyId> = None;

                if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
                    if m.current_hp <= 0 && !m.is_dying {
                        m.is_dying = true;
                        let m_id = crate::content::monsters::EnemyId::from_id(m.monster_type);
                        dying_monster_type = m_id;
                        is_gremlin_leader = m_id == Some(crate::content::monsters::EnemyId::GremlinLeader);
                        is_darkling_or_awakened = m_id == Some(crate::content::monsters::EnemyId::Darkling) || m_id == Some(crate::content::monsters::EnemyId::AwakenedOne);
                        triggered_death = true;
                    }
                }

                if triggered_death {
                    if let Some(m_id) = dying_monster_type {
                        if !is_darkling_or_awakened {
                            // Fetch monster entity clone to satisfy borrow checker
                            let m_clone = state.monsters.iter().find(|m| m.id == target_id).unwrap().clone();
                            let death_actions_on_entity = crate::content::monsters::resolve_on_death(m_id, state, &m_clone);
                            for a in death_actions_on_entity {
                                state.action_queue.push_back(a);
                            }
                        }
                    }

                    let death_actions = crate::content::relics::hooks::on_monster_death(state, target_id);
                    crate::engine::core::queue_actions(&mut state.action_queue, death_actions);

                    if is_gremlin_leader {
                        let minion_ids: Vec<_> = state.monsters.iter()
                            .filter(|min| min.id != target_id && !min.is_dying)
                            .map(|min| min.id)
                            .collect();
                        for minion_id in minion_ids {
                            state.action_queue.push_back(Action::Escape { target: minion_id });
                        }
                    }
                    if is_darkling_or_awakened {
                        if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
                            m.current_hp = 0; // Ensures it is dead but is_dying is reset to false, so it's not swept yet.
                            m.is_dying = false;
                            m.current_intent = crate::combat::Intent::Unknown;
                        }
                    }
                }
            }
        },
        Action::ApplyPower { source, target, power_id, mut amount } => {
            // === Java ApplyPowerAction constructor (inline at creation time) ===

            // C1: Snake Skull → +1 Poison (Java constructor L38-42)
            if amount > 0 && power_id == PowerId::Poison && state.player.has_relic(crate::content::relics::RelicId::SneckoSkull) {
                if source == 0 && target != 0 {
                    amount += 1;
                }
            }

            // === Java ApplyPowerAction.update() ===

            // U1: Dead/Escaped target guard (Java L91-94)
            if target == 0 {
                // Player target — always valid
            } else if let Some(m) = state.monsters.iter().find(|m| m.id == target) {
                if m.is_dying || m.current_hp <= 0 {
                    return; // Skip applying power to dead/escaping monster
                }
            }

            // U3: source.powers.forEach(p -> p.onApplyPower()) — Sadistic Nature (Java L100-103)
            if source != 0 || target != 0 { // source != null equivalent
                if let Some(source_powers) = state.power_db.get(&source).cloned() {
                    for power in &source_powers {
                        let hook_actions = crate::content::powers::resolve_power_on_apply_power(
                            power.power_type, power.amount, power_id, amount, target, source, state
                        );
                        let hook_actions_4: smallvec::SmallVec<[crate::action::ActionInfo; 4]> = hook_actions.into_iter().collect();
                        crate::engine::core::queue_actions(&mut state.action_queue, hook_actions_4);
                    }
                }
            }

            // U4: Champion Belt — player applies Vulnerable to enemy → also apply Weak (Java L105-107)
            // Note: Java checks !target.hasPower("Artifact") here, but this is redundant because
            // the recursive ApplyPower(Weak) will itself be blocked by Artifact below (U8).
            let champion_belt_actions = crate::content::relics::hooks::on_apply_power(state, power_id, target);
            crate::engine::core::queue_actions(&mut state.action_queue, champion_belt_actions);

            // U5: Monster re-check after hooks (Java L108-111)
            if target != 0 {
                if let Some(m) = state.monsters.iter().find(|m| m.id == target) {
                    if m.is_dying || m.current_hp <= 0 {
                        return;
                    }
                }
            }

            // U6+U7: Ginger (blocks Weak) + Turnip (blocks Frail) — Java L113-124
            // Uses on_receive_power_modify hook which zeroes amount if blocked
            if target == 0 {
                amount = crate::content::relics::hooks::on_receive_power_modify(state, power_id, amount);
                if amount == 0 && crate::content::powers::is_debuff(power_id, amount) {
                    return; // Blocked by Ginger/Turnip
                }
            }

            // U8: Artifact blocks all debuffs (Java L125-131)
            if crate::content::powers::is_debuff(power_id, amount) {
                let has_artifact = state.power_db.get(&target).map_or(false, |powers| {
                    powers.iter().any(|p| p.power_type == PowerId::Artifact)
                });
                if has_artifact {
                    // Consume 1 Artifact stack — Java: Artifact.onSpecificTrigger()
                    // → ReducePowerAction(Artifact, 1) or RemoveSpecificPowerAction
                    if let Some(powers) = state.power_db.get_mut(&target) {
                        if let Some(art) = powers.iter_mut().find(|p| p.power_type == PowerId::Artifact) {
                            art.amount -= 1;
                            if art.amount <= 0 {
                                powers.retain(|p| p.power_type != PowerId::Artifact);
                            }
                        }
                    }
                    return; // Power application completely blocked
                }
            }

            // === Core power application (Java L135-171) ===
            let powers = state.power_db.entry(target).or_insert_with(Vec::new);
            if let Some(existing) = powers.iter_mut().find(|p| p.power_type == power_id) {
                // U9: stackPower (Java L137)
                existing.amount += amount;
                // Combust: Java CombustPower.stackPower() increments hpLoss by 1 per stack
                if power_id == PowerId::Combust {
                    existing.extra_data += 1;
                }
                // Remove power if stacks reach 0 (debuffs being cleansed)
                if existing.amount <= 0 && power_id != PowerId::TimeWarp {
                    powers.retain(|p| p.power_type != power_id);
                }
            } else if amount > 0 {
                // U10: New power — onInitialApplication (Java L159-162)
                let extra_data = match power_id {
                    PowerId::Combust => 1,
                    _ => 0,
                };
                powers.push(crate::combat::Power { power_type: power_id, amount, extra_data, just_applied: true });
            }

            // C2: Corruption on-apply hook (Java constructor L43-59)
            if power_id == PowerId::Corruption {
                crate::content::cards::ironclad::corruption::corruption_on_apply(state);
            }
        },
        Action::RemovePower { target, power_id } => {
            if let Some(powers) = state.power_db.get_mut(&target) {
                powers.retain(|p| p.power_type != power_id);
            }
        },
        Action::RemoveAllDebuffs { target } => {
            if let Some(powers) = state.power_db.get_mut(&target) {
                // Keep only powers that are NOT debuffs
                powers.retain(|p| {
                    !crate::content::powers::is_debuff(p.power_type, p.amount)
                });
            }
        },
        Action::ApplyStasis { target_id } => {
            if state.draw_pile.is_empty() && state.discard_pile.is_empty() {
                return;
            }

            let source_pile_draw = !state.draw_pile.is_empty();
            let source_pile = if source_pile_draw { &state.draw_pile } else { &state.discard_pile };

            // Find all indices matching rarities
            let rarities_to_check = [
                crate::content::cards::CardRarity::Rare,
                crate::content::cards::CardRarity::Uncommon,
                crate::content::cards::CardRarity::Common,
            ];

            let mut candidates = Vec::new();
            for expected_rarity in rarities_to_check {
                for (i, card) in source_pile.iter().enumerate() {
                    let def = crate::content::cards::get_card_definition(card.id);
                    if def.rarity == expected_rarity {
                        candidates.push(i);
                    }
                }
                if !candidates.is_empty() {
                    break;
                }
            }

            // Fallback: if no rare/uncommon/common found (e.g. only Basics/Status/Curses), pick any
            if candidates.is_empty() {
                for i in 0..source_pile.len() {
                    candidates.push(i);
                }
            }

            // Java uses AbstractDungeon.cardRandomRng
            let pick_idx = if candidates.len() > 1 {
                let r = state.rng.card_random_rng.random(candidates.len() as i32 - 1) as usize;
                candidates[r]
            } else {
                candidates[0]
            };

            let card = if source_pile_draw {
                state.draw_pile.remove(pick_idx)
            } else {
                state.discard_pile.remove(pick_idx)
            };

            let uuid = card.uuid as i32;
            state.limbo.push(card);

            // Queue the power application so it goes through standard flow (Artifact block, etc.)
            // Java does addToTop, so we use push_front
            state.action_queue.push_front(Action::ApplyPower {
                source: target_id,
                target: target_id,
                power_id: PowerId::Stasis,
                amount: uuid,
            });
        },
        Action::GainBlock { target, amount } => {
            if target == 0 {
                // Don't give block to dead/dying player
                if state.player.current_hp > 0 {
                    state.player.block += amount;
                }
            } else if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                // Don't give block to dead monsters (Java skips GainBlock on dead/dying creatures)
                if m.current_hp > 0 {
                    m.block += amount;
                }
            }
        },
        Action::GainBlockRandomMonster { source, amount } => {
            // Java GainBlockRandomMonsterAction.update():
            //   excludes: m == source, m.intent == ESCAPE, m.isDying
            //   uses aiRng.random(validMonsters.size() - 1)
            let alive: Vec<usize> = state.monsters.iter()
                .filter(|m| m.id != source
                    && m.current_intent != Intent::Escape
                    && !m.is_dying)
                .map(|m| m.id)
                .collect();
            let target_id = if !alive.is_empty() {
                let idx = state.rng.ai_rng.random(alive.len() as i32 - 1) as usize;
                alive[idx]
            } else {
                source  // fallback to source (self) if no valid targets
            };
            if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
                m.block += amount;
            }
        },
        Action::LoseBlock { target, amount } => {
            if target == 0 {
                state.player.block = (state.player.block - amount).max(0);
            } else if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                m.block = (m.block - amount).max(0);
            }
        },
        Action::GainEnergy { amount } => {
            state.energy = (state.energy as i32 + amount).max(0) as u8;
        },
        Action::LoseHp { target, amount } => {
            if target == 0 {
                state.player.current_hp -= amount;
                if amount > 0 {
                    state.counters.times_damaged_this_combat += 1;
                }
            } else {
                let mut is_darkling_or_awakened = false;
                let mut is_gremlin_leader = false;
                let mut triggered_death = false;
                let mut dying_monster_type: Option<crate::content::monsters::EnemyId> = None;

                if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                    m.current_hp = (m.current_hp - amount).max(0);
                    if m.current_hp <= 0 && !m.is_dying {
                        m.is_dying = true;
                        let m_id = crate::content::monsters::EnemyId::from_id(m.monster_type);
                        dying_monster_type = m_id;
                        is_gremlin_leader = m_id == Some(crate::content::monsters::EnemyId::GremlinLeader);
                        is_darkling_or_awakened = m_id == Some(crate::content::monsters::EnemyId::Darkling) || m_id == Some(crate::content::monsters::EnemyId::AwakenedOne);
                        triggered_death = true;
                    }
                }

                if triggered_death {
                    if let Some(m_id) = dying_monster_type {
                        if !is_darkling_or_awakened {
                            // Fetch monster entity clone to satisfy borrow checker
                            let m_clone = state.monsters.iter().find(|m| m.id == target).unwrap().clone();
                            let death_actions_on_entity = crate::content::monsters::resolve_on_death(m_id, state, &m_clone);
                            for a in death_actions_on_entity {
                                state.action_queue.push_back(a);
                            }
                        }
                    }

                    let death_actions = crate::content::relics::hooks::on_monster_death(state, target);
                    crate::engine::core::queue_actions(&mut state.action_queue, death_actions);

                    if is_gremlin_leader {
                        let minion_ids: Vec<_> = state.monsters.iter()
                            .filter(|min| min.id != target && !min.is_dying)
                            .map(|min| min.id)
                            .collect();
                        for minion_id in minion_ids {
                            state.action_queue.push_back(Action::Escape { target: minion_id });
                        }
                    }
                    if is_darkling_or_awakened {
                        if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                            m.current_hp = 0;
                            m.is_dying = false;
                            m.current_intent = crate::combat::Intent::Unknown;
                        }
                    }
                }
            }
        },
        Action::GainMaxHp { amount } => {
            state.player.max_hp += amount;
            state.player.current_hp = (state.player.current_hp + amount).min(state.player.max_hp);
        },
        Action::LoseMaxHp { target, amount } => {
            if target == 0 {
                state.player.max_hp = (state.player.max_hp - amount).max(1);
                state.player.current_hp = state.player.current_hp.min(state.player.max_hp);
            }
        },
        Action::Heal { target, mut amount } => {
            if amount < 0 {
                let pct = (-amount) as f32 / 100.0;
                if target == 0 {
                    amount = std::cmp::max(1, (state.player.max_hp as f32 * pct) as i32);
                } else if let Some(m) = state.monsters.iter().find(|m| m.id == target) {
                    amount = std::cmp::max(1, (m.max_hp as f32 * pct) as i32);
                }
            }
            if target == 0 {
                state.player.current_hp = (state.player.current_hp + amount).min(state.player.max_hp);
            } else if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                m.current_hp = (m.current_hp + amount).min(m.max_hp);
            }
        },
        Action::DamageAllEnemies { source, damages, damage_type, is_modified } => {
            // Java: DamageAllEnemiesAction calls monster.damage(new DamageInfo(...)) 
            // for each monster, going through the full AbstractCreature.damage() path.
            // We decompose into individual Damage actions pushed to the front of the queue
            // (in reverse order so they execute left-to-right).
            let mut individual_damages: smallvec::SmallVec<[Action; 5]> = smallvec::SmallVec::new();
            for (i, &dmg) in damages.iter().enumerate() {
                if i >= state.monsters.len() { break; }
                let m = &state.monsters[i];
                if m.current_hp <= 0 || m.is_dying || m.is_escaped { continue; }
                individual_damages.push(Action::Damage(crate::action::DamageInfo {
                    source,
                    target: m.id,
                    base: dmg,
                    output: dmg,
                    damage_type,
                    is_modified, // preserve original is_modified flag
                }));
            }
            // Push in reverse so first monster is processed first (push_front = LIFO)
            for action in individual_damages.into_iter().rev() {
                state.action_queue.push_front(action);
            }
        },
        Action::AttackDamageRandomEnemy { base_damage, damage_type: _ } => {
            let alive: Vec<usize> = state.monsters.iter()
                .filter(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped)
                .map(|m| m.id)
                .collect();
            if !alive.is_empty() {
                let idx = state.rng.card_random_rng.random(alive.len() as i32 - 1) as usize;
                let target_id = alive[idx];
                if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
                    let mut final_damage = base_damage.max(0);
                    if m.block > 0 {
                        if final_damage >= m.block {
                            final_damage -= m.block;
                            m.block = 0;
                        } else {
                            m.block -= final_damage;
                            final_damage = 0;
                        }
                    }
                    if final_damage > 0 {
                        m.current_hp = (m.current_hp - final_damage).max(0);
                    }
                }
            }
        },
        Action::DropkickDamageAndEffect { target, damage_info } => {
            // Java DropkickAction.update():
            //   1. check Vulnerable → addToTop(DrawCard), addToTop(GainEnergy)
            //   2. addToTop(DamageAction)
            // Execution order (LIFO): Damage → Energy → Draw
            // We push in reverse order to front (push_front = LIFO)
            
            // Check Vulnerable BEFORE pushing damage (matches Java: check at action execution time)
            let has_vulnerable = state.power_db.get(&target).map_or(false, |powers| {
                powers.iter().any(|p| p.power_type == PowerId::Vulnerable && p.amount > 0)
            });
            
            if has_vulnerable {
                // Push draw first (will execute last due to LIFO)
                state.action_queue.push_front(Action::DrawCards(1));
                // Push energy gain (will execute after damage)
                state.action_queue.push_front(Action::GainEnergy { amount: 1 });
            }
            
            // Push standard Damage action (will execute first due to LIFO)
            // This goes through the full damage pipeline including on_attacked hooks (Malleable, etc.)
            state.action_queue.push_front(Action::Damage(damage_info));
        },
        Action::FiendFire { target, damage_info } => {
            // Exhaust all non-played cards in hand, deal damage per card exhausted
            let hand_cards: Vec<crate::combat::CombatCard> = state.hand.drain(..).collect();
            let count = hand_cards.len();
            for card in hand_cards {
                state.exhaust_pile.push(card);
                let exhaust_actions = crate::content::relics::hooks::on_exhaust(state);
                crate::engine::core::queue_actions(&mut state.action_queue, exhaust_actions);
            }
            // Deal damage once per exhausted card
            for _ in 0..count {
                let dmg_info = damage_info.clone();
                if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                    let mut final_damage = dmg_info.output.max(0);
                    if m.block > 0 {
                        if final_damage >= m.block {
                            final_damage -= m.block;
                            m.block = 0;
                        } else {
                            m.block -= final_damage;
                            final_damage = 0;
                        }
                    }
                    if final_damage > 0 {
                        m.current_hp = (m.current_hp - final_damage).max(0);
                    }
                }
            }
        },
        Action::Feed { target, damage_info, max_hp_amount } => {
            // Deal damage, if it kills: gain max HP
            let mut killed = false;
            if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                let mut final_damage = damage_info.output.max(0);
                if m.block > 0 {
                    if final_damage >= m.block {
                        final_damage -= m.block;
                        m.block = 0;
                    } else {
                        m.block -= final_damage;
                        final_damage = 0;
                    }
                }
                if final_damage > 0 {
                    m.current_hp = (m.current_hp - final_damage).max(0);
                }
                if m.current_hp <= 0 {
                    killed = true;
                }
            }
            if killed {
                state.player.max_hp += max_hp_amount;
                state.player.current_hp += max_hp_amount;
            }
        },
        Action::VampireDamage(info) => {
            // Deal damage minus block, heal player by unblocked damage amount
            let mut hp_lost = 0;
            if let Some(m) = state.monsters.iter_mut().find(|m| m.id == info.target) {
                let mut final_damage = info.output.max(0);
                if m.block > 0 {
                    if final_damage >= m.block {
                        final_damage -= m.block;
                        m.block = 0;
                    } else {
                        m.block -= final_damage;
                        final_damage = 0;
                    }
                }
                if final_damage > 0 {
                    m.current_hp = (m.current_hp - final_damage).max(0);
                    hp_lost = final_damage;
                }
            }
            if hp_lost > 0 {
                state.player.current_hp = (state.player.current_hp + hp_lost).min(state.player.max_hp);
            }
        },
        Action::VampireDamageAllEnemies { source: _, damages, damage_type: _ } => {
            let mut total_hp_lost = 0;
            for (i, &dmg) in damages.iter().enumerate() {
                let target_id = i + 1;
                if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
                    if m.current_hp <= 0 || m.is_dying { continue; }
                    let mut final_damage = dmg.max(0);
                    if m.block > 0 {
                        if final_damage >= m.block {
                            final_damage -= m.block;
                            m.block = 0;
                        } else {
                            m.block -= final_damage;
                            final_damage = 0;
                        }
                    }
                    if final_damage > 0 {
                        m.current_hp = (m.current_hp - final_damage).max(0);
                        total_hp_lost += final_damage;
                    }
                }
            }
            if total_hp_lost > 0 {
                state.player.current_hp = (state.player.current_hp + total_hp_lost).min(state.player.max_hp);
            }
        },
        Action::LimitBreak => {
            // Double player's Strength
            if let Some(powers) = state.power_db.get_mut(&0) {
                if let Some(str_power) = powers.iter_mut().find(|p| p.power_type == PowerId::Strength) {
                    str_power.amount *= 2;
                }
            }
        },
        Action::BlockPerNonAttack { block_per_card } => {
            // Second Wind: gain block per non-attack card in hand, exhaust them
            let non_attacks: Vec<u32> = state.hand.iter()
                .filter(|c| {
                    let def = crate::content::cards::get_card_definition(c.id);
                    def.card_type != crate::content::cards::CardType::Attack
                })
                .map(|c| c.uuid)
                .collect();
            let count = non_attacks.len() as i32;
            // Gain block
            state.player.block += block_per_card * count;
            // Exhaust each non-attack
            for uuid in non_attacks {
                crate::engine::core::queue_actions(&mut state.action_queue, smallvec::smallvec![
                    ActionInfo {
                        action: Action::ExhaustCard { card_uuid: uuid, source_pile: crate::state::PileType::Hand },
                        insertion_mode: AddTo::Bottom
                    }
                ]);
            }
        },
        Action::ExhaustAllNonAttack => {
            // Sever Soul: exhaust all non-attack cards in hand
            let non_attacks: Vec<u32> = state.hand.iter()
                .filter(|c| {
                    let def = crate::content::cards::get_card_definition(c.id);
                    def.card_type != crate::content::cards::CardType::Attack
                })
                .map(|c| c.uuid)
                .collect();
            for uuid in non_attacks {
                crate::engine::core::queue_actions(&mut state.action_queue, smallvec::smallvec![
                    ActionInfo {
                        action: Action::ExhaustCard { card_uuid: uuid, source_pile: crate::state::PileType::Hand },
                        insertion_mode: AddTo::Bottom
                    }
                ]);
            }
        },
        Action::ExhaustRandomCard { amount } => {
            // True Grit: exhaust random card(s) from hand
            for _ in 0..amount {
                if state.hand.is_empty() { break; }
                let idx = state.rng.card_random_rng.random(state.hand.len() as i32 - 1) as usize;
                let card = state.hand.remove(idx);
                state.exhaust_pile.push(card);
                let exhaust_actions = crate::content::relics::hooks::on_exhaust(state);
                crate::engine::core::queue_actions(&mut state.action_queue, exhaust_actions);
            }
        },
        Action::DrawCards(amount) => {
            // NoDraw power prevents all card draws
            let has_no_draw = state.power_db.get(&0).map_or(false, |powers| {
                powers.iter().any(|p| p.power_type == PowerId::NoDraw)
            });
            if has_no_draw {
                return;
            }
            for _ in 0..amount {
                // Shuffle discard into draw if draw is empty
                if state.draw_pile.is_empty() && !state.discard_pile.is_empty() {
                    state.draw_pile.append(&mut state.discard_pile);
                    // Shuffle using shuffle_rng
                    let len = state.draw_pile.len();
                    for i in (1..len).rev() {
                        let j = state.rng.shuffle_rng.random(i as i32) as usize;
                        state.draw_pile.swap(i, j);
                    }
                    // Fire on_shuffle relic hooks (Sundial, Abacus, Melange)
                    let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
                    crate::engine::core::queue_actions(&mut state.action_queue, shuffle_actions);
                }
                if state.draw_pile.is_empty() {
                    break;
                }
                let mut card = state.draw_pile.remove(0);
                
                if card.id == CardId::Void {
                    let void_actions = crate::content::cards::status::void::on_drawn(state);
                    crate::engine::core::queue_actions(&mut state.action_queue, void_actions);
                }

                // Corruption on-draw hook: set skill cost to 0
                let has_corruption = state.power_db.get(&0).map_or(false, |powers| {
                    powers.iter().any(|p| p.power_type == PowerId::Corruption)
                });
                if has_corruption {
                    crate::content::cards::ironclad::corruption::corruption_on_card_draw(state, &mut card);
                }

                if state.hand.len() < 10 {
                    state.hand.push(card);
                } else {
                    state.discard_pile.push(card);
                }
            }
        },
        Action::EmptyDeckShuffle => {
            if state.draw_pile.is_empty() && !state.discard_pile.is_empty() {
                state.draw_pile.append(&mut state.discard_pile);
                let len = state.draw_pile.len();
                for i in (1..len).rev() {
                    let j = state.rng.shuffle_rng.random(i as i32) as usize;
                    state.draw_pile.swap(i, j);
                }
                // Fire on_shuffle relic hooks (Sundial, Abacus, Melange)
                let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
                crate::engine::core::queue_actions(&mut state.action_queue, shuffle_actions);
            }
        },
        Action::DiscardCard { card_uuid } => {
            if let Some(pos) = state.hand.iter().position(|c| c.uuid == card_uuid) {
                let card = state.hand.remove(pos);
                state.discard_pile.push(card);
                let discard_actions = crate::content::relics::hooks::on_discard(state);
                crate::engine::core::queue_actions(&mut state.action_queue, discard_actions);
            }
        },
        Action::ExhaustCard { card_uuid, source_pile } => {
            let mut removed_card = None;
            match source_pile {
                crate::state::PileType::Hand => {
                    if let Some(pos) = state.hand.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.hand.remove(pos));
                    }
                },
                crate::state::PileType::Draw => {
                    if let Some(pos) = state.draw_pile.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.draw_pile.remove(pos));
                    }
                },
                crate::state::PileType::Discard => {
                    if let Some(pos) = state.discard_pile.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.discard_pile.remove(pos));
                    }
                },
                crate::state::PileType::Limbo => {
                    if let Some(pos) = state.limbo.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.limbo.remove(pos));
                    }
                },
                _ => {}
            }
            if let Some(card) = removed_card {
                let is_necronomicurse = card.id == CardId::Necronomicurse;
                state.exhaust_pile.push(card);
                let mut after_actions = crate::content::relics::hooks::on_exhaust(state);
                if is_necronomicurse {
                    after_actions.push(ActionInfo {
                        action: Action::MakeTempCardInHand { card_id: CardId::Necronomicurse, amount: 1, upgraded: false },
                        insertion_mode: AddTo::Bottom,
                    });
                }
                crate::engine::core::queue_actions(&mut state.action_queue, after_actions);
            }
        },
        Action::MakeTempCardInHand { card_id, amount, upgraded } => {
            for _ in 0..amount {
                state.card_uuid_counter += 1;
                let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
                if upgraded { card.upgrades = 1; }
                if state.hand.len() < 10 {
                    state.hand.push(card);
                } else {
                    state.discard_pile.push(card);
                }
            }
        },
        Action::MakeTempCardInDiscard { card_id, amount, upgraded } => {
            for _ in 0..amount {
                state.card_uuid_counter += 1;
                let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
                if upgraded { card.upgrades = 1; }
                state.discard_pile.push(card);
            }
        },
        Action::MakeTempCardInDrawPile { card_id, amount, random_spot, upgraded } => {
            for _ in 0..amount {
                state.card_uuid_counter += 1;
                let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
                if upgraded { card.upgrades = 1; }
                if random_spot && !state.draw_pile.is_empty() {
                    let idx = state.rng.card_random_rng.random(state.draw_pile.len() as i32) as usize;
                    state.draw_pile.insert(idx, card);
                } else {
                    state.draw_pile.push(card); // top of draw pile
                }
            }
        },
        Action::MakeCopyInHand { original, amount } => {
            for _ in 0..amount {
                state.card_uuid_counter += 1;
                let mut card = (*original).clone();
                card.uuid = state.card_uuid_counter;
                if state.hand.len() < 10 {
                    state.hand.push(card);
                } else {
                    state.discard_pile.push(card);
                }
            }
        },
        Action::MakeCopyInDiscard { original, amount } => {
            for _ in 0..amount {
                state.card_uuid_counter += 1;
                let mut card = (*original).clone();
                card.uuid = state.card_uuid_counter;
                state.discard_pile.push(card);
            }
        },
        Action::MoveCard { card_uuid, from, to } => {
            let mut removed_card = None;
            match from {
                crate::state::PileType::Hand => {
                    if let Some(pos) = state.hand.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.hand.remove(pos));
                    }
                },
                crate::state::PileType::Draw => {
                    if let Some(pos) = state.draw_pile.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.draw_pile.remove(pos));
                    }
                },
                crate::state::PileType::Discard => {
                    if let Some(pos) = state.discard_pile.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.discard_pile.remove(pos));
                    }
                },
                crate::state::PileType::Exhaust => {
                    if let Some(pos) = state.exhaust_pile.iter().position(|c| c.uuid == card_uuid) {
                        removed_card = Some(state.exhaust_pile.remove(pos));
                    }
                },
                _ => {}
            }
            if let Some(card) = removed_card {
                match to {
                    crate::state::PileType::Hand => {
                        if state.hand.len() < 10 { state.hand.push(card); }
                        else { state.discard_pile.push(card); }
                    },
                    crate::state::PileType::Draw => state.draw_pile.insert(0, card), // top of draw
                    crate::state::PileType::Discard => state.discard_pile.push(card),
                    crate::state::PileType::Exhaust => state.exhaust_pile.push(card),
                    _ => {}
                }
            }
        },
        Action::ReduceAllHandCosts { amount } => {
            for card in state.hand.iter_mut() {
                let def = crate::content::cards::get_card_definition(card.id);
                if def.cost >= 0 {
                    let current = card.cost_for_turn.unwrap_or(def.cost as u8);
                    card.cost_for_turn = Some(current.saturating_sub(amount));
                }
            }
        },
        Action::UpgradeAllInHand => {
            for card in state.hand.iter_mut() {
                card.upgrades += 1;
            }
        },
        Action::UpgradeAllBurns => {
            for card in state.draw_pile.iter_mut()
                .chain(state.discard_pile.iter_mut())
                .chain(state.hand.iter_mut())
            {
                if card.id == CardId::Burn {
                    card.upgrades += 1;
                }
            }
        },
        Action::UpgradeCard { card_uuid } => {
            for card in state.hand.iter_mut()
                .chain(state.draw_pile.iter_mut())
                .chain(state.discard_pile.iter_mut())
            {
                if card.uuid == card_uuid {
                    card.upgrades += 1;
                    break;
                }
            }
        },
        Action::UpgradeRandomCard => {
            // Java: UpgradeRandomCardAction — collect upgradeable non-STATUS cards,
            // shuffle with shuffleRng (Collections.shuffle + java.util.Random LCG),
            // upgrade the first one.
            let upgradeable_uuids: Vec<u32> = state.hand.iter()
                .filter(|c| c.upgrades == 0 && crate::content::cards::get_card_definition(c.id).card_type != crate::content::cards::CardType::Status)
                .map(|c| c.uuid)
                .collect();
            if !upgradeable_uuids.is_empty() {
                let mut shuffled = upgradeable_uuids;
                crate::rng::shuffle_with_random_long(&mut shuffled, &mut state.rng.shuffle_rng);
                let target_uuid = shuffled[0];
                if let Some(card) = state.hand.iter_mut().find(|c| c.uuid == target_uuid) {
                    card.upgrades += 1;
                }
            }
        },
        Action::ModifyCardMisc { card_uuid, amount } => {
            for card in state.hand.iter_mut()
                .chain(state.draw_pile.iter_mut())
                .chain(state.discard_pile.iter_mut())
                .chain(state.exhaust_pile.iter_mut())
            {
                if card.uuid == card_uuid {
                    card.misc_value = amount;
                    break;
                }
            }
        },
        Action::UpdatePowerExtraData { target, power_id, value } => {
            if let Some(powers) = state.power_db.get_mut(&target) {
                if let Some(power) = powers.iter_mut().find(|p| p.power_type == power_id) {
                    power.amount = value;
                }
            }
        },
        Action::UpdateRelicCounter { relic_id, counter } => {
            if let Some(relic) = state.player.relics.iter_mut().find(|r| r.id == relic_id) {
                relic.counter = counter;
            }
        },
        Action::UpdateRelicAmount { relic_id, amount } => {
            if let Some(relic) = state.player.relics.iter_mut().find(|r| r.id == relic_id) {
                relic.counter += amount;
            }
        },
        Action::UpdateRelicUsedUp { relic_id, used_up } => {
            if let Some(relic) = state.player.relics.iter_mut().find(|r| r.id == relic_id) {
                relic.used_up = used_up;
            }
        },
        Action::Suicide { target } => {
            if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                m.current_hp = 0;
                m.is_dying = true;
            }
        },
        Action::Escape { target } => {
            if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
                m.is_escaped = true;
            }
        },
        Action::MummifiedHandEffect => {
            // Reduce cost of a random card in hand by 1 for this turn
            let eligible: Vec<usize> = state.hand.iter().enumerate()
                .filter(|(_, c)| {
                    let def = crate::content::cards::get_card_definition(c.id);
                    def.cost > 0
                })
                .map(|(i, _)| i)
                .collect();
            if !eligible.is_empty() {
                let idx = state.rng.card_random_rng.random(eligible.len() as i32 - 1) as usize;
                let card_idx = eligible[idx];
                let card = &mut state.hand[card_idx];
                let def = crate::content::cards::get_card_definition(card.id);
                let current = card.cost_for_turn.unwrap_or(def.cost as u8);
                card.cost_for_turn = Some(current.saturating_sub(1));
            }
        },
        Action::UsePotion { slot, target } => {
            if let Some(Some(potion)) = state.potions.get(slot).cloned() {
                if potion.id == crate::content::potions::PotionId::FairyPotion {
                    return;
                }
                if potion.id == crate::content::potions::PotionId::SmokeBomb && state.is_boss_fight {
                    return;
                }
                let def = crate::content::potions::get_potion_definition(potion.id);
                let mut potency = def.base_potency;
                if state.player.has_relic(crate::content::relics::RelicId::SacredBark) {
                    potency *= 2;
                }
                let actions = crate::content::potions::potion_effects::get_potion_actions(potion.id, target, potency);
                let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
                let mut combined = relic_actions;
                combined.extend(actions);
                crate::engine::core::queue_actions(&mut state.action_queue, combined);
                state.potions[slot] = None;
            }
        },
        Action::DiscardPotion { slot } => {
            if slot < state.potions.len() {
                state.potions[slot] = None;
            }
        },
        Action::RandomizeHandCosts => {
            for card in state.hand.iter_mut() {
                let base_cost = crate::content::cards::get_card_definition(card.id).cost;
                if base_cost >= 0 {
                    let new_cost = state.rng.card_random_rng.random(3) as u8;
                    card.cost_for_turn = Some(new_cost);
                }
            }
        },
        Action::MakeRandomCardInHand { card_type, cost_for_turn } => {
            let mut pool: Vec<CardId> = Vec::new();
            for &rarity in &[
                crate::content::cards::CardRarity::Common,
                crate::content::cards::CardRarity::Uncommon,
                crate::content::cards::CardRarity::Rare,
            ] {
                for &id in crate::content::cards::ironclad_pool_for_rarity(rarity) {
                    if let Some(ct) = card_type {
                        let def = crate::content::cards::get_card_definition(id);
                        if def.card_type != ct { continue; }
                    }
                    pool.push(id);
                }
            }
            if !pool.is_empty() {
                let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
                let card_id = pool[idx];
                state.card_uuid_counter += 1;
                let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
                if let Some(cost) = cost_for_turn {
                    card.cost_for_turn = Some(cost);
                }
                if state.hand.len() < 10 {
                    state.hand.push(card);
                } else {
                    state.discard_pile.push(card);
                }
            }
        },
        Action::EndTurnTrigger => {
            let mut actions = smallvec::SmallVec::new();
            
            // 1. Player Powers (Java: AbstractRoom.endTurn -> applyEndOfTurnTriggers)
            // Monster powers are intentionally left for TurnTransition (Phase 2.3)
            if let Some(player_powers) = state.power_db.get(&0) {
                for power in player_powers.clone() {
                    actions.extend(crate::content::powers::resolve_power_at_end_of_turn(
                        power.power_type, state, 0, power.amount
                    ).into_iter().map(|a| ActionInfo { action: a, insertion_mode: crate::action::AddTo::Bottom }));
                }
            }
            
            // 2. Ethereal exhaust (Java: DiscardAtEndOfTurnAction -> c.triggerOnEndOfPlayerTurn)
            for card in &state.hand {
                let def = crate::content::cards::get_card_definition(card.id);
                if def.ethereal {
                    actions.push(ActionInfo {
                        action: Action::ExhaustCard { card_uuid: card.uuid, source_pile: crate::state::PileType::Hand },
                        insertion_mode: crate::action::AddTo::Bottom,
                    });
                }
            }

            // 3. Relics (Java: GameActionManager.callEndOfTurnActions -> applyEndOfTurnRelics)
            actions.extend(crate::content::relics::hooks::at_end_of_turn(state));

            // 4. Orbs (Java: TriggerEndOfTurnOrbsAction)
            actions.extend(crate::content::orbs::hooks::trigger_end_of_turn_orbs(state));

            // 5. Curses and Burns in hand (Java: c.triggerOnEndOfTurnForPlayingCard)
            for card in &state.hand {
                if card.id == CardId::Burn {
                    actions.extend(crate::content::cards::status::burn::on_end_turn_in_hand(state, card));
                }
                if card.id == CardId::Regret {
                    actions.extend(crate::content::cards::curses::regret::on_end_turn_in_hand(state));
                }
                if card.id == CardId::Decay {
                    actions.extend(crate::content::cards::curses::decay::on_end_turn_in_hand(state));
                }
                if card.id == CardId::Doubt {
                    actions.extend(crate::content::cards::curses::doubt::on_end_turn_in_hand(state));
                }
                if card.id == CardId::Pride {
                    actions.extend(crate::content::cards::curses::pride::on_end_turn_in_hand(state));
                }
                if card.id == CardId::Shame {
                    actions.extend(crate::content::cards::curses::shame::on_end_turn_in_hand(state));
                }
            }

            // 6. Stances (Java: stance.onEndOfTurn)
            actions.extend(crate::content::stances::hooks::on_end_of_turn(state));
            
            crate::engine::core::queue_actions(&mut state.action_queue, actions);
        },
        Action::ObtainPotion => {
            if state.player.has_relic(crate::content::relics::RelicId::Sozu) {
                return;
            }
            if let Some(slot) = state.potions.iter().position(|p| p.is_none()) {
                let potion_id = crate::content::potions::random_potion(
                    &mut state.rng.potion_rng,
                    crate::content::potions::PotionClass::Ironclad,
                    true,
                );
                state.potions[slot] = Some(crate::content::potions::Potion::new(potion_id, 40000 + slot as u32));
            }
        },
        Action::UseCardDone { should_exhaust } => {
            // Deferred card-to-pile: pop from limbo, move to exhaust or discard
            if let Some(card) = state.limbo.pop() {
                if should_exhaust {
                    state.exhaust_pile.push(card);
                    // on_exhaust hooks (DeadBranch, CharonsAshes, FeelNoPain)
                    let exhaust_actions = crate::content::relics::hooks::on_exhaust(state);
                    crate::engine::core::queue_actions(&mut state.action_queue, exhaust_actions);
                } else {
                    state.discard_pile.push(card);
                }
            }
        },
        Action::RollMonsterMove { monster_id } => {
            if let Some(m) = state.monsters.iter().find(|m| m.id == monster_id && !m.is_dying) {
                let entity_snapshot = m.clone();
                let num = state.rng.ai_rng.random(99);
                let (move_byte, intent) = crate::content::monsters::roll_monster_move(
                    &mut state.rng.ai_rng, &entity_snapshot, state.ascension_level, num, &state.monsters
                );
                if let Some(m) = state.monsters.iter_mut().find(|m| m.id == monster_id) {
                    m.next_move_byte = move_byte;
                    m.current_intent = intent;
                    m.move_history.push_back(move_byte);
                }
            }
        },
        Action::SetMonsterMove { monster_id, next_move_byte, intent } => {
            if let Some(m) = state.monsters.iter_mut().find(|m| m.id == monster_id) {
                m.next_move_byte = next_move_byte;
                m.current_intent = intent;
                m.move_history.push_back(next_move_byte);
            }
        },
        Action::SpawnMonsterSmart { monster_id, logical_position, current_hp, max_hp } => {
            let mut target_slot = 0;
            for m in &state.monsters {
                if logical_position > m.logical_position {
                    target_slot += 1;
                }
            }
            state.action_queue.push_front(Action::SpawnMonster {
                monster_id,
                slot: target_slot,
                current_hp,
                max_hp,
                logical_position,
            });
        },
        Action::SpawnMonster { monster_id, slot, current_hp, max_hp, logical_position } => {
            // Allocate a unique entity ID: max existing ID + 1
            let new_entity_id = state.monsters.iter().map(|m| m.id).max().unwrap_or(0) + 1;
            let enemy_id = monster_id;

            // Auto-roll HP if not specified (current_hp == 0)
            let (actual_hp, actual_max_hp) = if current_hp == 0 {
                let (hp_min, hp_max) = crate::content::monsters::get_hp_range(enemy_id, state.ascension_level);
                // Java calls monsterHpRng.random() TWICE per monster:
                //   1st in super() constructor (passed but overridden by setHp)
                //   2nd in setHp() (actual HP used)
                // We must consume one dummy call to keep RNG in sync.
                let _ = state.rng.monster_hp_rng.random_range(hp_min, hp_max); // constructor's unused roll
                let rolled = state.rng.monster_hp_rng.random_range(hp_min, hp_max); // setHp's actual roll
                (rolled, rolled)
            } else {
                (current_hp, max_hp)
            };

            let new_monster = crate::combat::MonsterEntity {
                id: new_entity_id,
                monster_type: enemy_id as usize,
                current_hp: actual_hp,
                max_hp: actual_max_hp,
                block: 0,
                slot,
                is_dying: false,
                is_escaped: false,
                next_move_byte: 0,
                current_intent: crate::combat::Intent::Unknown,
                move_history: std::collections::VecDeque::new(),
                intent_dmg: 0,
                logical_position,
            };

            state.monsters.insert(slot as usize, new_monster);

            // Run pre-battle actions for the spawned monster
            let pre_battle_actions = crate::content::monsters::resolve_pre_battle_action(
                enemy_id,
                &state.monsters[slot as usize],
                &mut state.rng.monster_hp_rng,
                state.ascension_level,
            );
            for a in pre_battle_actions {
                state.action_queue.push_back(a);
            }

            // Roll the monster's initial move
            state.action_queue.push_back(Action::RollMonsterMove { monster_id: new_entity_id });
        },
        Action::PlayCardDirect { card, target, purge } => {
            // Direct card play from DoubleTap/DuplicationPower/Necronomicon.
            // No energy cost, card is NOT from hand. Resolve its effects normally.
            let card_id = card.id;
            let mut played_card = *card;

            // Evaluate card (apply Strength/Dexterity/Vulnerable to damage/block)
            let effective_target = target;
            crate::content::cards::evaluate_card(&mut played_card, state, effective_target);

            // Resolve card play actions (damage, block, draw, etc.)
            let card_actions = crate::content::cards::resolve_card_play(card_id, state, &played_card, effective_target);
            crate::engine::core::queue_actions(&mut state.action_queue, card_actions);

            // on_use_card relic hooks
            let relic_actions = crate::content::relics::hooks::on_use_card(state, card_id);
            crate::engine::core::queue_actions(&mut state.action_queue, relic_actions);

            // on_card_played power hooks (for ALL creatures)
            for entity_id in std::iter::once(0usize).chain(state.monsters.iter().map(|m| m.id)) {
                if let Some(powers) = state.power_db.get(&entity_id).cloned() {
                    for power in &powers {
                        let hook_actions = crate::content::powers::resolve_power_on_card_played(
                            power.power_type, state, entity_id, &played_card, power.amount
                        );
                        for a in hook_actions {
                            state.action_queue.push_back(a);
                        }
                    }
                }
            }

            // Update counters
            state.counters.cards_played_this_turn += 1;
            let def = crate::content::cards::get_card_definition(card_id);
            if def.card_type == crate::content::cards::CardType::Attack {
                state.counters.attacks_played_this_turn += 1;
            }

            // Card goes to discard if not purged
            if !purge {
                state.discard_pile.push(played_card);
            }
        },
        Action::MakeRandomColorlessCardInHand { rarity: _, cost_for_turn } => {
            // Java: returnTrulyRandomColorlessCardInCombat() — picks from srcColorlessCardPool
            // srcColorlessCardPool = all Uncommon + Rare colorless cards, excluding HEALING tag
            let mut pool: Vec<CardId> = Vec::new();
            for &id in crate::content::cards::COLORLESS_UNCOMMON_POOL {
                let def = crate::content::cards::get_card_definition(id);
                if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
                    pool.push(id);
                }
            }
            for &id in crate::content::cards::COLORLESS_RARE_POOL {
                let def = crate::content::cards::get_card_definition(id);
                if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
                    pool.push(id);
                }
            }
            if !pool.is_empty() {
                let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
                let card_id = pool[idx];
                state.card_uuid_counter += 1;
                let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
                if let Some(cost) = cost_for_turn {
                    card.cost_for_turn = Some(cost);
                }
                if state.hand.len() < 10 {
                    state.hand.push(card);
                } else {
                    state.discard_pile.push(card);
                }
            }
        },
        Action::ClearCardQueue => {
            state.action_queue.retain(|a| {
                if let Action::PlayCardDirect { .. } = a { false }
                else if let Action::UseCardDone { .. } = a { false }
                else { true }
            });
            state.limbo.clear(); // Cards queued go poof
        },
        Action::MakeTempCardInDiscardAndDeck { card_id, amount } => {
            for _ in 0..amount {
                state.card_uuid_counter += 1;
                let card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
                state.discard_pile.push(card.clone());
                let pos = state.rng.card_random_rng.random(state.draw_pile.len() as i32) as usize;
                state.draw_pile.insert(pos, card);
            }
        },
        Action::AddCardToMasterDeck { card_id } => {
            state.meta_changes.push(crate::combat::MetaChange::AddCardToMasterDeck(card_id));
        },
        other => {
            #[cfg(debug_assertions)]
            eprintln!("[action_handlers] Unhandled action: {:?}", other);
        }
    }
}
