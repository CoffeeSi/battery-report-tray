use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=icon.ico");

    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

        let rc_path = out_dir.join("resource.rc");
        let obj_path = out_dir.join("resource.o");
        let icon_path = manifest_dir.join("icon.ico");

        let icon_path_str = icon_path.to_str().unwrap().replace('\\', "/");
        let rc_content = format!("1 ICON \"{}\"\n", icon_path_str);
        std::fs::write(&rc_path, rc_content).expect("Не удалось записать resource.rc");

        let status = Command::new("windres")
            .args([
                "-i",
                rc_path.to_str().unwrap(),
                "-o",
                obj_path.to_str().unwrap(),
            ])
            .status()
            .expect("Не удалось запустить windres. Убедитесь, что MinGW добавлен в PATH.");

        if !status.success() {
            panic!("windres завершился с ошибкой");
        }

        println!("cargo:rustc-link-arg={}", obj_path.display());
    }
}
