fn main() {
    let sysroot = std::env::var("GCW0_SYSROOT").unwrap_or_else(|_| {
        "/opt/gcw0-toolchain/mipsel-gcw0-linux-uclibc/sysroot".into()
    });
    let lib = format!("{sysroot}/usr/lib");
    println!("cargo:rerun-if-env-changed=GCW0_SYSROOT");
    println!("cargo:rustc-link-arg=--sysroot={sysroot}");
    println!("cargo:rustc-link-search=native={lib}");
    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=GLESv2");
}
