#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEquivalenceMode {
    Off,
    Safe,
    Experimental,
}

impl SearchEquivalenceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Safe => "safe",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEquivalenceKind {
    Exact,
    Heuristic,
}

impl SearchEquivalenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Heuristic => "heuristic",
        }
    }
}

pub(crate) fn default_equivalence_mode() -> SearchEquivalenceMode {
    match std::env::var("STS_SEARCH_EQUIVALENCE_MODE")
        .unwrap_or_else(|_| "safe".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => SearchEquivalenceMode::Off,
        "experimental" | "exp" => SearchEquivalenceMode::Experimental,
        _ => SearchEquivalenceMode::Safe,
    }
}
