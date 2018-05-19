use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DateTime {
    pub moment: u64,    // utc time stamp
    pub tz:     i32,    // offset for whatever time zone you want
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    pub distance: u64,
}

// Returns 0 on error, yeah trys to make std::time a bit easier. You've been warned...
pub fn now() -> DateTime {
    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_secs() as u64,
        Err(_) => 0,
    };
    DateTime { moment: time, tz: 0 }
}

pub fn hms(h: u64, m: u64, s: u64) -> DateTime {
    let time = (((h * 60) + m) * 60) + s;
    DateTime { moment: time, tz: 0 }
}

pub fn with_timezone(m: &DateTime, tz: i32) -> DateTime {
    DateTime { moment: m.moment, tz: tz }
}

pub fn moment(m: &DateTime) -> u64 {
    match m.tz <= 0 {
        true => reduce(m, &hms(0, 0, (m.tz.abs()) as u64)).moment,
        false => append(m, &hms(0, 0, m.tz as u64)).moment,
    }
}

// takes on timezone of first argument
pub fn append(m1 :&DateTime, m2: &DateTime) -> DateTime {
    DateTime { moment: m1.moment + m2.moment, tz: m1.tz }
}

pub fn diff(m1 :&DateTime, m2: &DateTime) -> Interval {
    Interval { distance: m2.moment - m1.moment }
}

pub fn add(m :&DateTime, i: &Interval) -> DateTime {
    DateTime { moment: m.moment + i.distance, tz: m.tz }
}

pub fn sub(m :&DateTime, i: &Interval) -> DateTime {
    DateTime { moment: m.moment - i.distance, tz: m.tz  }
}

// takes on timezone of first argument
pub fn reduce(m1 :&DateTime, m2: &DateTime) -> DateTime {
    DateTime { moment: m2.moment - m1.moment, tz: m1.tz }
}

pub fn seconds_later(m :&DateTime, seconds: u64) -> DateTime {
    DateTime { moment: m.moment + seconds, tz: m.tz }
}

pub fn minutes_later(m :&DateTime, minutes: u64) -> DateTime {
    DateTime { moment: m.moment + (minutes * 60), tz:m .tz }
}

pub fn hours_later(m :&DateTime, hours: u64) -> DateTime {
    DateTime { moment: m.moment + ((hours * 60) * 60), tz: m.tz }
}

pub fn days_later(m :&DateTime, days: u64) -> DateTime {
    DateTime { moment: m.moment + (((days * 24) * 60) * 60), tz: m.tz }
}

pub fn days_before(m :&DateTime, days: u64) -> DateTime {
    DateTime { moment: m.moment - (((days * 24) * 60) * 60), tz: m.tz }
}

// divide down to days and then multiply back up to seconds
// ignores leap seconds
pub fn floor_to_days(m: &DateTime) -> DateTime {
    DateTime { moment: ((((m.moment / 86400) * 24) * 60) * 60), tz: m.tz }
}

pub fn days(m: &DateTime) -> u64 {
    moment(m) / 86400
}

pub fn hours(m: &DateTime) -> u64 {
    moment(m) / 3600
}

pub fn minutes(m: &DateTime) -> u64 {
    moment(m) / 60
}

pub fn today() -> DateTime {
    floor_to_days(&now())
}
