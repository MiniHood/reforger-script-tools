//! Host-process Workbench window capture.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const DEFAULT_MAX_DIMENSION: u32 = 1_920;
pub const MIN_MAX_DIMENSION: u32 = 512;
pub const MAX_MAX_DIMENSION: u32 = 4_096;
pub const MAX_ENCODED_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRegion {
    #[schemars(range(min = 0.0, max = 1.0))]
    pub x: f64,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub y: f64,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub width: f64,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub height: f64,
}

impl CaptureRegion {
    pub fn to_pixels(
        self,
        source_width: u32,
        source_height: u32,
    ) -> Result<PixelRegion, CaptureError> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.width.is_finite()
            || !self.height.is_finite()
            || self.x < 0.0
            || self.y < 0.0
            || self.width <= 0.0
            || self.height <= 0.0
            || self.x + self.width > 1.0
            || self.y + self.height > 1.0
        {
            return Err(CaptureError::InvalidRegion);
        }

        let left = (self.x * f64::from(source_width)).floor() as u32;
        let top = (self.y * f64::from(source_height)).floor() as u32;
        let right = ((self.x + self.width) * f64::from(source_width)).ceil() as u32;
        let bottom = ((self.y + self.height) * f64::from(source_height)).ceil() as u32;
        let right = right.min(source_width);
        let bottom = bottom.min(source_height);
        if left >= right || top >= bottom {
            return Err(CaptureError::InvalidRegion);
        }

        Ok(PixelRegion {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchWindow {
    pub window_id: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
    pub minimized: bool,
    pub foreground: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchWindowList {
    pub process_id: u32,
    pub windows: Vec<WorkbenchWindow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapturedWindow {
    pub process_id: u32,
    pub window: WorkbenchWindow,
    pub source_width: u32,
    pub source_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub region: Option<CaptureRegion>,
    pub scale_milli: u32,
    pub png: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
    Unsupported,
    NoWindow,
    InvalidWindowId,
    Minimized,
    InvalidRegion,
    NativeCapture,
    Encoding,
    TooLarge,
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unsupported => "window capture is unsupported on this platform",
            Self::NoWindow => "no eligible Workbench window was found",
            Self::InvalidWindowId => "the selected Workbench window is unavailable",
            Self::Minimized => "the selected Workbench window is minimized",
            Self::InvalidRegion => "the capture region is invalid",
            Self::NativeCapture => "Windows could not render the selected Workbench window",
            Self::Encoding => "the captured Workbench window could not be encoded",
            Self::TooLarge => "the encoded Workbench screenshot exceeded the size limit",
        })
    }
}

pub fn bounded_dimensions(width: u32, height: u32, max_dimension: u32) -> (u32, u32) {
    if width == 0 || height == 0 || max_dimension == 0 || width.max(height) <= max_dimension {
        return (width, height);
    }

    if width >= height {
        (
            max_dimension,
            ((u64::from(height) * u64::from(max_dimension)) / u64::from(width)).max(1) as u32,
        )
    } else {
        (
            ((u64::from(width) * u64::from(max_dimension)) / u64::from(height)).max(1) as u32,
            max_dimension,
        )
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::mem::size_of;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetWindowDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, PW_RENDERFULLCONTENT,
    };

    struct WindowEnumeration {
        process_id: u32,
        windows: Vec<WorkbenchWindow>,
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, parameter: LPARAM) -> BOOL {
        let context = &mut *(parameter as *mut WindowEnumeration);
        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id != context.process_id || IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(hwnd, &mut rect) == 0 || rect.right <= rect.left || rect.bottom <= rect.top
        {
            return 1;
        }

        let title_length = GetWindowTextLengthW(hwnd);
        let mut title = vec![0u16; title_length.saturating_add(1) as usize];
        let title_length = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
        title.truncate(title_length.max(0) as usize);
        context.windows.push(WorkbenchWindow {
            window_id: window_id(hwnd),
            title: String::from_utf16_lossy(&title),
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
            visible: true,
            minimized: IsIconic(hwnd) != 0,
            foreground: false,
        });
        1
    }

    pub fn list_windows(process_id: u32) -> Result<Vec<WorkbenchWindow>, CaptureError> {
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let mut context = WindowEnumeration {
            process_id,
            windows: Vec::new(),
        };
        unsafe {
            EnumWindows(
                Some(enum_windows_callback),
                (&mut context as *mut WindowEnumeration) as LPARAM,
            );
            let foreground = GetForegroundWindow();
            for window in &mut context.windows {
                window.foreground = parse_window_id(&window.window_id) == Some(foreground);
            }
        }
        context.windows.sort_by(|left, right| {
            right
                .foreground
                .cmp(&left.foreground)
                .then_with(|| {
                    right
                        .width
                        .saturating_mul(right.height)
                        .cmp(&left.width.saturating_mul(left.height))
                })
                .then_with(|| left.window_id.cmp(&right.window_id))
        });
        Ok(context.windows)
    }

    pub fn capture_window(
        process_id: u32,
        window_id_input: Option<&str>,
        max_dimension: u32,
        region: Option<CaptureRegion>,
    ) -> Result<CapturedWindow, CaptureError> {
        let windows = list_windows(process_id)?;
        let window = match window_id_input {
            Some(window_id_input) => windows
                .iter()
                .find(|window| window.window_id == window_id_input)
                .cloned()
                .ok_or(CaptureError::InvalidWindowId)?,
            None => windows
                .iter()
                .find(|window| window.foreground)
                .or_else(|| windows.first())
                .cloned()
                .ok_or(CaptureError::NoWindow)?,
        };
        if window.minimized {
            return Err(CaptureError::Minimized);
        }

        let hwnd = parse_window_id(&window.window_id).ok_or(CaptureError::InvalidWindowId)?;
        let pixels = capture_native(process_id, hwnd, window.width, window.height)?;
        let source_width = window.width;
        let source_height = window.height;
        let pixel_region = region
            .map(|region| region.to_pixels(source_width, source_height))
            .transpose()?
            .unwrap_or(PixelRegion {
                x: 0,
                y: 0,
                width: source_width,
                height: source_height,
            });
        let cropped = crop_rgba(&pixels, source_width, pixel_region);
        let (output_width, output_height) =
            bounded_dimensions(pixel_region.width, pixel_region.height, max_dimension);
        let resized = resize_rgba(
            &cropped,
            pixel_region.width,
            pixel_region.height,
            output_width,
            output_height,
        );
        let png = encode_png(&resized, output_width, output_height)?;
        if png.len() > MAX_ENCODED_BYTES {
            return Err(CaptureError::TooLarge);
        }

        Ok(CapturedWindow {
            process_id,
            window,
            source_width,
            source_height,
            output_width,
            output_height,
            region,
            scale_milli: ((output_width as f64 / pixel_region.width as f64) * 1_000.0) as u32,
            png,
        })
    }

    fn window_id(hwnd: HWND) -> String {
        format!("hwnd-{hwnd:016x}", hwnd = hwnd as usize)
    }

    fn parse_window_id(value: &str) -> Option<HWND> {
        let value = value.strip_prefix("hwnd-")?;
        usize::from_str_radix(value, 16)
            .ok()
            .map(|value| value as HWND)
    }

    fn capture_native(
        process_id: u32,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, CaptureError> {
        let pixel_count = (width as usize)
            .checked_mul(height as usize)
            .and_then(|value| value.checked_mul(4))
            .ok_or(CaptureError::NativeCapture)?;
        let mut owner_process_id = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner_process_id) };
        if owner_process_id != process_id {
            return Err(CaptureError::InvalidWindowId);
        }
        let window_dc = unsafe { GetWindowDC(hwnd) };
        if window_dc.is_null() {
            return Err(CaptureError::NativeCapture);
        }
        let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
        if memory_dc.is_null() {
            unsafe { ReleaseDC(hwnd, window_dc) };
            return Err(CaptureError::NativeCapture);
        }

        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [windows_sys::Win32::Graphics::Gdi::RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        let mut bits = null_mut();
        let bitmap = unsafe {
            CreateDIBSection(
                memory_dc,
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut bits,
                null_mut(),
                0,
            )
        };
        if bitmap.is_null() || bits.is_null() {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(hwnd, window_dc);
            }
            return Err(CaptureError::NativeCapture);
        }
        let previous = unsafe { SelectObject(memory_dc, bitmap) };
        let mut owner_process_id = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner_process_id) };
        if owner_process_id != process_id {
            unsafe {
                SelectObject(memory_dc, previous);
                DeleteObject(bitmap);
                DeleteDC(memory_dc);
                ReleaseDC(hwnd, window_dc);
            }
            return Err(CaptureError::InvalidWindowId);
        }
        let rendered = unsafe { PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT) != 0 };
        if !rendered {
            unsafe {
                SelectObject(memory_dc, previous);
                DeleteObject(bitmap);
                DeleteDC(memory_dc);
                ReleaseDC(hwnd, window_dc);
            }
            return Err(CaptureError::NativeCapture);
        }

        let source = unsafe { std::slice::from_raw_parts(bits as *const u8, pixel_count) };
        let mut rgba = Vec::with_capacity(pixel_count);
        for pixel in source.chunks_exact(4) {
            rgba.extend([pixel[2], pixel[1], pixel[0], 255]);
        }
        unsafe {
            SelectObject(memory_dc, previous);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
        }
        Ok(rgba)
    }

    fn crop_rgba(source: &[u8], source_width: u32, region: PixelRegion) -> Vec<u8> {
        let row_bytes = region.width as usize * 4;
        let mut output = vec![0; row_bytes * region.height as usize];
        for row in 0..region.height as usize {
            let source_start =
                ((region.y as usize + row) * source_width as usize + region.x as usize) * 4;
            let output_start = row * row_bytes;
            output[output_start..output_start + row_bytes]
                .copy_from_slice(&source[source_start..source_start + row_bytes]);
        }
        output
    }

    fn resize_rgba(
        source: &[u8],
        source_width: u32,
        source_height: u32,
        output_width: u32,
        output_height: u32,
    ) -> Vec<u8> {
        if source_width == output_width && source_height == output_height {
            return source.to_vec();
        }
        let mut output = vec![0; output_width as usize * output_height as usize * 4];
        for y in 0..output_height {
            let source_y =
                ((y as f64 + 0.5) * source_height as f64 / output_height as f64 - 0.5).max(0.0);
            let y0 = source_y.floor() as u32;
            let y1 = (y0 + 1).min(source_height - 1);
            let y_weight = source_y - y0 as f64;
            for x in 0..output_width {
                let source_x =
                    ((x as f64 + 0.5) * source_width as f64 / output_width as f64 - 0.5).max(0.0);
                let x0 = source_x.floor() as u32;
                let x1 = (x0 + 1).min(source_width - 1);
                let x_weight = source_x - x0 as f64;
                let output_offset = (y * output_width + x) as usize * 4;
                for channel in 0..4 {
                    let top_left = source[(y0 * source_width + x0) as usize * 4 + channel] as f64;
                    let top_right = source[(y0 * source_width + x1) as usize * 4 + channel] as f64;
                    let bottom_left =
                        source[(y1 * source_width + x0) as usize * 4 + channel] as f64;
                    let bottom_right =
                        source[(y1 * source_width + x1) as usize * 4 + channel] as f64;
                    let top = top_left + (top_right - top_left) * x_weight;
                    let bottom = bottom_left + (bottom_right - bottom_left) * x_weight;
                    output[output_offset + channel] =
                        (top + (bottom - top) * y_weight).round() as u8;
                }
            }
        }
        output
    }

    fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, CaptureError> {
        let mut encoded = Vec::new();
        let mut encoder = png::Encoder::new(&mut encoded, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| CaptureError::Encoding)?;
        writer
            .write_image_data(rgba)
            .map_err(|_| CaptureError::Encoding)?;
        drop(writer);
        Ok(encoded)
    }
}

#[cfg(windows)]
pub use windows_impl::{capture_window, list_windows};

#[cfg(not(windows))]
pub fn list_windows(_process_id: u32) -> Result<Vec<WorkbenchWindow>, CaptureError> {
    Err(CaptureError::Unsupported)
}

#[cfg(not(windows))]
pub fn capture_window(
    _process_id: u32,
    _window_id: Option<&str>,
    _max_dimension: u32,
    _region: Option<CaptureRegion>,
) -> Result<CapturedWindow, CaptureError> {
    Err(CaptureError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::{bounded_dimensions, CaptureRegion, DEFAULT_MAX_DIMENSION};

    #[test]
    fn normalized_region_maps_to_native_pixels_without_reusing_overview_pixels() {
        let region = CaptureRegion {
            x: 0.25,
            y: 0.125,
            width: 0.5,
            height: 0.25,
        };

        assert_eq!(
            region.to_pixels(3840, 2160).unwrap(),
            super::PixelRegion {
                x: 960,
                y: 270,
                width: 1920,
                height: 540,
            }
        );
    }

    #[test]
    fn full_window_dimensions_are_bounded_by_the_requested_long_edge() {
        assert_eq!(bounded_dimensions(3840, 2160, 2560), (2560, 1440));
        assert_eq!(bounded_dimensions(1280, 720, 2560), (1280, 720));
        assert_eq!(
            bounded_dimensions(3840, 2160, DEFAULT_MAX_DIMENSION),
            (1920, 1080)
        );
    }

    #[test]
    fn invalid_regions_are_rejected_instead_of_clamped() {
        let region = CaptureRegion {
            x: 0.9,
            y: 0.1,
            width: 0.2,
            height: 0.2,
        };

        assert!(region.to_pixels(3840, 2160).is_err());
    }
}
