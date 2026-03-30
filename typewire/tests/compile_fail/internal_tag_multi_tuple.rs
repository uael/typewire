use typewire::Typewire;

#[derive(Typewire)]
#[serde(tag = "type")]
enum Bad {
    Unit,
    Multi(u32, String),
}

fn main() {}
