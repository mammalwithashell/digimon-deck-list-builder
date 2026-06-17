fn main() {
    tauri_build::build();

    // Defense-in-depth (D6 of the `desktop-engine-worker-thread` change): the
    // primary stack guarantee is the engine worker's explicit 64 MB stack
    // (`EngineHandle::spawn`), since all engine code runs there. But raise the
    // BINARY's linked main-thread stack reserve too, so any residual
    // main-thread engine path (tests, future code) has ample headroom beyond
    // the default 1 MB that overflowed during debug-build card-pack
    // deserialization.
    //
    // Windows MSVC honours a linker `/STACK` reserve; scope it to the binary
    // (`rustc-link-arg-bins`) to avoid workspace-wide cache churn / cross-crate
    // effects. On Linux the main-thread stack is governed by `RLIMIT_STACK`,
    // not the linker, and is not the constraint here (engine work is on the
    // 64 MB worker), so no Linux link-arg is needed.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // 64 MB reserve (0x0400_0000 == 67108864).
        println!("cargo::rustc-link-arg-bins=/STACK:67108864");
    }
}
