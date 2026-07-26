use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=model/cherry.bin");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("cherry.bin");
    let published = PathBuf::from("model/cherry.bin");
    if published.exists() {
        fs::copy(published, output).expect("copy published Cherry checkpoint");
    } else {
        fs::write(output, []).expect("write empty development checkpoint");
    }
}
