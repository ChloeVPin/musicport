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

// ---- functions ------------------------------------------------------------
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
