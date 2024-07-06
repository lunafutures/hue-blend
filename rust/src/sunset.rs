use chrono::{DateTime, Datelike, TimeZone};
use chrono_tz::Tz;
use sunrise::sunrise_sunset;

pub fn get_sunset_time(latitude: f64, longitude: f64, tz: Tz, now: &DateTime<Tz>) -> Result<DateTime<Tz>, String> {
    let (_, sunset_epoch) =
        sunrise_sunset(latitude, longitude, now.year(), now.month(), now.day());
	if sunset_epoch == 0 {
		return Err(format!("sunset_epoch is invalid ({sunset_epoch})."));
	}

    match tz.timestamp_opt(sunset_epoch, 0).earliest() {
        Some(local_datetime) => Ok(local_datetime),
        None => Err(format!("Could not convert {sunset_epoch} to local datetime."))
    }
}
