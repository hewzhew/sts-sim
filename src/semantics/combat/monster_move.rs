#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageKind {
    Normal,
    HpLoss,
    Thorns,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectStrength {
    Normal,
    Strong,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackSpec {
    pub base_damage: i32,
    pub hits: u8,
    pub damage_kind: DamageKind,
}

impl AttackSpec {
    pub fn total_base_damage(&self) -> i32 {
        self.base_damage.saturating_mul(self.hits.max(1) as i32)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BuffSpec {
    pub key: Option<&'static str>,
    pub amount: Option<i32>,
}

impl BuffSpec {
    pub fn unknown() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebuffSpec {
    pub key: Option<&'static str>,
    pub amount: Option<i32>,
    pub strength: EffectStrength,
}

impl Default for DebuffSpec {
    fn default() -> Self {
        Self {
            key: None,
            amount: None,
            strength: EffectStrength::Normal,
        }
    }
}

impl DebuffSpec {
    pub fn unknown() -> Self {
        Self::default()
    }

    pub fn strong_unknown() -> Self {
        Self {
            strength: EffectStrength::Strong,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DefendSpec {
    pub block: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecialMoveSpec {
    pub key: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonsterMoveSpec {
    Attack(AttackSpec),
    AttackBuff(AttackSpec, BuffSpec),
    AttackDebuff(AttackSpec, DebuffSpec),
    AttackDefend(AttackSpec, DefendSpec),
    Buff(BuffSpec),
    Debuff(DebuffSpec),
    StrongDebuff(DebuffSpec),
    Defend(DefendSpec),
    DefendDebuff(DefendSpec, DebuffSpec),
    DefendBuff(DefendSpec, BuffSpec),
    Escape,
    Magic,
    Sleep,
    Stun,
    Debug,
    None,
    Special(SpecialMoveSpec),
    Unknown,
}

impl MonsterMoveSpec {
    pub fn attack(&self) -> Option<&AttackSpec> {
        match self {
            Self::Attack(spec)
            | Self::AttackBuff(spec, _)
            | Self::AttackDebuff(spec, _)
            | Self::AttackDefend(spec, _) => Some(spec),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonsterTurnPlan {
    pub move_id: u8,
    pub spec: MonsterMoveSpec,
}

#[cfg(test)]
mod tests {
    use super::{AttackSpec, DamageKind, MonsterMoveSpec};

    #[test]
    fn attack_accessor_returns_embedded_attack_spec() {
        let spec = MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 7,
            hits: 3,
            damage_kind: DamageKind::Normal,
        });

        let attack = spec.attack().expect("attack spec");
        assert_eq!(attack.base_damage, 7);
        assert_eq!(attack.hits, 3);
    }
}
