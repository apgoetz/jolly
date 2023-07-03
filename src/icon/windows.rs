#![cfg(target_os = "windows")]

use super::{Context, Icon, IconError, FALLBACK_ICON};

use serde;

use std::path::Path;

use std::mem::{size_of, MaybeUninit};

use std::os::windows::ffi::OsStrExt;

use url::Url;

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{BOOL, HMODULE, HWND, MAX_PATH, SIZE};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::UI::Controls::IImageList;
use windows::Win32::UI::Shell::{
    AssocQueryStringW, IShellItemImageFactory, SHCreateItemFromParsingName, SHDefExtractIconW,
    SHGetImageList, SHGetStockIconInfo, SHLoadIndirectString, ASSOCF, ASSOCSTR, SHGSI_FLAGS,
    SHSTOCKICONID, SHSTOCKICONINFO, SIIGBF,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

const DEFAULT_ICONSIZE: u32 = 48;

// TODO
//
// This is a list of supported icon formats by iced_graphics.
// This is based on iced_graphics using image-rs to load images, and
// looking at what features are enabled on that package. In the future
// we may not compile support for all formats but (for now) we have a
// comphrensive list here
const SUPPORTED_ICON_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "pbm", "pam", "ppm", "pgm", "tiff", "tif", "tga", "dds",
    "bmp", "ico", "hdr", "exr", "ff", "qoi",
];

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Os;

impl Default for Os {
    fn default() -> Self {
        #[cfg(test)]
        unsafe {
            use windows::Win32::System::Com::CoIncrementMTAUsage;
            let _ = CoIncrementMTAUsage(); // hack to force COM to be initialized for testing
        }
        Self
    }
}

struct WideString(Vec<u16>);

impl<T: AsRef<Path>> From<T> for WideString {
    fn from(val: T) -> Self {
        Self(
            val.as_ref()
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect(),
        )
    }
}

impl From<WideString> for String {
    fn from(val: WideString) -> Self {
        Self::from_utf16_lossy(&val.0)
    }
}

impl WideString {
    fn pcwstr(&self) -> PCWSTR {
        PCWSTR(self.0.as_ptr())
    }
}

impl super::IconInterface for Os {
    fn get_default_icon(&self) -> Icon {
        let siid = SHSTOCKICONID(0); // SIID_DOCNOASSOC
        let uflags = SHGSI_FLAGS(0x000004000); // get system icon index

        let mut psii = SHSTOCKICONINFO {
            cbSize: size_of::<SHSTOCKICONINFO>() as u32,
            hIcon: Default::default(),
            iSysImageIndex: Default::default(),
            iIcon: Default::default(),
            szPath: [0; MAX_PATH as usize],
        };

        unsafe {
            let list: IImageList = match SHGetImageList(0x2) {
                Err(e) => {
                    println!("could not get imagelist {}", e);
                    return FALLBACK_ICON.clone();
                }
                Ok(l) => l,
            };

            // if we get an error, the lookup failed, fall back to builtin default
            let result = SHGetStockIconInfo(siid, uflags, std::ptr::addr_of_mut!(psii));
            if result.is_err() {
                println!("Cannot lookup fallback icon {:?}", result);
                return FALLBACK_ICON.clone();
            }

            let hicon = match list.GetIcon(psii.iSysImageIndex, 0) {
                Err(e) => {
                    println!("could not get imagelist {}", e);
                    return FALLBACK_ICON.clone();
                }
                Ok(i) => i,
            };

            let icon = Self::get_icon_from_handle(hicon);

            if !(hicon.is_invalid() || DestroyIcon(hicon).as_bool()) {
                panic!("Could not destroy default icon handle");
            }

            icon.unwrap_or_else(|e| {
                println!("{e}");
                FALLBACK_ICON.clone()
            })
        }
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Icon, IconError> {
        const EXTENDED_PATH_MAGIC: &str = r#"\\?\"#;
        let num_skip;
        if path
            .as_ref()
            .to_string_lossy()
            .starts_with(EXTENDED_PATH_MAGIC)
        {
            num_skip = EXTENDED_PATH_MAGIC.len();
        } else {
            num_skip = 0;
        }

        let wide_path = WideString(
            path.as_ref()
                .as_os_str()
                .encode_wide()
                .skip(num_skip)
                .chain(std::iter::once(0))
                .collect(),
        );

        if wide_path.0.len() > MAX_PATH as usize {
            return Err(format!(
                "full path of icon source is too long to access with shell api: {}",
                path.as_ref().to_string_lossy()
            )
            .into());
        }

        unsafe {
            let ifactory: IShellItemImageFactory =
                SHCreateItemFromParsingName(wide_path.pcwstr(), None).context(format!(
                    "could not get shell entry for path: {}",
                    String::from(wide_path)
                ))?;

            //IShellItemImageFactory::GetImage

            let sigbf = SIIGBF(
                0x1     // SIIGBF_BIGGERSIZEOK 
			       | 0x20, //SIIGBF_CROPTOSQUARE
            );
            let size = SIZE {
                cx: DEFAULT_ICONSIZE as i32,
                cy: DEFAULT_ICONSIZE as i32,
            };

            let hbitmap = ifactory
                .GetImage(size, sigbf)
                .context("could not get bitmap")?;

            let icon = Self::get_icon_from_hbm(hbitmap);

            if !DeleteObject(hbitmap).as_bool() {
                return Err("could not delete bitmap".into());
            }

            icon.context(format!("could not convert hbitmap to icon"))
        }
    }

    fn get_icon_for_url(&self, url: &str) -> Result<Icon, IconError> {
        //  https://devblogs.microsoft.com/oldnewthing/20150914-00/?p=91601
        let flags = ASSOCF(0x80 | 0x1000); // ASSOCF_REMAPRUNDLL | ASSOCF_ISPROTOCOL

        let assocstr = ASSOCSTR(15); //  ASSOCSTR_DEFAULTICON
        let scheme = WideString::from(Url::parse(url).context("url is not valid")?.scheme());
        let mut outsize = 0u32;
        unsafe {
            // query first to get the size of the result array
            let _ = AssocQueryStringW(
                flags,
                assocstr,
                scheme.pcwstr(),
                WideString::from("open").pcwstr(),
                PWSTR::null(),
                std::ptr::addr_of_mut!(outsize),
            );

            if outsize == 0 {
                return Err(format!(
                    "no icon defined for url with scheme {}",
                    String::from(scheme)
                )
                .into());
            }

            let mut outbuf = Vec::<u16>::with_capacity(outsize as usize);

            AssocQueryStringW(
                flags,
                assocstr,
                scheme.pcwstr(),
                WideString::from("open").pcwstr(),
                PWSTR(outbuf.as_mut_ptr()),
                std::ptr::addr_of_mut!(outsize),
            )
            .ok()
            .context("could not AssocQueryStringW")?;

            if outsize == 0 {
                return Err("AssocQueryStringW output length was 0".into());
            }

            outbuf.set_len(outsize as usize);

            // check if the icon is an "indirect string"
            let path: String = if outbuf.starts_with(&['@' as u16]) {
                let mut newpath = vec![0u16; MAX_PATH as usize];

                SHLoadIndirectString(PCWSTR(outbuf.as_ptr()), &mut newpath, None)
                    .context("Error with SHLoadIndirectString")?;

                // need to trim
                String::from_utf16_lossy(&newpath)
                    .trim_end_matches(0 as char)
                    .to_string()
            } else {
                String::from_utf16_lossy(outbuf.split_last().unwrap().1) // minus 1 to remove null terminator
            };

            if SUPPORTED_ICON_EXTS
                .iter()
                .find(|f| {
                    Path::new(&path)
                        .extension()
                        .is_some_and(|p| p.eq_ignore_ascii_case(f))
                })
                .is_some()
            {
                return Ok(Icon::from_path(path));
            }

            let (mut file, index) = path.rsplit_once(",").unwrap_or((&path, "0"));

            // if the file name is wrapped in double quotes, remove it
            let mut chars = file.chars();
            if chars.next().is_some_and(|c| c == '"') && chars.next_back().is_some_and(|c| c == '"')
            {
                file = file
                    .get(1..file.len() - 1)
                    .context("could not remove quotes from file name")?;
            }

            let index = index
                .parse::<i32>()
                .context(format!("cannot parse index as i32: {}", index))?;

            let mut hicon = HICON(0);

            let pcwstr = WideString::from(file).pcwstr();

            let result = SHDefExtractIconW(
                pcwstr,
                index,
                0,
                Some(std::ptr::addr_of_mut!(hicon)),
                None,
                DEFAULT_ICONSIZE,
            );

            if result.is_err() || hicon.is_invalid() {
                println!(
                    "Could not SHDefExtractIconW, try again with first indexed icon: {file},{index}",
                );

                let icon_groups = get_icon_groups(file)?;

                let first_icon = *icon_groups
                    .get(0)
                    .context("No icon groups in DefaultIcon resource file")?;

                SHDefExtractIconW(
                    pcwstr,
                    -first_icon,
                    0,
                    Some(std::ptr::addr_of_mut!(hicon)),
                    None,
                    DEFAULT_ICONSIZE,
                )
                .context("Fallback SHDefExtractIconW Failed")?;

                if hicon.is_invalid() {
                    return Err("SHDefExtractIconW did not return HICON".into());
                }
            }

            let result = Self::get_icon_from_handle(hicon);

            // soft error. Keep going
            DestroyIcon(hicon)
                .ok()
                .context(format!("Could not destroy icon handle for path {}", path))?;

            result.context("could not convert hicon to image")
        }
    }
}

impl Os {
    unsafe fn get_icon_from_handle(handle: HICON) -> Result<Icon, IconError> {
        if handle.is_invalid() {
            return Err("invalid handle".into());
        }

        //if we are here, then we successfully got a handle to the icon. Now we need bitmaps
        let mut iconinfo = MaybeUninit::<ICONINFO>::uninit();

        let retval = GetIconInfo(handle, iconinfo.as_mut_ptr()).as_bool();

        if !retval {
            return Err("Cannot get IconInfo".into());
        }

        let iconinfo = iconinfo.assume_init();

        if !DeleteObject(iconinfo.hbmMask).as_bool() {
            panic!("could not delete hbmMask");
        }

        let icon = Self::get_icon_from_hbm(iconinfo.hbmColor);

        if !DeleteObject(iconinfo.hbmColor).as_bool() {
            return Err("Cannot delete hbmColor".into());
        }
        icon
    }
    unsafe fn get_icon_from_hbm(hbm: HBITMAP) -> Result<Icon, IconError> {
        let mut cbitmap = MaybeUninit::<BITMAP>::uninit(); //color bitmap
        const BITMAP_SIZE: i32 = size_of::<MaybeUninit<BITMAP>>() as i32;

        if GetObjectW(hbm, BITMAP_SIZE, Some(cbitmap.as_mut_ptr().cast())) == 0 {
            return Err("Cannot get hbmColor bitmap object".into());
        }

        let cbitmap = cbitmap.assume_init_ref();

        let mut header = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: cbitmap.bmWidth,
            biHeight: -cbitmap.bmHeight,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        let mut pixels =
            Vec::<u8>::with_capacity((cbitmap.bmWidth * cbitmap.bmHeight * 4).try_into().unwrap());

        let dc = GetDC(HWND(0));
        assert!(!dc.is_invalid());

        let lines_read = GetDIBits(
            dc,
            hbm,
            0,
            cbitmap.bmHeight as u32,
            Some(pixels.as_mut_ptr().cast()),
            std::ptr::addr_of_mut!(header).cast(),
            DIB_RGB_COLORS,
        );

        if ReleaseDC(HWND(0), dc) == 0 {
            panic!("could not release DC");
        }

        if lines_read != cbitmap.bmHeight {
            return Err(format!("only wrote {} lines of DIBits", lines_read).into());
        }

        // we have the pixels, extend vec to contain them
        pixels.set_len(pixels.capacity());

        for chunk in pixels.chunks_exact_mut(4) {
            let [b, _, r, _] = chunk else {unreachable!()};
            std::mem::swap(b, r);
        }

        Ok(Icon::from_pixels(
            cbitmap.bmWidth.try_into().unwrap(),
            cbitmap.bmHeight.try_into().unwrap(),
            pixels.leak(), // TODO fix leak
        ))
    }
}

#[allow(non_snake_case)]
fn MAKEINTRESOURCEA(id: i32) -> PCWSTR {
    unsafe { std::mem::transmute::<_, PCWSTR>(id as usize) }
}

#[allow(non_snake_case)]
fn IS_INTRESOURCE(lptype: PCWSTR) -> Option<i32> {
    unsafe {
        let id: usize = std::mem::transmute(lptype);
        if id >> 16 == 0 {
            Some(id as i32)
        } else {
            None
        }
    }
}

// get icon groups of file
fn get_icon_groups<P: AsRef<std::path::Path>>(path: P) -> Result<Vec<i32>, IconError> {
    use windows::Win32::System::LibraryLoader::{
        EnumResourceNamesW, FreeLibrary, LoadLibraryExW, LOAD_LIBRARY_FLAGS,
    };
    const RT_GROUP_ICON: i32 = 3 + 11;

    if !path.as_ref().is_absolute() {
        return Err(format!("non-absolute path: {}", path.as_ref().to_string_lossy()).into());
    }

    unsafe extern "system" fn name_callback(
        _hmodule: HMODULE,
        lptype: PCWSTR,
        lpname: PCWSTR,
        lparam: isize,
    ) -> BOOL {
        let icon_groups: *mut Vec<i32> = std::mem::transmute(lparam);

        match IS_INTRESOURCE(lpname) {
            Some(id) if IS_INTRESOURCE(lptype).is_some_and(|t| t == RT_GROUP_ICON) => {
                (*icon_groups).push(id);
                BOOL(1)
            }
            _ => BOOL(0), // abort if we didnt get an icon_group
        }
    }

    let lflags = LOAD_LIBRARY_FLAGS(0x20 | 0x02 | 0x08); //LOAD_LIBRARY_AS_IMAGE_RESOURCE | LOAD_LIBRARY_AS_DATAFILE | LOAD_WITH_ALTERED_SEARCH_PATH

    let mut icon_groups = Vec::new();

    let rt_group_icon = MAKEINTRESOURCEA(RT_GROUP_ICON);

    unsafe {
        let hmodule = LoadLibraryExW(WideString::from(path.as_ref()).pcwstr(), None, lflags)
            .context("could not LoadLibraryEXW")?;

        EnumResourceNamesW(
            hmodule,
            rt_group_icon,
            Some(name_callback),
            std::mem::transmute(std::ptr::addr_of_mut!(icon_groups)),
        );

        FreeLibrary(hmodule).ok().context(format!(
            "Could not FreeLibrary {}",
            path.as_ref().to_string_lossy()
        ))?;
    }
    Ok(icon_groups)
}

#[cfg(test)]
mod tests {

    //TODO to test

    // have logic to test the following

    // entries with DefaultHandler wrapped in doublequotes work
    //
    // entries with incorrect resource id fallback to first actuall resource
    //
    // get rid of all panics
    // get rid of all prints

    // geticonforfile handles extended paths correctly (compare to windows)

    // example local files that work
    //"iTunes.m3u:", // files wrapped in doublequotes work
    //"zoommtg:",    // real value should be -101 or 0, actual value is 1
    //
    // undocumented ability to change icon size from 48 to something else
    //
}
