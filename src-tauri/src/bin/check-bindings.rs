fn main() {
    if let Err(err) = transonic_lib::bindings::check_typescript_bindings() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

// Keep this test on the binary target so Tauri embeds the Windows app manifest.
#[cfg(test)]
mod tests {
    #[test]
    fn typescript_bindings_are_up_to_date() {
        transonic_lib::bindings::check_typescript_bindings()
            .unwrap_or_else(|error| panic!("{error}"));
    }
}
