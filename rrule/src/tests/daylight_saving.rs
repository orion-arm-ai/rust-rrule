use crate::ParseError;
use crate::{tests::common::check_occurrences, RRuleSet};

/// Test that without `dst-fold-first` feature, ambiguous DST times return an error.
/// America/Mexico_City 2021 DST fall back:
/// - 2021-10-31 01:30 occurs twice due to DST fall back
/// - Without the feature, parsing should fail with ambiguous error
#[test]
#[cfg(not(feature = "dst-fold-first"))]
fn dst_ambiguous_time_returns_error() {
    // 2021-10-31 01:30 is ambiguous (DST fall back)
    let result: Result<RRuleSet, _> =
        "DTSTART;TZID=America/Mexico_City:20211031T013000\nRRULE:FREQ=YEARLY;COUNT=1".parse();

    // Without dst-fold-first feature, parsing should fail
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = format!("{}", err);
    assert!(
        err_str.contains("ambiguous"),
        "Expected ambiguous error, got: {}",
        err_str
    );
}

/// Test that the `dst-fold-first` feature correctly handles ambiguous DST times
/// by selecting the first (earlier) occurrence.
///
/// America/Mexico_City 2021 DST fall back:
/// - Clocks go back on 2021-10-31 at 03:00 -> 02:00
/// - 01:30 occurs twice: first at -05:00 (CDT), then at -06:00 (CST)
/// - Without the feature, this should error due to ambiguity
/// - With `dst-fold-first` feature, it should pick the first occurrence (CDT, -05:00)
#[test]
#[cfg(feature = "dst-fold-first")]
fn dst_fold_first_picks_earlier_time() {
    // 2021-10-31 01:30 occurs twice due to DST fall back
    // First occurrence: 01:30-05:00 (CDT)
    // Second occurrence: 01:30-06:00 (CST)
    // With dst-fold-first, should pick the first (earlier) one
    let rrule: RRuleSet =
        "DTSTART;TZID=America/Mexico_City:20211031T013000\nRRULE:FREQ=YEARLY;COUNT=2"
            .parse()
            .unwrap();

    let dates = rrule.all_unchecked();

    // With dst-fold-first, we should get the first occurrence (CDT, -05:00)
    check_occurrences(
        &dates,
        &[
            "2021-10-31T01:30:00-05:00", // First occurrence (CDT, before fall back)
            "2022-10-31T01:30:00-06:00", // Mexico eliminated DST in 2022, so -06:00
        ],
    );
}

/// Test that the Mexico City DST transition works with yearly recurrence
/// Note: Mexico eliminated DST after 2022, so times after 2022 should be in CST (-06:00)
#[test]
#[cfg(feature = "dst-fold-first")]
fn dst_fold_first_mexico_city_yearly() {
    // Start on 2021-10-31 01:30 which is ambiguous (DST fall back)
    let rrule: RRuleSet =
        "DTSTART;TZID=America/Mexico_City:20211031T013000\nRRULE:FREQ=YEARLY;COUNT=4"
            .parse()
            .unwrap();

    let dates = rrule.all_unchecked();

    check_occurrences(
        &dates,
        &[
            "2021-10-31T01:30:00-05:00", // DST active (CDT)
            "2022-10-31T01:30:00-06:00", // No DST (CST) - Mexico eliminated DST
            "2023-10-31T01:30:00-06:00", // No DST (CST)
            "2024-10-31T01:30:00-06:00", // No DST (CST)
        ],
    );
}

#[test]
fn daylight_savings_1() {
    let rrule: RRuleSet =
        "DTSTART;TZID=America/Vancouver:20210301T022210\nRRULE:FREQ=DAILY;COUNT=30"
            .parse()
            .unwrap();

    let dates = rrule.all_unchecked();
    check_occurrences(
        &dates,
        &[
            "2021-03-01T02:22:10-08:00",
            "2021-03-02T02:22:10-08:00",
            "2021-03-03T02:22:10-08:00",
            "2021-03-04T02:22:10-08:00",
            "2021-03-05T02:22:10-08:00",
            "2021-03-06T02:22:10-08:00",
            "2021-03-07T02:22:10-08:00",
            "2021-03-08T02:22:10-08:00",
            "2021-03-09T02:22:10-08:00",
            "2021-03-10T02:22:10-08:00",
            "2021-03-11T02:22:10-08:00",
            "2021-03-12T02:22:10-08:00",
            "2021-03-13T02:22:10-08:00",
            "2021-03-14T03:22:10-07:00",
            "2021-03-15T02:22:10-07:00",
            "2021-03-16T02:22:10-07:00",
            "2021-03-17T02:22:10-07:00",
            "2021-03-18T02:22:10-07:00",
            "2021-03-19T02:22:10-07:00",
            "2021-03-20T02:22:10-07:00",
            "2021-03-21T02:22:10-07:00",
            "2021-03-22T02:22:10-07:00",
            "2021-03-23T02:22:10-07:00",
            "2021-03-24T02:22:10-07:00",
            "2021-03-25T02:22:10-07:00",
            "2021-03-26T02:22:10-07:00",
            "2021-03-27T02:22:10-07:00",
            "2021-03-28T02:22:10-07:00",
            "2021-03-29T02:22:10-07:00",
            "2021-03-30T02:22:10-07:00",
        ],
    );
}

#[test]
fn daylight_savings_2() {
    let dates = "DTSTART;TZID=Europe/Paris:20210214T093000\n\
        RRULE:FREQ=WEEKLY;UNTIL=20210508T083000Z;INTERVAL=2;BYDAY=MO;WKST=MO"
        .parse::<RRuleSet>()
        .unwrap()
        .all(u16::MAX)
        .dates;
    check_occurrences(
        &dates,
        &[
            "2021-02-22T09:30:00+01:00",
            "2021-03-08T09:30:00+01:00",
            "2021-03-22T09:30:00+01:00",
            "2021-04-05T09:30:00+02:00", // Switching to daylight saving time.
            "2021-04-19T09:30:00+02:00",
            "2021-05-03T09:30:00+02:00",
        ],
    );
}
