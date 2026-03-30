use typewire::Typewire;

#[derive(Typewire)]
#[serde(transparent)]
struct Bad(u32, String);

fn main() {}
