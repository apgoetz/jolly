#![cfg(target_os = "windows")]

use super::{Context, Icon, IconError, DEFAULT_ICON_SIZE, SUPPORTED_ICON_EXTS};

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

impl Context<()> for BOOL {
    fn context<S: AsRef<str> + std::fmt::Display>(self, msg: S) -> Result<(), IconError> {
        self.ok().context(msg)
    }
}

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

// represents a
#[derive(Debug)]
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

impl TryFrom<WideString> for String {
    type Error = IconError;
    fn try_from(val: WideString) -> Result<Self, Self::Error> {
        Ok(Self::from_utf16(&val.0)
            .context("Invalid utf16")?
            .trim_end_matches(0 as char)
            .to_string())
    }
}

impl WideString {
    fn pcwstr(&self) -> PCWSTR {
        PCWSTR(self.0.as_ptr())
    }

    // goal here is to make sure extended paths are properly converted
    // widestrings. This can fail if extended path is to long to be
    // represented by a PCWSTR
    //
    // https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation?tabs=registry
    fn from_path<P: AsRef<Path>>(val: P) -> Result<Self, IconError> {
        use std::ffi::OsString;
        const EXT_PATH: &str = r#"\\?\"#;
        const UNC_PATH: &str = r#"\\?\UNC\"#;
        let words = val.as_ref().as_os_str().encode_wide();
        let path = val.as_ref().as_os_str().to_string_lossy(); // can be lossy since we are only checking first few chars in the string
        let words: Vec<_> = if path.starts_with(UNC_PATH) {
            OsString::from(r#"\\"#)
                .as_os_str()
                .encode_wide()
                .chain(words.skip(UNC_PATH.len()))
                .chain(std::iter::once(0))
                .collect()
        } else if path.starts_with(EXT_PATH) {
            words
                .skip(EXT_PATH.len())
                .chain(std::iter::once(0))
                .collect()
        } else {
            words.chain(std::iter::once(0)).collect()
        };
        if words.len() > MAX_PATH as usize {
            return Err(format!("Path {path} is longer than MAX_PATH").into());
        } else {
            Ok(Self(words))
        }
    }
}

impl super::IconInterface for Os {
    fn get_default_icon(&self) -> Result<Icon, IconError> {
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
            // TODO support other imagelists if the default icon size changes
            let list: IImageList = SHGetImageList(0x2).context("could not get imagelist")?;

            // if we get an error, the lookup failed, fall back to builtin default
            SHGetStockIconInfo(siid, uflags, std::ptr::addr_of_mut!(psii))
                .context("Cannot SHGetStockIconInfo default icon")?;

            let hicon = list
                .GetIcon(psii.iSysImageIndex, 0)
                .context("Could not GetIcon default icon")?;

            let icon = get_icon_from_handle(hicon);

            if hicon.is_invalid() {
                return Err("HICON is null for fallback image".into());
            }

            DestroyIcon(hicon).context("Could not DestroyIcon fallback icon handle")?;

            icon
        }
    }

    fn get_icon_for_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<Icon, IconError> {
        let wide_path = WideString::from_path(path.as_ref())?;

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
                    "could not get shell entry for path: {:?}",
                    String::try_from(wide_path)
                ))?;

            //IShellItemImageFactory::GetImage

            let sigbf = SIIGBF(
                0x1     // SIIGBF_BIGGERSIZEOK 
			       | 0x20, //SIIGBF_CROPTOSQUARE
            );
            let size = SIZE {
                cx: DEFAULT_ICON_SIZE as i32,
                cy: DEFAULT_ICON_SIZE as i32,
            };

            let hbitmap = ifactory
                .GetImage(size, sigbf)
                .context("could not get bitmap")?;

            let icon = get_icon_from_hbm(hbitmap);

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
                    "no icon defined for url with scheme {:?}",
                    String::try_from(scheme)
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
                String::from_utf16(&newpath)
                    .context(format!("invalid utf16 in defaulticon for {}", url))?
                    .trim_end_matches(0 as char)
                    .to_string()
            } else {
                String::from_utf16(outbuf.split_last().unwrap().1)
                    .context(format!("invalid utf16 in defaulticon for {}", url))?
                // minus 1 to remove null terminator
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

            // if we have gotten to this point, we assume that the
            // icon is of the form "file.exe,-1" where file.exe is the
            // path to the file that has the icon, and the number is
            // the index of the icon
            get_icon_from_file_and_index(path)
                .context("Could not load icon for {url}, stored as {path}")
        }
    }
}

// takes a string of the  form "file.exe,-1" and looks up the HICON that is associated with that index
//
// if the index is not specified, uses first icon in file
// if the file name is wrapped in quotes, removes them
// if the index is specified but doesnt exist, falls back to using the first entry in the file.
fn get_icon_from_file_and_index(path: String) -> Result<Icon, IconError> {
    let (mut file, index) = path.rsplit_once(",").unwrap_or((&path, "0"));

    // if the file name is wrapped in double quotes, remove it
    let mut chars = file.chars();
    if chars.next().is_some_and(|c| c == '"') && chars.next_back().is_some_and(|c| c == '"') {
        file = file
            .get(1..file.len() - 1)
            .context("could not remove quotes from file name")?;
    }

    let index = index
        .parse::<i32>()
        .context(format!("cannot parse index as i32: {}", index))?;

    let mut hicon = HICON(0);

    let pcwstr = WideString::from(file).pcwstr();
    unsafe {
        let result = SHDefExtractIconW(
            pcwstr,
            index,
            0,
            Some(std::ptr::addr_of_mut!(hicon)),
            None,
            DEFAULT_ICON_SIZE as u32,
        );

        // if the first request for the icon doesnt work, we can
        // fallback to using the first icon defined in the
        // resource file
        let hicon = if result.is_err() || hicon.is_invalid() {
            let lib = Library::open(file)?;
            let icon_groups = lib.get_first_icon_group()?;

            let first_icon = icon_groups.ok_or(IconError::from(
                "No icon groups in DefaultIcon resource file",
            ))?;

            let hicon = lib.get_hicon_by_group_id(first_icon, DEFAULT_ICON_SIZE as i32)?;

            lib.close()?;

            hicon
        } else {
            hicon
        };

        let result = get_icon_from_handle(hicon);

        // soft error. Keep going
        DestroyIcon(hicon.clone())
            .ok()
            .context(format!("Could not destroy HICON({})", hicon.0))?;

        result.context("could not convert hicon to image")
    }
}

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

    if let Err(e1) = DeleteObject(iconinfo.hbmMask).context("Could not delete hbmMask") {
        if let Err(e2) = DeleteObject(iconinfo.hbmColor).context("Could not delete hbmColor") {
            return Err(e1).context(format!(
                "While processing this error, another error occured: {}",
                e2
            ));
        } else {
            return Err(e1);
        }
    }

    let icon = get_icon_from_hbm(iconinfo.hbmColor);

    DeleteObject(iconinfo.hbmColor).context("Cannot delete hbmColor")?;

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

    if ReleaseDC(HWND(0), dc) != 1 {
        return Err("could not ReleaseDC".into());
    }

    if lines_read != cbitmap.bmHeight {
        return Err(format!("only wrote {} lines of DIBits", lines_read).into());
    }

    // we have the pixels, extend vec to contain them
    pixels.set_len(pixels.capacity());

    for chunk in pixels.chunks_exact_mut(4) {
        let [b, _, r, _] = chunk else { unreachable!() };
        std::mem::swap(b, r);
    }

    Ok(Icon::from_pixels(
        cbitmap.bmWidth.try_into().unwrap(),
        cbitmap.bmHeight.try_into().unwrap(),
        pixels.leak(), // TODO fix leak
    ))
}

#[allow(non_snake_case)]
fn MAKEINTRESOURCEW(id: i32) -> PCWSTR {
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

const RT_ICON: i32 = 3;
const RT_GROUP_ICON: i32 = RT_ICON + 11;

struct Library(HMODULE, String);

impl Library {
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, IconError> {
        use windows::Win32::System::LibraryLoader::{LoadLibraryExW, LOAD_LIBRARY_FLAGS};

        if !path.as_ref().is_absolute() {
            return Err(format!("non-absolute path: {}", path.as_ref().to_string_lossy()).into());
        }

        let pathstr = path.as_ref().as_os_str().to_string_lossy();

        let lflags = LOAD_LIBRARY_FLAGS(0x20 | 0x02 | 0x08); //LOAD_LIBRARY_AS_IMAGE_RESOURCE | LOAD_LIBRARY_AS_DATAFILE | LOAD_WITH_ALTERED_SEARCH_PATH

        unsafe {
            let hmodule = LoadLibraryExW(WideString::from(path.as_ref()).pcwstr(), None, lflags)
                .context(format!("could not LoadLibraryEXW for file {pathstr}"))?;
            Ok(Self(hmodule, pathstr.to_string()))
        }
    }

    fn inner_close(&mut self) -> Result<(), IconError> {
        use windows::Win32::System::LibraryLoader::FreeLibrary;
        let hmodule = self.0;

        self.0 = HMODULE(0);

        if hmodule == HMODULE(0) {
            return Ok(());
        }

        unsafe {
            FreeLibrary(hmodule)
                .ok()
                .context(format!("Could not FreeLibrary {}", self.1))?;
        }
        Ok(())
    }

    fn close(mut self) -> Result<(), IconError> {
        self.inner_close()
    }

    fn get_first_icon_group(&self) -> Result<Option<i32>, IconError> {
        use windows::Win32::System::LibraryLoader::EnumResourceNamesW;
        unsafe extern "system" fn get_first_icon_group(
            _hmodule: HMODULE,
            lptype: PCWSTR,
            lpname: PCWSTR,
            lparam: isize,
        ) -> BOOL {
            let icon_groups: *mut Option<i32> = std::mem::transmute(lparam);
            let icon_groups = icon_groups.as_mut().expect("icon group is NULL");

            match IS_INTRESOURCE(lpname) {
                Some(id) if IS_INTRESOURCE(lptype).is_some_and(|t| t == RT_GROUP_ICON) => {
                    // only remember the first icon group
                    if icon_groups.is_none() {
                        *icon_groups = Some(id);
                    }
                    BOOL(1)
                }
                _ => BOOL(0), // abort if we didnt get an icon_group
            }
        }

        let mut first_icon_group: Option<i32> = None;

        let rt_group_icon = MAKEINTRESOURCEW(RT_GROUP_ICON);

        unsafe {
            EnumResourceNamesW(
                self.0,
                rt_group_icon,
                Some(get_first_icon_group),
                std::mem::transmute(std::ptr::addr_of_mut!(first_icon_group)),
            )
            .context(format!("Could not EnumResourceNames for {}", self.1))?;
        }

        Ok(first_icon_group)
    }

    fn get_hicon_by_group_id(&self, icon_group: i32, size: i32) -> Result<HICON, IconError> {
        use windows::Win32::Foundation::GetLastError;
        use windows::Win32::System::LibraryLoader::{
            FindResourceW, LoadResource, LockResource, SizeofResource,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateIconFromResourceEx, LookupIconIdFromDirectoryEx, IMAGE_FLAGS,
        };

        unsafe {
            let hrsrc = FindResourceW(
                self.0,
                MAKEINTRESOURCEW(icon_group),
                MAKEINTRESOURCEW(RT_GROUP_ICON),
            );
            if hrsrc.is_invalid() {
                return Err(GetLastError()
                    .ok()
                    .context(format!("Could not FindResource {icon_group}"))
                    .unwrap_err());
            }

            let hglobal = LoadResource(self.0, hrsrc)
                .context(format!("Could not LoadResource {icon_group}"))?;

            let ptr: *const u8 = LockResource(hglobal).cast();

            if ptr.is_null() {
                return Err(IconError::from(format!(
                    "Could not LockResource {icon_group}"
                )));
            }

            let icon_id = LookupIconIdFromDirectoryEx(ptr, true, size, size, IMAGE_FLAGS(0));

            if icon_id == 0 {
                return Err(GetLastError()
                    .ok()
                    .context(format!(
                        "Could not LookupIconIdFromDirectoryEx {icon_group}"
                    ))
                    .unwrap_err());
            }

            let hrsrc = FindResourceW(self.0, MAKEINTRESOURCEW(icon_id), MAKEINTRESOURCEW(RT_ICON));
            if hrsrc.is_invalid() {
                return Err(GetLastError()
                    .ok()
                    .context(format!("Could not FindResource icon_id {icon_id}"))
                    .unwrap_err());
            }

            let rsrc_size = SizeofResource(self.0, hrsrc);

            if rsrc_size == 0 {
                return Err(GetLastError()
                    .ok()
                    .context(format!("Could not SizeofResource icon_id {icon_id}"))
                    .unwrap_err());
            }

            // now we have a resource id for best fitting icon
            let hglobal = LoadResource(self.0, hrsrc)
                .context(format!("Could not LoadResource for icon_id {icon_id}"))?;

            let ptr: *const u8 = LockResource(hglobal).cast();

            if ptr.is_null() {
                return Err(IconError::from(format!(
                    "Could not LockResource for icon_id {icon_id}"
                )));
            }

            let bytes = std::slice::from_raw_parts(ptr, rsrc_size as usize);

            let hicon = CreateIconFromResourceEx(bytes, true, 0x00030000, 0, 0, IMAGE_FLAGS(0))
                .context("Could not CreateIconFromResourceEx for icon_id {icon_id}")?;

            if hicon.is_invalid() {
                return Err("CreateIconFromResourceEx returned NULL HICON".into());
            } else {
                Ok(hicon)
            }
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        let _ = self.inner_close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wide_string() {
        let max_path = MAX_PATH as usize;

        assert_eq!(
            String::try_from(WideString::from_path(r#"\\?\asdf"#).unwrap()).unwrap(),
            "asdf"
        );
        assert_eq!(
            String::try_from(WideString::from_path(r#"\\?\UNC\asdf"#).unwrap()).unwrap(),
            r#"\\asdf"#
        );
        assert_eq!(
            String::try_from(WideString::from_path("asdf").unwrap()).unwrap(),
            "asdf"
        );

        WideString::from_path("a".repeat(max_path + 1)).unwrap_err();
        WideString::from_path("a".repeat(max_path)).unwrap_err();
        WideString::from_path("a".repeat(max_path - 1)).unwrap();
    }

    #[test]
    fn test_get_icon_for_file_max_path() {
        use crate::icon::IconInterface;

        let max_path = MAX_PATH as usize;
        let os = Os::default();

        let dir = tempfile::tempdir().unwrap();

        let path = dir.path();
        let remsize = max_path - path.as_os_str().encode_wide().count() - 1;

        let over = path.join("a".repeat(remsize + 1));
        let equal = path.join("a".repeat(remsize));
        let under = path.join("a".repeat(remsize - 1));

        std::fs::File::create(&over).unwrap();
        std::fs::File::create(&equal).unwrap();
        std::fs::File::create(&under).unwrap();

        os.get_icon_for_file(over).unwrap_err();
        os.get_icon_for_file(equal).unwrap_err();
        os.get_icon_for_file(under).unwrap();
    }

    #[test]
    fn test_indexed_resources() {
        use crate::icon::tests::hash_eq_icon;
        let filename = r#"C:\Windows\System32\shell32.dll"#;

        get_icon_from_file_and_index("nonexistant.dll".to_string()).unwrap_err();

        // previously this has been flaky, so we repeat the request
        // multiple times to try and confirm it is really fixed
        const N: usize = 16;

        let icons: Result<Vec<_>, IconError> = std::iter::repeat_with(|| {
            Ok((
                get_icon_from_file_and_index(filename.to_string()).context("no index: ")?,
                get_icon_from_file_and_index(format!("{filename},0")).context("index0")?,
                get_icon_from_file_and_index(format!("{filename},999")).context("index999")?,
                get_icon_from_file_and_index(format!("{filename},-999")).context("negindex999")?,
                get_icon_from_file_and_index(format!(r#""{filename}",0"#)).context("quoted")?,
            ))
        })
        .take(N)
        .collect();

        let (noindex, index0, index999, negindex999, quoted) =
            icons.unwrap().into_iter().next().unwrap();

        assert!(hash_eq_icon(&noindex, &index0));
        assert!(hash_eq_icon(&index999, &index0));
        assert!(hash_eq_icon(&negindex999, &index0));
        assert!(hash_eq_icon(&quoted, &index0));
    }

    //TODO to test

    // have logic to test the following
    // geticonforfile handles extended paths correctly (compare to windows)

    // undocumented ability to change icon size from 48 to something else
    //
}
