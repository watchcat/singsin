//! The SingSin entry point.

fn main() {
    console_error_panic_hook::set_once();
    singsin::app::run();
}
