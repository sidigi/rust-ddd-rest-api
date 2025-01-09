pub mod data;
pub mod domain;
pub mod service;
pub mod web;

pub use domain::clip::field::ShortCode;
pub use domain::clip::{ClipError, Clip};
pub use domain::time::Time;
pub use data::DataError;
pub use service::ServiceError;

use data::AppDatabase;
use rocket::fs::FileServer;
use rocket::{Build, Rocket};
use web::renderer::Renderer;
use crate::domain::maintenance::Maintenance;
use crate::web::hitcounter::HitCounter;

pub fn rocket(config: RocketConfig) -> Rocket<Build> {
    rocket::build()
        .manage::<Renderer>(config.renderer)
        .manage::<AppDatabase>(config.database)
        .manage::<HitCounter>(config.hit_counter)
        .manage::<Maintenance>(config.maintenance)
        .mount("/", web::http::routes())
        .mount("/api/clip", web::api::routes())
        .mount("/static", FileServer::from("static"))
        .register("/", web::http::catcher::catchers())
        .register("/api/clip", web::api::catcher::catchers())
}

pub struct RocketConfig {
    pub renderer: Renderer<'static>,
    pub database: AppDatabase,
    pub hit_counter: HitCounter,
    pub maintenance: Maintenance
}

#[cfg(test)]
pub mod test {
    pub fn async_runtime() ->  tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("Failed to spawn tokio runtime")
    }
}