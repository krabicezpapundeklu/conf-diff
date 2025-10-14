use std::{io::Result, process::Command};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=web/package-lock.json");
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/static");
    println!("cargo:rerun-if-changed=web/static");
    println!("cargo:rerun-if-changed=web/webpack.common.js");
    println!("cargo:rerun-if-changed=web/webpack.dev.js");
    println!("cargo:rerun-if-changed=web/webpack.prod.js");

    Command::new("npm").current_dir("./web").arg("i").output()?;

    Command::new("npm")
        .current_dir("./web")
        .arg("run")
        .arg("build")
        .output()?;

    Ok(())
}
