use typewire::Typewire;

#[derive(Typewire)]
union Bad {
    a: u32,
    b: f32,
}

fn main() {}
