use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=macos/vision_ocr.m");
        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
        let obj_path = out_dir.join("vision_ocr.o");
        let lib_path = out_dir.join("libytcs_vision_ocr.a");

        let mut clang = Command::new("clang");
        clang.args([
            "-c",
            "-O2",
            "-fobjc-arc",
            "-fPIC",
            "macos/vision_ocr.m",
            "-o",
        ]);
        clang.arg(&obj_path);
        if let Ok(arch) = env::var("CARGO_CFG_TARGET_ARCH") {
            let apple_arch = match arch.as_str() {
                "aarch64" => Some("arm64"),
                "x86_64" => Some("x86_64"),
                _ => None,
            };
            if let Some(a) = apple_arch {
                clang.args(["-arch", a]);
            }
        }

        let status = clang
            .status()
            .expect("Failed to invoke clang for macos/vision_ocr.m");
        assert!(status.success(), "clang failed compiling macos/vision_ocr.m");

        let ar_status = Command::new("ar")
            .arg("rcs")
            .arg(&lib_path)
            .arg(&obj_path)
            .status()
            .expect("Failed to invoke ar for libytcs_vision_ocr.a");
        assert!(ar_status.success(), "ar failed creating libytcs_vision_ocr.a");

        println!("cargo:rustc-link-search=native={}", out_dir.display());
        println!("cargo:rustc-link-lib=static=ytcs_vision_ocr");
        println!("cargo:rustc-link-lib=framework=Vision");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }

    tauri_build::build();
}
