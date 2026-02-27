// Rust inline mod blocks

pub mod helpers {
    pub fn trim_string(s: &str) -> String {
        s.trim().to_string()
    }

    pub struct Config {
        pub name: String,
    }

    fn internal_detail() -> bool {
        true
    }
}

mod private_mod {
    pub fn do_stuff() {}
}

mod external;

pub fn top_level() -> i32 {
    42
}
