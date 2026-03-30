use typewire::Typewire;

#[derive(Clone, PartialEq, Typewire)]
#[serde(tag = "type", untagged)]
enum Bad {
    A,
    B,
}

fn main() {}
