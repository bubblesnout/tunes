//! Steam overlay host (Windows).
//!
//! When Steam launches Tunes, it injects its overlay into `tunes.exe` and ties the overlay
//! to that process. But all of Tunes' drawing happens inside WebView2's own processes, so the
//! overlay in `tunes.exe` never sees a frame to draw on (you hear the overlay sound, nothing
//! appears).
//!
//! This creates a borderless, click-through, transparent window owned by the main window and
//! covering it exactly, and presents (empty, transparent) Direct3D 11 frames into it while
//! Tunes is the foreground app: ~5 fps when idle, every vsync while Steam is drawing (detected
//! by sampling a grid of pixels from each presented frame). Steam hooks those presents and draws the overlay
//! into this window, on top of Tunes. Transparency uses a layered-window colour key (black),
//! so the window is invisible and lets mouse input straight through to the app beneath.

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use windows::core::w;
    use windows::Win32::Foundation::{COLORREF, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D, D3D11_BOX,
        D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_MODE_DESC, DXGI_SAMPLE_DESC};
    use windows::Win32::Graphics::Dxgi::{
        IDXGISwapChain, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_SEQUENTIAL,
        DXGI_USAGE_RENDER_TARGET_OUTPUT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow, GetWindowRect,
        GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PeekMessageW, RegisterClassW, SetLayeredWindowAttributes,
        SetWindowPos, ShowWindow, TranslateMessage, HWND_TOP, LWA_COLORKEY, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
        SW_HIDE, SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
    };

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        // WM_MOUSEACTIVATE → never take focus from the main window.
        const WM_MOUSEACTIVATE: u32 = 0x0021;
        const MA_NOACTIVATE: isize = 3;
        if msg == WM_MOUSEACTIVATE {
            return LRESULT(MA_NOACTIVATE);
        }
        unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
    }

    fn foreground_is_ours() -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return false;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            pid == std::process::id()
        }
    }

    /// Grid of points sampled after each present to see whether Steam drew anything.
    const GRID: u32 = 8;

    struct Gpu {
        swap: IDXGISwapChain,
        ctx: ID3D11DeviceContext,
        device: ID3D11Device,
        rtv: Option<ID3D11RenderTargetView>,
        back: Option<ID3D11Texture2D>,
        staging: ID3D11Texture2D,
        size: (u32, u32),
        sampled: bool,
    }

    impl Gpu {
        fn new(hwnd: HWND, w: u32, h: u32) -> windows::core::Result<Self> {
            let desc = DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC { Width: w, Height: h, Format: DXGI_FORMAT_B8G8R8A8_UNORM, ..Default::default() },
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 1,
                OutputWindow: hwnd,
                Windowed: true.into(),
                // Blt-model swap chain (works with colour-keyed layered windows). SEQUENTIAL keeps
                // the presented image in the buffer, so we can read back what Steam drew on it.
                SwapEffect: DXGI_SWAP_EFFECT_SEQUENTIAL,
                Flags: 0,
            };
            let mut swap = None;
            let mut device = None;
            let mut ctx = None;
            unsafe {
                D3D11CreateDeviceAndSwapChain(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&desc),
                    Some(&mut swap),
                    Some(&mut device),
                    None,
                    Some(&mut ctx),
                )?;
            }
            let device: ID3D11Device = device.unwrap();
            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: GRID,
                Height: GRID,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut staging = None;
            unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging))? };
            let mut g = Gpu {
                swap: swap.unwrap(),
                ctx: ctx.unwrap(),
                device,
                rtv: None,
                back: None,
                staging: staging.unwrap(),
                size: (w, h),
                sampled: false,
            };
            g.make_rtv()?;
            Ok(g)
        }

        fn make_rtv(&mut self) -> windows::core::Result<()> {
            unsafe {
                let back: ID3D11Texture2D = self.swap.GetBuffer(0)?;
                let mut rtv = None;
                self.device.CreateRenderTargetView(&back, None, Some(&mut rtv))?;
                self.rtv = rtv;
                self.back = Some(back);
            }
            Ok(())
        }

        fn resize(&mut self, w: u32, h: u32) -> windows::core::Result<()> {
            self.rtv = None;
            self.back = None;
            self.sampled = false;
            self.size = (w, h);
            unsafe {
                self.ctx.OMSetRenderTargets(None, None);
                self.swap.ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0))?;
            }
            self.make_rtv()
        }

        /// Whether Steam drew anything on the previous frame (read from last frame's samples).
        fn overlay_drew(&mut self) -> bool {
            if !self.sampled {
                return false;
            }
            self.sampled = false;
            unsafe {
                let mut map = D3D11_MAPPED_SUBRESOURCE::default();
                if self.ctx.Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut map)).is_err() {
                    return false;
                }
                let mut drew = false;
                for row in 0..GRID as usize {
                    let line = std::slice::from_raw_parts((map.pData as *const u8).add(row * map.RowPitch as usize), GRID as usize * 4);
                    // BGRA: anything brighter than the black colour key means Steam drew there.
                    if line.chunks_exact(4).any(|px| px[0] as u16 + px[1] as u16 + px[2] as u16 > 6) {
                        drew = true;
                        break;
                    }
                }
                self.ctx.Unmap(&self.staging, 0);
                drew
            }
        }

        /// Clear to the colour key (fully transparent), present, then queue a copy of a grid of
        /// sample points from what was presented (including anything Steam drew on it).
        fn frame(&mut self, vsync: bool) {
            unsafe {
                if let Some(rtv) = &self.rtv {
                    // Re-bind every frame: the overlay changes pipeline state when it draws.
                    self.ctx.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
                    self.ctx.ClearRenderTargetView(rtv, &[0.0, 0.0, 0.0, 1.0]);
                    // Debug builds: TUNES_FAKE_OVERLAY=1 paints grey, standing in for Steam
                    // drawing, to exercise the detection path without Steam installed.
                    #[cfg(debug_assertions)]
                    if std::env::var_os("TUNES_FAKE_OVERLAY").is_some() {
                        self.ctx.ClearRenderTargetView(rtv, &[0.3, 0.3, 0.3, 1.0]);
                    }
                }
                let _ = self.swap.Present(u32::from(vsync), DXGI_PRESENT(0));
                if let Some(back) = &self.back {
                    let (w, h) = self.size;
                    for gy in 0..GRID {
                        for gx in 0..GRID {
                            let x = (w * (2 * gx + 1)) / (2 * GRID);
                            let y = (h * (2 * gy + 1)) / (2 * GRID);
                            let bx = D3D11_BOX { left: x, top: y, front: 0, right: x + 1, bottom: y + 1, back: 1 };
                            self.ctx.CopySubresourceRegion(&self.staging, 0, gx, gy, 0, back, 0, Some(&bx));
                        }
                    }
                    self.sampled = true;
                }
            }
        }
    }

    pub fn start(main: isize, stop: Arc<AtomicBool>) {
        std::thread::Builder::new()
            .name("tunes-overlay-host".into())
            .spawn(move || {
                if let Err(e) = run(HWND(main as *mut _), &stop) {
                    log::error!("Steam overlay host failed: {e}");
                }
            })
            .expect("spawn overlay host");
    }

    fn rect_of(hwnd: HWND) -> RECT {
        let mut r = RECT::default();
        unsafe {
            let _ = GetWindowRect(hwnd, &mut r);
        }
        r
    }

    fn run(main: HWND, stop: &AtomicBool) -> windows::core::Result<()> {
        unsafe {
            let hinst = GetModuleHandleW(None)?;
            let class = w!("TunesSteamOverlayHost");
            let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst.into(), lpszClassName: class, ..Default::default() };
            RegisterClassW(&wc);

            let r = rect_of(main);
            let (mut w, mut h) = ((r.right - r.left).max(1), (r.bottom - r.top).max(1));
            // Owned popup (owner = main window): always stacked just above Tunes, never above other apps.
            let host = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class,
                w!("Tunes overlay host"),
                WS_POPUP,
                r.left,
                r.top,
                w,
                h,
                Some(main),
                None,
                Some(hinst.into()),
                None,
            )?;
            SetLayeredWindowAttributes(host, COLORREF(0), 0, LWA_COLORKEY)?;
            let mut gpu = Gpu::new(host, w as u32, h as u32)?;
            log::info!("Steam overlay host ready ({w}x{h})");

            let mut shown = false;
            let mut last = r;
            let mut full_rate_until = Instant::now();
            let mut was_full = false;
            let mut msg = MSG::default();
            while !stop.load(Ordering::Relaxed) && IsWindow(Some(main)).as_bool() {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                let active = foreground_is_ours() && IsWindowVisible(main).as_bool() && !IsIconic(main).as_bool();
                if !active {
                    if shown {
                        let _ = ShowWindow(host, SW_HIDE);
                        shown = false;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                let r = rect_of(main);
                if r != last || !shown {
                    last = r;
                    let (nw, nh) = ((r.right - r.left).max(1), (r.bottom - r.top).max(1));
                    let _ = SetWindowPos(host, Some(HWND_TOP), r.left, r.top, nw, nh, SWP_NOACTIVATE | SWP_NOOWNERZORDER);
                    if (nw, nh) != (w, h) {
                        (w, h) = (nw, nh);
                        gpu.resize(w as u32, h as u32)?;
                    }
                }
                if !shown {
                    let _ = ShowWindow(host, SW_SHOWNOACTIVATE);
                    shown = true;
                }
                // Idle at ~5 fps (the overlay starts appearing within ~0.2s); once Steam is
                // drawing, present every vsync until it has been clear for a second.
                if gpu.overlay_drew() {
                    full_rate_until = Instant::now() + Duration::from_secs(1);
                }
                let full = Instant::now() < full_rate_until;
                if full != was_full {
                    log::info!("Steam overlay {}", if full { "drawing: full frame rate" } else { "idle: low frame rate" });
                    was_full = full;
                }
                gpu.frame(full);
                if !full {
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
            let _ = DestroyWindow(host);
        }
        Ok(())
    }
}

/// Start the overlay host over the main window (Windows only).
#[cfg(windows)]
pub fn start(main_hwnd: isize) {
    imp::start(main_hwnd, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
}

#[cfg(not(windows))]
pub fn start(_main_hwnd: isize) {}
