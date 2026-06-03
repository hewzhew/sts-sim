pub fn run_control_help() -> &'static str {
    "\
Help:
  Core:
    main/state, deck, map, ms/map-summary, mf/map-full, bd/boundary, rs/route-suggest, rg/route-go, relics, potions, inspect <id>, case [path], d/details, r/raw, mark <name>, marks, quit
    map = full visible map; ms = route summary; rs = route planner evidence
    bd = current NonCombatDecisionRecordV1 summary when stopped at a noncombat boundary
    n/next = advance to next human choice; ar/auto-run = longer route-planner automation; <id> chooses a visible option
    Enter chooses the single visible option when safe

  Combat:
    play <hand_idx> [target_slot], end, potion <slot> [target_slot], discard-potion <slot>
    draw, discard, exhaust, actions, action <idx>
    sc/search-combat [max_nodes=N] [wall_ms=N] [max_hp_loss=N|off] [potion=never|all|semantic] [max_potions=N] [rollout=enemy_mechanics_adaptive_no_potion|conservative_no_potion|phase_aware_no_potion|turn_beam_no_potion|disabled] [rollouts=N] [rollout_actions=N] [beam=N] [turn_plan=diagnostic_only|root_frontier_seed|turn_boundary_frontier_seed|tactical_enemy_turn_boundary_frontier_seed] [save=case|path]
    sc/n/nr high-stakes default: boss combat uses semantic potions with max_potions=2, elite combat uses max_potions=1, unless potion/defaults override it
    sd/search-defaults [max_nodes=N] [wall_ms=N] [max_hp_loss=N|off] [potion=never|all|semantic] [max_potions=N] sets session defaults for sc/n/nr; sd clear resets them

  Map/Event/Reward:
    rs/route-suggest = read-only route evidence; rg/route-go = execute selected route planner move; go <x>, fly <x> <y>, event <idx>, claim <idx>, pick <idx>, select <idx...>
    select <idx...> submits the visible selection surface; empty select chooses nothing when allowed
    hand-select <uuid...>, grid-select <uuid...>, choose <idx>, open, relic <idx>

  Shop/Campfire:
    card <idx>, relic <idx>, potion <idx>, buy card|relic|potion <idx>, purge <deck_idx>
    rest, smith <deck_idx>, dig, lift, recall, toke <deck_idx>

  Combat Capture / Benchmark:
    startup flag: --auto-capture-combat [--auto-capture-combat-root <benchmark_dir>]
      automatically captures each new combat at the first stable player-turn boundary
    cap <case_id> [label] = capture current combat under tools/artifacts/benchmarks/seed<seed>_act<act>
    b/baseline = save last completed combat baseline for the last capture-case
    capture <path> [label]
    capture-case <benchmark_dir> <case_id> [label]
    save-baseline <path> [case_id]
    save-baseline-case <benchmark_dir> <case_id>
    bench-add <benchmark_dir> <case_id>

  Automation:
    n/next/advance-to-human-boundary [route=manual|planner] [max_nodes=N] [wall_ms=N] [max_hp_loss=N|off] [potion=never|all|semantic] [max_potions=N] [rollout=enemy_mechanics_adaptive_no_potion|conservative_no_potion|phase_aware_no_potion|turn_beam_no_potion|disabled] [rollouts=N] [rollout_actions=N] [beam=N] [turn_plan=diagnostic_only|root_frontier_seed|turn_boundary_frontier_seed|tactical_enemy_turn_boundary_frontier_seed] [frontier=single_queue|round_robin_eval_buckets] [save=case|path] [max_ops=N]
    nr/next-route = n route=planner
    ar/auto-run = repeat guarded automation with route=planner and a larger default max_ops budget; stops at the next human-required boundary
    Boss/elite n/nr requires max_hp_loss=N or explicit max_hp_loss=off; this prevents auto-search from silently spending too much HP.
    If max_hp_loss is set, high-stakes auto combat first accepts a no-potion win under that limit before falling back to semantic potions.
    If max_hp_loss is set and the no/default-potion line misses the limit, n/nr may try one bounded potion-rescue search unless potion/max_potions were explicitly set.
    max_hp_loss also lets search stop early after an exact complete win within that hp-loss limit; this is a practical acceptance gate, not an optimality claim.
    startup flags: --search-max-nodes N, --search-wall-ms N, and --search-max-hp-loss N set initial defaults for sc/n/nr
    sd/search-defaults changes those defaults inside the current session
    startup flag: --record writes this new run to an auto-named trace so mark <name> can be used without typing a trace path
    mark <name> saves the current recorded trace position as a bookmark; start later with --goto <name>
    auto-reward
    auto-reward gold|potion|all on|off"
}

pub fn run_control_short_hint() -> &'static str {
    "main | n=advance | nr=route-advance | ar=auto-run | mark <name> | marks | deck | map=full-map | ms=route-summary | bd=boundary | rs=route-suggest | rg=route-go | sd=search-defaults | relics | potions | inspect <id> | auto-reward | details | raw | help"
}
