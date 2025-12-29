use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let dist_dir = manifest_dir.join("../../apps/web/dist");
    if !dist_dir.exists() {
        panic!(
            "missing web assets at {} (run `npm run build` in apps/web)",
            dist_dir.display()
        );
    }

    let mut files = Vec::new();
    collect_files(&dist_dir, &mut files);
    files.sort();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let dest_path = out_dir.join("embedded_assets.rs");

    let mut output = String::new();
    output.push_str("pub struct EmbeddedAsset {\n");
    output.push_str("    pub path: &'static str,\n");
    output.push_str("    pub mime: &'static str,\n");
    output.push_str("    pub bytes: &'static [u8],\n");
    output.push_str("}\n\n");
    output.push_str("pub static EMBEDDED_ASSETS: &[EmbeddedAsset] = &[\n");

    for file in &files {
        let rel_path = file
            .strip_prefix(&dist_dir)
            .expect("relative path")
            .to_string_lossy()
            .replace('\\', "/");
        let mime = mime_for_path(file);
        output.push_str(&format!(
            "    EmbeddedAsset {{ path: \"{}\", mime: \"{}\", bytes: include_bytes!(r#\"{}\"#) }},\n",
            rel_path,
            mime,
            file.display()
        ));
        println!("cargo:rerun-if-changed={}", file.display());
    }

    output.push_str("];\n");
    fs::write(&dest_path, output).expect("write embedded_assets.rs");

    println!("cargo:rerun-if-changed={}", dist_dir.display());
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "map" => "application/json; charset=utf-8",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}
