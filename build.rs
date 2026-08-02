#![allow(clippy::expect_used)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));

    stage("frontend/dist/plugin.js", &out.join("plugin.js"));

    let css = [
        "frontend/dist/plugin.css",
        "frontend/dist/cs2-addons-plugin.css",
        "frontend/dist/style.css",
    ]
        .iter()
        .find(|p| fs::metadata(p).is_ok())
        .copied();
    match css {
        Some(p) => stage(p, &out.join("plugin.css")),
        None => fs::write(out.join("plugin.css"), b"").expect("write empty css stub"),
    }

    println!("cargo:rerun-if-changed=frontend/dist");
}

fn stage(src: &str, dst: &Path) {
    match fs::read(src) {
        Ok(bytes) => fs::write(dst, bytes).expect("stage frontend asset"),
        Err(_) => {
            println!("cargo:warning=missing {src}; building without frontend bundle");
            fs::write(dst, b"").expect("write empty asset stub");
        }
    }
}
