use std::env;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=monitor.ico");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let ico_path = out_dir.join("app.ico");
    let rc_path = out_dir.join("icon.rc");

    let source = image::open("monitor.ico")
        .expect("Failed to open monitor.ico")
        .into_rgba8();

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16_u32, 32, 48, 256] {
        let resized =
            image::imageops::resize(&source, size, size, image::imageops::FilterType::Lanczos3);
        let icon_image = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir.add_entry(
            ico::IconDirEntry::encode(&icon_image).expect("Failed to encode icon size"),
        );
    }

    icon_dir
        .write(File::create(&ico_path).expect("Failed to create app.ico"))
        .expect("Failed to write multi-size app.ico");

    // RC.EXE accepts forward slashes in resource paths.
    let ico_for_rc = ico_path.to_string_lossy().replace('\\', "/");
    std::fs::write(&rc_path, format!("1 ICON \"{ico_for_rc}\"\n"))
        .expect("Failed to write generated icon.rc");

    embed_resource::compile(&rc_path, embed_resource::NONE)
        .manifest_required()
        .unwrap_or_else(|err| panic!("Failed to embed application icon: {err}"));
}
