extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-search=native=C:/Users/markrad/s/azure-sdk-for-c-win/build/sdk/src/azure/core/Debug");
    println!("cargo:rustc-link-search=native=C:/Users/markrad/s/azure-sdk-for-c-win/build/sdk/src/azure/iot/Debug");
    println!("cargo:rustc-link-search=native=C:/Users/markrad/s/azure-sdk-for-c-win/build/sdk/src/azure/platform/Debug");
    println!("cargo:rustc-link-search=native=C:/Program Files/OpenSSL-Win64/lib");
    println!("cargo:rustc-link-lib=az_core");
    println!("cargo:rustc-link-lib=az_iot_common");
    println!("cargo:rustc-link-lib=az_iot_hub");
    println!("cargo:rustc-link-lib=az_win32");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Add additional Clang include directories
        .clang_arg("-IC:/Users/markrad/s/azure-sdk-for-c-win/sdk/inc")
        .clang_arg("-IC:/Users/markrad/s/BearSSL/inc")
        .clang_arg("-IC:/Users/markrad/s/MQTT-C/include")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}