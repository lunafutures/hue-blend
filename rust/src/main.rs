mod schedule;
mod sunset;

#[macro_use] extern crate rocket; // XXX TODO necessary?

use chrono::Local;
use rocket::{serde::{json::Json, Serialize}, tokio::sync::RwLock, State};

use schedule::ScheduleInfo;

#[get("/")] // XXX remove
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Serialize)]
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

#[get("/now")]
async fn now(state: &State<RwLock<ScheduleInfo>>) -> String {
    let x = state.read().await;
    format!("{:?}", (*x).now())
}

struct AppState {
    // schedule_info: Option<ScheduleInfo>,
    // asdf: String,
    asdf: rocket::tokio::sync::RwLock<String>,
}

#[get("/schedule")]
async fn get_schedule(state: &State<AppState>) -> String { // XXX remove
    {
        let x = state.asdf.read().await;
        println!("x: {}", x);
    };
    let t = {
        let mut w = state.asdf.write().await;
        (*w).push_str("1");
        (*w).clone()
    };

    t
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
        .mount("/", routes![index, time, todo, get_schedule, get_schedule2, now])
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