use crate::{domain::user::User, use_cases::user_database::UserDatabase};

pub fn login(database: impl UserDatabase, username: String, password: String, jwt_secret: String) -> String {}

pub fn register(database: impl UserDatabase, username: String, password: String, email: String) {
    let user = User {

    }
    database.create_user(user)
}
