fn main() {
    transonic_lib::bindings::export_typescript_bindings()
        .expect("Failed to export TypeScript bindings");
}
