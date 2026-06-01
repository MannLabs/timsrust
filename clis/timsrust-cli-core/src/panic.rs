use std::panic;

pub fn install_panic_hook() {
    let default = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        eprintln!("fatal error: {info}");
        default(info);
    }));
}
