//! IANA time zones backed by `jiff`'s bundled Time Zone Database, exposed
//! through `chrono`'s [`TimeZone`] / [`Offset`] traits.
//!
//! This replaces `chrono_tz::Tz`. The zone rules come from the `jiff-tzdb`
//! crate (compiled into the binary, never read from the operating system), so
//! every platform computes the same offsets. Upgrading the time zone data is a
//! `cargo update -p jiff-tzdb`.
//!
//! Semantics intentionally mirror `chrono_tz::Tz`:
//! - names are looked up case-sensitively (`"america/new_york"` is rejected);
//! - equality is by name (`UTC` and `Etc/UTC` are different zones);
//! - a local time in a gap maps to `LocalResult::None`, a local time in a fold
//!   maps to `LocalResult::Ambiguous(earliest, latest)`;
//! - `Display` / `Debug` of the zone print its name, of the offset its
//!   abbreviation (e.g. `PST`), or `+HH[MM]` when the zone has none.

use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use chrono::{
    FixedOffset, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Timelike,
};
use jiff::tz::{AmbiguousOffset, TimeZone as JiffTimeZone};

/// Version of the IANA Time Zone Database compiled into this crate,
/// e.g. `"2026f"`.
#[must_use]
pub fn tzdb_version() -> &'static str {
    jiff_tzdb::VERSION
}

/// An IANA time zone such as `America/Vancouver`.
///
/// Cheap to copy: zones are parsed once on first use and then interned for
/// the lifetime of the process (bounded by the ~600 names in the database).
#[derive(Clone, Copy)]
pub struct IanaTz {
    name: &'static str,
    /// `None` means UTC (so that [`IanaTz::UTC`] can be a `const`).
    zone: Option<&'static JiffTimeZone>,
}

/// Error returned when a time zone name is not in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTimeZone(pub String);

impl fmt::Display for UnknownTimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' is not a valid timezone", self.0)
    }
}

impl std::error::Error for UnknownTimeZone {}

static UTC_ZONE: JiffTimeZone = JiffTimeZone::UTC;

fn interned() -> &'static Mutex<HashMap<&'static str, &'static JiffTimeZone>> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static JiffTimeZone>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl IanaTz {
    /// The `UTC` zone.
    pub const UTC: Self = Self {
        name: "UTC",
        zone: None,
    };

    /// Looks up a zone by its exact (case-sensitive) IANA name.
    #[must_use]
    pub fn get(name: &str) -> Option<Self> {
        if name == "UTC" {
            return Some(Self::UTC);
        }
        let (canonical, data) = jiff_tzdb::get(name)?;
        // jiff-tzdb matches case-insensitively; chrono-tz did not.
        if canonical != name {
            return None;
        }
        let mut cache = interned().lock().unwrap_or_else(|e| e.into_inner());
        let zone = match cache.get(canonical) {
            Some(zone) => *zone,
            None => {
                let zone: &'static JiffTimeZone =
                    Box::leak(Box::new(JiffTimeZone::tzif(canonical, data).ok()?));
                cache.insert(canonical, zone);
                zone
            }
        };
        Some(Self {
            name: canonical,
            zone: Some(zone),
        })
    }

    /// The IANA name of this zone.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    fn zone(&self) -> &'static JiffTimeZone {
        self.zone.unwrap_or(&UTC_ZONE)
    }

    fn offset_at_utc(&self, utc_secs: i64) -> IanaOffset {
        let offset = self.zone().to_offset(timestamp(utc_secs));
        IanaOffset::new(*self, offset.seconds(), utc_secs)
    }
}

impl PartialEq for IanaTz {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for IanaTz {}

impl Hash for IanaTz {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl fmt::Debug for IanaTz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl fmt::Display for IanaTz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl FromStr for IanaTz {
    type Err = UnknownTimeZone;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::get(s).ok_or_else(|| UnknownTimeZone(s.to_string()))
    }
}

/// The offset of an [`IanaTz`] at a particular instant.
#[derive(Clone, Copy)]
pub struct IanaOffset {
    tz: IanaTz,
    fixed: FixedOffset,
    /// The UTC instant (seconds) this offset was resolved for; only used to
    /// look up the abbreviation when the offset is displayed.
    utc_secs: i64,
}

impl IanaOffset {
    fn new(tz: IanaTz, offset_secs: i32, utc_secs: i64) -> Self {
        // tzdata offsets are within ±15h; chrono accepts anything below ±24h.
        let fixed = FixedOffset::east_opt(offset_secs)
            .unwrap_or_else(|| FixedOffset::east_opt(0).expect("zero offset is valid"));
        Self {
            tz,
            fixed,
            utc_secs,
        }
    }

    /// The zone this offset belongs to.
    #[must_use]
    pub fn tz(&self) -> IanaTz {
        self.tz
    }
}

impl Offset for IanaOffset {
    fn fix(&self) -> FixedOffset {
        self.fixed
    }
}

impl fmt::Display for IanaOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let info = self.tz.zone().to_offset_info(timestamp(self.utc_secs));
        f.write_str(info.abbreviation())
    }
}

impl fmt::Debug for IanaOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Seconds → jiff timestamp, clamped to jiff's supported range (years ±9999).
fn timestamp(secs: i64) -> jiff::Timestamp {
    let secs = secs.clamp(
        jiff::Timestamp::MIN.as_second(),
        jiff::Timestamp::MAX.as_second(),
    );
    jiff::Timestamp::from_second(secs).expect("clamped into range")
}

/// chrono local datetime → jiff civil datetime, clamped to years ±9999.
fn civil(local: &NaiveDateTime) -> jiff::civil::DateTime {
    use chrono::Datelike;
    let year = local.year();
    if year < -9999 {
        return jiff::civil::DateTime::MIN;
    }
    if year > 9999 {
        return jiff::civil::DateTime::MAX;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    jiff::civil::date(year as i16, local.month() as i8, local.day() as i8).at(
        local.hour() as i8,
        local.minute() as i8,
        local.second() as i8,
        0,
    )
}

impl TimeZone for IanaTz {
    type Offset = IanaOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        offset.tz
    }

    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<Self::Offset> {
        // Same rule as chrono-tz: any offset that occurred on that day is
        // acceptable, prefer one that is unambiguous.
        let earliest = self.offset_from_local_datetime(&local.and_time(NaiveTime::MIN));
        let latest = self.offset_from_local_datetime(
            &local
                .and_hms_opt(23, 59, 59)
                .expect("23:59:59 is a valid time"),
        );
        match (earliest, latest) {
            (result @ LocalResult::Single(_), _) | (_, result @ LocalResult::Single(_)) => result,
            (LocalResult::Ambiguous(offset, _), _) | (_, LocalResult::Ambiguous(offset, _)) => {
                LocalResult::Single(offset)
            }
            (LocalResult::None, LocalResult::None) => LocalResult::None,
        }
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        let local_secs = local.and_utc().timestamp();
        let at = |offset: jiff::tz::Offset| {
            let secs = offset.seconds();
            IanaOffset::new(*self, secs, local_secs - i64::from(secs))
        };
        // An offset is a real reading of this wall time only if the zone was
        // actually at that offset at the resulting instant. jiff 1.0.1
        // reports a bogus fold at transitions that keep the same offset
        // (e.g. America/Nuuk 2023-10-29T01:00Z, -02 -> -02); filtering the
        // candidates this way keeps the results identical to chrono-tz.
        let valid = |offset: jiff::tz::Offset| {
            let secs = offset.seconds();
            self.zone()
                .to_offset(timestamp(local_secs - i64::from(secs)))
                .seconds()
                == secs
        };
        match self.zone().to_ambiguous_timestamp(civil(local)).offset() {
            AmbiguousOffset::Unambiguous { offset } => LocalResult::Single(at(offset)),
            // `before` is the offset in effect before the transition, so for
            // a fold it gives the earlier of the two instants.
            AmbiguousOffset::Gap { before, after } | AmbiguousOffset::Fold { before, after } => {
                match (valid(before), valid(after)) {
                    (true, true) => LocalResult::Ambiguous(at(before), at(after)),
                    (true, false) => LocalResult::Single(at(before)),
                    (false, true) => LocalResult::Single(at(after)),
                    (false, false) => LocalResult::None,
                }
            }
        }
    }

    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset {
        self.offset_from_utc_datetime(&utc.and_time(NaiveTime::MIN))
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset {
        self.offset_at_utc(utc.and_utc().timestamp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn lookup_is_case_sensitive_like_chrono_tz() {
        assert!(IanaTz::get("America/New_York").is_some());
        assert!(IanaTz::get("america/new_york").is_none());
        assert!(IanaTz::get("Not/AZone").is_none());
        assert_eq!(IanaTz::get("UTC"), Some(IanaTz::UTC));
        assert_ne!(IanaTz::get("Etc/UTC"), Some(IanaTz::UTC));
        assert_eq!(IanaTz::get("US/Pacific").unwrap().name(), "US/Pacific");
    }

    #[test]
    fn gap_and_fold() {
        let ny: IanaTz = "America/New_York".parse().unwrap();
        // 2024-03-10 02:30 does not exist.
        let gap = NaiveDate::from_ymd_opt(2024, 3, 10)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        assert!(matches!(ny.from_local_datetime(&gap), LocalResult::None));
        // 2024-11-03 01:30 happens twice: first EDT (-4), then EST (-5).
        let fold = NaiveDate::from_ymd_opt(2024, 11, 3)
            .unwrap()
            .and_hms_opt(1, 30, 0)
            .unwrap();
        let result = ny.from_local_datetime(&fold);
        assert!(matches!(result, LocalResult::Ambiguous(..)));
        let (a, b) = (result.earliest().unwrap(), result.latest().unwrap());
        assert_eq!(a.offset().fix().local_minus_utc(), -4 * 3600);
        assert_eq!(b.offset().fix().local_minus_utc(), -5 * 3600);
        assert!(a < b);
        assert_eq!(a.offset().to_string(), "EDT");
        assert_eq!(b.offset().to_string(), "EST");
    }

    #[test]
    fn no_fold_at_transition_without_offset_change() {
        // America/Nuuk went from -02 (summer) to -02 (standard) at
        // 2023-10-29T01:00Z; 23:30 local the day before is a plain -02 time.
        let nuuk: IanaTz = "America/Nuuk".parse().unwrap();
        let local = NaiveDate::from_ymd_opt(2023, 10, 28)
            .unwrap()
            .and_hms_opt(23, 30, 0)
            .unwrap();
        let dt = nuuk.from_local_datetime(&local).single().unwrap();
        assert_eq!(
            dt.with_timezone(&Utc),
            Utc.with_ymd_and_hms(2023, 10, 29, 1, 30, 0).unwrap()
        );
    }

    #[test]
    fn vancouver_is_permanent_utc_minus_7_from_2026() {
        let van: IanaTz = "America/Vancouver".parse().unwrap();
        let dt = van.with_ymd_and_hms(2026, 11, 19, 10, 30, 0).unwrap();
        assert_eq!(
            dt.with_timezone(&Utc),
            Utc.with_ymd_and_hms(2026, 11, 19, 17, 30, 0).unwrap()
        );
    }

    #[test]
    fn numeric_abbreviation_matches_chrono_tz_format() {
        let tz: IanaTz = "America/Sao_Paulo".parse().unwrap();
        let dt = tz.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        assert_eq!(dt.offset().to_string(), "-03");
        let tz: IanaTz = "Asia/Kathmandu".parse().unwrap();
        let dt = tz.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        assert_eq!(dt.offset().to_string(), "+0545");
    }

    #[test]
    fn tzdb_is_at_least_2026f() {
        assert!(tzdb_version() >= "2026f", "{}", tzdb_version());
    }
}
