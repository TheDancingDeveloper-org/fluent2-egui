//! `CalendarDate` and `TimeOfDay` convert to and from chrono (WI-1943).
#![cfg(feature = "chrono")]

use chrono::{NaiveDate, NaiveTime};
use fluent2_egui::controls::{CalendarDate, TimeOfDay};

#[test]
fn dates_and_times_round_trip() {
    for (y, m, d) in [(2026, 10, 20), (2000, 2, 29), (1900, 1, 1), (2399, 12, 31)] {
        let ours = CalendarDate::new(y, m, d);
        let theirs = NaiveDate::from_ymd_opt(y, u32::from(m), u32::from(d));
        assert_eq!(ours.map(NaiveDate::from), theirs);
        assert_eq!(theirs.map(CalendarDate::from), ours);
        // Our epoch arithmetic agrees with chrono's.
        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1);
        if let (Some(ours), Some(theirs), Some(epoch)) = (ours, theirs, epoch) {
            assert_eq!(ours.days_since_epoch(), (theirs - epoch).num_days());
        }
    }
    let time = TimeOfDay::new(14, 30);
    assert_eq!(time.map(NaiveTime::from), NaiveTime::from_hms_opt(14, 30, 0));
    assert_eq!(
        NaiveTime::from_hms_opt(23, 59, 59).map(TimeOfDay::from),
        TimeOfDay::new(23, 59)
    );
}
