fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico")
        .set_icon_with_id("icon.ico", "test")
        .set("InternalName", "push2_soundboard.exe")
        // manually set version 1.0.0.0
        .set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000);
    res.compile().unwrap();
}
