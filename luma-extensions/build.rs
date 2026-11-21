fn main() {
    // Cargo exposes TARGET to build scripts, not to `env!` in library code.
    let target = std::env::var("TARGET").expect("Cargo must set TARGET for build scripts");
    println!("cargo:rustc-env=LUMA_HOST_TARGET={target}");
}
