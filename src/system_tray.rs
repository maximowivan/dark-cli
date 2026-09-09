use std::sync::atomic::AtomicBool;
use tray_item::{IconSource, TrayItem};

/// Флаг видимости консоли для межпоточного отслеживания
pub static CONSOLE_HIDDEN: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
pub mod window_control {
    use super::CONSOLE_HIDDEN;
    use std::sync::atomic::Ordering;

    unsafe extern "system" {
        fn GetConsoleWindow() -> isize;
        fn ShowWindow(h_wnd: isize, n_cmd_show: i32) -> i32;
        fn IsWindowVisible(h_wnd: isize) -> i32;
        fn SetForegroundWindow(h_wnd: isize) -> i32;
    }

    const SW_HIDE: i32 = 0;
    const SW_RESTORE: i32 = 9;

    /// Скрыть консольное окно Dark-CLI в трей
    pub fn hide_console() {
        unsafe {
            let hwnd = GetConsoleWindow();
            if hwnd != 0 {
                ShowWindow(hwnd, SW_HIDE);
                CONSOLE_HIDDEN.store(true, Ordering::SeqCst);
            }
        }
    }

    /// Восстановить консольное окно Dark-CLI на передний план
    pub fn show_console() {
        unsafe {
            let hwnd = GetConsoleWindow();
            if hwnd != 0 {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
                CONSOLE_HIDDEN.store(false, Ordering::SeqCst);
            }
        }
    }

    /// Переключить видимость консольного окна
    pub fn toggle_console() -> bool {
        unsafe {
            let hwnd = GetConsoleWindow();
            if hwnd != 0 {
                if IsWindowVisible(hwnd) != 0 {
                    ShowWindow(hwnd, SW_HIDE);
                    CONSOLE_HIDDEN.store(true, Ordering::SeqCst);
                    false
                } else {
                    ShowWindow(hwnd, SW_RESTORE);
                    SetForegroundWindow(hwnd);
                    CONSOLE_HIDDEN.store(false, Ordering::SeqCst);
                    true
                }
            } else {
                false
            }
        }
    }

    /// Проверить, видно ли сейчас консольное окно
    #[allow(dead_code)]
    pub fn is_console_visible() -> bool {
        unsafe {
            let hwnd = GetConsoleWindow();
            if hwnd != 0 {
                IsWindowVisible(hwnd) != 0
            } else {
                true
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod window_control {
    pub fn hide_console() {}
    pub fn show_console() {}
    pub fn toggle_console() -> bool { true }
    pub fn is_console_visible() -> bool { true }
}

/// Менеджер системного трея
pub struct TrayManager {
    _tray: TrayItem,
}

impl TrayManager {
    /// Инициализация системного трея Dark-CLI с интерактивным контекстным меню
    pub fn init() -> Result<Self, String> {
        let tray_res = TrayItem::new(
            "Dark-CLI: Zapret & TG Proxy",
            IconSource::Resource("tray-default"),
        );

        let mut tray = match tray_res {
            Ok(t) => t,
            Err(_) => {
                #[cfg(target_os = "windows")]
                {
                    let default_icon = unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::LoadIconW(
                            0,
                            windows_sys::Win32::UI::WindowsAndMessaging::IDI_APPLICATION,
                        )
                    };
                    TrayItem::new(
                        "Dark-CLI: Zapret & TG Proxy",
                        IconSource::RawIcon(default_icon),
                    )
                    .map_err(|e| format!("Не удалось инициализировать иконку в трее: {}", e))?
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Err("Трей поддерживается только на Windows".to_string());
                }
            }
        };

        // 1. Заголовок
        tray.add_label("⚡ Dark-CLI v1.0.0")
            .map_err(|e| e.to_string())?;

        // 2. Показать / Скрыть окно
        tray.add_menu_item("🖥 Показать / Скрыть окно", move || {
            window_control::toggle_console();
        })
        .map_err(|e| e.to_string())?;

        // 3. Zapret: Запуск службы
        tray.add_menu_item("⚡ Запустить Zapret", move || {
            let _ = std::process::Command::new("net")
                .args(["start", "zapret"])
                .output();
        })
        .map_err(|e| e.to_string())?;

        // 4. Zapret: Остановка службы
        tray.add_menu_item("🛑 Остановить Zapret", move || {
            let _ = std::process::Command::new("net")
                .args(["stop", "zapret"])
                .output();
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/FI", "IMAGENAME eq winws*"])
                .output();
        })
        .map_err(|e| e.to_string())?;

        // 5. Flowseal TG WS Proxy: Запуск
        tray.add_menu_item("✈️ Запустить TG Proxy", move || {
            let _ = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Start-Process -FilePath TgWsProxy_windows.exe -ErrorAction SilentlyContinue",
                ])
                .output();
        })
        .map_err(|e| e.to_string())?;

        // 6. Flowseal TG WS Proxy: Подключить в Telegram
        tray.add_menu_item("🔗 Подключить Telegram (1 клик)", move || {
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "tg://proxy?server=127.0.0.1&port=1443"])
                .output();
        })
        .map_err(|e| e.to_string())?;

        // 7. Flowseal TG WS Proxy: Остановка
        tray.add_menu_item("🛑 Остановить TG Proxy", move || {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/FI", "IMAGENAME eq TgWsProxy*"])
                .output();
        })
        .map_err(|e| e.to_string())?;

        // 8. Выход из программы
        tray.add_menu_item("❌ Выход из Dark-CLI", move || {
            std::process::exit(0);
        })
        .map_err(|e| e.to_string())?;

        Ok(Self { _tray: tray })
    }
}
