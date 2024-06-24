mod schedule;
mod sunset;

#[macro_use] extern crate rocket;

use chrono::Local;
use rocket::serde::{json::Json, Serialize};

use schedule::{ProcessedScheduleItem, ScheduleInfo};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Task {
    dog: String,
}

#[get("/todo")]
fn todo() -> Json<Task> {
    Json(Task { dog: String::from("woof") })
}

#[get("/time")]
fn time() -> String {
    let now = Local::now();
    let str = format!("now is {}", now.format("%Y-%m-%d %H:%M:%S"));
    println!("{}", &str);
    str
}

#[launch]
fn rocket() -> _ {
    main2();
    rocket::build()
        .mount("/", routes![index, time, todo])
}

fn main2() {
    match dotenvy::dotenv() {
        Err(e) => println!("WARNING! .env NOT LOADED: {}", e),
        Ok(_) => println!("Successfully loaded .env"),
    };
    let mut schedule = ScheduleInfo::new().unwrap();
    schedule.set_today();
    println!("{:#?}", schedule);
}