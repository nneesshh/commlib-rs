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
    #[cfg(target_os = "windows")]
    {
        // Include cpplib target dir
        println!(
            "cargo:rustc-link-search=native={}",
            manifest_path
                .join("../cpplibs/libs/win/Release")
                .as_path()
                .display()
        );

        // Link static cpplib library
        println!("cargo:rustc-link-lib=static=commlib");
    }

    // Re-run
    println!("cargo:rerun-if-changed=src/main.rs");

    // Protos
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .build_transport(false)
        .out_dir("protos/out")
        .compile(&["protos/cmd.proto", "protos/base.proto"], &["protos"])
        .unwrap();

    Ok(())
}
