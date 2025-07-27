#![cfg(target_os = "macos")]

use super::{Context, Icon, IconError, IconInterface, DEFAULT_ICON_SIZE};
use objc2::rc::Retained;
use objc2_app_kit::{NSImage, NSWorkspace};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGDataProvider, CGImage};
use objc2_foundation::{self, NSString, NSURL};
use objc2_uniform_type_identifiers::{self, UTType};
use serde;
use url::Url;

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Os;

impl IconInterface for Os {
    fn get_default_icon(&self) -> Result<Icon, IconError> {
        let ident = objc2_foundation::ns_string!("public.item");

        unsafe {
            let typ = UTType::typeWithIdentifier(ident).context("type was null")?;
            let workspace = NSWorkspace::sharedWorkspace();
            let icon = workspace.iconForContentType(&typ);
            image2icon(icon).context("Could not get default icon")
        }
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Icon, IconError> {
        // now we have an icon! At this point, we can start
        // using the nicer wrappers from core_graphics-rs
        unsafe {
            icon_for_file(NSString::from_str(
                path.as_ref()
                    .to_str()
                    .context("requested icon for non-utf8 path")?,
            ))
        }
    }

    fn get_icon_for_url(&self, url: &str) -> Result<Icon, IconError> {
        Url::parse(url).context("url is not valid")?; // TODO, hoist this out of all 3 implementations

        unsafe {
            let webstr = NSString::from_str(url);

            // for full url
            let url = NSURL::URLWithString(&webstr).context("Could not make NSURL")?;

            let workspace = NSWorkspace::sharedWorkspace();

            let appurl = workspace
                .URLForApplicationToOpenURL(&url)
                .context("Could not get url of app for opening url")?;

            let path = appurl.path().context("Could not get path of app url")?;

            icon_for_file(path)
        }
        // convert to URL. Determine application url, get path to application, get icon for file

        // if we cannot convert to URL, assume it is a file.

        // if the file exists, use iconForFile

        // if the file does not exist, take its extension, and use typeWithFilenameExtention, and then iconForContentType
    }
}

unsafe fn image2icon(image: Retained<NSImage>) -> Result<Icon, IconError> {
    let mut rect = CGRect {
        origin: CGPoint::new(0.0, 0.0),
        size: CGSize::new(DEFAULT_ICON_SIZE as f64, DEFAULT_ICON_SIZE as f64),
    };

    let cgicon = image
        .CGImageForProposedRect_context_hints(&mut rect, None, None)
        .context("Cannot get CGImage")?;

    // we dont know for sure we got RGBA data but we assume it for the rest of this function
    let bpp = CGImage::bits_per_pixel(Some(&cgicon));
    let bpc = CGImage::bits_per_component(Some(&cgicon));
    if bpc != 8 || bpp != 32 {
        return Err(format!("CGImage does not have 32bit depth: bpc: {bpc} bpp: {bpp}").into());
    }

    let h = CGImage::height(Some(&cgicon)) as u32;
    let w = CGImage::width(Some(&cgicon)) as u32;

    // copies
    let provider =
        CGImage::data_provider(Some(&cgicon)).context("could not get data provider for CGImage")?;
    let data = CGDataProvider::data(Some(&provider)).context("could not get CGData for image")?;
    let pixels = data.to_vec();

    Ok(Icon::from_rgba(h, w, pixels))
}

unsafe fn icon_for_file(path: Retained<NSString>) -> Result<Icon, IconError> {
    let workspace = NSWorkspace::sharedWorkspace();
    let icon = workspace.iconForFile(&path);

    image2icon(icon)
}
