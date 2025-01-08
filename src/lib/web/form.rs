use rocket::FromForm;
use serde::Serialize;
use crate::domain::clip::field;

#[derive(Debug, Serialize, FromForm)]
pub struct NewClip {
    pub title: field::Title,
    pub content: field::Content,
    pub password: field::Password,
    pub expires: field::Expires,
}

#[derive(Debug, Serialize, FromForm)]
pub struct GetPasswordProtectedClip {
    pub password: field::Password,
}