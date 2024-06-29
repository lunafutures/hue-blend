use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone};

pub fn tz_now<T: TimeZone>(tz: &T) -> DateTime<T> {
	let now = chrono::Utc::now().naive_local();
	tz.from_utc_datetime(&now)
}

pub fn time_to_today_tz<T: TimeZone>(tz: &T, today: NaiveDate, hour: u8, minute: u8) -> anyhow::Result<DateTime<T>> {
	time_to_datetime_tz(tz, hour, minute, today)
}

fn time_to_datetime_tz<T: TimeZone>(tz: &T, hour: u8, minute: u8, date: NaiveDate) -> anyhow::Result<DateTime<T>> {
	let naive_time = match NaiveTime::from_hms_opt(hour.into(), minute.into(), 0) {
		Some(t) => t,
		None => return Err(anyhow::anyhow!("Could not construct NaiveTime from hour={}, minute={}.", hour, minute)),
	};
	let naive_datetime = date.and_time(naive_time);
	match tz.from_local_datetime(&naive_datetime).earliest() {
		Some(t) => Ok(t),
		None => Err(anyhow::anyhow!("Could not convert local ({naive_datetime}) to tz datetime.")),
	}
}
