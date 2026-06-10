use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=schemas-ref/fixtures");
    println!("cargo:rerun-if-changed=fixtures/placeholders");

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set"));
    let schemas_ref = manifest_dir.join("schemas-ref");
    let schemas_fixtures = schemas_ref.join("fixtures");

    let has_submodule_marker = schemas_ref.join(".git").exists();
    let has_real_fixtures = schemas_fixtures.join("valid").is_dir();

    if has_submodule_marker && has_real_fixtures {
        println!(
            "cargo:rustc-env=UBU_SCHEMAS_FIXTURES={}",
            schemas_fixtures.display()
        );
        println!("cargo:rustc-env=UBU_SCHEMAS_REF_PRESENT=1");
        return;
    }

    if schemas_ref.exists() && !has_submodule_marker {
        panic!(
            "schemas-ref exists but is not a git submodule checkout. Remove the directory or run `git submodule update --init --recursive`."
        );
    }

    println!(
        "cargo:warning=schemas-ref submodule is absent; using explicit local placeholder fixtures. TODO: initialize/update submodule."
    );

    let placeholder_fixtures = manifest_dir.join("fixtures").join("placeholders");
    println!(
        "cargo:rustc-env=UBU_SCHEMAS_FIXTURES={}",
        placeholder_fixtures.display()
    );
    println!("cargo:rustc-env=UBU_SCHEMAS_REF_PRESENT=0");
}
