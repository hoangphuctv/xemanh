//! Writes Windows UserChoice registry entries so XemAnh becomes the default image
//! viewer during installation. Hash algorithm based on Mozilla Firefox's
//! WindowsUserChoice.cpp (MPL 2.0).

#![cfg(windows)]

use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

use md5::{Digest, Md5};
use winapi::shared::minwindef::{DWORD, FILETIME, HKEY};
use winapi::shared::sddl::ConvertSidToStringSidW;
use winapi::um::minwinbase::SYSTEMTIME;
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
use winapi::um::winbase::LocalFree;
use winapi::um::winnt::{TokenUser, HANDLE, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, TOKEN_QUERY};
use winapi::um::winreg::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegRenameKey, RegSetValueExW, HKEY_CURRENT_USER,
};

const ERROR_SUCCESS: i32 = 0;

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHChangeNotify(wEventId: i32, uFlags: u32, dwItem1: *mut std::ffi::c_void, dwItem2: *mut std::ffi::c_void);
}

const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
const SHCNF_IDLIST: u32 = 0;

const PROG_ID: &str = "XemAnh.Image";
const USER_EXPERIENCE: &str =
    "User Choice set via Windows User Experience {D18B6DD5-6124-4341-9318-804003BAFA0B}";

const EXTENSIONS: &[&str] = &[
    ".jpg", ".jpeg", ".jpe", ".jfif", ".png", ".bmp", ".dib", ".gif", ".tga",
];

fn main() {
    let mut failures = 0usize;
    for ext in EXTENSIONS {
        if !set_default_for_extension(ext) {
            eprintln!("failed to set default handler for {ext}");
            failures += 1;
        }
    }

    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, null_mut(), null_mut());
    }

    if failures > 0 {
        std::process::exit(1);
    }
}

fn set_default_for_extension(ext: &str) -> bool {
    let sid = match current_user_sid() {
        Some(sid) => sid,
        None => return false,
    };

    let mut timestamp = unsafe { std::mem::zeroed() };
    unsafe {
        winapi::um::sysinfoapi::GetSystemTime(&mut timestamp);
    }

    let hash = match generate_user_choice_hash(ext, &sid, PROG_ID, &timestamp) {
        Some(hash) => hash,
        None => return false,
    };

    write_user_choice(ext, PROG_ID, &hash)
}

fn current_user_sid() -> Option<String> {
    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }

        let mut size = 0u32;
        winapi::um::securitybaseapi::GetTokenInformation(
            token,
            TokenUser,
            null_mut(),
            0,
            &mut size,
        );
        let mut buffer = vec![0u8; size as usize];
        if winapi::um::securitybaseapi::GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        ) == 0
        {
            return None;
        }

        let user = &*(buffer.as_ptr() as *const winapi::um::winnt::TOKEN_USER);
        let mut sid_string = null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid_string) == 0 {
            return None;
        }

        let mut len = 0usize;
        while *sid_string.add(len) != 0 {
            len += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(sid_string, len));
        LocalFree(sid_string as _);
        Some(sid)
    }
}

fn generate_user_choice_hash(
    ext: &str,
    sid: &str,
    progid: &str,
    timestamp: &SYSTEMTIME,
) -> Option<String> {
    let formatted = format_user_choice_string(ext, sid, progid, timestamp)?;
    hash_string(&formatted)
}

fn format_user_choice_string(
    ext: &str,
    sid: &str,
    progid: &str,
    timestamp: &SYSTEMTIME,
) -> Option<String> {
    let mut ts = *timestamp;
    ts.wSecond = 0;
    ts.wMilliseconds = 0;

    let mut file_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    unsafe {
        if winapi::um::timezoneapi::SystemTimeToFileTime(&ts, &mut file_time) == 0 {
            return None;
        }
    }

    let mut value = format!(
        "{ext}{sid}{progid}{:08x}{:08x}{USER_EXPERIENCE}",
        file_time.dwHighDateTime, file_time.dwLowDateTime
    );
    value.make_ascii_lowercase();
    Some(value)
}

fn hash_string(input: &str) -> Option<String> {
    let input_wide: Vec<u16> = input.encode_utf16().chain([0]).collect();
    let input_bytes =
        unsafe { std::slice::from_raw_parts(input_wide.as_ptr().cast(), input_wide.len() * 2) };

    const DWORDS_PER_BLOCK: usize = 2;
    const BLOCK_SIZE: usize = size_of::<u32>() * DWORDS_PER_BLOCK;
    let block_count = input_bytes.len() / BLOCK_SIZE;
    if block_count == 0 {
        return None;
    }

    let digest = Md5::digest(input_bytes);
    let md5: [u32; 4] = [
        u32::from_le_bytes(digest[0..4].try_into().ok()?),
        u32::from_le_bytes(digest[4..8].try_into().ok()?),
        u32::from_le_bytes(digest[8..12].try_into().ok()?),
        u32::from_le_bytes(digest[12..16].try_into().ok()?),
    ];

    let C0S: [[u32; 5]; 2] = [
        [md5[0] | 1, 0xCF98_B111, 0x8708_5B9F, 0x12CE_B96D, 0x257E_1D83],
        [md5[1] | 1, 0xA274_16F5, 0xD383_96FF, 0x7C93_2B89, 0xBFA4_9F69],
    ];
    let C1S: [[u32; 5]; 2] = [
        [md5[0] | 1, 0xEF05_69FB, 0x689B_6B9F, 0x79F8_A395, 0xC3EF_EA97],
        [md5[1] | 1, 0xC317_13DB, 0xDDCD_1F0F, 0x59C3_AF2D, 0x35BD_1EC9],
    ];

    let mut h0 = 0u32;
    let mut h1 = 0u32;
    let mut h0_acc = 0u32;
    let mut h1_acc = 0u32;

    for block in 0..block_count {
        for j in 0..DWORDS_PER_BLOCK {
            let offset = (block * DWORDS_PER_BLOCK + j) * size_of::<u32>();
            let input = u32::from_le_bytes(input_bytes[offset..offset + 4].try_into().ok()?);
            let c0 = C0S[j];
            let c1 = C1S[j];

            h0 = h0.wrapping_add(input);
            h0 = h0.wrapping_mul(c0[0]);
            h0 = word_swap(h0).wrapping_mul(c0[1]);
            h0 = word_swap(h0).wrapping_mul(c0[2]);
            h0 = word_swap(h0).wrapping_mul(c0[3]);
            h0 = word_swap(h0).wrapping_mul(c0[4]);
            h0_acc = h0_acc.wrapping_add(h0);

            h1 = h1.wrapping_add(input);
            h1 = word_swap(h1).wrapping_mul(c1[1]).wrapping_add(h1.wrapping_mul(c1[0]));
            h1 = (h1 >> 16).wrapping_mul(c1[2]).wrapping_add(h1.wrapping_mul(c1[3]));
            h1 = word_swap(h1).wrapping_mul(c1[4]).wrapping_add(h1);
            h1_acc = h1_acc.wrapping_add(h1);
        }
    }

    let hash = [h0 ^ h1, h0_acc ^ h1_acc];
    let hash_bytes: [u8; 8] = [
        hash[0].to_le_bytes()[0],
        hash[0].to_le_bytes()[1],
        hash[0].to_le_bytes()[2],
        hash[0].to_le_bytes()[3],
        hash[1].to_le_bytes()[0],
        hash[1].to_le_bytes()[1],
        hash[1].to_le_bytes()[2],
        hash[1].to_le_bytes()[3],
    ];
    Some(base64_encode(&hash_bytes))
}

fn word_swap(v: u32) -> u32 {
    v.rotate_right(16)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut idx = 0;
    while idx < bytes.len() {
        let b0 = bytes[idx];
        let b1 = bytes.get(idx + 1).copied().unwrap_or(0);
        let b2 = bytes.get(idx + 2).copied().unwrap_or(0);

        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if idx + 1 < bytes.len() {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }

        if idx + 2 < bytes.len() {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }

        idx += 3;
    }
    out
}

fn write_user_choice(ext: &str, progid: &str, hash: &str) -> bool {
    let assoc_path = format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\{ext}"
    );
    let temp_name = format!("{{{:016x}}}", unsafe {
        winapi::um::sysinfoapi::GetTickCount64()
    });

    unsafe {
        let mut assoc_key: HKEY = null_mut();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            to_wide(&assoc_path).as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            null_mut(),
            &mut assoc_key,
            null_mut(),
        ) != ERROR_SUCCESS
        {
            return false;
        }

        RegDeleteTreeW(assoc_key, to_wide("UserChoice").as_ptr());
        RegDeleteTreeW(assoc_key, to_wide("UserChoiceNew").as_ptr());

        if RegRenameKey(assoc_key, null_mut(), to_wide(&temp_name).as_ptr()) != ERROR_SUCCESS {
            RegCloseKey(assoc_key);
            return false;
        }

        let renamed_path = format!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts\\{temp_name}"
        );
        RegCloseKey(assoc_key);

        let mut renamed_key: HKEY = null_mut();
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            to_wide(&renamed_path).as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            null_mut(),
            &mut renamed_key,
            null_mut(),
        ) != ERROR_SUCCESS
        {
            return false;
        }

        let mut user_choice_key: HKEY = null_mut();
        if RegCreateKeyExW(
            renamed_key,
            to_wide("UserChoice").as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            null_mut(),
            &mut user_choice_key,
            null_mut(),
        ) != ERROR_SUCCESS
        {
            RegCloseKey(renamed_key);
            return false;
        }

        let ok = set_sz_value(user_choice_key, "ProgId", progid)
            && set_sz_value(user_choice_key, "Hash", hash);

        RegCloseKey(user_choice_key);

        let rename_back_ok =
            RegRenameKey(renamed_key, null_mut(), to_wide(ext).as_ptr()) == ERROR_SUCCESS;
        RegCloseKey(renamed_key);

        ok && rename_back_ok
    }
}

unsafe fn set_sz_value(key: HKEY, name: &str, value: &str) -> bool {
    let wide_name = to_wide(name);
    let wide_value = to_wide(value);
    let byte_len = ((wide_value.len() - 1) * 2) as DWORD;
    unsafe {
        RegSetValueExW(
            key,
            wide_name.as_ptr(),
            0,
            REG_SZ,
            wide_value.as_ptr().cast(),
            byte_len,
        ) == ERROR_SUCCESS
    }
}

fn to_wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain([0]).collect()
}

#[cfg(not(windows))]
fn main() {}
