"""Data models for coverage analysis."""
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class EntityCategory(Enum):
    POWER = "power"
    RELIC = "relic"
    CARD = "card"
    POTION = "potion"
    MONSTER = "monster"


class HookStatus(Enum):
    IMPLEMENTED = "implemented"   # match arm exists in Rust dispatch
    MISSING = "missing"          # Java has it, Rust doesn't
    SKIPPED = "skipped"          # Cosmetic-only hook (updateDescription, makeCopy)
    INLINE = "inline"            # Logic is inline in hooks.rs, not a separate module


# Java hooks that are cosmetic / not relevant to simulator logic
SKIPPABLE_HOOKS = frozenset({
    "updateDescription", "makeCopy", "getUpdatedDescription",
    "setDescription", "use",  # use() is the card play — tracked separately
})


@dataclass
class JavaHook:
    """A single hook method in Java (e.g., onUseCard, atBattleStart)."""
    name: str
    status: HookStatus = HookStatus.MISSING
    rust_function: Optional[str] = None  # e.g. "resolve_power_on_use_card"


@dataclass
class JavaEntity:
    """A Java class (Power, Relic, Card) from the source extractor."""
    class_name: str        # e.g. "AngerPower"
    java_id: str           # e.g. "Anger" (the string ID used in-game)
    category: EntityCategory
    java_file: str = ""    # e.g. "powers\\AngerPower.java"
    hooks: list[JavaHook] = field(default_factory=list)
    has_scattered_logic: bool = False
    scattered_refs: int = 0


@dataclass
class RustEntity:
    """A Rust implementation file or match branch."""
    enum_variant: str      # e.g. "Anger" (PowerId::Anger)
    file_path: Optional[str] = None  # e.g. "content/powers/core/anger.rs"
    file_exists: bool = False
    matched_hooks: list[str] = field(default_factory=list)  # hooks where this ID appears


@dataclass
class CoverageEntry:
    """Combined Java + Rust coverage for one entity."""
    java: JavaEntity
    rust: Optional[RustEntity] = None
    hook_details: list[JavaHook] = field(default_factory=list)

    @property
    def total_hooks(self) -> int:
        return len([h for h in self.hook_details if h.status != HookStatus.SKIPPED])

    @property
    def implemented_hooks(self) -> int:
        return len([h for h in self.hook_details
                    if h.status in (HookStatus.IMPLEMENTED, HookStatus.INLINE)])

    @property
    def coverage_pct(self) -> float:
        total = self.total_hooks
        if total == 0:
            return 100.0
        return (self.implemented_hooks / total) * 100.0

    @property
    def status_icon(self) -> str:
        if self.rust is None or not self.rust.file_exists:
            return "❌"
        pct = self.coverage_pct
        if pct >= 100:
            return "✅"
        elif pct > 0:
            return "🟡"
        return "❌"


@dataclass
class CategorySummary:
    """Aggregated stats for one entity category."""
    category: EntityCategory
    total_java: int = 0
    has_rust_file: int = 0
    fully_covered: int = 0
    partially_covered: int = 0
    not_covered: int = 0
    entries: list[CoverageEntry] = field(default_factory=list)

    @property
    def coverage_pct(self) -> float:
        if self.total_java == 0:
            return 0.0
        return (self.fully_covered / self.total_java) * 100.0
