mod schedule;
mod sunset;

#[macro_use] extern crate rocket; // XXX TODO necessary?

use chrono::{DateTime, Local};
use chrono_tz::Tz;
use rocket::{serde::{self, json::Json}, tokio::sync::RwLock, State};

use schedule::ScheduleInfo;

#[get("/")] // XXX remove
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(serde::Serialize)]
#[serde(crate = "rocket::serde")]
struct Task {
    dog: String,
}

#[get("/todo")] // XXX remove
fn todo() -> Json<Task> {
    Json(Task { dog: String::from("woof") })
}

#[get("/time")] // XXX remove
fn time() -> String {
    let now = Local::now();
    let str = format!("now is {}", now.format("%Y-%m-%d %H:%M:%S"));
    println!("{}", &str);
    str
}

#[derive(Debug, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
struct Error {
    error: String,
}

#[derive(Responder)]
enum Responses<T> {
    #[response(status = 400)]
    Bad(Json<Error>),
    #[response(status = 200)]
    Good(Json<T>),
}

impl<T> Responses<T> {
    fn bad(s: String) -> Responses<T> {
        Responses::Bad(Json(Error { error: s }))
    }

    fn good(t: T) -> Responses<T> {
        Responses::Good(Json(t))
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
struct NowResponse {
    now: DateTime<Tz>,
    change_action: schedule::ChangeAction,
    just_updated: bool,
}

async fn update_if_necessary(state: &State<RwLock<ScheduleInfo>>) -> anyhow::Result<bool> {
    let should_update: bool = {
        let reader = state.read().await;
        (*reader).todays_schedule.is_none()
    };
    
    if should_update {
        let mut writer = state.write().await;
        match (*writer).set_today() {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    } else {
        Ok(false)
    }
}

#[get("/now")]
async fn now(state: &State<RwLock<ScheduleInfo>>) -> Responses<NowResponse> {
    let updated = match update_if_necessary(state).await {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string()),
    };
    // let should_update: bool = {
    //     let reader = state.read().await;
    //     (*reader).todays_schedule.is_none()
    // };
    
    // if should_update {
    //     let mut writer = state.write().await;
    //     match (*writer).set_today() {
    //         Ok(_) => (),
    //         Err(e) => return Responses::bad(e.to_string()),
    //     }
    // }

    let reader = state.read().await;
    let now = {
        match (*reader).now() {
            Ok(o) => o,
            Err(e) => return Responses::bad(e.to_string()),
        }
    };
    let change_action = match (*reader).get_action_for_now(now) {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string()),
    };

    Responses::good(NowResponse { now, change_action, just_updated: updated })
}

struct AppState {
    // schedule_info: Option<ScheduleInfo>,
    // asdf: String,
    asdf: rocket::tokio::sync::RwLock<String>,
}

#[get("/debug")]
async fn get_debug_info(state: &State<RwLock<ScheduleInfo>>) -> Responses<schedule::DebugInfo> {
    let updated = match update_if_necessary(state).await {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string()),
    };

    let reader = state.read().await;
    let debug_info = match (*reader).get_debug_info() {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string()),
    };

    Responses::good(debug_info)

}

#[get("/schedule2")]
async fn get_schedule2(state: &State<RwLock<ScheduleInfo>>) -> String {
    let mut x = state.write().await;
    match &(*x).todays_schedule {
        Some(s) => format!("{s:#?}"),
        None => match (*x).set_today() {
            Ok(_o) => format!("{:#?}", (*x).todays_schedule),
            Err(e) => String::from(format!("Bad: {e}")),
        },
    }
}

#[launch]
fn rocket() -> _ {
    main2();
    rocket::build()
        .manage(AppState { asdf: RwLock::new(String::from("asdf")) })
        .manage(RwLock::new(ScheduleInfo::new().unwrap()))
        .mount("/", routes![index, time, todo, get_debug_info, get_schedule2, now])
}

fn main2() {
    match dotenvy::dotenv() {
        Err(e) => println!("WARNING! .env NOT LOADED: {}", e),
        Ok(_) => println!("Successfully loaded .env"),
    };
    let mut schedule = ScheduleInfo::new().unwrap();
    schedule.set_today().unwrap();
    println!("schedule: {schedule:#?}");

    let now = schedule.now().unwrap();
    let (a, b) = schedule.get_surrounding_schedule_items(Some(now.clone())).unwrap();
    println!("a: {a:#?}\nb: {b:#?}");

    let action = schedule.get_action_for_time(a, b, &now).unwrap();
    println!("action: {action:#?}");
}