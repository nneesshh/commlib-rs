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

    // Libs
    let manifest_path = std::path::PathBuf::from(manifest_dir);

    // This assumes all your C++ bindings are in main.rs
    println!("cargo:rerun-if-changed=src/ffi_main.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/main.rs");

    // Define path to resolve #include relative position
    let include_paths = vec![
        manifest_path.join("cpp"),
    ];

    // Protos
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .build_transport(false)
        .out_dir("protos/out")
        .compile(&["protos/voting.proto"], &["protos"])
        .unwrap();

    // Bridge -- cxx
    cxx_build::bridge("src/ffi_main.rs")
        .flag("-I/usr/local/include")
        .flag_if_supported("-std=c++14")
        .includes(&include_paths)
        .file("cpp/signal.cpp")
        .compile("cxxbridge-demo");

    // Add instructions to link to any C++ libraries you need.
    Ok(())
}
