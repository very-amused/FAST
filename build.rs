use std::env;
use std::path::PathBuf;

fn main() {
	// Generate bindings for include/fast.h
	let header_entry = "include/fast.h";
	let bindings = bindgen::Builder::default()
		.header(header_entry)
		// Tell cargo to invalidate the built crate whenever any of the included header files changed
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		.generate()
		.expect(format!("Failed to generate Rust bindings for {}", header_entry).as_str());

	// Write generated bindings to $OUT_DIR/bindings.rs
	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings.write_to_file(out_path.join("bindings.rs"))
		.expect(format!("Failed to write bindings to {}", out_path.display()).as_str());
}
