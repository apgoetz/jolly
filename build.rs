// build script to add icon to Jolly executable.
// this script is only used on windows platforms

#[cfg(unix)]
fn main() {}

#[cfg(windows)]
fn main() {
    // determine path to save icon to
    let out_file = format!("{}/jolly.ico", std::env::var("OUT_DIR").unwrap());

    // render SVG as PNG
    let opt = usvg::Options::default();
    let svg_data = std::fs::read("icon/jolly.svg").unwrap();
    let rtree = usvg::Tree::from_data(&svg_data, &opt).unwrap();
    let width: u32 = 256;
    let height = width;
    let pixmap_size = rtree
        .size
        .scale_to(usvg::Size::new(width.into(), height.into()).unwrap())
        .to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(
        &rtree,
        usvg::FitTo::Width(width),
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .unwrap();
    let bytes = pixmap.encode_png().unwrap();

    // Create a new, empty icon collection:
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    // Read a PNG file from disk and add it to the collection:
    let image = ico::IconImage::read_png(bytes.as_slice()).unwrap();
    icon_dir.add_entry(ico::IconDirEntry::encode(&image).unwrap());
    // Finally, write the ICO file to disk:
    let file = std::fs::File::create(&out_file).unwrap();
    icon_dir.write(file).unwrap();

    // attach icon to resources for this executable
    let mut res = winres::WindowsResource::new();
    res.set_icon(&out_file);
    res.compile().unwrap();
}
