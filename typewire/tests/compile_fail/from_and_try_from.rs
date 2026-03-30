use typewire::Typewire;

#[derive(Clone, Typewire)]
#[serde(from = "String", try_from = "String")]
struct Bad(String);

impl From<String> for Bad {
    fn from(s: String) -> Self {
        Bad(s)
    }
}

fn main() {}
