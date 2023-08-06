// build script to add icon to Jolly executable.
// this script is only used on windows platforms

fn common() {
    use chrono::Utc;
    let date = Utc::now().date_naive();
    println!("cargo:rustc-env=JOLLY_BUILD_DATE={date}");
}

// no build requirements for macos FOR NOW
#[cfg(target_os = "macos")]
fn main() {
    common();
}

// check to make sure dependencies are installed
#[cfg(all(unix, not(target_os = "macos")))]
fn main() {
    use std::env;

    common();

    let theme = env::var("JOLLY_DEFAULT_THEME").unwrap_or("gnome".into());
    println!("cargo:rustc-env=JOLLY_DEFAULT_THEME={}", theme);

    // check default theme is installed
    let themes = freedesktop_icons::list_themes();
    if themes
        .iter()
        .filter(|t| t.to_uppercase() == theme.to_uppercase())
        .next()
        .is_none()
    {
        println!("cargo:warning=Jolly default icon theme '{}' does not seem to be installed. You can override the default theme via environment variable JOLLY_DEFAULT_THEME", theme);
    }

    // check  xdg-utils is installed
    let path = env::var("PATH").unwrap_or("".into());
    if path
        .split(":")
        .map(std::path::PathBuf::from)
        .find(|p| p.join("xdg-settings").exists())
        .is_none()
    {
        println!("cargo:warning=package `xdg-utils` does not seem to be installed. Icon support may be broken");
    }

    // check shared-mime-info installed
    let mut xdg_data_dirs = env::var("XDG_DATA_DIRS").unwrap_or("".into());

    if xdg_data_dirs.is_empty() {
        xdg_data_dirs = "/usr/local/share/:/usr/share/".into();
    }

    let data_home = dirs::data_dir().unwrap_or("/nonexistant/path".into());

    if std::iter::once(data_home)
        .chain(xdg_data_dirs.split(":").map(std::path::PathBuf::from))
        .find(|p| p.join("mime/mime.cache").exists())
        .is_none()
    {
        println!("cargo:warning=package `shared-mime-info` does not seem to be installed. Icon support may be broken");
    }
}

// set a nice icon
#[cfg(windows)]
fn main() {
    common();

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
