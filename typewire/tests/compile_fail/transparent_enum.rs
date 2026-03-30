use typewire::Typewire;

#[derive(Typewire)]
#[serde(transparent)]
enum Bad {
    A(u32),
    B(String),
}

fn main() {}
