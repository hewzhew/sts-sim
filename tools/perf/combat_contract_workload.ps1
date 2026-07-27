function Get-StsCombatContractWorkloadArguments([string] $CasePath) {
    return @(
        "--case", $CasePath,
        "--max-nodes", "20000",
        "--max-selections", "20000",
        "--wall-ms", "5000",
        "--max-potions-used", "2",
        "--improve-incumbent",
        "--typed-plan-guide",
        "--expect-witness",
        "--expect-min-final-hp", "70",
        "--performance-only"
    )
}
