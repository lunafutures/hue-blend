mod schedule;
mod sunset;

#[macro_use] extern crate rocket; // XXX TODO necessary?

use chrono::DateTime;
use chrono_tz::Tz;
use rocket::{serde::{self, json::Json}, tokio::sync::RwLock, State};

use schedule::Schedule;

#[get("/")] // XXX remove
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Debug, serde::Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
struct ErrorResponse {
    error: String,
}

#[derive(Responder)]
enum Responses<T> {
    #[response(status = 400)]
    Bad(Json<ErrorResponse>),
    #[response(status = 200)]
    Good(Json<T>),
}

impl<T> Responses<T> {
    fn bad(s: String) -> Responses<T> {
        Responses::Bad(Json(ErrorResponse { error: s }))
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

#[launch]
fn rocket() -> _ {
    #[cfg(debug_assertions)]
    {
        match dotenvy::dotenv() {
            Err(e) => println!("WARNING! .env NOT LOADED: {}", e),
            Ok(_) => println!("Successfully loaded .env"),
        };
    }

    rocket::build()
        .manage(RwLock::new(Schedule::new().unwrap())) // XXX TODO Arc
        .mount("/", routes![index, get_debug_info, now])
}
