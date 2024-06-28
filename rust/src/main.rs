mod schedule;
mod sunset;

#[macro_use] extern crate rocket; // XXX TODO necessary?

use chrono::DateTime;
use chrono_tz::Tz;
use rocket::{serde::{self, json::Json}, tokio::sync::Mutex, State};

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

#[get("/now")]
async fn now(state: &State<Mutex<Schedule>>) -> Responses<NowResponse> {
    let mut guard = state.lock().await;
    let updated = match (*guard).try_update() {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string())
    };

    let now = (*guard).now();
    let change_action = match (*guard).get_action_for_now(&now) {
        Ok(o) => o,
        Err(e) => return Responses::bad(e.to_string()),
    };

    Responses::good(NowResponse { now, change_action, just_updated: updated })
}

#[get("/debug")]
async fn get_debug_info(state: &State<Mutex<Schedule>>) -> Responses<schedule::DebugInfo> {
    let mut guard = state.lock().await;

    // get_debug_info() will automatically update
    let debug_info = match (*guard).get_debug_info() {
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
        .manage(Mutex::new(Schedule::new().unwrap())) // XXX TODO Arc
        .mount("/", routes![index, get_debug_info, now])
}
