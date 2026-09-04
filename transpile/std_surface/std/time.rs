//! `std::time`
//!
//! `Duration::from_millis` is the corpus's only real use (retry backoff and the
//! websocket reconnect timer); `Instant` is here as its obvious neighbour.
//! Neither exists in the browser the way it does natively — that is the
//! emission layer's problem, not this declaration's.

pub struct Duration;

impl Duration {
    pub const ZERO: Duration = Duration;
    pub fn new(secs: u64, nanos: u32) -> Duration { todo!() }
    pub fn from_secs(secs: u64) -> Duration { todo!() }
    pub fn from_millis(millis: u64) -> Duration { todo!() }
    pub fn from_micros(micros: u64) -> Duration { todo!() }
    pub fn from_nanos(nanos: u64) -> Duration { todo!() }
    pub fn from_secs_f64(secs: f64) -> Duration { todo!() }
    pub fn as_secs(&self) -> u64 { todo!() }
    pub fn as_millis(&self) -> u128 { todo!() }
    pub fn as_micros(&self) -> u128 { todo!() }
    pub fn as_nanos(&self) -> u128 { todo!() }
    pub fn as_secs_f64(&self) -> f64 { todo!() }
    pub fn subsec_millis(&self) -> u32 { todo!() }
    pub fn subsec_nanos(&self) -> u32 { todo!() }
    pub fn checked_add(self, rhs: Duration) -> Option<Duration> { todo!() }
    pub fn checked_sub(self, rhs: Duration) -> Option<Duration> { todo!() }
    pub fn saturating_add(self, rhs: Duration) -> Duration { todo!() }
    pub fn saturating_sub(self, rhs: Duration) -> Duration { todo!() }
    pub fn is_zero(&self) -> bool { todo!() }
}

impl Clone for Duration { fn clone(&self) -> Duration { todo!() } }
impl Copy for Duration {}
impl Debug for Duration { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Default for Duration { fn default() -> Duration { todo!() } }
impl PartialEq for Duration { fn eq(&self, other: &Duration) -> bool { todo!() } }
impl Eq for Duration {}
impl PartialOrd for Duration { fn partial_cmp(&self, other: &Duration) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for Duration { fn cmp(&self, other: &Duration) -> std::cmp::Ordering { todo!() } }
impl Add for Duration { type Output = Duration; fn add(self, rhs: Duration) -> Duration { todo!() } }
impl Sub for Duration { type Output = Duration; fn sub(self, rhs: Duration) -> Duration { todo!() } }
impl Mul<u32> for Duration { type Output = Duration; fn mul(self, rhs: u32) -> Duration { todo!() } }

pub struct Instant;

impl Instant {
    pub fn now() -> Instant { todo!() }
    pub fn elapsed(&self) -> Duration { todo!() }
    pub fn duration_since(&self, earlier: Instant) -> Duration { todo!() }
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> { todo!() }
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration { todo!() }
}

impl Clone for Instant { fn clone(&self) -> Instant { todo!() } }
impl Copy for Instant {}
impl Debug for Instant { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl PartialEq for Instant { fn eq(&self, other: &Instant) -> bool { todo!() } }
impl Eq for Instant {}
impl PartialOrd for Instant { fn partial_cmp(&self, other: &Instant) -> Option<std::cmp::Ordering> { todo!() } }
impl Ord for Instant { fn cmp(&self, other: &Instant) -> std::cmp::Ordering { todo!() } }
impl Sub for Instant { type Output = Duration; fn sub(self, rhs: Instant) -> Duration { todo!() } }
impl Add<Duration> for Instant { type Output = Instant; fn add(self, rhs: Duration) -> Instant { todo!() } }

pub struct SystemTime;
pub struct SystemTimeError;

pub const UNIX_EPOCH: SystemTime = SystemTime;

impl SystemTime {
    pub const UNIX_EPOCH: SystemTime = SystemTime;
    pub fn now() -> SystemTime { todo!() }
    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> { todo!() }
    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> { todo!() }
}

impl Clone for SystemTime { fn clone(&self) -> SystemTime { todo!() } }
impl Copy for SystemTime {}
impl Debug for SystemTime { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Debug for SystemTimeError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl Display for SystemTimeError { fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { todo!() } }
impl std::error::Error for SystemTimeError {}
