use std::collections::HashMap;

/// A sample struct
#[derive(Debug, Clone)]
pub struct User {
    pub name: String,
    pub age: u32,
    email: String,
}

impl User {
    pub fn new(name: String, age: u32, email: String) -> Self {
        Self { name, age, email }
    }

    pub fn greeting(&self) -> String {
        format!("Hello, {}!", self.name)
    }

    fn validate_email(&self) -> bool {
        self.email.contains('@')
    }
}

#[derive(Debug)]
pub enum Role {
    Admin,
    User,
    Guest,
}

pub trait Authenticatable {
    fn authenticate(&self, token: &str) -> bool;
    fn roles(&self) -> Vec<Role>;
}

pub const MAX_USERS: usize = 1000;

pub type UserMap = HashMap<String, User>;

fn private_helper() -> bool {
    true
}

pub fn public_utility(input: &str) -> String {
    input.to_uppercase()
}

macro_rules! create_user {
    ($name:expr) => {
        User::new($name.to_string(), 0, String::new())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("Alice".into(), 30, "alice@example.com".into());
        assert_eq!(user.name, "Alice");
    }
}
