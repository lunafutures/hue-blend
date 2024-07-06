use rocket::{Request, Data, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Status;

pub struct AutoLogger;

#[rocket::async_trait]
impl Fairing for AutoLogger {
    fn info(&self) -> Info {
        Info {
            name: "GET/POST Counter",
            kind: Kind::Request | Kind::Response
        }
    }

    async fn on_request(&self, _request: &mut Request<'_>, _: &mut Data<'_>) {
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let body = response.body_mut().to_string().await;
        if let Ok(body) = body {
            info!(
                "Response for {} {}: {} {}",
                request.method(),
                request.uri(),
                response.status(),
                body,
            );
        } else {
            info!(
                "Response for {} {}: {}",
                request.method(),
                request.uri(),
                response.status(),
            );
        }

        if response.status() != Status::Ok {
            warn!(
                "Non-OK status: {} for {} {}",
                response.status(),
                request.method(),
                request.uri(),
            );
        }
    }
}
