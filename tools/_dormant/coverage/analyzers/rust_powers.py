"""Analyze Rust powers/mod.rs to extract which PowerIds appear in each hook dispatch function."""
import re
from pathlib import Path


# Map Rust function names → Java hook names
POWER_DISPATCH_MAP = {
    "resolve_power_on_apply": "onInitialApplication",
    "resolve_power_on_card_draw": "onCardDraw",
    "resolve_power_on_use_card": "onUseCard",
    "resolve_power_on_player_card_played": "onAfterUseCard",
    "resolve_power_on_exhaust": "onExhaust",
    "resolve_power_on_hp_lost": "wasHPLost",
    "resolve_power_on_card_played": "onUseCard",  # monster-side onUseCard
    "resolve_power_at_turn_start": "atStartOfTurn",
    "resolve_power_on_post_draw": "atStartOfTurnPostDraw",
    "resolve_power_at_end_of_turn": "atEndOfTurn",
    "resolve_power_at_end_of_round": "atEndOfRound",
    "resolve_power_on_card_drawn": "onCardDraw",
    "resolve_power_on_attack": "onAttack",
    "resolve_power_on_block_gained": "onGainedBlock",
    "resolve_power_on_attacked": "onAttacked",
    "resolve_power_on_death": "onDeath",
    "resolve_power_on_remove": "onRemove",
    "resolve_power_on_attack_to_change_damage": "atDamageGive",
    "resolve_power_on_attacked_to_change_damage": "atDamageReceive",
    "resolve_power_on_calculate_damage_to_enemy": "atDamageGive",
    "resolve_power_on_calculate_block": "onPlayerGainedBlock",
    "resolve_power_on_calculate_damage_from_player": "atDamageReceive",
}


def analyze_power_dispatch(mod_rs_path: Path) -> dict[str, list[str]]:
    """Parse powers/mod.rs → {PowerId_variant: [rust_function_names]}.

    Scans each resolve_power_* function for PowerId::Xxx match arms.
    Returns which dispatch functions contain each PowerId.
    """
    if not mod_rs_path.exists():
        return {}

    text = mod_rs_path.read_text(encoding="utf-8")
    result: dict[str, list[str]] = {}

    # Find each function and its body
    # Pattern: `pub fn resolve_power_xxx(` ... until next `pub fn` or EOF
    fn_pattern = re.compile(r"pub fn (resolve_power_\w+)\(")
    fn_starts = [(m.group(1), m.start()) for m in fn_pattern.finditer(text)]

    for i, (fn_name, start) in enumerate(fn_starts):
        end = fn_starts[i + 1][1] if i + 1 < len(fn_starts) else len(text)
        fn_body = text[start:end]

        # Find all PowerId::Xxx in match arms
        for m in re.finditer(r"PowerId::(\w+)", fn_body):
            variant = m.group(1)
            if variant not in result:
                result[variant] = []
            if fn_name not in result[variant]:
                result[variant].append(fn_name)

    return result


def get_java_hooks_for_power(power_variant: str,
                              dispatch_map: dict[str, list[str]]) -> list[tuple[str, str]]:
    """For a PowerId variant, return [(java_hook_name, rust_fn_name)] pairs."""
    rust_fns = dispatch_map.get(power_variant, [])
    pairs = []
    for fn_name in rust_fns:
        java_hook = POWER_DISPATCH_MAP.get(fn_name, fn_name)
        pairs.append((java_hook, fn_name))
    return pairs
