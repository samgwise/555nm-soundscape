use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DateTime {
    pub moment: u64,
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
    DateTime { moment: time }
}

pub fn hms(h: u64, m: u64, s: u64) -> DateTime {
    let time = (((h * 60) + m) * 60) + s;
    DateTime { moment: time }
}

pub fn sum(m1 :&DateTime, m2: &DateTime) -> DateTime {
    DateTime { moment: m1.moment + m2.moment }
}

pub fn diff(m1 :&DateTime, m2: &DateTime) -> Interval {
    Interval { distance: m2.moment - m1.moment }
}

pub fn add(m :&DateTime, i: &Interval) -> DateTime {
    DateTime { moment: m.moment + i.distance }
}

pub fn less(m :&DateTime, i: &Interval) -> DateTime {
    DateTime { moment: m.moment - i.distance }
}

pub fn sub(m1 :&DateTime, m2: &DateTime) -> DateTime {
    DateTime { moment: m2.moment - m1.moment }
}

pub fn seconds_later(m :&DateTime, seconds: u64) -> DateTime {
    DateTime { moment: m.moment + seconds }
}

pub fn minutes_later(m :&DateTime, minutes: u64) -> DateTime {
    DateTime { moment: m.moment + (minutes * 60) }
}

pub fn hours_later(m :&DateTime, hours: u64) -> DateTime {
    DateTime { moment: m.moment + ((hours * 60) * 60) }
}

pub fn days_later(m :&DateTime, days: u64) -> DateTime {
    DateTime { moment: m.moment + (((days * 24) * 60) * 60) }
}

// divide down to days and then multiply back up to seconds
// ignores leap seconds
pub fn floor_to_days(m: &DateTime) -> DateTime {
    DateTime { moment: ((((m.moment / 86400) * 24) * 60) * 60) }
}
