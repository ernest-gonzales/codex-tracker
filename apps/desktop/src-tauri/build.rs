fn main() {
    println!("cargo:rerun-if-changed=../../web/dist");
    tauri_build::build();
}
