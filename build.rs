fn main() {
    // 嵌入图标与 manifest（DPI 感知、新版通用控件）
    embed_resource::compile("res/app.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
    println!("cargo:rerun-if-changed=res");
    println!("cargo:rerun-if-changed=assets");
}
