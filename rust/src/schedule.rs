use std::{env, fs::File, io::BufReader};

use anyhow::Context;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Deserialize;

use crate::sunset::get_sunset_time;

#[derive(Debug, Deserialize)]
struct LocationConfig {
	longitude: f64,
	latitude: f64,
	timezone: String,
}

enum Action { // XXX TODO
}

#[derive(Debug, Deserialize)]
struct ChangeItem {
	action: String,
    mirek: Option<u16>,
    brightness: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct ScheduleItem {
	hour: Option<i8>,
	minute: Option<i8>,
	from: Option<String>,
	change: ChangeItem,
}

#[derive(Debug, Deserialize)]
struct ScheduleConfig {
	location: LocationConfig,
	schedule: Vec<ScheduleItem>,
}

pub struct ScheduleInfo {
    tz: Tz,
	config: ScheduleConfig,
}

impl ScheduleInfo {
	pub fn new() -> anyhow::Result<ScheduleInfo> {
		Self::from_env("SCHEDULE_YAML_PATH")
	}

	pub fn from_env(env_path_var: &str) -> anyhow::Result<ScheduleInfo> {
		let schedule_path = env::var(env_path_var)
			.context(format!("Unable to load env var: {env_path_var}"))?;
		let schedule_file = File::open(&schedule_path)
			.context(format!("Unable to open file at {}", &schedule_path))?;
		let reader = BufReader::new(schedule_file);
		let schedule_config: ScheduleConfig = serde_yaml::from_reader(reader)
			.context("Unable to parse schedule yaml file.")?;
		let tz = match schedule_config.location.timezone.parse::<Tz>() {
			Ok(tz) => Ok(tz),
			Err(e) => Err(anyhow::Error::msg(format!("{e}"))),
		}?;
		
		Ok(ScheduleInfo {
			tz,
			config: schedule_config,
		})
	}

	pub fn get_sunset_time(&self) -> anyhow::Result<DateTime<Tz>> {
		match get_sunset_time(self.config.location.latitude, self.config.location.longitude, self.tz, Utc::now()) {
			Ok(time) => Ok(time),
			Err(e) => Err(anyhow::Error::msg(format!("{e}"))),
		}
	}
}

pub fn time_to_today_tz<T: TimeZone>(tz: T, hour: u8, minute: u8) -> anyhow::Result<DateTime<T>> {
	let now = chrono::Local::now();
	let today = now.date_naive();
	time_to_datetime_tz(tz, hour, minute, today)
}

fn time_to_datetime_tz<T: TimeZone>(tz: T, hour: u8, minute: u8, date: NaiveDate) -> anyhow::Result<DateTime<T>> {
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
