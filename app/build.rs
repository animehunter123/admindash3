use std::process::Command;

// This is a cool way to set environment variables that
// cargo will use when building. These are in-turn read
// by the main.rs as well as my docker build container script
// so that the container version matches the git commit hash.

// -----------------------------------------------------------------------------
// ADMINDASH3 BUILD SCRIPT
// -----------------------------------------------------------------------------
//
// build.rs runs automatically before Cargo compiles the application.
//
// We use it to discover information about the build environment and expose
// that information to the Rust application through compile-time environment
// variables.
//
// This gives us:
//
//     GIT_HASH
//         The short Git commit that produced this build.
//
//     RUSTC_VERSION
//         The Rust compiler version used to produce this build.
//
// main.rs and footer.rs can then access these values with:
//
//     env!("GIT_HASH")
//     env!("RUSTC_VERSION")
//
// Because these are compile-time values, there is no runtime lookup required.
// -----------------------------------------------------------------------------

fn main() {
    // -------------------------------------------------------------------------
    // GET GIT COMMIT HASH
    // -------------------------------------------------------------------------
    //
    // Ask Git for the short version of the current commit.
    //
    // For example:
    //
    //     a83f91c
    //
    // This is useful because the application version alone does not tell us
    // exactly which source-code revision was used to create the binary.
    // -------------------------------------------------------------------------

    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to get git commit hash");

    let git_hash = String::from_utf8(output.stdout).expect("Git returned invalid UTF-8");

    let git_hash = git_hash.trim();

    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    // -------------------------------------------------------------------------
    // GET RUST COMPILER VERSION
    // -------------------------------------------------------------------------
    //
    // rustc_version asks Cargo/Rust which compiler version is being used for
    // this build.
    //
    // For example:
    //
    //     1.92.0
    //
    // The version is then exported to the application as:
    //
    //     RUSTC_VERSION
    //
    // main.rs and footer.rs can read it with:
    //
    //     env!("RUSTC_VERSION")
    // -------------------------------------------------------------------------

    let version = rustc_version::version().expect("Failed to determine Rust compiler version");

    println!("cargo:rustc-env=RUSTC_VERSION={version}");

    // -------------------------------------------------------------------------
    // GET DIOXUS WEB VERSION FROM CARGO.TOML
    // -------------------------------------------------------------------------
    //
    // Cargo does not automatically expose a dependency's version as an
    // environment variable. Read the dioxus-web dependency declaration and
    // export it so footer.rs can display the actual version used by this app.
    //
    // Keeping the version in Cargo.toml as the source of truth means there is
    // no second hardcoded version to remember to update.
    // -------------------------------------------------------------------------

    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");

    let dioxus_web_version = cargo_toml
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("dioxus-web") && line.contains('='))
        .and_then(|line| line.split('"').nth(1))
        .expect("Could not find a quoted dioxus-web version in Cargo.toml");

    println!("cargo:rustc-env=DIOXUS_WEB_VERSION={dioxus_web_version}");

    // -------------------------------------------------------------------------
    // BUILD SCRIPT RERUN RULES
    // -------------------------------------------------------------------------
    //
    // Tell Cargo when this build script should run again.
    //
    // The Git hash should be refreshed when HEAD changes.
    //
    // The Rust compiler version normally changes when the toolchain changes,
    // and Cargo will generally rebuild when the build environment changes.
    //
    // This line makes the Git dependency explicit to Cargo.
    // -------------------------------------------------------------------------

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
