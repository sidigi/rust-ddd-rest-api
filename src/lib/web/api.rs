use std::str::FromStr;
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, Responder, State};
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use serde::Serialize;
use crate::data::AppDatabase;
use crate::service::action;
use crate::{service, ServiceError, ShortCode};
use crate::web::hitcounter::HitCounter;
use crate::web::PASSWORD_COOKIE;

pub const API_KEY_HEADER: &str = "x-api-key";

#[derive(Debug, Clone)]
pub struct ApiKey(Vec<u8>);

#[derive(Responder, Debug, thiserror::Error, Serialize)]
pub enum ApiKeyError {
    #[error("Invalid API key")]
    #[response(status = 400, content_type = "json")]
    DecodeError(String),
    #[error("API key not found")]
    #[response(status = 404, content_type = "json")]
    NotFound(String),
}

impl ApiKey{
    pub fn to_base64(&self) -> String {
        base64::encode(&self.0.as_slice())
    }
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl Default for ApiKey {
    fn default() -> Self {
        let key = (0..16).map(|_| rand::random::<u8>()).collect();
        Self(key)
    }
}

impl FromStr for ApiKey{
    type Err = ApiKeyError;

    fn from_str(key: &str) -> Result<Self, Self::Err> {
        base64::decode(key).map(ApiKey).map_err(|e| Self::Err::DecodeError(e.to_string()))
    }
}

#[derive(Responder, Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found")]
    #[response(status = 404, content_type = "json")]
    NotFound(Json<String>),
    #[error("server error")]
    #[response(status = 500, content_type = "json")]
    ServerError(Json<String>),
    #[error("client error")]
    #[response(status = 401, content_type = "json")]
    User(Json<String>),
    #[error("key error")]
    #[response(status = 400, content_type = "json")]
    KeyError(Json<ApiKeyError>),
}

impl From<ServiceError> for ApiError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::Clip(c) => Self::User(Json(format!("clip parsing error: {}", c))),
            ServiceError::NotFound => Self::NotFound(Json("entity not found".to_owned())),
            ServiceError::Data(_) => Self::ServerError(Json("a server error occurred".to_owned())),
            ServiceError::PermissionError(msg) => Self::User(Json(msg)),
        }
    }

}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey {
    type Error = ApiError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        fn server_error() -> Outcome<ApiKey, ApiError> {
            Outcome::Error((
                Status::InternalServerError,
                ApiError::ServerError(Json("a server error occurred".to_owned()))
            ))
        }

        fn key_error(e: ApiKeyError) -> Outcome<ApiKey, ApiError> {
            Outcome::Error((
                Status::BadRequest,
                ApiError::KeyError(Json(e))
            ))
        }
        
        match req.headers().get_one(API_KEY_HEADER) {
            None => key_error(ApiKeyError::NotFound("API key not found".to_string())),
            Some(key) => {
                let db = match req.guard::<&State<AppDatabase>>().await {
                    Outcome::Success(db) => db,
                    _ => return server_error(),
                };

                let api_key = match ApiKey::from_str(key) {
                    Ok(key) => key,
                    Err(e) => return key_error(e),
                };
                
                match action::api_key_is_valid(api_key.clone(), db.get_pool()).await {
                    Ok(valid) if valid => {
                        Outcome::Success(api_key)
                    }
                    Ok(valid) if !valid => {
                        key_error(ApiKeyError::NotFound("API key not found".to_string()))
                    }
                    _ => server_error(),
                }
            }
        }
    }
}

#[rocket::get("/key")]
pub async fn new_api_key(database: &State<AppDatabase>) -> Result<Json<&str>, ApiError> {
    let key = action::generate_api_key(database.get_pool()).await?;
    println!("Generated API key: {}", key.to_base64());
    Ok(Json("API key generated"))
}

#[rocket::get("/<shortcode>")]
pub async fn get_clip(
    shortcode: &str,
    database: &State<AppDatabase>,
    cookie: &CookieJar<'_>,
    hit_counter: &State<HitCounter>,
    _api_key: ApiKey
) -> Result<Json<crate::Clip>, ApiError> {
    use crate::domain::clip::field::Password;

    let req = service::ask::GetClip {
        shortcode: shortcode.into(),
        password: cookie
            .get(PASSWORD_COOKIE)
            .map(|cookie| cookie.value())
            .map(|raw_password| Password::new(raw_password.to_string()).ok())
            .flatten()
            .unwrap_or_else(Password::default),
    };
    let clip = action::get_clip(req, database.get_pool()).await?;
    hit_counter.hit(shortcode.into(), 1);
    Ok(Json(clip))
}

#[rocket::post("/", data = "<req>")]
pub async fn new_clip(
    req: Json<service::ask::NewClip>,
    database: &State<AppDatabase>,
    _api_key: ApiKey
) -> Result<Json<crate::Clip>, ApiError> {
    let clip = action::new_clip(req.into_inner(), database.get_pool()).await?;
    Ok(Json(clip))
}
#[rocket::put("/", data = "<req>")]
pub async fn update_clip(
    req: Json<service::ask::UpdateClip>,
    database: &State<AppDatabase>,
    _api_key: ApiKey
) -> Result<Json<crate::Clip>, ApiError> {
    let clip = action::update_clip(req.into_inner(), database.get_pool()).await?;
    Ok(Json(clip))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![get_clip, new_clip, update_clip, new_api_key]
}


pub mod catcher {
    use rocket::serde::json::Json;
    use rocket::Request;
    use rocket::{catch, catchers, Catcher};

    #[catch(default)]
    fn default(req: &Request) -> Json<&'static str>{
        eprintln!("General error: {:?}", req);
        Json("something went wrong...")
    }

    #[catch(500)]
    fn internal_error(req: &Request) -> Json<&'static str> {
        eprintln!("Internal error: {:?}", req);
        Json("internal server error...")
    }

    #[catch(404)]
    fn not_found() -> Json<&'static str> {
        Json("404")
    }

    #[catch(400)]
    fn request_error() -> Json<&'static str> {
        Json("request error")
    }

    #[catch(401)]
    fn missing_api_key() -> Json<&'static str> {
        Json("missing API key")
    }
    pub fn catchers() -> Vec<Catcher> {
        catchers![default, internal_error, not_found, missing_api_key, request_error]
    }
}