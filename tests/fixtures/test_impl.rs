// test_impl.rs
pub struct User {
    name: String,
    age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    fn validate(&self) -> bool {
        self.age > 0 && !self.name.is_empty()
    }

    pub fn display(&self) -> String {
        format!("{} ({})", self.name, self.age)
    }
}

pub trait Greeter {
    fn greet(&self) -> String;
}

impl Greeter for User {
    fn greet(&self) -> String {
        format!("Hello, {}", self.name)
    }
}
