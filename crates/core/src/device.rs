//! safe wrappers for libimobiledevice - discovery, lockdown, AFC

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr::null_mut;

use anyhow::{anyhow, bail, Result};

use crate::ffi;

pub const LABEL: &str = "musicport";

fn cstr(s: &str) -> CString {
    CString::new(s).expect("no interior NUL bytes in strings we pass to FFI")
}

fn check(rc: c_int, what: &str) -> Result<()> {
    // Every libimobiledevice success constant is 0.
    if rc == 0 {
        Ok(())
    } else {
        Err(anyhow!("{what} failed (error {rc})"))
    }
}

/// connected iphone with lockdown handle
pub struct Device {
    raw: ffi::idevice_t,
    udid: String,
}

/// Information about the device, fetched from lockdown.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DeviceInfo {
    pub udid: String,
    pub name: Option<String>,
    pub product_type: Option<String>,
    pub ios_version: Option<String>,
    pub build: Option<String>,
}

/// One entry of the discovered-device list (no connection made yet).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceListing {
    pub udid: String,
    /// "usb" for a wired connection, "network" for Wi-Fi.
    pub connection: String,
}

/// list devices via usbmuxd (no connect, no Trust popup)
pub fn list_devices() -> Result<Vec<DeviceListing>> {
    let mut devices: ffi::idevice_info_t = null_mut();
    let mut count: c_int = 0;
    check(
        unsafe { ffi::idevice_get_device_list_extended(&mut devices, &mut count) },
        "list devices",
    )?;
    if devices.is_null() {
        return Ok(Vec::new());
    }
    // devices is an array of pointers, not structs - index as pointer array or you get garbage
    let arr = devices as *mut ffi::idevice_info_t;
    let mut out = Vec::new();
    for i in 0..count.max(0) as usize {
        let info_ptr = unsafe { *arr.add(i) };
        if info_ptr.is_null() {
            continue;
        }
        let info = unsafe { &*info_ptr };
        let udid = unsafe { CStr::from_ptr(info.udid) }.to_string_lossy().into_owned();
        let connection = match info.conn_type {
            ffi::CONNECTION_USBMUXD => "usb",
            ffi::CONNECTION_NETWORK => "network",
            _ => "unknown",
        }
        .to_string();
        out.push(DeviceListing { udid, connection });
    }
    unsafe { ffi::idevice_device_list_extended_free(devices) };
    Ok(out)
}

impl Device {
    /// connect to udid or first usb device
    pub fn new(udid: Option<&str>) -> Result<Self> {
        let mut raw: ffi::idevice_t = null_mut();
        let rc = match udid {
            Some(u) => unsafe { ffi::idevice_new(&mut raw, cstr(u).as_ptr()) },
            None => unsafe { ffi::idevice_new_with_options(&mut raw, std::ptr::null(), ffi::IDEVICE_LOOKUP_USBMUX) },
        };
        if rc != ffi::IDEVICE_E_SUCCESS || raw.is_null() {
            bail!(
                "no iPhone found over USB - plug it in, unlock it, and tap \
                 \"Trust This Computer\" if prompted"
            );
        }
        let mut cudid: *mut c_char = null_mut();
        check(unsafe { ffi::idevice_get_udid(raw, &mut cudid) }, "get device UDID")?;
        let udid = unsafe { CStr::from_ptr(cudid) }.to_string_lossy().into_owned();
        unsafe { libc::free(cudid as *mut c_void) };
        Ok(Self { raw, udid })
    }

    pub fn udid(&self) -> &str {
        &self.udid
    }

    /// Read device info over a lockdown session.
    pub fn info(&self) -> Result<DeviceInfo> {
        let ld = self.lockdown()?;
        let info = DeviceInfo {
            udid: self.udid.clone(),
            name: lockdown_string(ld, "DeviceName")?,
            product_type: lockdown_string(ld, "ProductType")?,
            ios_version: lockdown_string(ld, "ProductVersion")?,
            build: lockdown_string(ld, "BuildVersion")?,
        };
        unsafe { ffi::lockdownd_client_free(ld) };
        Ok(info)
    }

    /// Open an AFC session (the phone's media partition, /var/mobile/Media).
    pub fn open_afc(&self) -> Result<Afc> {
        let mut client: ffi::afc_client_t = null_mut();
        check(
            unsafe { ffi::afc_client_start_service(self.raw, &mut client, cstr(LABEL).as_ptr()) },
            "start AFC service",
        )?;
        if client.is_null() {
            bail!("AFC service returned a null client");
        }
        Ok(Afc { raw: client })
    }

    fn lockdown(&self) -> Result<ffi::lockdownd_client_t> {
        let mut client: ffi::lockdownd_client_t = null_mut();
        check(
            unsafe { ffi::lockdownd_client_new_with_handshake(self.raw, &mut client, cstr(LABEL).as_ptr()) },
            "lockdown handshake",
        )?;
        Ok(client)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { ffi::idevice_free(self.raw) };
    }
}

fn lockdown_string(ld: ffi::lockdownd_client_t, key: &str) -> Result<Option<String>> {
    let mut value: ffi::plist_t = null_mut();
    let rc = unsafe { ffi::lockdownd_get_value(ld, std::ptr::null(), cstr(key).as_ptr(), &mut value) };
    if rc != ffi::LOCKDOWN_E_SUCCESS || value.is_null() {
        return Ok(None);
    }
    let mut s: *mut c_char = null_mut();
    unsafe { ffi::plist_get_string_val(value, &mut s) };
    let out = if s.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned())
    };
    unsafe {
        if !s.is_null() {
            libc::free(s as *mut c_void);
        }
        ffi::plist_free(value);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// AFC
// ---------------------------------------------------------------------------

/// A session with the device's media partition.
pub struct Afc {
    raw: ffi::afc_client_t,
}

impl Afc {
    /// List directory entries (relative to /var/mobile/Media).
    pub fn listdir(&self, path: &str) -> Result<Vec<String>> {
        let mut list: *mut *mut c_char = null_mut();
        check(
            unsafe { ffi::afc_read_directory(self.raw, cstr(path).as_ptr(), &mut list) },
            "read directory",
        )?;
        let out = c_str_array_to_vec(list);
        unsafe { ffi::afc_dictionary_free(list) };
        Ok(out)
    }

    /// True if the path exists on the device.
    pub fn exists(&self, path: &str) -> Result<bool> {
        let mut info: *mut *mut c_char = null_mut();
        let rc = unsafe { ffi::afc_get_file_info(self.raw, cstr(path).as_ptr(), &mut info) };
        match rc {
            ffi::AFC_E_SUCCESS => {
                if !info.is_null() {
                    unsafe { ffi::afc_dictionary_free(info) };
                }
                Ok(true)
            }
            ffi::AFC_E_OBJECT_NOT_FOUND => Ok(false),
            other => Err(anyhow!("get_file_info failed (error {other})")),
        }
    }

    /// Download a file's full contents.
    pub fn read_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let mut handle: u64 = 0;
        check(
            unsafe { ffi::afc_file_open(self.raw, cstr(path).as_ptr(), ffi::AFC_FOPEN_RDONLY, &mut handle) },
            "open file for read",
        )?;
        let mut out = Vec::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let mut got: u32 = 0;
            check(
                unsafe {
                    ffi::afc_file_read(
                        self.raw,
                        handle,
                        buf.as_mut_ptr() as *mut c_char,
                        buf.len() as u32,
                        &mut got,
                    )
                },
                "read file",
            )?;
            if got == 0 {
                break;
            }
            out.extend_from_slice(&buf[..got as usize]);
        }
        unsafe { ffi::afc_file_close(self.raw, handle) };
        Ok(out)
    }

    /// Upload a file, truncating if it already exists.
    pub fn write_bytes(&self, path: &str, data: &[u8]) -> Result<()> {
        let mut handle: u64 = 0;
        check(
            unsafe { ffi::afc_file_open(self.raw, cstr(path).as_ptr(), ffi::AFC_FOPEN_WRONLY, &mut handle) },
            "open file for write",
        )?;
        let mut written_total = 0usize;
        while written_total < data.len() {
            let end = (written_total + 64 * 1024).min(data.len());
            let mut written: u32 = 0;
            check(
                unsafe {
                    ffi::afc_file_write(
                        self.raw,
                        handle,
                        data[written_total..end].as_ptr() as *const c_char,
                        (end - written_total) as u32,
                        &mut written,
                    )
                },
                "write file",
            )?;
            if written == 0 {
                break;
            }
            written_total += written as usize;
        }
        unsafe { ffi::afc_file_close(self.raw, handle) };
        if written_total != data.len() {
            bail!("short write: {written_total}/{} bytes", data.len());
        }
        Ok(())
    }

    pub fn make_dir(&self, path: &str) -> Result<()> {
        check(
            unsafe { ffi::afc_make_directory(self.raw, cstr(path).as_ptr()) },
            "make directory",
        )
    }

    /// Delete a single file or empty directory.
    pub fn remove_path(&self, path: &str) -> Result<()> {
        check(
            unsafe { ffi::afc_remove_path(self.raw, cstr(path).as_ptr()) },
            "remove path",
        )
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<()> {
        check(
            unsafe { ffi::afc_rename_path(self.raw, cstr(from).as_ptr(), cstr(to).as_ptr()) },
            "rename path",
        )
    }
}

impl Drop for Afc {
    fn drop(&mut self) {
        unsafe { ffi::afc_client_free(self.raw) };
    }
}

fn c_str_array_to_vec(list: *mut *mut c_char) -> Vec<String> {
    if list.is_null() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    unsafe {
        loop {
            let p = *list.add(i);
            if p.is_null() {
                break;
            }
            out.push(CStr::from_ptr(p).to_string_lossy().into_owned());
            i += 1;
        }
    }
    out
}
