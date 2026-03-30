use typewire::Typewire;

#[derive(Typewire)]
#[serde(transparent)]
struct Bad {
    a: u32,
    b: String,
}

fn main() {}
