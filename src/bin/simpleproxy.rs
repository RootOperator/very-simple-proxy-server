use std::any::type_name;

fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}


fn main() {
    let s: u16 = 4000;
    println!("{:#?}", type_of(([0,0,0,0], s)));
}