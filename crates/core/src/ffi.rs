//! hand-written libimobiledevice ffi - just what we use
//! matches 1.4.0 headers (LGPL-2.1), kept small on purpose

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::ffi::{c_char, c_int, c_void};

// ---- opaque handles -------------------------------------------------------
pub type plist_t = *mut c_void;
pub type idevice_t = *mut c_void;
pub type lockdownd_client_t = *mut c_void;
pub type lockdownd_service_descriptor_t = *mut lockdownd_service_descriptor;
pub type afc_client_t = *mut c_void;

// ---- structs --------------------------------------------------------------
#[repr(C)]
pub struct lockdownd_service_descriptor {
    pub port: u16,
    pub ssl_enabled: u8,
    pub identifier: *mut c_char,
}

/// One entry of an extended device list (idevice_get_device_list_extended).
#[repr(C)]
pub struct idevice_info {
    pub udid: *mut c_char,
    pub conn_type: c_int, // idevice_connection_type: CONNECTION_USBMUXD / CONNECTION_NETWORK
    pub conn_data: *mut c_void,
}
pub type idevice_info_t = *mut idevice_info;

// ---- constants ------------------------------------------------------------
pub const IDEVICE_E_SUCCESS: c_int = 0;
pub const LOCKDOWN_E_SUCCESS: c_int = 0;
pub const AFC_E_SUCCESS: c_int = 0;
pub const AFC_E_OBJECT_NOT_FOUND: c_int = 8;

pub const IDEVICE_LOOKUP_USBMUX: c_int = 1 << 1;
pub const IDEVICE_LOOKUP_NETWORK: c_int = 1 << 2;

pub const CONNECTION_USBMUXD: c_int = 1;
pub const CONNECTION_NETWORK: c_int = 2;

pub const AFC_FOPEN_RDONLY: c_int = 0x00000001;
pub const AFC_FOPEN_WRONLY: c_int = 0x00000003;

// ---- real FFI on unix (needs libimobiledevice) ----------------------------
#[cfg(not(windows))]
unsafe extern "C" {
    // idevice
    pub fn idevice_new(device: *mut idevice_t, udid: *const c_char) -> c_int;
    pub fn idevice_new_with_options(device: *mut idevice_t, udid: *const c_char, options: c_int) -> c_int;
    pub fn idevice_free(device: idevice_t) -> c_int;
    pub fn idevice_get_udid(device: idevice_t, udid: *mut *mut c_char) -> c_int;
    pub fn idevice_get_device_version(device: idevice_t) -> u32;
    pub fn idevice_get_device_list_extended(devices: *mut idevice_info_t, count: *mut c_int) -> c_int;
    pub fn idevice_device_list_extended_free(devices: idevice_info_t) -> c_int;
    pub fn idevice_strerror(err: c_int) -> *const c_char;

    // lockdown
    pub fn lockdownd_client_new_with_handshake(
        device: idevice_t,
        client: *mut lockdownd_client_t,
        label: *const c_char,
    ) -> c_int;
    pub fn lockdownd_client_free(client: lockdownd_client_t) -> c_int;
    pub fn lockdownd_get_value(
        client: lockdownd_client_t,
        domain: *const c_char,
        key: *const c_char,
        value: *mut plist_t,
    ) -> c_int;
    pub fn lockdownd_start_service(
        client: lockdownd_client_t,
        identifier: *const c_char,
        service: *mut lockdownd_service_descriptor_t,
    ) -> c_int;
    pub fn lockdownd_service_descriptor_free(service: lockdownd_service_descriptor_t) -> c_int;
    pub fn lockdownd_get_device_name(client: lockdownd_client_t, device_name: *mut *mut c_char) -> c_int;
    pub fn lockdownd_strerror(err: c_int) -> *const c_char;

    // afc
    pub fn afc_client_start_service(device: idevice_t, client: *mut afc_client_t, label: *const c_char) -> c_int;
    pub fn afc_client_new(
        device: idevice_t,
        service: lockdownd_service_descriptor_t,
        client: *mut afc_client_t,
    ) -> c_int;
    pub fn afc_client_free(client: afc_client_t) -> c_int;
    pub fn afc_get_device_info(client: afc_client_t, device_information: *mut *mut *mut c_char) -> c_int;
    pub fn afc_read_directory(
        client: afc_client_t,
        path: *const c_char,
        directory_information: *mut *mut *mut c_char,
    ) -> c_int;
    pub fn afc_get_file_info(client: afc_client_t, path: *const c_char, file_information: *mut *mut *mut c_char) -> c_int;
    pub fn afc_file_open(client: afc_client_t, filename: *const c_char, file_mode: c_int, handle: *mut u64) -> c_int;
    pub fn afc_file_close(client: afc_client_t, handle: u64) -> c_int;
    pub fn afc_file_read(client: afc_client_t, handle: u64, data: *mut c_char, length: u32, bytes_read: *mut u32) -> c_int;
    pub fn afc_file_write(
        client: afc_client_t,
        handle: u64,
        data: *const c_char,
        length: u32,
        bytes_written: *mut u32,
    ) -> c_int;
    pub fn afc_file_seek(client: afc_client_t, handle: u64, offset: i64, whence: c_int) -> c_int;
    pub fn afc_remove_path(client: afc_client_t, path: *const c_char) -> c_int;
    pub fn afc_make_directory(client: afc_client_t, path: *const c_char) -> c_int;
    pub fn afc_rename_path(client: afc_client_t, from: *const c_char, to: *const c_char) -> c_int;
    pub fn afc_dictionary_free(dictionary: *mut *mut c_char) -> c_int;
    pub fn afc_strerror(err: c_int) -> *const c_char;

    // plist
    pub fn plist_free(node: plist_t);
    pub fn plist_get_string_val(node: plist_t, val: *mut *mut c_char);
    pub fn plist_get_uint_val(node: plist_t, val: *mut u64);
}

// ---- stub FFI on windows (no libimobiledevice, just build) ---------------
#[cfg(windows)]
pub unsafe fn idevice_new(_device: *mut idevice_t, _udid: *const c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn idevice_new_with_options(_device: *mut idevice_t, _udid: *const c_char, _options: c_int) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn idevice_free(_device: idevice_t) -> c_int { 0 }
#[cfg(windows)]
pub unsafe fn idevice_get_udid(_device: idevice_t, _udid: *mut *mut c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn idevice_get_device_version(_device: idevice_t) -> u32 { 0 }
#[cfg(windows)]
pub unsafe fn idevice_get_device_list_extended(_devices: *mut idevice_info_t, _count: *mut c_int) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn idevice_device_list_extended_free(_devices: idevice_info_t) -> c_int { 0 }
#[cfg(windows)]
pub unsafe fn idevice_strerror(_err: c_int) -> *const c_char { std::ptr::null() }
#[cfg(windows)]
pub unsafe fn lockdownd_client_new_with_handshake(_device: idevice_t, _client: *mut lockdownd_client_t, _label: *const c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn lockdownd_client_free(_client: lockdownd_client_t) -> c_int { 0 }
#[cfg(windows)]
pub unsafe fn lockdownd_get_value(_client: lockdownd_client_t, _domain: *const c_char, _key: *const c_char, _value: *mut plist_t) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn lockdownd_start_service(_client: lockdownd_client_t, _identifier: *const c_char, _service: *mut lockdownd_service_descriptor_t) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn lockdownd_service_descriptor_free(_service: lockdownd_service_descriptor_t) -> c_int { 0 }
#[cfg(windows)]
pub unsafe fn lockdownd_get_device_name(_client: lockdownd_client_t, _device_name: *mut *mut c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn lockdownd_strerror(_err: c_int) -> *const c_char { std::ptr::null() }
#[cfg(windows)]
pub unsafe fn afc_client_start_service(_device: idevice_t, _client: *mut afc_client_t, _label: *const c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_client_new(_device: idevice_t, _service: lockdownd_service_descriptor_t, _client: *mut afc_client_t) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_client_free(_client: afc_client_t) -> c_int { 0 }
#[cfg(windows)]
pub unsafe fn afc_get_device_info(_client: afc_client_t, _device_information: *mut *mut *mut c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_read_directory(_client: afc_client_t, _path: *const c_char, _directory_information: *mut *mut *mut c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_get_file_info(_client: afc_client_t, _path: *const c_char, _file_information: *mut *mut *mut c_char) -> c_int { 8 }
#[cfg(windows)]
pub unsafe fn afc_file_open(_client: afc_client_t, _filename: *const c_char, _file_mode: c_int, _handle: *mut u64) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_file_close(_client: afc_client_t, _handle: u64) -> c_int { 0 }
#[cfg(windows)]
pub unsafe fn afc_file_read(_client: afc_client_t, _handle: u64, _data: *mut c_char, _length: u32, _bytes_read: *mut u32) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_file_write(_client: afc_client_t, _handle: u64, _data: *const c_char, _length: u32, _bytes_written: *mut u32) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_file_seek(_client: afc_client_t, _handle: u64, _offset: i64, _whence: c_int) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_remove_path(_client: afc_client_t, _path: *const c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_make_directory(_client: afc_client_t, _path: *const c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_rename_path(_client: afc_client_t, _from: *const c_char, _to: *const c_char) -> c_int { 1 }
#[cfg(windows)]
pub unsafe fn afc_dictionary_free(_dictionary: *mut *mut c_char) -> c_int { 0 }
#[cfg(windows)]
pub unsafe fn afc_strerror(_err: c_int) -> *const c_char { std::ptr::null() }
#[cfg(windows)]
pub unsafe fn plist_free(_node: plist_t) {}
#[cfg(windows)]
pub unsafe fn plist_get_string_val(_node: plist_t, _val: *mut *mut c_char) {}
#[cfg(windows)]
pub unsafe fn plist_get_uint_val(_node: plist_t, _val: *mut u64) {}
