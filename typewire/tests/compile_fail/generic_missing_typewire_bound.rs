use typewire::Typewire;

struct NotTypewire;

#[derive(Typewire)]
struct Wrapper<T> {
    value: T,
}

// The derive adds `T: Typewire` bound to the impl, so this fails.
fn check() -> Option<Wrapper<NotTypewire>> {
    Wrapper::<NotTypewire>::or_default()
}

fn main() {}
