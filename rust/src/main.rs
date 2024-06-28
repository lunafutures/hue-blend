mod schedule;
mod sunset;

#[macro_use] extern crate rocket; // XXX TODO necessary?

use chrono::{DateTime, Local};
use chrono_tz::Tz;
use rocket::{serde::{self, json::Json}, tokio::sync::RwLock, State};

use schedule::Schedule;

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

async fn update_if_necessary(state: &State<RwLock<Schedule>>) -> anyhow::Result<bool> {
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
async fn now(state: &State<RwLock<Schedule>>) -> Responses<NowResponse> {
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
    let now = (*reader).now();
    let change_action = match (*reader).get_action_for_now(&now) {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string()),
    };

    Responses::good(NowResponse { now, change_action, just_updated: updated })
}

#[get("/debug")]
async fn get_debug_info(state: &State<RwLock<Schedule>>) -> Responses<schedule::DebugInfo> {
    let mut writer = state.write().await;
    let debug_info = match (*writer).get_debug_info() {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string()),
    };

    Responses::good(debug_info)
}

// #[launch]
// fn rocket() -> _ {
//     main2();
//     rocket::build()
//         .manage(RwLock::new(Schedule::new().unwrap())) // XXX TODO Arc
//         .mount("/", routes![index, time, todo, get_debug_info, now])
// }

fn main() {
    match dotenvy::dotenv() {
        Err(e) => println!("WARNING! .env NOT LOADED: {}", e),
        Ok(_) => println!("Successfully loaded .env"),
    };
    let mut schedule = Schedule::new().unwrap();
    schedule.set_today().unwrap();
    println!("schedule: {schedule:#?}");

    let now = schedule.now();
    println!("now: {now}");
    let (a, b) = schedule.get_surrounding_schedule_items(Some(now.clone())).unwrap();
    println!("a: {a:#?}\nb: {b:#?}");

    let action = schedule.get_action_for_now(&now).unwrap();
    println!("action: {action:#?}");
}
