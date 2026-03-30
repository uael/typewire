use typewire::Typewire;

#[derive(Typewire)]
#[serde(transparent)]
struct Empty;

fn main() {}
