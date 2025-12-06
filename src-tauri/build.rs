fn main() {
    // 创建空的 models 目录用于发布
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let models_dir = std::path::Path::new(&out_dir).join("../../../models");
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir).ok();
    }

    tauri_build::build()
}
