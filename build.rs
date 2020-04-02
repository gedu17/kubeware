use std::path::Path;
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let target_dir_path = env::var("OUT_DIR").unwrap();
    copy(&target_dir_path, "config.toml");
    tonic_build::compile_protos("proto/service.proto")?;
   Ok(())
}

fn copy<S: AsRef<std::ffi::OsStr> + ?Sized, P: Copy + AsRef<Path>>(target_dir_path: &S, file_name: P) {
    fs::copy(file_name, Path::new(&target_dir_path).join("../../..").join(file_name)).unwrap();
}