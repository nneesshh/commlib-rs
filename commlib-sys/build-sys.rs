fn main() -> miette::Result<()> {
    // Cargo envs
    let pkgname = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME was not set");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR was not set");
    println!(
        "==== build.rs: CARGO_PKG_NAME={} CARGO_MANIFEST_DIR={} ====",
        &pkgname, &manifest_dir
    );

    let linkage = std::env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or(String::new());
    if linkage.contains("crt-static") {
        println!("==== build.rs: the C runtime will be statically linked ====");
    } else {
        println!("==== build.rs: the C runtime will be dynamically linked ====");
    }

    // Other envs
    let profile = std::env::var("PROFILE").expect("PROFILE was not set");
    let target = std::env::var("TARGET").expect("TARGET was not set");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR was not set");
    println!(
        "==== build.rs: PROFILE={} TARGET={} OUT_DIR={} ====",
        &profile, &target, &out_dir
    );

    // Bridge -- cxx
    let ffi_files = vec!["ffi/crypto.rs", "ffi/hash.rs", "ffi/signal.rs"];
    for file in &ffi_files {
        println!("cargo:rerun-if-changed={}", file);
    }

    // Re-run
    println!("cargo:rerun-if-changed=src/lib.rs");

    // Includes and libs
    let manifest_path = std::path::PathBuf::from(manifest_dir);

    // Define path to resolve #include relative position
    let include_paths = vec![
        manifest_path.join("cpp"),
    ];

    // Define path to resolve #include relative position
    let cpp_paths = vec![
        manifest_path.join("cpp/crypto_bindings.cc"),
        manifest_path.join("cpp/hash_bindings.cc"),
        manifest_path.join("cpp/signal_bindings.cc"),
        manifest_path.join("cpp/crypto/blowfish.cc"),
        manifest_path.join("cpp/crypto/blowfish_cfb64.cc"),
        manifest_path.join("cpp/hash/md5.cc"),
    ];

    // Cxx
    cxx_build::bridges(ffi_files)
        .flag("-I/usr/local/include")
        .flag_if_supported("-std=c++17")
        .includes(&include_paths)
        .files(&cpp_paths)
        .compile("commlib_sys");

    // Add instructions to link to any C++ libraries you need.
    Ok(())
}
