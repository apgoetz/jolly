#![cfg(target_os = "macos")]

use super::{Context, Icon, IconError, IconInterface, DEFAULT_ICON_SIZE};
use core_graphics::geometry::{CGPoint, CGRect, CGSize};
use core_graphics::image::CGImageRef;
use objc::rc::StrongPtr;
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use serde;
use url::Url;

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Os;

impl IconInterface for Os {
    fn get_default_icon(&self) -> Result<Icon, IconError> {
        let ident: NSString = "public.item".into();

        unsafe {
            let typ: *mut Object = msg_send![class!(UTType), typeWithIdentifier: ident];
            let workspace = get_workspace()?;

            let icon: *mut Object = msg_send![workspace, iconForContentType: typ];
            let icon = icon.as_mut().context("iconForContentType was null")?;
            image2icon(icon).context("Could not get default icon")
        }
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Icon, IconError> {
        // now we have an icon! At this point, we can start
        // using the nicer wrappers from core_graphics-rs
        unsafe { icon_for_file(path.as_ref().as_os_str().into()) }
    }

    fn get_icon_for_url(&self, url: &str) -> Result<Icon, IconError> {
        Url::parse(url).context("url is not valid")?; // TODO, hoist this out of all 3 implementations

        unsafe {
            let webstr: NSString = url.into();

            let nsurl = class!(NSURL);

            // for full url
            let url: *mut Object = msg_send![nsurl, URLWithString: webstr];

            if url.is_null() {
                return Err("Could not make NSURL".into());
            }

            let workspace = get_workspace()?;

            // get app url
            let appurl: *mut Object = msg_send![workspace, URLForApplicationToOpenURL: url];

            if appurl.is_null() {
                return Err("Could not get url of app for opening url".into());
            }

            let path: *mut Object = msg_send![appurl, path];

            if path.is_null() {
                return Err("Could not get path of app url".into());
            }

            icon_for_file(path.try_into()?)
        }
        // convert to URL. Determine application url, get path to application, get icon for file

        // if we cannot convert to URL, assume it is a file.

        // if the file exists, use iconForFile

        // if the file does not exist, take its extension, and use typeWithFilenameExtention, and then iconForContentType
    }
}

unsafe fn get_workspace() -> Result<&'static mut Object, IconError> {
    let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
    workspace
        .as_mut()
        .context("Could not get sharedWorkspace: Null")
}

unsafe fn image2icon(image: &mut Object) -> Result<Icon, IconError> {
    let rect = CGRect {
        origin: CGPoint::new(0.0, 0.0),
        size: CGSize::new(DEFAULT_ICON_SIZE as f64, DEFAULT_ICON_SIZE as f64),
    };

    let cgicon: *mut CGImageRef = msg_send![image, CGImageForProposedRect:&rect context:0 hints:0];
    let cgicon = cgicon.as_ref().ok_or("Cannot get CGImage")?;

    // we dont know for sure we got RGBA data but we assume it for the rest of this function
    let bpc = cgicon.bits_per_component();
    let bpp = cgicon.bits_per_pixel();
    if bpc != 8 || bpp != 32 {
        return Err(format!("CGImage does not have 32bit depth: bpc: {bpc} bpp: {bpp}").into());
    }

    let h = cgicon.height() as u32;
    let w = cgicon.width() as u32;

    // copies
    let pixels = Vec::from(cgicon.data().bytes());

    Ok(Icon::from_pixels(h, w, pixels.leak()))
}

unsafe fn icon_for_file(path: NSString) -> Result<Icon, IconError> {
    let workspace = get_workspace()?;

    let icon: *mut Object = msg_send![workspace, iconForFile: path];

    image2icon(icon.as_mut().context("Could not get iconForFile: null")?)
}

struct NSString(StrongPtr);

impl NSString {
    unsafe fn from_raw(b: *const u8, len: usize) -> Self {
        let nsstring = class!(NSString);
        let obj = StrongPtr::new(msg_send![nsstring, alloc]);
        if obj.is_null() {
            panic!("failed to alloc NSString")
        }
        let outstr = StrongPtr::new(msg_send![*obj, initWithBytes:b length:len encoding:4]);

        outstr.as_ref().expect("Could not init NSString");

        Self(outstr)
    }
}

use std::ffi::OsStr;
impl From<&OsStr> for NSString {
    fn from(s: &OsStr) -> NSString {
        use std::os::unix::ffi::OsStrExt;
        unsafe {
            let b = s.as_bytes();
            NSString::from_raw(b.as_ptr(), b.len())
        }
    }
}

impl std::ops::Deref for NSString {
    type Target = *mut Object;
    fn deref(&self) -> &*mut Object {
        self.0.deref()
    }
}

use std::fmt::{Formatter, Pointer};
impl Pointer for NSString {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for NSString {
    fn from(s: &str) -> NSString {
        let b = s.as_bytes();
        unsafe { NSString::from_raw(b.as_ptr(), b.len()) }
    }
}

impl TryFrom<*mut Object> for NSString {
    type Error = IconError;
    fn try_from(s: *mut Object) -> Result<Self, Self::Error> {
        use objc::runtime::{BOOL, NO};
        unsafe {
            let p = s.as_ref().context("Null Ptr While converting NSString")?;

            let result: BOOL = msg_send![p, isKindOfClass: class!(NSString)];

            if result == NO {
                return Err("Conversion error: Object is not isKindOfClass NSString".into());
            }

            Ok(NSString(StrongPtr::new(s)))
        }
    }
}
