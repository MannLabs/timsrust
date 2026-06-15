use std::{env, fs, path::PathBuf};

fn main() {
    const LIB_NAME: &str = "timsdata";

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // Name of the runtime artifact that must sit next to the final binary.
    // The import library (`timsdata.lib`) on Windows is only needed at link
    // time and does not need to be copied to the target dir.
    let runtime_lib_name = match target_os.as_str() {
        "linux" => format!("lib{}.so", LIB_NAME),
        "windows" => format!("{}.dll", LIB_NAME),
        other => panic!(
            "timsrust-sdk: unsupported target OS '{}'. Only linux and windows are supported.",
            other
        ),
    };

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Allow overriding the directory that contains the timsdata library via an env var.
    // This is useful when building against a git dependency where the library is not
    // committed to the repo (it is gitignored). Example:
    //   TIMSDATA_LIB_DIR=/path/to/timsrust-sdk/lib cargo build --features sdk
    let lib_path = if let Ok(override_dir) = env::var("TIMSDATA_LIB_DIR") {
        PathBuf::from(override_dir)
    } else {
        crate_dir.join("lib")
    };

    // Tell Rust where to find the timsdata library at build time.
    // On Linux this resolves to libtimsdata.so; on Windows MSVC picks up
    // timsdata.lib (the import library for timsdata.dll) from the same dir.
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=dylib={}", LIB_NAME);

    // Embed a relative rpath so the .so is found next to the executable at
    // runtime. This is a GNU-ld flag and only applies on Linux; on Windows
    // the loader finds timsdata.dll next to the .exe automatically.
    if target_os == "linux" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    // Determine the actual target/<profile> directory Cargo is building into.
    // OUT_DIR is e.g. <target_dir>/build/timsrust-sdk-<hash>/out, so go up 3 levels.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR does not have enough ancestors")
        .to_path_buf();

    // Source file you want to copy
    let source = lib_path.join(&runtime_lib_name);
    // Destination in the target folder
    let destination = target_dir.join(&runtime_lib_name);
    // Create directory if missing
    fs::create_dir_all(&target_dir).unwrap();
    // Copy the file
    fs::copy(&source, &destination).unwrap_or_else(|e| {
        panic!("Failed to copy {:?} to {:?}: {}", source, destination, e)
    });

    // Logging
    println!("cargo:warning=Target dir: {}", target_dir.display());
    println!("cargo:warning=Source dir: {}", source.display());
    // Re-run this script if the file changes
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-env-changed=TIMSDATA_LIB_DIR");
}
