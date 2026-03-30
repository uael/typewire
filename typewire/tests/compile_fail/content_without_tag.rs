use typewire::Typewire;

#[derive(Clone, PartialEq, Typewire)]
#[serde(content = "data")]
enum Bad {
    A,
    B,
}

fn main() {}
