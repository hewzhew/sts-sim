use super::*;
use crate::rewards::state::RewardCard;
use crate::state::core::RunPendingChoiceReason;

pub fn build_action_candidates(
    legal_actions: &[ClientInput],
    ctx: Option<&EpisodeContext>,
) -> Vec<RunActionCandidate> {
    let combat = ctx.and_then(|ctx| ctx.combat_state.as_ref());
    let mut candidates: Vec<RunActionCandidate> = legal_actions
        .iter()
        .enumerate()
        .map(|(action_index, action)| {
            let action_key = action_key_for_input(action, combat);
            let action_id = stable_action_id(&action_key);
            let card = ctx.and_then(|ctx| card_feature_for_action(action, ctx));
            let plan_delta = ctx.and_then(|ctx| {
                let delta = candidate_plan_delta_for_action(action, ctx);
                if delta == empty_candidate_plan_delta() {
                    None
                } else {
                    Some(delta)
                }
            });
            let reward_structure = ctx.and_then(|ctx| {
                let rs = reward_action_structure_for_action(action, ctx);
                if rs == empty_reward_action_structure() {
                    None
                } else {
                    Some(rs)
                }
            });
            let semantic_descriptor =
                ctx.and_then(|ctx| action_semantic_descriptor_for_action(action, ctx));
            let recording = ctx
                .map(|ctx| {
                    recording_action_for_action(
                        action,
                        ctx,
                        &action_key,
                        semantic_descriptor.as_ref(),
                    )
                })
                .unwrap_or_else(|| {
                    (
                        format!("MISSING_RUST_RECORDING_LABEL: {action_key}"),
                        None,
                        "missing_context".to_string(),
                    )
                });
            let choice_option = ctx
                .map(|ctx| {
                    choice_option_for_action(
                        action,
                        ctx,
                        action_index,
                        action_id,
                        &action_key,
                        &recording.0,
                        reward_structure.as_ref(),
                        semantic_descriptor.as_ref(),
                    )
                })
                .unwrap_or_else(|| {
                    generic_choice_option(
                        action_index,
                        action_id,
                        &action_key,
                        &recording.0,
                        "unavailable",
                        Some("missing_episode_context".to_string()),
                    )
                });
            RunActionCandidate {
                action_index,
                action_id,
                action_key,
                recording_label: recording.0,
                recording_detail: recording.1,
                recording_kind: recording.2,
                action: trace_input_from_client_input(action),
                card,
                plan_delta,
                reward_structure,
                semantic_descriptor,
                choice_option,
                dominated: false,
                dominated_by_index: None,
            }
        })
        .collect();

    // Annotate Pareto-dominated candidates for diagnostic visibility.
    // We don't remove them from the mask (yet) — only mark them so audits can
    // measure how often a dominated candidate "wins" under rollout.
    annotate_dominated_candidates(&mut candidates);
    candidates
}

fn recording_action_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
    action_key: &str,
    descriptor: Option<&ActionSemanticDescriptorV1>,
) -> (String, Option<String>, String) {
    if let Some(recording) = recording_event_choice_for_action(action, ctx, action_key, descriptor)
    {
        return recording;
    }
    if let Some(descriptor) = descriptor {
        if !descriptor.label.is_empty() {
            return (
                descriptor.label.clone(),
                recording_detail_from_descriptor(descriptor),
                descriptor.action_type.clone(),
            );
        }
    }
    match action {
        ClientInput::EndTurn => (
            "End turn".to_string(),
            Some("End current player turn".to_string()),
            "combat_end_turn".to_string(),
        ),
        ClientInput::PlayCard { card_index, target } => {
            let Some(combat) = ctx.combat_state.as_ref() else {
                return missing_recording(action_key, "combat state missing");
            };
            let Some(card) = combat.zones.hand.get(*card_index) else {
                return missing_recording(action_key, "hand card missing");
            };
            let def = crate::content::cards::get_card_definition(card.id);
            let target_label = target
                .and_then(|entity_id| recording_monster_target(combat, entity_id))
                .unwrap_or_else(|| "none".to_string());
            (
                format!("Play {} h{} -> {}", def.name, card_index, target_label),
                Some(format!(
                    "cost={} playable={}",
                    card.cost_for_turn_java(),
                    crate::content::cards::can_play_card(card, combat).is_ok()
                )),
                "combat_play_card".to_string(),
            )
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => {
            let Some(potion) = ctx
                .run_state
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
            else {
                return missing_recording(action_key, "potion missing");
            };
            let def = crate::content::potions::get_potion_definition(potion.id);
            let target_label = ctx
                .combat_state
                .as_ref()
                .and_then(|combat| {
                    target.and_then(|entity_id| recording_monster_target(combat, entity_id))
                })
                .unwrap_or_else(|| "none".to_string());
            (
                format!("Use {} slot {} -> {}", def.name, potion_index, target_label),
                None,
                "potion_use".to_string(),
            )
        }
        ClientInput::DiscardPotion(index) => (
            format!("Discard potion slot {index}"),
            None,
            "potion_discard".to_string(),
        ),
        ClientInput::SelectMapNode(x) => (
            recording_map_label(&ctx.run_state, *x, None),
            None,
            "map_select".to_string(),
        ),
        ClientInput::FlyToNode(x, y) => (
            recording_map_label(&ctx.run_state, *x, Some(*y)),
            Some("Wing Boots route".to_string()),
            "map_fly".to_string(),
        ),
        ClientInput::BuyCard(index) => {
            let EngineState::Shop(shop) = &ctx.engine_state else {
                return missing_recording(action_key, "shop state missing");
            };
            let Some(item) = shop.cards.get(*index) else {
                return missing_recording(action_key, "shop card missing");
            };
            let def = crate::content::cards::get_card_definition(item.card_id);
            (
                format!("Buy {} for {} Gold", def.name, item.price),
                None,
                "shop_buy_card".to_string(),
            )
        }
        ClientInput::BuyRelic(index) => {
            let EngineState::Shop(shop) = &ctx.engine_state else {
                return missing_recording(action_key, "shop state missing");
            };
            let Some(item) = shop.relics.get(*index) else {
                return missing_recording(action_key, "shop relic missing");
            };
            (
                format!("Buy {:?} for {} Gold", item.relic_id, item.price),
                None,
                "shop_buy_relic".to_string(),
            )
        }
        ClientInput::BuyPotion(index) => {
            let EngineState::Shop(shop) = &ctx.engine_state else {
                return missing_recording(action_key, "shop state missing");
            };
            let Some(item) = shop.potions.get(*index) else {
                return missing_recording(action_key, "shop potion missing");
            };
            let def = crate::content::potions::get_potion_definition(item.potion_id);
            (
                format!("Buy {} for {} Gold", def.name, item.price),
                None,
                "shop_buy_potion".to_string(),
            )
        }
        ClientInput::PurgeCard(index) => {
            let Some(card) = ctx.run_state.master_deck.get(*index) else {
                return missing_recording(action_key, "deck card missing");
            };
            let def = crate::content::cards::get_card_definition(card.id);
            (
                format!(
                    "Remove {}{} for {} Gold",
                    def.name,
                    upgrade_suffix(card.upgrades),
                    shop_purge_cost(&ctx.engine_state)
                ),
                None,
                "shop_purge_card".to_string(),
            )
        }
        ClientInput::CampfireOption(choice) => recording_campfire_label(*choice, ctx, action_key),
        ClientInput::SubmitRelicChoice(index) => {
            let EngineState::BossRelicSelect(state) = &ctx.engine_state else {
                return missing_recording(action_key, "boss relic state missing");
            };
            let Some(relic) = state.relics.get(*index) else {
                return missing_recording(action_key, "boss relic missing");
            };
            (
                format!("Take boss relic {:?}", relic),
                None,
                "boss_relic_select".to_string(),
            )
        }
        ClientInput::OpenChest => (
            "Open chest".to_string(),
            None,
            "treasure_open_chest".to_string(),
        ),
        ClientInput::Proceed => ("Proceed".to_string(), None, "proceed".to_string()),
        ClientInput::Cancel => ("Cancel".to_string(), None, "cancel".to_string()),
        ClientInput::ClaimReward(index) => {
            let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
                return missing_recording(action_key, "reward state missing");
            };
            if reward_state.pending_card_choice.is_some() {
                return missing_recording(
                    action_key,
                    "reward claim unavailable during card choice",
                );
            }
            let Some(item) = reward_state.items.get(*index) else {
                return missing_recording(action_key, "reward item missing");
            };
            (
                format!("Claim {}", recording_reward_item_label(item)),
                None,
                "reward_claim".to_string(),
            )
        }
        ClientInput::SelectCard(index) => {
            let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
                return missing_recording(action_key, "reward card state missing");
            };
            let Some(cards) = reward_state.pending_card_choice.as_ref() else {
                return missing_recording(action_key, "reward card choice missing");
            };
            let Some(card) = cards.get(*index) else {
                return missing_recording(action_key, "reward card missing");
            };
            (
                format!("Take {}", recording_reward_card_label(card)),
                None,
                "reward_card_select".to_string(),
            )
        }
        ClientInput::SelectEventOption(index) | ClientInput::EventChoice(index) => {
            let options = crate::engine::event_handler::get_event_options(&ctx.run_state);
            if let Some(option) = options.get(*index) {
                (
                    option.ui.text.clone(),
                    option.ui.disabled_reason.clone(),
                    "event_choice".to_string(),
                )
            } else {
                missing_recording(action_key, "event option missing")
            }
        }
        ClientInput::SubmitSelection(selection) => (
            format!(
                "Select {}",
                selection
                    .selected
                    .iter()
                    .filter_map(|target| recording_selection_target(target, ctx))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            None,
            "selection_submit".to_string(),
        ),
        ClientInput::SubmitCardChoice(indices) => (
            format!(
                "Choose cards {}",
                recording_pending_card_indices(indices, ctx).join(", ")
            ),
            None,
            "pending_card_choice".to_string(),
        ),
        ClientInput::SubmitDiscoverChoice(index) => (
            format!("Choose {}", recording_discover_choice_label(*index, ctx)),
            None,
            "pending_discover_choice".to_string(),
        ),
        ClientInput::SubmitScryDiscard(indices) => {
            let labels = recording_scry_indices(indices, ctx);
            (
                if labels.is_empty() {
                    "Keep all scry cards".to_string()
                } else {
                    format!("Discard from scry {}", labels.join(", "))
                },
                None,
                "pending_scry_discard".to_string(),
            )
        }
        ClientInput::SubmitHandSelect(uuids) => (
            format!(
                "Select hand {}",
                recording_combat_uuid_labels(uuids, ctx).join(", ")
            ),
            None,
            "pending_hand_select".to_string(),
        ),
        ClientInput::SubmitGridSelect(uuids) => (
            format!(
                "Select grid {}",
                recording_combat_uuid_labels(uuids, ctx).join(", ")
            ),
            None,
            "pending_grid_select".to_string(),
        ),
        ClientInput::SubmitDeckSelect(indices) => (
            format!(
                "Select deck {}",
                recording_deck_index_labels(indices, ctx).join(", ")
            ),
            None,
            "pending_deck_select".to_string(),
        ),
    }
}

fn choice_option_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
    option_id: usize,
    action_id: u32,
    action_key: &str,
    label: &str,
    reward_structure: Option<&RewardActionStructureV0>,
    descriptor: Option<&ActionSemanticDescriptorV1>,
) -> RunChoiceOptionV1 {
    let mut option = generic_choice_option(
        option_id,
        action_id,
        action_key,
        label,
        if descriptor.is_some() {
            "partial"
        } else {
            "unavailable"
        },
        if descriptor.is_some() {
            None
        } else {
            Some("projection_not_implemented".to_string())
        },
    );
    if let Some(descriptor) = descriptor {
        if !descriptor.effects.is_empty() || !descriptor.costs.is_empty() {
            option.delta_summary = Some(descriptor_effect_summary(descriptor));
            option.preview_status = "partial".to_string();
            option.unavailable_reason = None;
        }
    }
    if let Some(reward_structure) = reward_structure {
        apply_reward_choice_option_flags(&mut option, reward_structure);
    }
    if let Some(pending_option) =
        pending_choice_option_for_action(action, ctx, option_id, action_id, action_key, label)
    {
        return pending_option;
    }
    option
}

fn generic_choice_option(
    option_id: usize,
    action_id: u32,
    action_key: &str,
    label: &str,
    preview_status: &str,
    unavailable_reason: Option<String>,
) -> RunChoiceOptionV1 {
    RunChoiceOptionV1 {
        schema_name: "ChoiceOptionV1".to_string(),
        schema_version: 1,
        option_id,
        action_id,
        action_key: action_key.to_string(),
        label: label.to_string(),
        subject_ref: None,
        before_summary: None,
        after_summary: None,
        delta_summary: None,
        preview_status: preview_status.to_string(),
        unavailable_reason,
        danger_flags: Vec::new(),
        requires_confirmation: false,
        confirmation_command: None,
    }
}

fn apply_reward_choice_option_flags(
    option: &mut RunChoiceOptionV1,
    reward_structure: &RewardActionStructureV0,
) {
    if reward_structure.is_proceed_with_unclaimed_rewards
        && reward_structure.unclaimed_reward_count > 0
    {
        option
            .danger_flags
            .push("leaves_unclaimed_rewards".to_string());
        option.requires_confirmation = true;
    }
    if reward_structure.skip_card_choice {
        option.danger_flags.push("skip_card_reward".to_string());
    }
}

fn descriptor_effect_summary(descriptor: &ActionSemanticDescriptorV1) -> String {
    let mut parts = Vec::new();
    for cost in &descriptor.costs {
        parts.push(format!("cost:{}", semantic_effect_short(cost)));
    }
    for effect in &descriptor.effects {
        parts.push(format!("effect:{}", semantic_effect_short(effect)));
    }
    if parts.is_empty() {
        descriptor.label.clone()
    } else {
        parts.join("; ")
    }
}

fn semantic_effect_short(effect: &ActionSemanticEffectV1) -> String {
    let mut parts = vec![effect.effect_type.clone()];
    if let Some(amount) = effect.amount {
        parts.push(format!("amount={amount}"));
    }
    if let Some(count) = effect.count {
        parts.push(format!("count={count}"));
    }
    if let Some(kind) = &effect.kind {
        parts.push(format!("kind={kind}"));
    }
    if let Some(target) = &effect.target {
        parts.push(format!("target={target}"));
    }
    parts.join(",")
}

fn pending_choice_option_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
    option_id: usize,
    action_id: u32,
    action_key: &str,
    label: &str,
) -> Option<RunChoiceOptionV1> {
    let EngineState::PendingChoice(PendingChoice::HandSelect { reason, .. }) = &ctx.engine_state
    else {
        return None;
    };
    let ClientInput::SubmitHandSelect(uuids) = action else {
        return None;
    };
    if uuids.len() != 1 {
        let mut option = generic_choice_option(
            option_id,
            action_id,
            action_key,
            label,
            "unavailable",
            Some("multi_card_selection_preview_not_implemented".to_string()),
        );
        option.subject_ref = Some(format!("hand_card_uuids:{:?}", uuids));
        return Some(option);
    }
    let uuid = uuids[0];
    let card = ctx
        .combat_state
        .as_ref()
        .and_then(|combat| find_combat_card_by_uuid_for_choice_option(combat, uuid));
    let Some(card) = card else {
        let mut option = generic_choice_option(
            option_id,
            action_id,
            action_key,
            label,
            "unavailable",
            Some("selected_card_not_found".to_string()),
        );
        option.subject_ref = Some(format!("hand_card_uuid:{uuid}"));
        return Some(option);
    };
    let preview = hand_select_choice_preview(reason, card);
    Some(RunChoiceOptionV1 {
        schema_name: "ChoiceOptionV1".to_string(),
        schema_version: 1,
        option_id,
        action_id,
        action_key: action_key.to_string(),
        label: label.to_string(),
        subject_ref: Some(format!("hand_card_uuid:{uuid}")),
        before_summary: preview.before_summary,
        after_summary: preview.after_summary,
        delta_summary: preview.delta_summary,
        preview_status: preview.status,
        unavailable_reason: preview.unavailable_reason,
        danger_flags: Vec::new(),
        requires_confirmation: false,
        confirmation_command: None,
    })
}

struct ChoicePreview {
    before_summary: Option<String>,
    after_summary: Option<String>,
    delta_summary: Option<String>,
    status: String,
    unavailable_reason: Option<String>,
}

fn hand_select_choice_preview(
    reason: &crate::state::HandSelectReason,
    card: &crate::runtime::combat::CombatCard,
) -> ChoicePreview {
    let before = Some(choice_card_summary_for_upgrades(card.id, card.upgrades));
    match reason {
        crate::state::HandSelectReason::Upgrade => {
            let after_upgrades = card.upgrades.saturating_add(1);
            ChoicePreview {
                before_summary: before,
                after_summary: Some(choice_card_summary_for_upgrades(card.id, after_upgrades)),
                delta_summary: Some(format!(
                    "upgrade: {}",
                    choice_card_upgrade_delta_summary(card.id, card.upgrades)
                )),
                status: "available".to_string(),
                unavailable_reason: None,
            }
        }
        crate::state::HandSelectReason::Exhaust => {
            available_choice_preview(before, "exhaust selected card")
        }
        crate::state::HandSelectReason::Discard | crate::state::HandSelectReason::GamblingChip => {
            available_choice_preview(before, "discard selected card")
        }
        crate::state::HandSelectReason::PutOnDrawPile | crate::state::HandSelectReason::Setup => {
            available_choice_preview(before, "put selected card on top of draw pile")
        }
        crate::state::HandSelectReason::PutToBottomOfDraw => {
            available_choice_preview(before, "put selected card on bottom of draw pile")
        }
        crate::state::HandSelectReason::Retain => {
            available_choice_preview(before, "retain selected card")
        }
        crate::state::HandSelectReason::Recycle => {
            available_choice_preview(before, "exhaust selected card for energy")
        }
        crate::state::HandSelectReason::Copy { amount } => available_choice_preview(
            before,
            &format!("create {amount} copy/copies of selected card"),
        ),
        crate::state::HandSelectReason::Nightmare { amount } => {
            available_choice_preview(before, &format!("create {amount} copy/copies next turn"))
        }
    }
}

fn available_choice_preview(before_summary: Option<String>, delta_summary: &str) -> ChoicePreview {
    ChoicePreview {
        before_summary,
        after_summary: None,
        delta_summary: Some(delta_summary.to_string()),
        status: "available".to_string(),
        unavailable_reason: None,
    }
}

fn choice_card_summary_for_upgrades(card_id: CardId, upgrades: u8) -> String {
    format!(
        "{} [{}]",
        choice_card_name_for_upgrades(card_id, upgrades),
        choice_card_effect_summary_for_upgrades(card_id, upgrades)
    )
}

fn choice_card_name_for_upgrades(card_id: CardId, upgrades: u8) -> String {
    let def = crate::content::cards::get_card_definition(card_id);
    if upgrades > 0 {
        format!("{}+{}", def.name, upgrades)
    } else {
        def.name.to_string()
    }
}

fn choice_card_effect_summary_for_upgrades(card_id: CardId, upgrades: u8) -> String {
    let parts = choice_card_numeric_effect_parts(card_id, upgrades)
        .into_iter()
        .map(|(label, value)| format!("{label} {value}"))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "no direct numeric effect".to_string()
    } else {
        parts.join(", ")
    }
}

fn choice_card_upgrade_delta_summary(card_id: CardId, upgrades: u8) -> String {
    let before = choice_card_numeric_effect_parts(card_id, upgrades);
    let after = choice_card_numeric_effect_parts(card_id, upgrades.saturating_add(1));
    let mut labels = before.iter().map(|(label, _)| *label).collect::<Vec<_>>();
    for (label, _) in &after {
        if !labels.contains(label) {
            labels.push(*label);
        }
    }
    let mut deltas = Vec::new();
    for label in labels {
        let before_value = before
            .iter()
            .find(|(candidate, _)| *candidate == label)
            .map(|(_, value)| *value);
        let after_value = after
            .iter()
            .find(|(candidate, _)| *candidate == label)
            .map(|(_, value)| *value);
        if before_value == after_value {
            continue;
        }
        match (before_value, after_value) {
            (Some(before), Some(after)) => deltas.push(format!("{label} {before} -> {after}")),
            (None, Some(after)) => deltas.push(format!("{label} {after}")),
            (Some(before), None) => deltas.push(format!("{label} {before} -> none")),
            (None, None) => {}
        }
    }
    if deltas.is_empty() {
        "no numeric card text delta".to_string()
    } else {
        deltas.join(", ")
    }
}

fn choice_card_numeric_effect_parts(card_id: CardId, upgrades: u8) -> Vec<(&'static str, i32)> {
    let def = crate::content::cards::get_card_definition(card_id);
    let damage = def.base_damage + def.upgrade_damage * upgrades as i32;
    let block = def.base_block + def.upgrade_block * upgrades as i32;
    let magic = def.base_magic + def.upgrade_magic * upgrades as i32;
    let mut parts = Vec::new();
    if damage > 0 {
        parts.push(("dmg", damage));
    }
    if block > 0 || card_is_block_core(card_id) {
        parts.push(("block", block.max(0)));
    }
    if magic > 0 {
        if card_applies_vulnerable(card_id) {
            parts.push(("vuln", magic));
        } else if card_applies_weak(card_id) {
            parts.push(("weak", magic));
        } else if card_draws_cards(card_id) {
            parts.push(("draw", magic));
        } else if card_gains_energy(card_id) {
            parts.push(("energy", magic));
        } else {
            parts.push(("magic", magic));
        }
    }
    parts
}

fn find_combat_card_by_uuid_for_choice_option(
    combat: &CombatState,
    uuid: u32,
) -> Option<&crate::runtime::combat::CombatCard> {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .chain(combat.zones.exhaust_pile.iter())
        .chain(combat.zones.limbo.iter())
        .find(|card| card.uuid == uuid)
        .or_else(|| {
            combat
                .zones
                .queued_cards
                .iter()
                .map(|queued| &queued.card)
                .find(|card| card.uuid == uuid)
        })
}

fn missing_recording(action_key: &str, reason: &str) -> (String, Option<String>, String) {
    (
        format!("MISSING_RUST_RECORDING_LABEL: {action_key}"),
        Some(reason.to_string()),
        "missing_recording_label".to_string(),
    )
}

fn recording_detail_from_descriptor(descriptor: &ActionSemanticDescriptorV1) -> Option<String> {
    let _ = descriptor;
    None
}

fn recording_event_choice_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
    action_key: &str,
    descriptor: Option<&ActionSemanticDescriptorV1>,
) -> Option<(String, Option<String>, String)> {
    let index = match action {
        ClientInput::SelectEventOption(index) | ClientInput::EventChoice(index) => *index,
        _ => return None,
    };
    let options = crate::engine::event_handler::get_event_options(&ctx.run_state);
    let Some(option) = options.get(index) else {
        return Some(missing_recording(action_key, "event option missing"));
    };
    let mut label = option.ui.text.clone();

    if label.contains("Boss Relic") && label.contains("starter Relic") {
        let starter = ctx
            .run_state
            .relics
            .first()
            .map(|relic| format!("{:?}", relic.id))
            .unwrap_or_else(|| "starter relic".to_string());
        label = format!("Boss relic swap: lose {starter}, gain random boss relic.");
    } else if let Some(descriptor) = descriptor {
        if !descriptor.unknown_fields.is_empty()
            && descriptor.unknown_fields.iter().any(|field| {
                field == "exact_reward_card" || field == "exact_reward_screen_contents"
            })
            && label.to_ascii_lowercase().contains("choose")
            && !label.contains("[")
        {
            label.push_str(" [follow-up]");
        }
    }

    Some((
        label,
        option.ui.disabled_reason.clone(),
        descriptor
            .map(|descriptor| descriptor.action_type.clone())
            .unwrap_or_else(|| "event_choice".to_string()),
    ))
}

fn recording_monster_target(combat: &CombatState, entity_id: usize) -> Option<String> {
    combat
        .entities
        .monsters
        .iter()
        .enumerate()
        .find(|(_, monster)| monster.id == entity_id)
        .map(|(slot, monster)| {
            let name = crate::content::monsters::EnemyId::from_id(monster.monster_type)
                .map(|enemy| enemy.get_name())
                .unwrap_or("Unknown Monster");
            format!(
                "monster_slot:{slot} {name} hp={}/{}",
                monster.current_hp, monster.max_hp
            )
        })
}

fn recording_map_label(run_state: &RunState, x: usize, y_override: Option<usize>) -> String {
    if run_state.map.current_y == 14 {
        return "Boss node".to_string();
    }
    let y = y_override.map(|value| value as i32).unwrap_or_else(|| {
        if run_state.map.current_y == -1 {
            0
        } else {
            run_state.map.current_y + 1
        }
    });
    let room = if y >= 0
        && (y as usize) < run_state.map.graph.len()
        && x < run_state.map.graph[y as usize].len()
    {
        run_state.map.graph[y as usize][x].class
    } else {
        None
    };
    let room_label = match room {
        Some(crate::map::node::RoomType::MonsterRoom) => "Monster",
        Some(crate::map::node::RoomType::MonsterRoomElite) => "Elite",
        Some(crate::map::node::RoomType::MonsterRoomBoss) => "Boss",
        Some(crate::map::node::RoomType::RestRoom) => "Rest",
        Some(crate::map::node::RoomType::ShopRoom) => "Shop",
        Some(crate::map::node::RoomType::EventRoom) => "Event",
        Some(crate::map::node::RoomType::TreasureRoom) => "Chest",
        Some(crate::map::node::RoomType::TrueVictoryRoom) => "True Victory",
        None => "Unknown",
    };
    format!("Go x={x} y={y} {room_label}")
}

fn recording_reward_card_label(card: &RewardCard) -> String {
    let def = crate::content::cards::get_card_definition(card.id);
    format!(
        "{}{} ({:?} {:?}) cost={}",
        def.name,
        upgrade_suffix(card.upgrades),
        def.card_type,
        def.rarity,
        def.cost
    )
}

fn recording_reward_item_label(item: &crate::rewards::state::RewardItem) -> String {
    match item {
        crate::rewards::state::RewardItem::Gold { amount } => format!("{amount} Gold"),
        crate::rewards::state::RewardItem::StolenGold { amount } => {
            format!("{amount} Stolen Gold")
        }
        crate::rewards::state::RewardItem::Card { cards } => {
            format!("card reward: choose 1 of {}", cards.len())
        }
        crate::rewards::state::RewardItem::Relic { relic_id } => format!("{relic_id:?}"),
        crate::rewards::state::RewardItem::Potion { potion_id } => {
            let def = crate::content::potions::get_potion_definition(*potion_id);
            def.name.to_string()
        }
        crate::rewards::state::RewardItem::EmeraldKey => "Emerald Key".to_string(),
        crate::rewards::state::RewardItem::SapphireKey => "Sapphire Key".to_string(),
    }
}

fn recording_campfire_label(
    choice: CampfireChoice,
    ctx: &EpisodeContext,
    action_key: &str,
) -> (String, Option<String>, String) {
    match choice {
        CampfireChoice::Rest => (
            format!("Rest (heal {} HP)", ctx.run_state.max_hp * 30 / 100),
            None,
            "campfire_rest".to_string(),
        ),
        CampfireChoice::Smith(index) => {
            let Some(card) = ctx.run_state.master_deck.get(index) else {
                return missing_recording(action_key, "smith card missing");
            };
            let def = crate::content::cards::get_card_definition(card.id);
            (
                format!("Smith {}{}", def.name, upgrade_suffix(card.upgrades)),
                None,
                "campfire_smith".to_string(),
            )
        }
        CampfireChoice::Dig => ("Dig".to_string(), None, "campfire_dig".to_string()),
        CampfireChoice::Lift => ("Lift".to_string(), None, "campfire_lift".to_string()),
        CampfireChoice::Toke(index) => {
            let Some(card) = ctx.run_state.master_deck.get(index) else {
                return missing_recording(action_key, "toke card missing");
            };
            let def = crate::content::cards::get_card_definition(card.id);
            (
                format!("Toke {}{}", def.name, upgrade_suffix(card.upgrades)),
                None,
                "campfire_toke".to_string(),
            )
        }
        CampfireChoice::Recall => (
            "Recall Ruby Key".to_string(),
            None,
            "campfire_recall".to_string(),
        ),
    }
}

fn shop_purge_cost(engine_state: &EngineState) -> i32 {
    match engine_state {
        EngineState::Shop(shop) => shop.purge_cost,
        _ => 0,
    }
}

fn recording_selection_target(
    target: &crate::state::selection::SelectionTargetRef,
    ctx: &EpisodeContext,
) -> Option<String> {
    let crate::state::selection::SelectionTargetRef::CardUuid(uuid) = target;
    ctx.run_state
        .master_deck
        .iter()
        .find(|card| card.uuid == *uuid)
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            format!(
                "{}{} uuid={}",
                def.name,
                upgrade_suffix(card.upgrades),
                uuid
            )
        })
        .or_else(|| {
            ctx.combat_state.as_ref().and_then(|combat| {
                combat
                    .zones
                    .hand
                    .iter()
                    .chain(combat.zones.draw_pile.iter())
                    .chain(combat.zones.discard_pile.iter())
                    .chain(combat.zones.exhaust_pile.iter())
                    .find(|card| card.uuid == *uuid)
                    .map(|card| {
                        let def = crate::content::cards::get_card_definition(card.id);
                        format!(
                            "{}{} uuid={}",
                            def.name,
                            upgrade_suffix(card.upgrades),
                            uuid
                        )
                    })
            })
        })
}

fn recording_card_id_label(card_id: crate::content::cards::CardId, upgrades: u8) -> String {
    let def = crate::content::cards::get_card_definition(card_id);
    format!("{}{}", def.name, upgrade_suffix(upgrades))
}

fn recording_pending_card_indices(indices: &[usize], ctx: &EpisodeContext) -> Vec<String> {
    let EngineState::PendingChoice(choice) = &ctx.engine_state else {
        return indices.iter().map(|index| format!("#{index}")).collect();
    };
    indices
        .iter()
        .map(|index| match choice {
            PendingChoice::CardRewardSelect { cards, .. } => cards
                .get(*index)
                .map(|card| recording_card_id_label(*card, 0))
                .unwrap_or_else(|| format!("#{index}")),
            PendingChoice::ChooseOneSelect { choices } => choices
                .get(*index)
                .map(|choice| recording_card_id_label(choice.card_id, choice.upgrades))
                .unwrap_or_else(|| format!("#{index}")),
            _ => format!("#{index}"),
        })
        .collect()
}

fn recording_discover_choice_label(index: usize, ctx: &EpisodeContext) -> String {
    let EngineState::PendingChoice(choice) = &ctx.engine_state else {
        return format!("#{index}");
    };
    match choice {
        PendingChoice::DiscoverySelect(choice) => choice
            .cards
            .get(index)
            .map(|card| recording_card_id_label(*card, 0))
            .unwrap_or_else(|| format!("#{index}")),
        PendingChoice::ForeignInfluenceSelect { cards, upgraded } => cards
            .get(index)
            .map(|card| recording_card_id_label(*card, if *upgraded { 1 } else { 0 }))
            .unwrap_or_else(|| format!("#{index}")),
        PendingChoice::ChooseOneSelect { choices } => choices
            .get(index)
            .map(|choice| recording_card_id_label(choice.card_id, choice.upgrades))
            .unwrap_or_else(|| format!("#{index}")),
        PendingChoice::StanceChoice => match index {
            0 => "Wrath".to_string(),
            1 => "Calm".to_string(),
            _ => format!("#{index}"),
        },
        _ => format!("#{index}"),
    }
}

fn recording_scry_indices(indices: &[usize], ctx: &EpisodeContext) -> Vec<String> {
    let EngineState::PendingChoice(PendingChoice::ScrySelect { cards, .. }) = &ctx.engine_state
    else {
        return indices.iter().map(|index| format!("#{index}")).collect();
    };
    indices
        .iter()
        .map(|index| {
            cards
                .get(*index)
                .map(|card| recording_card_id_label(*card, 0))
                .unwrap_or_else(|| format!("#{index}"))
        })
        .collect()
}

fn recording_combat_uuid_labels(uuids: &[u32], ctx: &EpisodeContext) -> Vec<String> {
    uuids
        .iter()
        .map(|uuid| {
            ctx.combat_state
                .as_ref()
                .and_then(|combat| {
                    combat
                        .zones
                        .hand
                        .iter()
                        .chain(combat.zones.draw_pile.iter())
                        .chain(combat.zones.discard_pile.iter())
                        .chain(combat.zones.exhaust_pile.iter())
                        .chain(combat.zones.limbo.iter())
                        .find(|card| card.uuid == *uuid)
                })
                .map(|card| recording_card_id_label(card.id, card.upgrades))
                .or_else(|| {
                    ctx.run_state
                        .master_deck
                        .iter()
                        .find(|card| card.uuid == *uuid)
                        .map(|card| recording_card_id_label(card.id, card.upgrades))
                })
                .unwrap_or_else(|| format!("uuid={uuid}"))
        })
        .collect()
}

fn recording_deck_index_labels(indices: &[usize], ctx: &EpisodeContext) -> Vec<String> {
    indices
        .iter()
        .map(|index| {
            ctx.run_state
                .master_deck
                .get(*index)
                .map(|card| recording_card_id_label(card.id, card.upgrades))
                .unwrap_or_else(|| format!("#{index}"))
        })
        .collect()
}

fn upgrade_suffix(upgrades: u8) -> String {
    if upgrades > 0 {
        format!("+{upgrades}")
    } else {
        String::new()
    }
}

pub fn event_option_observations(run_state: &RunState) -> Vec<RunEventOptionObservationV0> {
    let event_id = run_state.event_state.as_ref().map(|state| state.id);
    crate::engine::event_handler::get_event_options(run_state)
        .into_iter()
        .enumerate()
        .map(|(option_index, option)| {
            let descriptor =
                action_semantic_descriptor_for_event_option(event_id, option_index, &option);
            RunEventOptionObservationV0 {
                option_index,
                label: option.ui.text.clone(),
                disabled: option.ui.disabled,
                disabled_reason: option.ui.disabled_reason.clone(),
                semantic_descriptor: descriptor,
            }
        })
        .collect()
}

fn action_semantic_descriptor_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> Option<ActionSemanticDescriptorV1> {
    match action {
        ClientInput::SelectEventOption(index) | ClientInput::EventChoice(index) => {
            let event_id = ctx.run_state.event_state.as_ref().map(|state| state.id);
            let options = crate::engine::event_handler::get_event_options(&ctx.run_state);
            options
                .get(*index)
                .map(|option| action_semantic_descriptor_for_event_option(event_id, *index, option))
        }
        ClientInput::SubmitSelection(selection) => {
            action_semantic_descriptor_for_selection(selection, ctx)
        }
        ClientInput::SelectCard(index) => {
            action_semantic_descriptor_for_reward_card_choice(*index, ctx)
        }
        ClientInput::ClaimReward(index) => action_semantic_descriptor_for_reward_claim(*index, ctx),
        ClientInput::Proceed => action_semantic_descriptor_for_proceed(ctx),
        _ => None,
    }
}

fn action_semantic_descriptor_for_reward_claim(
    index: usize,
    ctx: &EpisodeContext,
) -> Option<ActionSemanticDescriptorV1> {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return None;
    };
    if reward_state.pending_card_choice.is_some() {
        return None;
    }
    let item = reward_state.items.get(index)?;
    let item_obs = reward_item_observation(&ctx.run_state, index, item);
    let mut effects = Vec::new();
    let mut unknown_fields = Vec::new();
    let label = match item_obs.item_type.as_str() {
        "gold" => {
            effects.push(semantic_effect(
                "gain_gold",
                Some(item_obs.amount),
                None,
                None,
            ));
            format!("Claim {} Gold", item_obs.amount)
        }
        "potion" => {
            effects.push(semantic_effect(
                "obtain_potion",
                None,
                Some(1),
                item_obs.potion_id.clone(),
            ));
            let potion_label = match item {
                crate::rewards::state::RewardItem::Potion { potion_id } => {
                    crate::content::potions::get_potion_definition(*potion_id)
                        .name
                        .to_string()
                }
                _ => "Potion".to_string(),
            };
            format!("Claim {potion_label}")
        }
        "relic" => {
            effects.push(semantic_effect(
                "obtain_relic",
                None,
                Some(1),
                item_obs.relic_id.clone(),
            ));
            format!(
                "Claim {}",
                item_obs
                    .relic_id
                    .clone()
                    .unwrap_or_else(|| "Relic".to_string())
            )
        }
        "card_reward" => {
            effects.push(semantic_effect(
                "open_card_reward",
                None,
                Some(item_obs.card_choice_count),
                Some("reward_card_choice".to_string()),
            ));
            unknown_fields.push("selected_reward_card".to_string());
            format!(
                "Claim card reward: choose 1 of {}",
                item_obs.card_choice_count
            )
        }
        other => {
            effects.push(semantic_effect(
                "claim_reward_item",
                Some(item_obs.amount),
                None,
                Some(other.to_string()),
            ));
            format!("Claim {other}")
        }
    };
    Some(ActionSemanticDescriptorV1 {
        schema_name: "ActionDescriptorV1".to_string(),
        schema_version: 1,
        action_type: "reward_claim".to_string(),
        semantic_status: if unknown_fields.is_empty() {
            "known".to_string()
        } else {
            "partial".to_string()
        },
        coverage_level: "runtime_resolved_reward_item".to_string(),
        event_id: None,
        event_name: None,
        option_index: Some(index),
        label,
        costs: Vec::new(),
        effects,
        constraints: Vec::new(),
        transition: if item_obs.opens_card_choice {
            "OpenRewardCardChoice".to_string()
        } else {
            "RewardItemClaimed".to_string()
        },
        unknown_fields,
        source_chain: vec![
            "rust_runtime_reward_screen".to_string(),
            "rust_runtime_resolved_reward_item".to_string(),
        ],
    })
}

fn action_semantic_descriptor_for_proceed(
    ctx: &EpisodeContext,
) -> Option<ActionSemanticDescriptorV1> {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return None;
    };
    let (label, semantic_status, coverage_level, transition, unknown_fields) =
        if reward_state.pending_card_choice.is_some() {
            (
                "Skip card reward (back to rewards)".to_string(),
                "known".to_string(),
                "runtime_resolved_reward_card_skip".to_string(),
                "ReturnToRewardItems".to_string(),
                Vec::new(),
            )
        } else if reward_state.items.is_empty() {
            (
                "Proceed".to_string(),
                "known".to_string(),
                "runtime_resolved_reward_cleanup".to_string(),
                "LeaveRewardScreen".to_string(),
                Vec::new(),
            )
        } else {
            (
                format!(
                    "Proceed with {} unclaimed reward(s)",
                    reward_state.items.len()
                ),
                "partial".to_string(),
                "runtime_resolved_reward_proceed_with_unclaimed_items".to_string(),
                "LeaveRewardScreenWithUnclaimedRewards".to_string(),
                vec!["unclaimed_reward_items".to_string()],
            )
        };
    Some(ActionSemanticDescriptorV1 {
        schema_name: "ActionDescriptorV1".to_string(),
        schema_version: 1,
        action_type: "reward_proceed".to_string(),
        semantic_status,
        coverage_level,
        event_id: None,
        event_name: None,
        option_index: None,
        label,
        costs: Vec::new(),
        effects: Vec::new(),
        constraints: Vec::new(),
        transition,
        unknown_fields,
        source_chain: vec!["rust_runtime_reward_screen".to_string()],
    })
}

fn action_semantic_descriptor_for_selection(
    selection: &crate::state::selection::SelectionResolution,
    ctx: &EpisodeContext,
) -> Option<ActionSemanticDescriptorV1> {
    let EngineState::RunPendingChoice(choice) = &ctx.engine_state else {
        return None;
    };
    let mut selected_cards = Vec::new();
    for target in &selection.selected {
        let crate::state::selection::SelectionTargetRef::CardUuid(uuid) = target;
        let Some(card) = ctx
            .run_state
            .master_deck
            .iter()
            .find(|card| card.uuid == *uuid)
        else {
            continue;
        };
        selected_cards.push(card);
    }
    if selected_cards.is_empty() {
        return None;
    }
    let effect_type = run_pending_choice_effect_type(choice.reason);
    let label_cards = selected_cards
        .iter()
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            if card.upgrades > 0 {
                format!("{}+{}", def.name, card.upgrades)
            } else {
                def.name.to_string()
            }
        })
        .collect::<Vec<_>>();
    let effects = selected_cards
        .iter()
        .map(|card| {
            let mut effect =
                semantic_effect(effect_type, None, Some(1), Some(format!("{:?}", card.id)));
            effect.target = Some(format!("card_uuid:{}", card.uuid));
            effect
        })
        .collect::<Vec<_>>();
    let source_chain = run_selection_source_chain(&ctx.run_state);
    Some(ActionSemanticDescriptorV1 {
        schema_name: "ActionDescriptorV1".to_string(),
        schema_version: 1,
        action_type: "run_pending_card_selection".to_string(),
        semantic_status: "known".to_string(),
        coverage_level: "runtime_resolved_selection_target".to_string(),
        event_id: ctx
            .run_state
            .event_state
            .as_ref()
            .map(|event| event_name_label(event.id)),
        event_name: ctx
            .run_state
            .event_state
            .as_ref()
            .map(|event| event_name_label(event.id)),
        option_index: None,
        label: format!(
            "{} {}",
            run_pending_choice_label(choice.reason),
            label_cards.join(", ")
        ),
        costs: if effect_is_selection_cost(choice.reason) {
            effects.clone()
        } else {
            Vec::new()
        },
        effects: if effect_is_selection_cost(choice.reason) {
            Vec::new()
        } else {
            effects
        },
        constraints: vec![format!(
            "select {}-{} card(s)",
            choice.min_choices, choice.max_choices
        )],
        transition: format!("ReturnTo({:?})", choice.return_state),
        unknown_fields: Vec::new(),
        source_chain,
    })
}

fn action_semantic_descriptor_for_reward_card_choice(
    index: usize,
    ctx: &EpisodeContext,
) -> Option<ActionSemanticDescriptorV1> {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return None;
    };
    let card = reward_state.pending_card_choice.as_ref()?.get(index)?;
    Some(action_semantic_descriptor_for_reward_card(
        index,
        card,
        &ctx.run_state,
    ))
}

pub fn action_semantic_descriptor_for_reward_card(
    index: usize,
    card: &RewardCard,
    run_state: &RunState,
) -> ActionSemanticDescriptorV1 {
    let def = crate::content::cards::get_card_definition(card.id);
    let mut effect = semantic_effect("obtain_card", None, Some(1), Some(format!("{:?}", card.id)));
    effect.details.push(format!("card_name:{}", def.name));
    effect
        .details
        .push(format!("card_type:{:?}", def.card_type));
    effect.details.push(format!("rarity:{:?}", def.rarity));
    effect.details.push(format!("cost:{}", def.cost));
    if card.upgrades > 0 {
        effect.details.push(format!("upgrades:{}", card.upgrades));
    }
    let semantics = base_semantics_for_card(card.id, card.upgrades);
    if !semantics.is_empty() {
        effect.details.push(format!("tags:{}", semantics.join(",")));
    }
    ActionSemanticDescriptorV1 {
        schema_name: "ActionDescriptorV1".to_string(),
        schema_version: 1,
        action_type: "reward_card_choice".to_string(),
        semantic_status: "known".to_string(),
        coverage_level: "runtime_resolved_reward_card".to_string(),
        event_id: run_state
            .event_state
            .as_ref()
            .map(|event| event_name_label(event.id)),
        event_name: run_state
            .event_state
            .as_ref()
            .map(|event| event_name_label(event.id)),
        option_index: Some(index),
        label: if card.upgrades > 0 {
            format!("Obtain {}+{}", def.name, card.upgrades)
        } else {
            format!("Obtain {}", def.name)
        },
        costs: Vec::new(),
        effects: vec![effect],
        constraints: Vec::new(),
        transition: "RewardCardSelected".to_string(),
        unknown_fields: Vec::new(),
        source_chain: vec![
            "rust_runtime_reward_screen".to_string(),
            "rust_runtime_resolved_card_choice".to_string(),
            "rust_card_definition".to_string(),
        ],
    }
}

fn run_pending_choice_effect_type(reason: RunPendingChoiceReason) -> &'static str {
    match reason {
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled => "remove_card",
        RunPendingChoiceReason::Upgrade | RunPendingChoiceReason::TransformUpgraded => {
            "upgrade_card"
        }
        RunPendingChoiceReason::Transform | RunPendingChoiceReason::TransformNonBottled => {
            "transform_card"
        }
        RunPendingChoiceReason::Duplicate => "duplicate_card",
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => "bottle_card",
    }
}

fn run_pending_choice_label(reason: RunPendingChoiceReason) -> &'static str {
    match reason {
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled => "Remove",
        RunPendingChoiceReason::Upgrade | RunPendingChoiceReason::TransformUpgraded => "Upgrade",
        RunPendingChoiceReason::Transform | RunPendingChoiceReason::TransformNonBottled => {
            "Transform"
        }
        RunPendingChoiceReason::Duplicate => "Duplicate",
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => "Bottle",
    }
}

fn effect_is_selection_cost(reason: RunPendingChoiceReason) -> bool {
    let _ = reason;
    false
}

fn run_selection_source_chain(run_state: &RunState) -> Vec<String> {
    let mut source_chain = vec!["rust_runtime_run_pending_choice".to_string()];
    if run_state
        .event_state
        .as_ref()
        .is_some_and(|event| event.id == crate::state::events::EventId::Neow)
    {
        source_chain.push("rust_content_events_neow_followup".to_string());
    }
    source_chain
}

fn action_semantic_descriptor_for_event_option(
    event_id: Option<crate::state::events::EventId>,
    option_index: usize,
    option: &crate::state::events::EventOption,
) -> ActionSemanticDescriptorV1 {
    let mut costs = Vec::new();
    let mut effects = Vec::new();
    let mut unknown_fields = Vec::new();
    for effect in &option.semantics.effects {
        let exported = action_semantic_effect(effect);
        if effect_is_cost(effect) {
            costs.push(exported);
        } else {
            effects.push(exported);
        }
        unknown_fields.extend(effect_unknown_fields(effect));
    }
    if matches!(
        option.semantics.transition,
        crate::state::events::EventOptionTransition::OpenSelection(_)
    ) {
        unknown_fields.push("followup_selection_target".to_string());
    }
    if matches!(
        option.semantics.transition,
        crate::state::events::EventOptionTransition::OpenReward
    ) {
        unknown_fields.push("exact_reward_screen_contents".to_string());
    }
    unknown_fields.sort();
    unknown_fields.dedup();

    let has_structured_semantics = !costs.is_empty()
        || !effects.is_empty()
        || !option.semantics.constraints.is_empty()
        || !matches!(
            option.semantics.transition,
            crate::state::events::EventOptionTransition::None
        )
        || !matches!(
            option.semantics.action,
            crate::state::events::EventActionKind::Unknown
        );
    let semantic_status = if !has_structured_semantics {
        "unknown"
    } else if unknown_fields.is_empty() {
        "known"
    } else {
        "partial"
    };
    let coverage_level = if !has_structured_semantics {
        "label_only"
    } else if unknown_fields.is_empty() {
        "label_plus_structured_effect"
    } else {
        "label_plus_structured_effect_partial"
    };
    let event_name = event_id.map(event_name_label);
    let mut source_chain = vec!["rust_runtime_event_state".to_string()];
    if event_id == Some(crate::state::events::EventId::Neow) {
        source_chain.push("rust_content_events_neow".to_string());
        source_chain.push("java_source_backed_neow_event_reward_model".to_string());
    } else if event_id.is_some() {
        source_chain.push("rust_content_events_structured_options".to_string());
    }
    ActionSemanticDescriptorV1 {
        schema_name: "ActionDescriptorV1".to_string(),
        schema_version: 1,
        action_type: "event_choice".to_string(),
        semantic_status: semantic_status.to_string(),
        coverage_level: coverage_level.to_string(),
        event_id: event_name.clone(),
        event_name,
        option_index: Some(option_index),
        label: option.ui.text.clone(),
        costs,
        effects,
        constraints: option
            .semantics
            .constraints
            .iter()
            .map(|constraint| format!("{constraint:?}"))
            .collect(),
        transition: format!("{:?}", option.semantics.transition),
        unknown_fields,
        source_chain,
    }
}

fn event_name_label(event_id: crate::state::events::EventId) -> String {
    match event_id {
        crate::state::events::EventId::Neow => "Neow".to_string(),
        other => format!("{other:?}"),
    }
}

fn action_semantic_effect(effect: &crate::state::events::EventEffect) -> ActionSemanticEffectV1 {
    use crate::state::events::EventEffect;
    match effect {
        EventEffect::GainGold(amount) => semantic_effect("gain_gold", Some(*amount), None, None),
        EventEffect::LoseGold(amount) => semantic_effect("lose_gold", Some(*amount), None, None),
        EventEffect::LoseHp(amount) => semantic_effect("lose_hp", Some(*amount), None, None),
        EventEffect::LoseMaxHp(amount) => semantic_effect("lose_max_hp", Some(*amount), None, None),
        EventEffect::Heal(amount) => semantic_effect("heal", Some(*amount), None, None),
        EventEffect::GainMaxHp(amount) => semantic_effect("gain_max_hp", Some(*amount), None, None),
        EventEffect::ObtainRelic { count, kind } => semantic_effect(
            "obtain_relic",
            None,
            Some(*count),
            Some(format!("{kind:?}")),
        ),
        EventEffect::ObtainPotion { count } => {
            semantic_effect("obtain_potion", None, Some(*count), None)
        }
        EventEffect::ObtainCard { count, kind } => {
            semantic_effect("obtain_card", None, Some(*count), Some(format!("{kind:?}")))
        }
        EventEffect::ObtainColorlessCard { count, kind } => semantic_effect(
            "obtain_colorless_card",
            None,
            Some(*count),
            Some(format!("{kind:?}")),
        ),
        EventEffect::ObtainCurse { count, kind } => semantic_effect(
            "obtain_curse",
            None,
            Some(*count),
            Some(format!("{kind:?}")),
        ),
        EventEffect::RemoveCard {
            count,
            target_uuid,
            kind,
        } => {
            let mut effect =
                semantic_effect("remove_card", None, Some(*count), Some(format!("{kind:?}")));
            effect.target = target_uuid.map(|uuid| format!("card_uuid:{uuid}"));
            effect
        }
        EventEffect::UpgradeCard { count } => {
            semantic_effect("upgrade_card", None, Some(*count), None)
        }
        EventEffect::TransformCard { count } => {
            semantic_effect("transform_card", None, Some(*count), None)
        }
        EventEffect::DuplicateCard { count } => {
            semantic_effect("duplicate_card", None, Some(*count), None)
        }
        EventEffect::LoseRelic {
            specific,
            starter_only,
        } => {
            let mut effect = semantic_effect(
                "lose_relic",
                None,
                Some(1),
                specific.map(|id| format!("{id:?}")),
            );
            if *starter_only {
                effect.details.push("starter_only".to_string());
            }
            effect
        }
        EventEffect::LoseStarterRelic { specific } => semantic_effect(
            "lose_starter_relic",
            None,
            Some(1),
            specific.map(|id| format!("{id:?}")),
        ),
        EventEffect::StartCombat => semantic_effect("start_combat", None, None, None),
    }
}

fn semantic_effect(
    effect_type: &str,
    amount: Option<i32>,
    count: Option<usize>,
    kind: Option<String>,
) -> ActionSemanticEffectV1 {
    ActionSemanticEffectV1 {
        effect_type: effect_type.to_string(),
        amount,
        count,
        kind,
        target: None,
        details: Vec::new(),
    }
}

fn effect_is_cost(effect: &crate::state::events::EventEffect) -> bool {
    use crate::state::events::EventEffect;
    matches!(
        effect,
        EventEffect::LoseGold(_)
            | EventEffect::LoseHp(_)
            | EventEffect::LoseMaxHp(_)
            | EventEffect::ObtainCurse { .. }
            | EventEffect::LoseRelic { .. }
            | EventEffect::LoseStarterRelic { .. }
    )
}

fn effect_unknown_fields(effect: &crate::state::events::EventEffect) -> Vec<String> {
    use crate::state::events::{EventCardKind, EventEffect, EventRelicKind};
    match effect {
        EventEffect::ObtainRelic {
            kind: EventRelicKind::RandomRelic | EventRelicKind::Unknown,
            ..
        } => vec!["exact_reward_relic".to_string()],
        EventEffect::ObtainCard {
            kind: EventCardKind::RandomClassCard | EventCardKind::Unknown,
            ..
        } => vec!["exact_reward_card".to_string()],
        EventEffect::ObtainColorlessCard { .. } => vec!["exact_reward_card".to_string()],
        EventEffect::ObtainPotion { .. } => vec!["exact_reward_potion".to_string()],
        EventEffect::RemoveCard {
            target_uuid: None, ..
        } => vec!["selected_card".to_string()],
        EventEffect::TransformCard { .. } => vec!["selected_card".to_string()],
        EventEffect::UpgradeCard { .. } => vec!["selected_card".to_string()],
        EventEffect::LoseStarterRelic { specific: None } => vec!["starter_relic_id".to_string()],
        _ => Vec::new(),
    }
}

pub fn empty_reward_action_structure() -> RewardActionStructureV0 {
    RewardActionStructureV0 {
        screen_phase: "none".to_string(),
        ..RewardActionStructureV0::default()
    }
}

pub fn reward_action_structure_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> RewardActionStructureV0 {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return empty_reward_action_structure();
    };
    if reward_state.pending_card_choice.is_some() {
        return RewardActionStructureV0 {
            screen_phase: "card_choice".to_string(),
            is_reward_action: matches!(action, ClientInput::SelectCard(_) | ClientInput::Proceed),
            skip_card_choice: matches!(action, ClientInput::Proceed),
            proceed_is_cleanup: false,
            ..RewardActionStructureV0::default()
        };
    }

    let unclaimed_reward_count = reward_state.items.len();
    let unclaimed_card_reward_count = reward_state
        .items
        .iter()
        .filter(|item| matches!(item, RewardItem::Card { .. }))
        .count();
    match action {
        ClientInput::ClaimReward(index) => reward_state
            .items
            .get(*index)
            .map(|item| {
                let item_obs = reward_item_observation(&ctx.run_state, *index, item);
                RewardActionStructureV0 {
                    screen_phase: "claim_items".to_string(),
                    is_reward_action: true,
                    unclaimed_reward_count,
                    unclaimed_card_reward_count,
                    claim_reward_item_type: Some(item_obs.item_type),
                    claim_opens_card_choice: item_obs.opens_card_choice,
                    ..RewardActionStructureV0::default()
                }
            })
            .unwrap_or_else(empty_reward_action_structure),
        ClientInput::Proceed => RewardActionStructureV0 {
            screen_phase: if unclaimed_reward_count > 0 {
                "claim_items".to_string()
            } else {
                "cleanup".to_string()
            },
            is_reward_action: true,
            is_proceed_with_unclaimed_rewards: unclaimed_reward_count > 0,
            unclaimed_reward_count,
            unclaimed_card_reward_count,
            proceed_is_cleanup: unclaimed_reward_count == 0,
            ..RewardActionStructureV0::default()
        },
        _ => empty_reward_action_structure(),
    }
}

pub fn card_feature_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> Option<RunCardFeatureV0> {
    match action {
        ClientInput::PlayCard { card_index, .. } => ctx
            .combat_state
            .as_ref()
            .and_then(|combat| combat.zones.hand.get(*card_index))
            .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
        ClientInput::SelectCard(index) => match &ctx.engine_state {
            EngineState::RewardScreen(reward_state) => reward_state
                .pending_card_choice
                .as_ref()
                .and_then(|cards| cards.get(*index))
                .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
            _ => None,
        },
        ClientInput::BuyCard(index) => match &ctx.engine_state {
            EngineState::Shop(shop) => shop
                .cards
                .get(*index)
                .map(|card| build_card_feature(card.card_id, 0, &ctx.run_state)),
            _ => None,
        },
        ClientInput::CampfireOption(CampfireChoice::Smith(index))
        | ClientInput::CampfireOption(CampfireChoice::Toke(index))
        | ClientInput::PurgeCard(index) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
        _ => None,
    }
}

pub fn candidate_plan_delta_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> CandidatePlanDeltaV0 {
    match action {
        ClientInput::SelectCard(index) => match &ctx.engine_state {
            EngineState::RewardScreen(reward_state) => reward_state
                .pending_card_choice
                .as_ref()
                .and_then(|cards| cards.get(*index))
                .map(|card| add_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
                .unwrap_or_else(empty_candidate_plan_delta),
            _ => empty_candidate_plan_delta(),
        },
        ClientInput::BuyCard(index) => match &ctx.engine_state {
            EngineState::Shop(shop) => shop
                .cards
                .get(*index)
                .map(|card| add_card_plan_delta(card.card_id, 0, &ctx.run_state))
                .unwrap_or_else(empty_candidate_plan_delta),
            _ => empty_candidate_plan_delta(),
        },
        ClientInput::CampfireOption(CampfireChoice::Smith(index)) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| upgrade_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
            .unwrap_or_else(empty_candidate_plan_delta),
        ClientInput::CampfireOption(CampfireChoice::Toke(index))
        | ClientInput::PurgeCard(index) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| remove_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
            .unwrap_or_else(empty_candidate_plan_delta),
        _ => empty_candidate_plan_delta(),
    }
}

pub fn empty_candidate_plan_delta() -> CandidatePlanDeltaV0 {
    CandidatePlanDeltaV0 {
        ..CandidatePlanDeltaV0::default()
    }
}

pub fn add_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let affordance = card_plan_affordance(card_id, upgrades);
    let starter_basic_burden_delta = duplicate_card_burden_delta(card_id, run_state);
    delta_from_affordance(affordance, starter_basic_burden_delta)
}

pub fn upgrade_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    _run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let before = card_plan_affordance(card_id, upgrades);
    let after = card_plan_affordance(card_id, upgrades.saturating_add(1));
    let affordance = after.subtract(before);
    delta_from_affordance(affordance, 0)
}

pub fn remove_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    _run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let affordance = card_plan_affordance(card_id, upgrades);
    let burden_delta = if crate::content::cards::is_starter_basic(card_id) {
        -10
    } else {
        0
    };
    delta_from_affordance(
        CardPlanAffordance {
            frontload: -affordance.frontload,
            block: -affordance.block,
            draw: -affordance.draw,
            scaling: -affordance.scaling,
            aoe: -affordance.aoe,
            exhaust: -affordance.exhaust,
            kill_window: -affordance.kill_window,
            setup_cashout_risk: -affordance.setup_cashout_risk,
        },
        burden_delta,
    )
}

pub fn delta_from_affordance(
    affordance: CardPlanAffordance,
    starter_basic_burden_delta: i32,
) -> CandidatePlanDeltaV0 {
    CandidatePlanDeltaV0 {
        frontload_delta: affordance.frontload,
        block_delta: affordance.block,
        draw_delta: affordance.draw,
        scaling_delta: affordance.scaling,
        aoe_delta: affordance.aoe,
        exhaust_delta: affordance.exhaust,
        kill_window_delta: affordance.kill_window,
        starter_basic_burden_delta,
        setup_cashout_risk_delta: affordance.setup_cashout_risk,
    }
}

pub fn duplicate_card_burden_delta(card_id: CardId, run_state: &RunState) -> i32 {
    let copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count() as i32;
    -(copies * 2)
}

pub fn build_card_feature(card_id: CardId, upgrades: u8, run_state: &RunState) -> RunCardFeatureV0 {
    let def = crate::content::cards::get_card_definition(card_id);
    let deck_copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count();
    RunCardFeatureV0 {
        card_id: format!("{card_id:?}"),
        card_id_hash: stable_action_id(&format!("card:{card_id:?}")),
        card_type_id: card_type_id(def.card_type),
        rarity_id: card_rarity_id(def.rarity),
        cost: def.cost,
        upgrades,
        base_damage: def.base_damage,
        base_block: def.base_block,
        base_magic: def.base_magic,
        upgraded_damage: def.base_damage + def.upgrade_damage * upgrades as i32,
        upgraded_block: def.base_block + def.upgrade_block * upgrades as i32,
        upgraded_magic: def.base_magic + def.upgrade_magic * upgrades as i32,
        exhaust: def.exhaust,
        ethereal: def.ethereal,
        innate: def.innate,
        aoe: matches!(def.target, crate::content::cards::CardTarget::AllEnemy),
        multi_damage: def.is_multi_damage || card_is_multi_hit(card_id),
        starter_basic: crate::content::cards::is_starter_basic(card_id),
        draws_cards: card_draws_cards(card_id),
        gains_energy: card_gains_energy(card_id),
        applies_weak: card_applies_weak(card_id),
        applies_vulnerable: card_applies_vulnerable(card_id),
        scaling_piece: card_is_scaling_piece(card_id),
        deck_copies,
    }
}

pub fn card_type_id(card_type: CardType) -> u8 {
    match card_type {
        CardType::Attack => 1,
        CardType::Skill => 2,
        CardType::Power => 3,
        CardType::Status => 4,
        CardType::Curse => 5,
    }
}

pub fn card_rarity_id(rarity: CardRarity) -> u8 {
    match rarity {
        CardRarity::Basic => 1,
        CardRarity::Common => 2,
        CardRarity::Uncommon => 3,
        CardRarity::Rare => 4,
        CardRarity::Special => 5,
        CardRarity::Curse => 6,
    }
}

pub fn card_draws_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BattleTrance
            | CardId::BurningPact
            | CardId::DarkEmbrace
            | CardId::DeepBreath
            | CardId::Dropkick
            | CardId::Evolve
            | CardId::Finesse
            | CardId::FlashOfSteel
            | CardId::GoodInstincts
            | CardId::MasterOfStrategy
            | CardId::Offering
            | CardId::PommelStrike
            | CardId::ShrugItOff
            | CardId::Warcry
            | CardId::Acrobatics
            | CardId::Backflip
            | CardId::Prepared
            | CardId::DaggerThrow
            | CardId::Adrenaline
    )
}

pub fn card_gains_energy(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Bloodletting
            | CardId::Berserk
            | CardId::Offering
            | CardId::SeeingRed
            | CardId::Sentinel
            | CardId::Adrenaline
    )
}

pub fn card_applies_weak(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Clothesline
            | CardId::Intimidate
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::Blind
    )
}

pub fn card_applies_vulnerable(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Bash | CardId::Shockwave | CardId::ThunderClap | CardId::Trip | CardId::Uppercut
    )
}

pub fn card_is_multi_hit(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Pummel
            | CardId::SwordBoomerang
            | CardId::TwinStrike
            | CardId::Whirlwind
            | CardId::Reaper
    )
}

pub fn card_exhausts_other_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BurningPact
            | CardId::FiendFire
            | CardId::SecondWind
            | CardId::SeverSoul
            | CardId::TrueGrit
            | CardId::Purity
    )
}

pub fn card_is_scaling_piece(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::DemonForm
            | CardId::Inflame
            | CardId::LimitBreak
            | CardId::Rupture
            | CardId::SpotWeakness
            | CardId::Barricade
            | CardId::Entrench
            | CardId::Juggernaut
            | CardId::Metallicize
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Corruption
            | CardId::Evolve
            | CardId::FireBreathing
            | CardId::Footwork
            | CardId::NoxiousFumes
            | CardId::AfterImage
    )
}

pub fn card_is_block_core(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Defend
            | CardId::DefendG
            | CardId::Apparition
            | CardId::GhostlyArmor
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::PowerThrough
            | CardId::ShrugItOff
            | CardId::Backflip
            | CardId::CloakAndDagger
            | CardId::GoodInstincts
            | CardId::DarkShackles
    )
}

// ---------------------------------------------------------------------------
// Pareto dominance on plan delta vectors
// ---------------------------------------------------------------------------

/// Core dimensions for Pareto comparison: (name, higher_is_better)
const DOMINANCE_DIMS: &[(&str, bool)] = &[
    ("frontload_delta", true),
    ("block_delta", true),
    ("draw_delta", true),
    ("scaling_delta", true),
    ("aoe_delta", true),
    ("exhaust_delta", true),
    ("kill_window_delta", true),
    ("starter_basic_burden_delta", true),
    ("setup_cashout_risk_delta", false), // lower risk = better
];

fn delta_dim(delta: &CandidatePlanDeltaV0, dim: &str, higher_better: bool) -> i32 {
    let raw = match dim {
        "frontload_delta" => delta.frontload_delta,
        "block_delta" => delta.block_delta,
        "draw_delta" => delta.draw_delta,
        "scaling_delta" => delta.scaling_delta,
        "aoe_delta" => delta.aoe_delta,
        "exhaust_delta" => delta.exhaust_delta,
        "kill_window_delta" => delta.kill_window_delta,
        "starter_basic_burden_delta" => delta.starter_basic_burden_delta,
        "setup_cashout_risk_delta" => delta.setup_cashout_risk_delta,
        _ => 0,
    };
    if higher_better {
        raw
    } else {
        -raw
    }
}

/// Returns true if the delta has any non-zero value in the dominance dimensions.
/// A candidate whose 10 dominance dimensions are all zero has its value entirely
/// outside the current vector model (e.g. DualWield, Corruption).  These
/// context-dependent cards are excluded from Pareto comparison.
fn is_vectorizable(delta: &CandidatePlanDeltaV0) -> bool {
    for &(dim, _) in DOMINANCE_DIMS {
        if delta_dim(delta, dim, true) != 0 {
            return true;
        }
    }
    false
}

/// Annotate candidates in-place: set `dominated` and `dominated_by_index`.
/// Skips candidates whose plan_delta is None or has no vectorizable dimensions.
pub fn annotate_dominated_candidates(candidates: &mut [RunActionCandidate]) {
    let n = candidates.len();
    for i in 0..n {
        let di = match &candidates[i].plan_delta {
            Some(d) if is_vectorizable(d) => d,
            _ => continue,
        };
        for j in 0..n {
            if i == j {
                continue;
            }
            let dj = match &candidates[j].plan_delta {
                Some(d) if is_vectorizable(d) => d,
                _ => continue,
            };
            if pareto_dominates(di, dj) {
                candidates[j].dominated = true;
                if candidates[j].dominated_by_index.is_none() {
                    candidates[j].dominated_by_index = Some(i);
                }
            }
        }
    }
}

/// Returns true if `a` Pareto-dominates `b`:
/// a ≥ b in all core dimensions, and a > b in at least one.
pub fn pareto_dominates(a: &CandidatePlanDeltaV0, b: &CandidatePlanDeltaV0) -> bool {
    let mut has_strict = false;
    for &(dim, higher_better) in DOMINANCE_DIMS {
        let va = delta_dim(a, dim, higher_better);
        let vb = delta_dim(b, dim, higher_better);
        if va < vb {
            return false;
        }
        if va > vb {
            has_strict = true;
        }
    }
    has_strict
}

/// Returns indices of non-dominated candidates.
/// Among equivalent candidates (mutual non-domination with identical deltas),
/// only the first in order is kept.
pub fn filter_dominated_candidates(candidates: &[RunActionCandidate]) -> Vec<usize> {
    let n = candidates.len();
    let mut dominated = vec![false; n];

    for i in 0..n {
        if dominated[i] {
            continue;
        }
        let di = match &candidates[i].plan_delta {
            Some(d) => d,
            None => continue,
        };
        for j in 0..n {
            if i == j || dominated[j] {
                continue;
            }
            let dj = match &candidates[j].plan_delta {
                Some(d) => d,
                None => continue,
            };
            if pareto_dominates(di, dj) {
                dominated[j] = true;
            }
        }
    }

    // Also collapse equivalent candidates (identical deltas)
    let mut keep = Vec::new();
    for i in 0..n {
        if dominated[i] {
            continue;
        }
        // Check if any earlier kept candidate has identical delta
        let di = &candidates[i].plan_delta;
        let is_dup = keep
            .iter()
            .any(|&k: &usize| &candidates[k].plan_delta == di);
        if !is_dup {
            keep.push(i);
        }
    }

    keep
}

/// Diagnostic: for a candidate at `index`, return the index of a candidate that
/// dominates it, if any.
pub fn dominated_by(candidates: &[RunActionCandidate], index: usize) -> Option<usize> {
    let d = candidates[index].plan_delta.as_ref()?;
    for (i, c) in candidates.iter().enumerate() {
        if i == index {
            continue;
        }
        if let Some(di) = &c.plan_delta {
            if pareto_dominates(di, d) {
                return Some(i);
            }
        }
    }
    None
}
