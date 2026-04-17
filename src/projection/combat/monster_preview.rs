use crate::semantics::combat::MonsterMoveSpec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibleIntentKind {
    Attack,
    AttackBuff,
    AttackDebuff,
    AttackDefend,
    Buff,
    Debuff,
    StrongDebuff,
    Defend,
    DefendDebuff,
    DefendBuff,
    Escape,
    Magic,
    Sleep,
    Stun,
    Debug,
    None,
    Special,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonsterMovePreview {
    pub damage_per_hit: Option<i32>,
    pub hits: u8,
    pub total_damage: Option<i32>,
    pub visible_intent: VisibleIntentKind,
}

impl MonsterMovePreview {
    pub fn from_spec(spec: &MonsterMoveSpec, damage_per_hit: Option<i32>) -> Self {
        let hits = spec.attack().map(|attack| attack.hits.max(1)).unwrap_or(0);
        let total_damage = damage_per_hit.map(|damage| damage.saturating_mul(hits as i32));

        Self {
            damage_per_hit,
            hits,
            total_damage,
            visible_intent: visible_intent_kind(spec),
        }
    }
}

fn visible_intent_kind(spec: &MonsterMoveSpec) -> VisibleIntentKind {
    match spec {
        MonsterMoveSpec::Attack(_) => VisibleIntentKind::Attack,
        MonsterMoveSpec::AttackBuff(_, _) => VisibleIntentKind::AttackBuff,
        MonsterMoveSpec::AttackDebuff(_, _) => VisibleIntentKind::AttackDebuff,
        MonsterMoveSpec::AttackDefend(_, _) => VisibleIntentKind::AttackDefend,
        MonsterMoveSpec::Buff(_) => VisibleIntentKind::Buff,
        MonsterMoveSpec::Debuff(_) => VisibleIntentKind::Debuff,
        MonsterMoveSpec::StrongDebuff(_) => VisibleIntentKind::StrongDebuff,
        MonsterMoveSpec::Defend(_) => VisibleIntentKind::Defend,
        MonsterMoveSpec::DefendDebuff(_, _) => VisibleIntentKind::DefendDebuff,
        MonsterMoveSpec::DefendBuff(_, _) => VisibleIntentKind::DefendBuff,
        MonsterMoveSpec::Escape => VisibleIntentKind::Escape,
        MonsterMoveSpec::Magic => VisibleIntentKind::Magic,
        MonsterMoveSpec::Sleep => VisibleIntentKind::Sleep,
        MonsterMoveSpec::Stun => VisibleIntentKind::Stun,
        MonsterMoveSpec::Debug => VisibleIntentKind::Debug,
        MonsterMoveSpec::None => VisibleIntentKind::None,
        MonsterMoveSpec::Special(_) => VisibleIntentKind::Special,
        MonsterMoveSpec::Unknown => VisibleIntentKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{MonsterMovePreview, VisibleIntentKind};
    use crate::semantics::combat::{AttackSpec, DamageKind, MonsterMoveSpec};

    #[test]
    fn preview_multiplies_damage_by_hit_count() {
        let spec = MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 6,
            hits: 2,
            damage_kind: DamageKind::Normal,
        });

        let preview = MonsterMovePreview::from_spec(&spec, Some(8));
        assert_eq!(preview.damage_per_hit, Some(8));
        assert_eq!(preview.hits, 2);
        assert_eq!(preview.total_damage, Some(16));
        assert_eq!(preview.visible_intent, VisibleIntentKind::Attack);
    }
}
