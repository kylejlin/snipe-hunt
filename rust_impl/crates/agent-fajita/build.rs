use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=model/fajita.bin");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("fajita.bin");
    let published = PathBuf::from("model/fajita.bin");
    if published.exists() {
        fs::copy(published, output).expect("copy published Fajita checkpoint");
    } else {
        fs::write(output, []).expect("write empty development checkpoint");
    }
}
