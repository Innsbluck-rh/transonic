fn main() {
    if let Err(err) = transonic_lib::bindings::check_typescript_bindings() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
