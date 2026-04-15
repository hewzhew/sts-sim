//! Bit-perfect replicas of the Slay the Spire RNG subsystems.
//!
//! The game uses two distinct PRNG algorithms:
//! - **RandomXS128** (LibGDX): xorshift128+ for all gameplay randomness
//! - **java.util.Random** (LCG): only used by `Collections.shuffle()`
//!
//! Both are replicated at the bit level. Using any other RNG source
//! (e.g., `rand` crate, `thread_rng`) would break deterministic replay.

// ─── RandomXS128 (LibGDX) ───────────────────────────────────────────────────

/// Bit-perfect replica of LibGDX `RandomXS128`, the core PRNG for Slay the Spire.
///
/// Seeded via `murmurHash3(seed)` exactly as in Java. Every public method
/// increments `counter` to enable RNG consumption tracking.
#[derive(Clone, Debug, PartialEq)]
pub struct StsRng {
    pub seed0: u64,
    pub seed1: u64,
    pub counter: u32,
}

impl StsRng {
    /// Create a new RNG from a game seed, matching Java's `new Random(seed)`.
    pub fn new(seed: u64) -> Self {
        let seed = if seed == 0 { i64::MIN as u64 } else { seed };
        let seed0 = murmur_hash3(seed);
        let seed1 = murmur_hash3(seed0);
        Self {
            seed0,
            seed1,
            counter: 0,
        }
    }

    /// Advance the RNG by `count` calls, used for save-file restoration.
    pub fn new_with_counter(seed: u64, count: u32) -> Self {
        let mut rng = Self::new(seed);
        for _ in 0..count {
            rng.random(999);
        }
        rng
    }

    // ── Core xorshift128+ ──────────────────────────────────────────────

    #[inline]
    fn next_long(&mut self) -> u64 {
        let mut s1 = self.seed0;
        let s0 = self.seed1;
        self.seed0 = s0;
        s1 ^= s1 << 23;
        self.seed1 = s1 ^ s0 ^ (s1 >> 17) ^ (s0 >> 26);
        self.seed1.wrapping_add(s0)
    }

    /// Bounded u64 with rejection sampling (mirrors `RandomXS128.nextLong(n)`).
    #[inline]
    fn next_long_bounded(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        loop {
            let bits = self.next_long() >> 1;
            let value = bits % n;
            if bits.wrapping_sub(value).wrapping_add(n - 1) <= u64::MAX / 2 {
                return value;
            }
        }
    }

    #[allow(dead_code)]
    #[inline]
    fn next_int(&mut self) -> i32 {
        self.next_long() as i32
    }

    #[inline]
    fn next_int_bounded(&mut self, n: i32) -> i32 {
        self.next_long_bounded(n as u64) as i32
    }

    #[inline]
    fn next_float(&mut self) -> f32 {
        (self.next_long() >> 40) as f32 * (1.0 / (1u64 << 24) as f32)
    }

    #[allow(dead_code)]
    #[inline]
    fn next_double(&mut self) -> f64 {
        (self.next_long() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    #[inline]
    fn next_boolean(&mut self) -> bool {
        (self.next_long() & 1) != 0
    }

    // ── STS wrapper layer (maps to com.megacrit.cardcrawl.random.Random) ─

    /// `random(range)` → uniform integer in `[0, range]` (inclusive).
    #[track_caller]
    pub fn random(&mut self, range: i32) -> i32 {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random({}) at {}:{}",
                self.counter,
                range,
                loc.file(),
                loc.line()
            );
        }
        self.next_int_bounded(range + 1)
    }

    /// `random(start, end)` → uniform integer in `[start, end]` (inclusive).
    #[track_caller]
    pub fn random_range(&mut self, start: i32, end: i32) -> i32 {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random_range({},{}) at {}:{}",
                self.counter,
                start,
                end,
                loc.file(),
                loc.line()
            );
        }
        start + self.next_int_bounded(end - start + 1)
    }

    /// `randomBoolean()` → 50/50.
    #[track_caller]
    pub fn random_boolean(&mut self) -> bool {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random_boolean() at {}:{}",
                self.counter,
                loc.file(),
                loc.line()
            );
        }
        self.next_boolean()
    }

    /// `randomBoolean(chance)` → true with probability `chance`.
    #[track_caller]
    pub fn random_boolean_chance(&mut self, chance: f32) -> bool {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random_boolean_chance({}) at {}:{}",
                self.counter,
                chance,
                loc.file(),
                loc.line()
            );
        }
        self.next_float() < chance
    }

    /// `random()` → float in `[0.0, 1.0)`.
    #[track_caller]
    pub fn random_f32(&mut self) -> f32 {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random_f32() at {}:{}",
                self.counter,
                loc.file(),
                loc.line()
            );
        }
        self.next_float()
    }

    /// `random(range)` → float in `[0.0, range)`.
    #[track_caller]
    pub fn random_f32_range(&mut self, range: f32) -> f32 {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random_f32_range({}) at {}:{}",
                self.counter,
                range,
                loc.file(),
                loc.line()
            );
        }
        self.next_float() * range
    }

    /// `random(min, max)` → float in `[min, max)` (MathUtils.random matching).
    #[track_caller]
    pub fn random_f32_min_max(&mut self, min: f32, max: f32) -> f32 {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random_f32_min_max({},{}) at {}:{}",
                self.counter,
                min,
                max,
                loc.file(),
                loc.line()
            );
        }
        min + self.next_float() * (max - min)
    }

    /// `randomLong()` → raw u64, used as seed for `java.util.Random` in shuffles.
    #[track_caller]
    pub fn random_long(&mut self) -> u64 {
        self.counter += 1;
        if std::env::var("RNG_TRACE").is_ok() {
            let loc = std::panic::Location::caller();
            eprintln!(
                "  [RNG] counter={} random_long() at {}:{}",
                self.counter,
                loc.file(),
                loc.line()
            );
        }
        self.next_long()
    }
}

fn murmur_hash3(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

// ─── java.util.Random (LCG) ─────────────────────────────────────────────────

/// Bit-perfect replica of `java.util.Random` (48-bit LCG).
///
/// Used exclusively by `Collections.shuffle(list, new Random(seed))` in the
/// Java engine. This is NOT the game's primary PRNG.
#[derive(Clone, Debug)]
pub struct JavaUtilRandom {
    seed: u64,
}

const LCG_MULTIPLIER: u64 = 0x5DEECE66D;
const LCG_ADDEND: u64 = 0xB;
const LCG_MASK: u64 = (1u64 << 48) - 1;

impl JavaUtilRandom {
    pub fn new(seed: u64) -> Self {
        Self {
            seed: (seed ^ LCG_MULTIPLIER) & LCG_MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self
            .seed
            .wrapping_mul(LCG_MULTIPLIER)
            .wrapping_add(LCG_ADDEND)
            & LCG_MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// `nextInt(bound)` with rejection sampling, matching Java exactly.
    pub fn next_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }
        if bound & (bound - 1) == 0 {
            return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                return val;
            }
        }
    }
}

// ─── Shuffle ─────────────────────────────────────────────────────────────────

/// Mirrors `Collections.shuffle(list, new Random(rng.randomLong()))`.
///
/// Consumes exactly one `nextLong()` from the game's xorshift128+ RNG,
/// then uses `java.util.Random` (LCG) for the Fisher-Yates shuffle.
pub fn shuffle_with_random_long<T>(items: &mut [T], game_rng: &mut StsRng) {
    let seed = game_rng.random_long();
    let mut jur = JavaUtilRandom::new(seed);
    for i in (1..items.len()).rev() {
        let j = jur.next_int((i + 1) as i32) as usize;
        items.swap(i, j);
    }
}

// ─── RNG Pool ────────────────────────────────────────────────────────────────

/// All 12 RNG streams used by the Java engine.
///
/// Seven are persistent across floors (seeded once at run start):
///   `monster_rng`, `event_rng`, `merchant_rng`, `card_rng`,
///   `treasure_rng`, `relic_rng`, `potion_rng`
///
/// Five are re-seeded per floor via `generate_floor_seeds()`:
///   `monster_hp_rng`, `ai_rng`, `shuffle_rng`, `card_random_rng`, `misc_rng`
#[derive(Clone, Debug, PartialEq)]
pub struct RngPool {
    // Persistent (run-level)
    pub monster_rng: StsRng,
    pub event_rng: StsRng,
    pub merchant_rng: StsRng,
    pub card_rng: StsRng,
    pub treasure_rng: StsRng,
    pub relic_rng: StsRng,
    pub potion_rng: StsRng,
    // Per-floor (re-seeded on room transition)
    pub monster_hp_rng: StsRng,
    pub ai_rng: StsRng,
    pub shuffle_rng: StsRng,
    pub card_random_rng: StsRng,
    pub misc_rng: StsRng,
}

impl Default for RngPool {
    fn default() -> Self {
        Self::new(0)
    }
}

impl RngPool {
    /// Initialize all streams from a single game seed (mirrors `generateSeeds()`).
    pub fn new(seed: u64) -> Self {
        Self {
            monster_rng: StsRng::new(seed),
            event_rng: StsRng::new(seed),
            merchant_rng: StsRng::new(seed),
            card_rng: StsRng::new(seed),
            treasure_rng: StsRng::new(seed),
            relic_rng: StsRng::new(seed),
            potion_rng: StsRng::new(seed),
            monster_hp_rng: StsRng::new(seed),
            ai_rng: StsRng::new(seed),
            shuffle_rng: StsRng::new(seed),
            card_random_rng: StsRng::new(seed),
            misc_rng: StsRng::new(seed),
        }
    }

    /// Re-seed per-floor streams (mirrors `nextRoomTransition()` in Java).
    ///
    /// ```java
    /// monsterHpRng  = new Random(Settings.seed + (long)floorNum);
    /// aiRng         = new Random(Settings.seed + (long)floorNum);
    /// shuffleRng    = new Random(Settings.seed + (long)floorNum);
    /// cardRandomRng = new Random(Settings.seed + (long)floorNum);
    /// miscRng       = new Random(Settings.seed + (long)floorNum);
    /// ```
    pub fn generate_floor_seeds(&mut self, seed: u64, floor_num: i32) {
        let floor_seed = seed.wrapping_add(floor_num as u64);
        self.monster_hp_rng = StsRng::new(floor_seed);
        self.ai_rng = StsRng::new(floor_seed);
        self.shuffle_rng = StsRng::new(floor_seed);
        self.card_random_rng = StsRng::new(floor_seed);
        self.misc_rng = StsRng::new(floor_seed);
    }
}
