#[rustfmt::skip]
pub fn has_overlapping() {
    let f = || { || {
            println!("whee");
        }
    };
}

fn something_else() {
    hello();
    hoo();
    raa();
    roo();
}
