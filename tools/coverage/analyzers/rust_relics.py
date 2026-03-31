"""Analyze Rust relics/hooks.rs to extract which RelicIds appear in each hook function."""
import re
from pathlib import Path


# Map Rust function names → Java hook names
RELIC_DISPATCH_MAP = {
    "at_battle_start": "atBattleStart",
    "on_shuffle": "onShuffle",
    "on_spawn_monster": "onSpawnMonster",
    "on_exhaust": "onExhaust",
    "on_lose_hp": "wasHPLost",
    "on_victory": "onVictory",
    "at_turn_start": "atBattleStart",  # Some map to atPreBattle, some to atBattleStartPreDraw
    "at_end_of_turn": "onPlayerEndTurn",
    "on_use_card": "onUseCard",
    "on_apply_power": "onTrigger",  # Champion Belt uses onTrigger in Java
    "on_monster_death": "onMonsterDeath",
    "on_discard": "onManualDiscard",
    "on_calculate_heal": "onPlayerHeal",
    "on_calculate_x_cost": "onUseCard",  # Chemical X is in onUseCard context
    "on_calculate_block_retained": "atEndOfTurn",  # Calipers
    "on_calculate_energy_retained": "atEndOfTurn",  # Ice Cream
    "on_scry": "onScry",
    "on_receive_power_modify": "onReceivePowerModify",
    "on_calculate_vulnerable_multiplier": "onAttacked",
    "on_use_potion": "onUsePotion",
    "on_change_stance": "onChangeStance",
}


def analyze_relic_dispatch(hooks_rs_path: Path) -> dict[str, list[str]]:
    """Parse relics/hooks.rs → {RelicId_variant: [rust_fn_names]}.

    Scans each pub fn for RelicId::Xxx match arms.
    """
    if not hooks_rs_path.exists():
        return {}

    text = hooks_rs_path.read_text(encoding="utf-8")
    result: dict[str, list[str]] = {}

    # Find each function
    fn_pattern = re.compile(r"pub fn (\w+)\(")
    fn_starts = [(m.group(1), m.start()) for m in fn_pattern.finditer(text)]

    for i, (fn_name, start) in enumerate(fn_starts):
        if fn_name.startswith("_") or fn_name == "r_idx":
            continue
        end = fn_starts[i + 1][1] if i + 1 < len(fn_starts) else len(text)
        fn_body = text[start:end]

        # Find all RelicId::Xxx
        for m in re.finditer(r"RelicId::(\w+)", fn_body):
            variant = m.group(1)
            if variant not in result:
                result[variant] = []
            if fn_name not in result[variant]:
                result[variant].append(fn_name)

    return result
