use rocket::{Request, Data, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Status;

pub struct AutoLogger;

#[rocket::async_trait]
impl Fairing for AutoLogger {
    fn info(&self) -> Info {
        Info {
            name: "AutoLogger",
            kind: Kind::Request | Kind::Response
        }
    }

    async fn on_request(&self, _request: &mut Request<'_>, _: &mut Data<'_>) {
        // Don't need to log since rocket will do that automatically
        // if `log_level = "normal"` is set in Rocket.toml
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let mut body_message = String::from("(no body)");

        let body = response.body_mut().to_string().await;
        if let Ok(body) = body {
            body_message = body.to_string();

            // If we read the response body like we did in this function, we need to then
            // call set_sized_body(). Otherwise, the client will receive no response body.
            response.set_sized_body(body.len(), std::io::Cursor::new(body));
        }

        if response.status() == Status::Ok {
            info!(
                "Response for {} {}: {} {body_message}",
                request.method(),
                request.uri(),
                response.status(),
            );
        } else {
            warn!(
                "Non-OK status: {} for {} {} {body_message}",
                response.status(),
                request.method(),
                request.uri(),
            );
        }
    }
}
