use std::{env, fmt, fs::File, io::BufReader, str::FromStr};

use anyhow::Context;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeDelta, TimeZone};
use chrono_tz::Tz;
use rocket::serde;

use crate::sunset::get_sunset_time;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct LocationConfig {
	longitude: f64,
	latitude: f64,
	timezone: String,
}

#[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
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

#[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct ChangeItem {
	action: Action,
    mirek: Option<u16>,
    brightness: Option<u8>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RawScheduleItem {
	hour: Option<i8>,
	minute: Option<i8>,
	from: Option<From>,
	change: ChangeItem,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ProcessedScheduleItem {
	time: DateTime<Tz>,
	change: ChangeItem,
}

#[derive(Debug, serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ScheduleYamlConfig {
	location: LocationConfig,
	schedule: Vec<RawScheduleItem>,
}

#[derive(Debug)]
pub struct Schedule {
    tz: Tz,
	location: LocationConfig,
	pub raw_schedule: Vec<RawScheduleItem>,
	pub todays_schedule: Option<Vec<ProcessedScheduleItem>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct DebugSurrounding {
	first: ProcessedScheduleItem,
	last: ProcessedScheduleItem,
}

#[derive(Debug, serde::Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DebugInfo {
	updated: bool,
	tz: String,
    raw_schedule: Vec<RawScheduleItem>,
	processed_schedule: Vec<ProcessedScheduleItem>,
	now: DateTime<Tz>,
	surrounding_items: DebugSurrounding,
	change_action: ChangeAction,
}

impl Schedule {
	pub fn get_debug_info(&mut self) -> anyhow::Result<DebugInfo> {
		let updated = self.try_update()?;

		let todays_schedule = match self.todays_schedule.clone() {
			Some(s) => s,
			None => return Err(anyhow::anyhow!("todays_schedule is unexpected None")),
		};

		let now = self.now()?;
		let surrounding_items = {
			let (first, last) = self.get_surrounding_schedule_items(Some(now))?;
			DebugSurrounding { first: first.clone(), last: last.clone() }
		};
		let change_action = self.get_action_for_now(&now)?;

		Ok(DebugInfo {
			tz: self.tz.to_string(),
			updated,
			raw_schedule: self.raw_schedule.clone(),
			processed_schedule: todays_schedule,
			now,
			surrounding_items,
			change_action,
		})
	}

	pub fn new() -> anyhow::Result<Self> {
		Self::from_env("SCHEDULE_YAML_PATH")
	}

	pub fn from_env(env_path_var: &str) -> anyhow::Result<Self> {
		let schedule_path = env::var(env_path_var)
			.context(format!("Unable to find env var: {env_path_var}"))?;
		let schedule_file = File::open(&schedule_path)
			.context(format!("Unable to open file at {}", &schedule_path))?;
		let reader = BufReader::new(schedule_file);
		let schedule_yaml_config: ScheduleYamlConfig = serde_yaml::from_reader(reader)
			.context("Unable to parse schedule yaml file.")?;
		let tz = match schedule_yaml_config.location.timezone.parse::<Tz>() {
			Ok(tz) => Ok(tz),
			Err(e) => Err(anyhow::Error::msg(format!("{e}"))),
		}?;
		if schedule_yaml_config.schedule.len() == 0 {
			return Err(anyhow::Error::msg("Schedule must have at least 1 item in it."));
		}
		
		Ok(Schedule {
			tz,
			location: schedule_yaml_config.location,
			raw_schedule: schedule_yaml_config.schedule,
			todays_schedule: None,
		})
	}

	pub fn get_sunset_time(&self) -> anyhow::Result<DateTime<Tz>> {
		match get_sunset_time(self.location.latitude, self.location.longitude, self.tz, chrono::Local::now()) {
			Ok(time) => Ok(time),
			Err(e) => Err(anyhow::Error::msg(format!("{e}"))),
		}
	}

	pub fn try_update(&mut self) -> anyhow::Result<bool> {
		let updated = if self.todays_schedule.is_none() {
			self.set_today()?;
			true
		} else {
			false
		};

		Ok(updated)
	}

	pub fn set_today(&mut self) -> anyhow::Result<()> {
		let sunset_time = self.get_sunset_time().context("Unable to get sunset time.")?;
		println!("sunset_time: {sunset_time:?}");
		let mut todays_schedule: Vec<ProcessedScheduleItem> = match self.raw_schedule
				.iter()
				.map(|raw_item| ProcessedScheduleItem::from(&self.tz, raw_item, &sunset_time))
				.collect() {
			Ok(o) => o,
			Err(e) => Err(e)?,
		};

		let first_item = todays_schedule.get(0).context("Unable to get first element of todays_schedule.")?;
		let mut first_repeat = first_item.clone();
		first_repeat.time += TimeDelta::try_days(1).context("Unable to create a 1-day delta to add to create the last item.")?;
		todays_schedule.push(first_repeat);

		for i in 0..(todays_schedule.len() - 1) {
			let before = todays_schedule
				.get(i)
				.ok_or(anyhow::anyhow!("Index out of bounds while asserted sorted: {i}"))?;
			let after = todays_schedule
				.get(i + 1)
				.ok_or(anyhow::anyhow!("Index out of bounds while asserted sorted: {}", i + 1))?;
			if before.time > after.time {
				return Err(anyhow::anyhow!(
					"Processed schedule is not sorted by item: [{i}] {before:#?} is later than [{}] {after:#?}", i + 1));
			}
		}

		self.todays_schedule = Some(todays_schedule);
		Ok(())
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

	pub fn get_surrounding_schedule_items(&self, now: Option<DateTime<Tz>>) -> anyhow::Result<(&ProcessedScheduleItem, &ProcessedScheduleItem)> {
		let now : DateTime<Tz> = match now {
			Some(now) => now,
			None => self.now().context("Unable to get now to find surrounding schedule items.")?,
		};
		let todays_schedule = self.todays_schedule.as_ref().context("todays_schedule has not been set.")?;
		for i in 0..(todays_schedule.len() - 1) {
			let before = todays_schedule.get(i).expect("Before too much");
			let after = todays_schedule.get(i + 1).expect("After too much");

			if before.time <= now && now < after.time {
				return Ok((before, after))
			}
		}

		let last_time = todays_schedule.last().context("Unable to get last element of todays_schedule")?.time;
		if last_time < now {
			return Err(anyhow::anyhow!("now ({now}) is later than last_time ({last_time})."))
		}

		let first_time = todays_schedule.first().context("Unable to get first element of todays_schedule")?.time;
		if now < first_time {
			return Err(anyhow::anyhow!("now ({now}) is later than first_time ({first_time})."))
		}

		Err(anyhow::anyhow!("now ({now}) has reached an unknown error."))
	}

	pub fn get_action_for_now(&self, now: &DateTime<Tz>) -> anyhow::Result<ChangeAction> {
		let (a, b) = 
			self.get_surrounding_schedule_items(Some(now.clone()))?;
		blend_actions(a, b, now)
	}

	pub fn now(&self) -> anyhow::Result<DateTime<Tz>> {
		match tz_now(&self.tz) {
			Some(o) => Ok(o),
			None => Err(anyhow::anyhow!("Unable to construct now for timezone: {}", &self.tz)),
		}
	}
}

pub fn blend_actions(a: &ProcessedScheduleItem, b: &ProcessedScheduleItem, now: &DateTime<Tz>) -> anyhow::Result<ChangeAction> {
	if a.time > b.time {
		return Err(anyhow::anyhow!("a.time ({a:?}) should not be after b.time ({b:?})"));
	} else if now < &a.time {
		return Err(anyhow::anyhow!("now ({now}) should not be after a.time ({a:?})"));
	} else if &b.time < now {
		return Err(anyhow::anyhow!("b.time ({b:?}) should not be after now ({now})"));
	}

	match a.change.action {
		Action::Stop => Ok(ChangeAction::None),
		Action::Color => match b.change.action {
			Action::Stop => {
				let mirek = a.change.mirek.context(format!("Expected mirek in change: {:#?}", a.change))?;
				let brightness = a.change.brightness.context(format!("Expected brightness in change: {:#?}", a.change))?;
				Ok(ChangeAction::Color { mirek, brightness })
			},
			Action::Color => {
				let a_factor: f64 = (b.time - now).num_milliseconds() as f64 / (b.time - a.time).num_milliseconds() as f64;
				let b_factor: f64 = 1.0 - a_factor;

				let a_mirek = a.change.mirek.context(format!("Expected mirek in change: {:#?}", a.change))?;
				let a_brightness = a.change.brightness.context(format!("Expected brightness in change: {:#?}", a.change))?;

				let b_mirek = b.change.mirek.context(format!("Expected mirek in change: {:#?}", b.change))?;
				let b_brightness = b.change.brightness.context(format!("Expected brightness in change: {:#?}", b.change))?;

				Ok(ChangeAction::Color { 
					mirek: fraction(
						a_factor, a_mirek,
						b_factor, b_mirek) as u16,
					brightness: fraction(
						a_factor, a_brightness,
						b_factor, b_brightness) as u8,
				})
			}
		}
	}
}

pub fn fraction<T>(a_factor: f64, a_value: T, b_factor: f64, b_value: T) -> f64
where T: Into<f64>{
	a_factor * a_value.into() + b_factor * b_value.into()
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub enum ChangeAction {
	None,
	Color {mirek: u16, brightness: u8},
}

impl ProcessedScheduleItem { // XXX TODO move closer to definition
	pub fn from(tz: &Tz, raw: &RawScheduleItem, sunset_time: &DateTime<Tz>) -> anyhow::Result<Self> {
		let hour = raw.hour.unwrap_or(0);
		let minute = raw.minute.unwrap_or(0);
		let time = match &raw.from {
			Some(s) if s == &From::Sunset => {
				let delta = TimeDelta::hours(hour as i64) + TimeDelta::minutes(minute as i64);
				let r: DateTime<Tz> = *sunset_time + delta;
				r
			},
			Some(s) => Err(anyhow::anyhow!(
				"Unexpected `from` value {s} while constructing {}.",
				std::any::type_name::<ProcessedScheduleItem>()))?,
			None => time_to_today_tz(tz, hour as u8, minute as u8)
				.context(format!("Unable to convert hour {hour} and minute {minute} to time tz."))?,
		};
		Ok(ProcessedScheduleItem {
			change: raw.change.clone(),
			time,
		})
	}
}

// XXX TODO: move fns to time.rs
pub fn tz_now<T: TimeZone>(tz: &T) -> Option<DateTime<T>> {
	let now = chrono::Local::now().naive_local();
	tz.from_local_datetime(&now).earliest()
}

pub fn time_to_today_tz<T: TimeZone>(tz: &T, hour: u8, minute: u8) -> anyhow::Result<DateTime<T>> {
	let now = chrono::Local::now();
	let today = now.date_naive();
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

#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, TimeZone};
    use chrono_tz::{Tz, US::Eastern};
    use super::{blend_actions, Action, ChangeItem, ProcessedScheduleItem, ChangeAction};

	const TEST_TZ: Tz = Eastern;

	fn get_naive_datetime(hour: u32, minute: u32) -> NaiveDateTime {
		chrono::NaiveDate::from_ymd_opt(1999, 1, 1)
			.unwrap()
			.and_time(chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap())
	}

	fn get_tz_datetime(hour: u32, minute: u32) -> chrono::DateTime<Tz> {
		TEST_TZ.from_local_datetime(&get_naive_datetime(hour, minute))
			.earliest()
			.unwrap()
	}

	fn create_processed_schedule_item_color(hour: u32, minute: u32, mirek: u16, brightness: u8) -> ProcessedScheduleItem {
		ProcessedScheduleItem {
			time: get_tz_datetime(hour, minute),
			change: ChangeItem {
				action: Action::Color,
				mirek: Some(mirek),
				brightness: Some(brightness),
			},
		}
	}

	fn create_processed_schedule_item_stop(hour: u32, minute: u32) -> ProcessedScheduleItem {
		ProcessedScheduleItem {
			time: get_tz_datetime(hour, minute),
			change: ChangeItem {
				action: Action::Stop,
				mirek: None,
				brightness: None,
			},
		}
	}

    #[test]
    fn test_blend_action_stop_before() {
		let stop_12 = create_processed_schedule_item_stop(12, 0);
		let color_13 = create_processed_schedule_item_color(13, 0, 123, 50);

		assert_eq!(
			blend_actions(&stop_12, &color_13, &get_tz_datetime(12, 0)).expect("Expected action is obtainable"),
			ChangeAction::None,
		);

		assert_eq!(
			blend_actions(&stop_12, &color_13, &get_tz_datetime(12, 59)).expect("Expected action is obtainable"),
			ChangeAction::None,
		);
    }

    #[test]
	fn test_blend_action_stop_after() {
		let color_10 = create_processed_schedule_item_color(10, 0, 123, 50);
		let stop_12 = create_processed_schedule_item_stop(12, 0);

		assert_eq!(
			blend_actions(&color_10, &stop_12, &get_tz_datetime(10, 0)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 123, brightness: 50 },
		);

		assert_eq!(
			blend_actions(&color_10, &stop_12, &get_tz_datetime(11, 30)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 123, brightness: 50 },
		);
	}

	#[test]
	fn test_blend_action_invalid() {
		let color_10 = create_processed_schedule_item_color(10, 0, 123, 50);
		let stop_12 = create_processed_schedule_item_stop(12, 0);

		assert!(blend_actions(&color_10, &stop_12, &get_tz_datetime(9, 59)).is_err());
		assert!(blend_actions(&color_10, &stop_12, &get_tz_datetime(12, 1)).is_err());
		assert!(blend_actions(&stop_12, &color_10, &get_tz_datetime(11, 0)).is_err());
	}

	#[test]
	fn test_blend_action_2_colors() {
		let color_10 = create_processed_schedule_item_color(10, 0, 200, 10);
		let color_20 = create_processed_schedule_item_color(20, 0, 400, 90);

		assert_eq!(
			blend_actions(&color_10, &color_20, &get_tz_datetime(10, 0)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 200, brightness: 10 },
		);

		assert_eq!(
			blend_actions(&color_10, &color_20, &get_tz_datetime(15, 0)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 300, brightness: 50 },
		);

		assert_eq!(
			blend_actions(&color_10, &color_20, &get_tz_datetime(19, 30)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 390, brightness: 86 },
		);
	}
}