use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=model/kiwi.bin");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("kiwi.bin");
    let published = PathBuf::from("model/kiwi.bin");
    if published.exists() {
        fs::copy(published, output).expect("copy published Kiwi checkpoint");
    } else {
        fs::write(output, []).expect("write empty development checkpoint");
    }
}
