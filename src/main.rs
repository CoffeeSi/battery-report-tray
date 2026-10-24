#![windows_subsystem = "windows"]

use std::os::raw::c_void;

use windows::Win32::{
    Devices::DeviceAndDriverInstallation::{
        DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, GUID_DEVCLASS_BATTERY, SP_DEVICE_INTERFACE_DATA,
        SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList,
        SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    },
    Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HWND, LPARAM, LRESULT, WPARAM},
    Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    },
    System::{
        IO::DeviceIoControl,
        LibraryLoader::GetModuleHandleW,
        Power::{
            BATTERY_POWER_ON_LINE, BATTERY_STATUS, BATTERY_WAIT_STATUS, IOCTL_BATTERY_QUERY_STATUS,
            IOCTL_BATTERY_QUERY_TAG,
        },
    },
    UI::{
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
            Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HICON,
            IDI_APPLICATION, IMAGE_ICON, KillTimer, LR_DEFAULTSIZE, LR_SHARED, LoadIconW,
            LoadImageW, MSG, PostQuitMessage, RegisterClassW, SetTimer, TranslateMessage,
            WINDOW_EX_STYLE, WM_APP, WM_DESTROY, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_OVERLAPPED,
        },
    },
};
use windows_core::{PCWSTR, w};

const TIMER_ID: usize = 1;
const WM_TRAY_CALLBACK: u32 = WM_APP + 1;
static mut BATTERY_PATH: Option<Vec<u16>> = None;

unsafe fn get_battery_path() -> windows_core::Result<Vec<u16>> {
    unsafe {
        let hdevinfo = SetupDiGetClassDevsW(
            Some(&GUID_DEVCLASS_BATTERY),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )?;

        let mut iface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };

        SetupDiEnumDeviceInterfaces(hdevinfo, None, &GUID_DEVCLASS_BATTERY, 0, &mut iface_data)?;

        let mut required_size = 0u32;
        let _ = SetupDiGetDeviceInterfaceDetailW(
            hdevinfo,
            &iface_data as *const _,
            None,
            0,
            Some(&mut required_size as *mut u32),
            None,
        );

        let mut buffer = vec![0u8; required_size as usize];
        let detail = buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;

        #[cfg(target_pointer_width = "64")]
        let header_size = 8;

        (*detail).cbSize = header_size;

        SetupDiGetDeviceInterfaceDetailW(
            hdevinfo,
            &iface_data as *const _,
            Some(detail),
            required_size,
            None,
            None,
        )?;

        let _ = SetupDiDestroyDeviceInfoList(hdevinfo);

        let path_ptr = std::ptr::addr_of!((*detail).DevicePath) as *const u16;
        let mut len = 0;
        while *path_ptr.add(len) != 0 {
            len += 1;
        }

        let slice = std::slice::from_raw_parts(path_ptr, len + 1);
        Ok(slice.to_vec())
    }
}

unsafe fn query_battery_status(path: &[u16]) -> windows_core::Result<BATTERY_STATUS> {
    unsafe {
        let handle = CreateFileW(
            PCWSTR(path.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?;

        let timeout: u32 = 0;
        let mut battery_tag: u32 = 0;
        let mut bytes_returened = 0u32;

        DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_TAG,
            Some(&timeout as *const _ as *const c_void),
            size_of::<u32>() as u32,
            Some(&mut battery_tag as *mut _ as *mut c_void),
            size_of::<u32>() as u32,
            Some(&mut bytes_returened),
            None,
        )?;

        let wait_status = BATTERY_WAIT_STATUS {
            BatteryTag: battery_tag,
            Timeout: 0,
            PowerState: 0,
            LowCapacity: 0,
            HighCapacity: 0,
        };

        let mut status = BATTERY_STATUS::default();

        DeviceIoControl(
            handle,
            IOCTL_BATTERY_QUERY_STATUS,
            Some(&wait_status as *const _ as *const c_void),
            size_of::<BATTERY_WAIT_STATUS>() as u32,
            Some(&mut status as *mut _ as *mut c_void),
            size_of::<BATTERY_STATUS>() as u32,
            Some(&mut bytes_returened),
            None,
        )?;

        let _ = CloseHandle(handle);
        Ok(status)
    }
}

fn format_tooltip(status: &BATTERY_STATUS) -> [u16; 128] {
    let is_online = (status.PowerState & BATTERY_POWER_ON_LINE) != 0;
    let capacity_mwh = status.Capacity;
    let rate_mw = status.Rate;
    let rate_watts = rate_mw.abs() as f64 / 1000.0;

    let text = if is_online {
        format!(
            "Status: AC Connected\nCapacity: {} mWh\nCharge: {:.2} W",
            capacity_mwh, rate_watts
        )
    } else {
        let remaining_h = if rate_watts > 0.1 {
            format!("{:.1} h", capacity_mwh as f64 / rate_mw.abs() as f64)
        } else {
            "Calculating...".to_string()
        };

        format!(
            "Discharge Watts: {:.2} W\nRemaining Capacity: {} mWh\nRemaining Time: {}",
            rate_watts, capacity_mwh, remaining_h
        )
    };

    let mut tip = [0u16; 128];
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let copy_len = utf16.len().min(127);
    tip[..copy_len].copy_from_slice(&utf16[..copy_len]);
    tip
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_TIMER => {
                if let Some(ref path) = BATTERY_PATH {
                    if let Ok(status) = query_battery_status(path) {
                        let nid = NOTIFYICONDATAW {
                            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                            hWnd: hwnd,
                            uID: 1,
                            uFlags: NIF_TIP,
                            szTip: format_tooltip(&status),
                            ..Default::default()
                        };
                        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
                    }
                }
                LRESULT(0)
            }
            WM_TRAY_CALLBACK => {
                if lparam.0 as u32 == WM_RBUTTONUP {
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = KillTimer(Some(hwnd), TIMER_ID);
                let nid = NOTIFYICONDATAW {
                    cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: 1,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn main() -> windows_core::Result<()> {
    unsafe {
        BATTERY_PATH = get_battery_path().ok();

        let instance = GetModuleHandleW(None)?;
        let class_name = w!("BatteryMonitorTrayClass");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("Battery Monitor"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )?;

        let initial_tip = if let Some(ref path) = BATTERY_PATH {
            query_battery_status(path)
                .map(|s| format_tooltip(&s))
                .unwrap_or([0u16; 128])
        } else {
            [0u16; 128]
        };

        let h_icon = LoadImageW(
            Some(instance.into()),
            PCWSTR(1 as *const u16),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE | LR_SHARED,
        )
        .map(|handle| HICON(handle.0))
        .unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap_or_default());

        let nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_TIP | NIF_MESSAGE,
            uCallbackMessage: WM_TRAY_CALLBACK,
            hIcon: h_icon,
            szTip: initial_tip,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);

        SetTimer(Some(hwnd), TIMER_ID, 3000, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}
