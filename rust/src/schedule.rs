use std::{env, fmt, fs::File, io::BufReader, str::FromStr};

use anyhow::Context;
use chrono::{DateTime, NaiveDate, TimeDelta};
use chrono_tz::Tz;
use rocket::serde;

use crate::{sunset::get_sunset_time, time::{time_to_today_tz, tz_now}};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct LocationConfig {
	longitude: f64,
	latitude: f64,
	timezone: String,
}

#[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
enum FromRefTime {
	Sunset
}

impl fmt::Display for FromRefTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            FromRefTime::Sunset => "sunset",
        })
    }
}

impl FromStr for FromRefTime {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
			"sunset" => Ok(FromRefTime::Sunset),
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

#[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct ChangeItem {
	action: Action,
    mirek: Option<u16>,
    brightness: Option<u8>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct RawScheduleItem {
	hour: Option<i8>,
	minute: Option<i8>,
	from: Option<FromRefTime>,
	change: ChangeItem,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct ProcessedScheduleItem {
	time: DateTime<Tz>,
	change: ChangeItem,
}

impl ProcessedScheduleItem {
	fn from(tz: &Tz, raw: &RawScheduleItem, today: NaiveDate, sunset_time: &DateTime<Tz>) -> anyhow::Result<Self> {
		let hour = raw.hour.unwrap_or(0);
		let minute = raw.minute.unwrap_or(0);
		let time = match &raw.from {
			Some(s) if s == &FromRefTime::Sunset => {
				let delta = TimeDelta::hours(hour as i64) + TimeDelta::minutes(minute as i64);
				let r: DateTime<Tz> = *sunset_time + delta;
				r
			},
			Some(s) => Err(anyhow::anyhow!(
				"Unexpected `from` value {s} while constructing {}.",
				std::any::type_name::<ProcessedScheduleItem>()))?,
			None => time_to_today_tz(tz, today, hour as u8, minute as u8)
				.context(format!("Unable to convert hour {hour} and minute {minute} to time tz."))?,
		};
		Ok(ProcessedScheduleItem {
			change: raw.change.clone(),
			time,
		})
	}
}

#[derive(Debug, serde::Deserialize)]
#[serde(crate = "rocket::serde")]
struct ScheduleYamlConfig {
	location: LocationConfig,
	schedule: Vec<RawScheduleItem>,
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
	just_updated: bool,
	tz: String,
    raw_schedule: Vec<RawScheduleItem>,
	processed_schedule: Vec<ProcessedScheduleItem>,
	now: DateTime<Tz>,
	surrounding_items: DebugSurrounding,
	change_action: ChangeAction,
}

#[derive(Debug)]
pub struct Schedule {
    tz: Tz,
	location: LocationConfig,
	raw_schedule: Vec<RawScheduleItem>,
	todays_schedule: Option<Vec<ProcessedScheduleItem>>,
}

impl Schedule {
	pub fn get_debug_info(&mut self) -> anyhow::Result<DebugInfo> {
		let now = self.now();
		let just_updated = self.try_update(now)?;

		let todays_schedule = match self.todays_schedule.clone() {
			Some(s) => s,
			None => return Err(anyhow::anyhow!("todays_schedule is unexpected None")),
		};

		let surrounding_items = {
			let (first, last) = self.get_surrounding_schedule_items(now)?;
			DebugSurrounding { first: first.clone(), last: last.clone() }
		};
		let change_action = self.get_action_for_now(&now)?;

		Ok(DebugInfo {
			tz: self.tz.to_string(),
			just_updated,
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

	fn from_env(env_path_var: &str) -> anyhow::Result<Self> {
		let yaml_path = env::var(env_path_var)
			.context(format!("Unable to find env var: {env_path_var}"))?;
		let schedule_file = File::open(&yaml_path)
			.context(format!("Unable to open file at {}", &yaml_path))?;
		let reader = BufReader::new(schedule_file);
		let schedule_yaml_config: ScheduleYamlConfig = serde_yaml::from_reader(reader)
			.context("Unable to parse schedule yaml file.")?;
		let tz = match schedule_yaml_config.location.timezone.parse::<Tz>() {
			Ok(tz) => Ok(tz),
			Err(e) => Err(anyhow::Error::msg(e.to_string())),
		}?;
		if schedule_yaml_config.schedule.is_empty() {
			return Err(anyhow::Error::msg("Schedule must have at least 1 item in it."));
		}
		
		Ok(Schedule {
			tz,
			location: schedule_yaml_config.location,
			raw_schedule: schedule_yaml_config.schedule,
			todays_schedule: None,
		})
	}

	fn get_sunset_time(&self, now: &DateTime<Tz>) -> anyhow::Result<DateTime<Tz>> {
		match get_sunset_time(self.location.latitude, self.location.longitude, self.tz, now) {
			Ok(time) => Ok(time),
			Err(e) => Err(anyhow::Error::msg(e.to_string())),
		}
	}

	pub fn try_update(&mut self, now: DateTime<Tz>) -> anyhow::Result<bool> {
		let updated = if self.todays_schedule.is_none() {
			self.set_today(&now)?;
			true
		} else {
			let latest = match self.latest_scheduled_time() {
				Some(s) => s,
				None => return Err(anyhow::anyhow!("Should have a latest scheduled time after update.")),
			};

			if latest <= now {
				self.set_today(&now)?;
				true
			} else {
				false
			}
		};

		Ok(updated)
	}

	pub fn set_today(&mut self, now: &DateTime<Tz>) -> anyhow::Result<()> {
		let sunset_time = self.get_sunset_time(now).context("Unable to get sunset time.")?;

		let today = now.date_naive();
		let mut todays_schedule: Vec<ProcessedScheduleItem> = match self.raw_schedule
				.iter()
				.map(|raw_item| ProcessedScheduleItem::from(&self.tz, raw_item, today, &sunset_time))
				.collect() {
			Ok(o) => o,
			Err(e) => Err(e)?,
		};

		let first_item = todays_schedule.first().context("Unable to get first element of todays_schedule.")?;
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

	fn latest_scheduled_time(&self) -> Option<DateTime<Tz>> {
		match &self.todays_schedule {
			None => None,
			Some(todays_schedule) => {
				todays_schedule.last().map(|schedule_item| schedule_item.time)
			}
		}
	}

	fn get_surrounding_schedule_items(&self, now: DateTime<Tz>) -> anyhow::Result<(&ProcessedScheduleItem, &ProcessedScheduleItem)> {
		let todays_schedule = self.todays_schedule
			.as_ref()
			.context("todays_schedule has not been set.")?;

		get_surrounding_schedule_items(todays_schedule, now)
	}

	pub fn get_action_for_now(&self, now: &DateTime<Tz>) -> anyhow::Result<ChangeAction> {
		let (a, b) = 
			self.get_surrounding_schedule_items(*now)?;

		blend_actions(a, b, now)
	}

	pub fn now(&self) -> DateTime<Tz> {
		tz_now(&self.tz)
	}
}

fn blend_actions(a: &ProcessedScheduleItem, b: &ProcessedScheduleItem, now: &DateTime<Tz>) -> anyhow::Result<ChangeAction> {
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

fn get_surrounding_schedule_items(schedule: &[ProcessedScheduleItem], now: DateTime<Tz>) -> anyhow::Result<(&ProcessedScheduleItem, &ProcessedScheduleItem)> {
	for i in 0..(schedule.len() - 1) {
		let before = schedule.get(i).expect("Before too much");
		let after = schedule.get(i + 1).expect("After too much");

		if before.time <= now && now < after.time {
			return Ok((before, after))
		}
	}

	let last_time = schedule.last().context("Unable to get last element of todays_schedule")?.time;
	if last_time < now {
		return Err(anyhow::anyhow!("now ({now}) is later than last_time ({last_time})."))
	}

	let first_time = schedule.first().context("Unable to get first element of todays_schedule")?.time;
	if now < first_time {
		return Err(anyhow::anyhow!("now ({now}) is later than first_time ({first_time})."))
	}

	Err(anyhow::anyhow!("now ({now}) has reached an unknown error."))
}

fn fraction<T>(a_factor: f64, a_value: T, b_factor: f64, b_value: T) -> f64
where T: Into<f64>{
	a_factor * a_value.into() + b_factor * b_value.into()
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub enum ChangeAction {
	None,
	Color {mirek: u16, brightness: u8},
}

#[cfg(test)]
mod tests {
	use chrono::{NaiveDateTime, TimeZone};
	use chrono_tz::{Tz, US::Eastern};
	use crate::schedule::{blend_actions, get_surrounding_schedule_items,
		Action, ChangeAction, ChangeItem, FromRefTime, ProcessedScheduleItem, RawScheduleItem};

	const TEST_TZ: Tz = Eastern;

	fn get_naive_datetime(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
		chrono::NaiveDate::from_ymd_opt(year, month, day)
			.unwrap()
			.and_time(chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap())
	}

	fn get_tz_datetime_hm(hour: u32, minute: u32) -> chrono::DateTime<Tz> {
		TEST_TZ.from_local_datetime(&get_naive_datetime(1999, 1, 1, hour, minute))
			.earliest()
			.unwrap()
	}

	fn create_processed_schedule_item_color(hour: u32, minute: u32, mirek: u16, brightness: u8) -> ProcessedScheduleItem {
		ProcessedScheduleItem {
			time: get_tz_datetime_hm(hour, minute),
			change: ChangeItem {
				action: Action::Color,
				mirek: Some(mirek),
				brightness: Some(brightness),
			},
		}
	}

	fn create_processed_schedule_item_stop(hour: u32, minute: u32) -> ProcessedScheduleItem {
		ProcessedScheduleItem {
			time: get_tz_datetime_hm(hour, minute),
			change: ChangeItem {
				action: Action::Stop,
				mirek: None,
				brightness: None,
			},
		}
	}

	mod schedule_tests {
		use chrono::{Datelike, TimeZone};
		use chrono_tz::Tz;
		use crate::schedule::{Action, ChangeItem, LocationConfig, RawScheduleItem, Schedule};
		use super::{get_naive_datetime, TEST_TZ};

		fn get_tz_datetime_dhm(day: u32, hour: u32, minute: u32) -> chrono::DateTime<Tz> {
			TEST_TZ.from_local_datetime(&get_naive_datetime(1990, 1, day, hour, minute))
				.earliest()
				.unwrap()
		}

		impl Schedule {
			fn new_for_test(raw_schedule: Vec<RawScheduleItem>) -> Schedule {
				Schedule {
					tz: TEST_TZ,
					location: LocationConfig {
						longitude: 10.,
						latitude: 20.,
						timezone: String::from("fake timezone"),
					},
					raw_schedule,
					todays_schedule: None,
				}
			}
		}

		fn fake_schedule_item(hour: i8, minute: i8) -> RawScheduleItem {
			RawScheduleItem {
				hour: Some(hour),
				minute: Some(minute),
				from: None,
				change: ChangeItem {
					action: Action::Color,
					mirek: Some(321),
					brightness: Some(50),
				},
			}
		}

		#[test]
		fn schedule_test() {
			let mut schedule = Schedule::new_for_test(vec![
				fake_schedule_item(1, 0), fake_schedule_item(10, 30),
			]);

			let day1 = get_tz_datetime_dhm(1, 10, 30);
			assert_eq!(schedule.todays_schedule, None);
			schedule.try_update(day1).unwrap();

			assert!(schedule.todays_schedule.is_some());
			let schedule1 = schedule.todays_schedule.clone().unwrap();
			assert_eq!(schedule1.len(), 3);
			assert_eq!(schedule1[0].time.day(), 1);

			let day2_within = get_tz_datetime_dhm(2, 0, 0);
			schedule.try_update(day2_within).unwrap();

			let schedule2_within = schedule.todays_schedule.clone().unwrap();
			assert_eq!(schedule2_within.len(), 3);
			assert_eq!(schedule2_within[0].time.day(), 1); // no change

			let day2_after = get_tz_datetime_dhm(2, 1, 0);
			schedule.try_update(day2_after).unwrap();

			let schedule2_after = schedule.todays_schedule.clone().unwrap();
			assert_eq!(schedule2_after.len(), 3);
			assert_eq!(schedule2_after[0].time.day(), 2);
		}
	}


	#[test]
	fn test_blend_action_stop_before() {
		let stop_12 = create_processed_schedule_item_stop(12, 0);
		let color_13 = create_processed_schedule_item_color(13, 0, 123, 50);

		assert_eq!(
			blend_actions(&stop_12, &color_13, &get_tz_datetime_hm(12, 0)).expect("Expected action is obtainable"),
			ChangeAction::None,
		);

		assert_eq!(
			blend_actions(&stop_12, &color_13, &get_tz_datetime_hm(12, 59)).expect("Expected action is obtainable"),
			ChangeAction::None,
		);
	}

	#[test]
	fn test_blend_action_stop_after() {
		let color_10 = create_processed_schedule_item_color(10, 0, 123, 50);
		let stop_12 = create_processed_schedule_item_stop(12, 0);

		assert_eq!(
			blend_actions(&color_10, &stop_12, &get_tz_datetime_hm(10, 0)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 123, brightness: 50 },
		);

		assert_eq!(
			blend_actions(&color_10, &stop_12, &get_tz_datetime_hm(11, 30)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 123, brightness: 50 },
		);
	}

	#[test]
	fn test_blend_action_invalid() {
		let color_10 = create_processed_schedule_item_color(10, 0, 123, 50);
		let stop_12 = create_processed_schedule_item_stop(12, 0);

		assert!(blend_actions(&color_10, &stop_12, &get_tz_datetime_hm(9, 59)).is_err());
		assert!(blend_actions(&color_10, &stop_12, &get_tz_datetime_hm(12, 1)).is_err());
		assert!(blend_actions(&stop_12, &color_10, &get_tz_datetime_hm(11, 0)).is_err());
	}

	#[test]
	fn test_blend_action_2_colors() {
		let color_10 = create_processed_schedule_item_color(10, 0, 200, 10);
		let color_20 = create_processed_schedule_item_color(20, 0, 400, 90);

		assert_eq!(
			blend_actions(&color_10, &color_20, &get_tz_datetime_hm(10, 0)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 200, brightness: 10 },
		);

		assert_eq!(
			blend_actions(&color_10, &color_20, &get_tz_datetime_hm(15, 0)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 300, brightness: 50 },
		);

		assert_eq!(
			blend_actions(&color_10, &color_20, &get_tz_datetime_hm(19, 30)).expect("Expected action is obtainable"),
			ChangeAction::Color { mirek: 390, brightness: 86 },
		);
	}

	fn create_test_schedule() -> Vec<ProcessedScheduleItem> {
		vec![
			create_processed_schedule_item_color(1, 0, 456, 50),
			create_processed_schedule_item_color(10, 59, 456, 50),
			create_processed_schedule_item_color(20, 30, 456, 50),
		]
	}

	#[test]
	fn test_surrounding_within() {
		let schedule = create_test_schedule();
		assert_eq!(
			get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(1, 0))
				.expect("Expect to get surrounding schedule items."),
			(&schedule[0], &schedule[1])
		);

		assert_eq!(
			get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(9, 0))
				.expect("Expect to get surrounding schedule items."),
			(&schedule[0], &schedule[1])
		);

		assert_eq!(
			get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(10, 0))
				.expect("Expect to get surrounding schedule items."),
			(&schedule[0], &schedule[1])
		);

		assert_eq!(
			get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(11, 0))
				.expect("Expect to get surrounding schedule items."),
			(&schedule[1], &schedule[2])
		);

		assert_eq!(
			get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(20, 29))
				.expect("Expect to get surrounding schedule items."),
			(&schedule[1], &schedule[2])
		);
	}

	#[test]
	fn test_surrounding_outside() {
		let schedule = create_test_schedule();
		assert!(get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(0, 0)).is_err());
		assert!(get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(20, 30)).is_err());
		assert!(get_surrounding_schedule_items(&schedule, get_tz_datetime_hm(20, 31)).is_err());
	}

	fn assert_schedule(
		hour: Option<i8>,
		minute: Option<i8>,
		from: Option<FromRefTime>,
		sunset_hour: u32,
		sunset_minute: u32,
		expected_hour: u32,
		expected_minute: u32)
	{
		let today = chrono::NaiveDate::from_ymd_opt(1999, 1, 1).expect("Getting today");
		let none_change = ChangeItem {
			action: Action::Stop,
			mirek: Some(123),
			brightness: None,
		};
		let item = ProcessedScheduleItem::from(
			&TEST_TZ,
			&RawScheduleItem { hour, minute, from, change: none_change.clone() },
			today,
			&get_tz_datetime_hm(sunset_hour, sunset_minute)).expect("Expected item1 config to be fine.");

		assert_eq!(item.time, get_tz_datetime_hm(expected_hour, expected_minute));
		assert_eq!(item.change, none_change);
	}

	#[test]
	fn test_schedule_item_processing () {
		assert_schedule(Some(10), Some(20), None,
			2, 2, 10, 20);
		assert_schedule(Some(10), Some(20), Some(FromRefTime::Sunset),
			2, 2, 12, 22);
		assert_schedule(Some(-3), None, Some(FromRefTime::Sunset),
			20, 40, 17, 40);
		assert_schedule(None, Some(-3), Some(FromRefTime::Sunset),
			20, 40, 20, 37);
		assert_schedule(None, Some(120), Some(FromRefTime::Sunset),
			10, 30, 12, 30);
	}
}