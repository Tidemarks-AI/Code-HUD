use std::collections::HashMap;

#[derive(Debug)]
pub struct User {
    pub name: String,
    pub email: String,
}

impl User {
    pub fn new(name: String, email: String) -> Self {
        Self { name, email }
    }

    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

#[test]
fn test_user() {
    let user = User::new("Alice".to_string(), "alice@example.com".to_string());
    assert_eq!(user.greet(), "Hello, Alice!");
}
