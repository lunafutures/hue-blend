use std::{env, fmt, fs::File, io::BufReader, str::FromStr};

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

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum From {
	Sunset
}

impl fmt::Display for From {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            From::Sunset => "sunset",
        })
    }
}

impl FromStr for From {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
			"sunset" => Ok(From::Sunset),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
	Color,
	Stop
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Action::Color => "color",
			Action::Stop => "stop",
        })
    }
}

impl FromStr for Action {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
			"color" => Ok(Action::Color),
			"stop" => Ok(Action::Stop),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChangeItem {
	action: Action,
    mirek: Option<u16>,
    brightness: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct RawScheduleItem {
	hour: Option<i8>,
	minute: Option<i8>,
	from: Option<From>,
	change: ChangeItem,
}

pub struct ProcessedScheduleItem {
	time: DateTime<Tz>,
	change: ChangeItem,
}

#[derive(Debug, Deserialize)]
struct ScheduleConfig<ItemT> {
	location: LocationConfig,
	schedule: Vec<ItemT>,
}

#[derive(Debug)]
pub struct ScheduleInfo<ItemT> {
    tz: Tz,
	config: ScheduleConfig<ItemT>,
}

impl ScheduleInfo<RawScheduleItem> {
	pub fn new() -> anyhow::Result<Self> {
		Self::from_env("SCHEDULE_YAML_PATH")
	}

	pub fn from_env(env_path_var: &str) -> anyhow::Result<Self> {
		let schedule_path = env::var(env_path_var)
			.context(format!("Unable to load env var: {env_path_var}"))?;
		let schedule_file = File::open(&schedule_path)
			.context(format!("Unable to open file at {}", &schedule_path))?;
		let reader = BufReader::new(schedule_file);
		let schedule_config: ScheduleConfig<RawScheduleItem> = serde_yaml::from_reader(reader)
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

impl ScheduleInfo<ProcessedScheduleItem> {
	pub fn convert(orig: ScheduleInfo<RawScheduleItem>) -> Self {
		todo!();
	}
}

impl ProcessedScheduleItem {
	pub fn from(orig: RawScheduleItem, sunset_time: DateTime<Tz>) -> Self {
		todo!();
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
