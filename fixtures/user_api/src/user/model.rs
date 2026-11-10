
enum Password {
    // only old passwords are still stored like this
    SHA_256(String),
    // new passwords will only be stored like this
    Argon2id(String),
}

pub struct Email {
    address: String,
    promotional_emails: bool,
    confirmed_by_user: Datetime,
}

pub struct User {
    username: String,
    password: Password,

    email: Email,

}