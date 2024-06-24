use std::{env, fmt, fs::File, io::BufReader, str::FromStr};

use anyhow::Context;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Deserialize;

use crate::sunset::get_sunset_time;

#[derive(Debug, Deserialize)]
struct LocationConfig {
	longitude: f64,
	latitude: f64,
	timezone: String,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
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

#[derive(Debug, PartialEq, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
struct ChangeItem {
	action: Action,
    mirek: Option<u16>,
    brightness: Option<u8>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RawScheduleItem {
	hour: Option<i8>,
	minute: Option<i8>,
	from: Option<From>,
	change: ChangeItem,
}

#[derive(Debug, Clone)]
pub struct ProcessedScheduleItem {
	time: DateTime<Tz>,
	change: ChangeItem,
}

#[derive(Debug, Deserialize)]
struct ScheduleConfig {
	location: LocationConfig,
	schedule: Vec<RawScheduleItem>,
}

#[derive(Debug)]
pub struct ScheduleInfo {
    tz: Tz,
	config: ScheduleConfig,
	todays_schedule: Option<Vec<ProcessedScheduleItem>>,
}

impl ScheduleInfo {
	pub fn new() -> anyhow::Result<Self> {
		Self::from_env("SCHEDULE_YAML_PATH")
	}

	pub fn from_env(env_path_var: &str) -> anyhow::Result<Self> {
		let schedule_path = env::var(env_path_var)
			.context(format!("Unable to find env var: {env_path_var}"))?;
		let schedule_file = File::open(&schedule_path)
			.context(format!("Unable to open file at {}", &schedule_path))?;
		let reader = BufReader::new(schedule_file);
		let schedule_config: ScheduleConfig = serde_yaml::from_reader(reader)
			.context("Unable to parse schedule yaml file.")?;
		let tz = match schedule_config.location.timezone.parse::<Tz>() {
			Ok(tz) => Ok(tz),
			Err(e) => Err(anyhow::Error::msg(format!("{e}"))),
		}?;
		if schedule_config.schedule.len() == 0 {
			return Err(anyhow::Error::msg("Schedule must have at least 1 item in it."));
		}
		
		Ok(ScheduleInfo {
			tz,
			config: schedule_config,
			todays_schedule: None,
		})
	}

	pub fn get_sunset_time(&self) -> anyhow::Result<DateTime<Tz>> {
		match get_sunset_time(self.config.location.latitude, self.config.location.longitude, self.tz, Utc::now()) {
			Ok(time) => Ok(time),
			Err(e) => Err(anyhow::Error::msg(format!("{e}"))),
		}
	}

	pub fn set_today(&mut self) {
		let sunset_time = self.get_sunset_time().unwrap(); // XXX unwrap
		let mut todays_schedule: Vec<ProcessedScheduleItem> = self.config.schedule
				.iter()
				.map(|raw_item| ProcessedScheduleItem::from(&self.tz, raw_item, &sunset_time))
				.collect();

		let first_item = todays_schedule.get(0).unwrap();
		let mut first_repeat = first_item.clone();
		first_repeat.time += TimeDelta::try_days(1).unwrap();
		todays_schedule.push(first_repeat);

		// TODO: Assert sorted

		self.todays_schedule = Some(todays_schedule);
	}

	pub fn latest_scheduled_time(&self) -> Option<DateTime<Tz>> {
		match &self.todays_schedule {
			None => None,
			Some(todays_schedule) => {
				match todays_schedule.last() {
					None => None,
					Some(schedule_item) => Some(schedule_item.time)
				}
			}
		}
	}

	pub fn get_surrounding_schedule_items(&self) -> anyhow::Result<(&ProcessedScheduleItem, &ProcessedScheduleItem)> {
		let now = tz_now(&self.tz).unwrap();
		let todays_schedule = self.todays_schedule.as_ref().unwrap();
		for i in 0..(todays_schedule.len() - 1) {
			let before = todays_schedule.get(i).expect("Before too much");
			let after = todays_schedule.get(i + 1).expect("After too much");

			if before.time <= now && now < after.time {
				return Ok((before, after))
			}
		}

		let last_time = todays_schedule.last().unwrap().time;
		if last_time < now {
			return Err(anyhow::anyhow!("now ({now}) is later than last_time ({last_time})."))
		}

		let first_time = todays_schedule.first().unwrap().time;
		if now < first_time {
			return Err(anyhow::anyhow!("now ({now}) is later than first_time ({first_time})."))
		}

		Err(anyhow::anyhow!("now ({now}) has reached an unknown error."))
	}
}

impl ProcessedScheduleItem {
	pub fn from(tz: &Tz, raw: &RawScheduleItem, sunset_time: &DateTime<Tz>) -> Self {
		let hour = raw.hour.unwrap_or(0);
		let minute = raw.minute.unwrap_or(0);
		let time = match &raw.from {
			Some(s) if s == &From::Sunset => {
				let delta = TimeDelta::hours(hour as i64) + TimeDelta::minutes(minute as i64);
				let r: DateTime<Tz> = *sunset_time + delta;
				r
			},
			Some(_) => panic!("bad"),
			None => {
				time_to_today_tz(tz, hour as u8, minute as u8).unwrap()
			},
		};
		ProcessedScheduleItem {
			change: raw.change.clone(),
			time,
		}
	}
}

pub fn time_to_today_tz<T: TimeZone>(tz: &T, hour: u8, minute: u8) -> anyhow::Result<DateTime<T>> {
	let now = chrono::Local::now();
	let today = now.date_naive();
	time_to_datetime_tz(tz, hour, minute, today)
}

fn tz_now<T: TimeZone>(tz: &T) -> Option<DateTime<T>> {
	let now = chrono::Local::now().naive_local();
	tz.from_local_datetime(&now).earliest()
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
