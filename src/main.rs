#![windows_subsystem = "windows"]

mod desktop;
mod i18n;
mod review;
mod store;
mod window_state;

use desktop::ComGuard;
use native_windows_derive::NwgUi;
use native_windows_gui as nwg;
use nwg::NativeUi;
use rust_i18n::t;
use std::cell::RefCell;
use store::IconEntry;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, IDC_HAND, LoadCursorW, SetCursor, SM_CMONITORS, SW_SHOWNORMAL, WM_SETCURSOR,
};
use windows::core::{PCWSTR, w};

rust_i18n::i18n!("locales", fallback = "en");

const APP_ICON: &[u8] = include_bytes!("../monitor.ico");
const ABOUT_URL: &str = "https://diyers.com.br/pt/2026/10/18/ice-iconsaver/";

#[derive(Default, NwgUi)]
pub struct IceIconSaverApp {
    #[nwg_resource(source_bin: Some(APP_ICON))]
    app_icon: nwg::Icon,

    #[nwg_control(
        size: (720, 480),
        center: true,
        title: "IceIconSaver",
        flags: "WINDOW|VISIBLE|RESIZABLE",
        icon: Some(&data.app_icon)
    )]
    #[nwg_events(
        OnWindowClose: [IceIconSaverApp::exit],
        OnInit: [IceIconSaverApp::on_init]
    )]
    window: nwg::Window,

    #[nwg_control(parent: window, text: "&File")]
    file_menu: nwg::Menu,

    #[nwg_control(parent: file_menu, text: "&Load")]
    #[nwg_events(OnMenuItemSelected: [IceIconSaverApp::on_load])]
    file_load_item: nwg::MenuItem,

    #[nwg_control(parent: file_menu, text: "&Save")]
    #[nwg_events(OnMenuItemSelected: [IceIconSaverApp::on_save])]
    file_save_item: nwg::MenuItem,

    #[nwg_control(parent: file_menu)]
    file_separator: nwg::MenuSeparator,

    #[nwg_control(parent: file_menu, text: "E&xit")]
    #[nwg_events(OnMenuItemSelected: [IceIconSaverApp::exit])]
    file_exit_item: nwg::MenuItem,

    #[nwg_control(parent: window, text: "&Help")]
    help_menu: nwg::Menu,

    #[nwg_control(parent: help_menu, text: "&Language")]
    language_menu: nwg::Menu,

    #[nwg_control(parent: language_menu, text: "&English")]
    #[nwg_events(OnMenuItemSelected: [IceIconSaverApp::on_lang_en])]
    lang_en_item: nwg::MenuItem,

    #[nwg_control(parent: language_menu, text: "&Português (Brasil)")]
    #[nwg_events(OnMenuItemSelected: [IceIconSaverApp::on_lang_pt])]
    lang_pt_item: nwg::MenuItem,

    #[nwg_control(parent: help_menu, text: "&About")]
    #[nwg_events(OnMenuItemSelected: [IceIconSaverApp::on_about])]
    help_about_item: nwg::MenuItem,

    #[nwg_control(
        size: (520, 260),
        center: true,
        title: "About Ice IconSaver",
        flags: "WINDOW",
        icon: Some(&data.app_icon)
    )]
    #[nwg_events(OnWindowClose: [IceIconSaverApp::close_about(SELF, EVT_DATA)])]
    about_window: nwg::Window,

    #[nwg_control(parent: about_window, text: "", position: (20, 20), size: (470, 24))]
    about_name: nwg::Label,

    #[nwg_control(parent: about_window, text: "", position: (20, 52), size: (470, 24))]
    about_version: nwg::Label,

    #[nwg_control(parent: about_window, text: "", position: (20, 84), size: (470, 24))]
    about_author: nwg::Label,

    #[nwg_control(parent: about_window, text: "", position: (20, 116), size: (470, 24))]
    about_more_label: nwg::Label,

    #[nwg_control(
        parent: about_window,
        text: "https://diyers.com.br/pt/2026/10/18/ice-iconsaver/",
        position: (20, 144),
        size: (470, 24)
    )]
    #[nwg_events(OnLabelClick: [IceIconSaverApp::open_about_url])]
    about_url: nwg::Label,

    #[nwg_control(parent: about_window, text: "OK", position: (210, 190), size: (90, 32))]
    #[nwg_events(OnButtonClick: [IceIconSaverApp::hide_about])]
    about_ok: nwg::Button,

    #[nwg_layout(parent: window, spacing: 8, margin: [10, 10, 10, 10])]
    layout: nwg::GridLayout,

    #[nwg_control(
        parent: window,
        list_style: nwg::ListViewStyle::Detailed,
        ex_flags: nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID
    )]
    #[nwg_layout_item(layout: layout, col: 0, row: 0, col_span: 2, row_span: 8)]
    icon_list: nwg::ListView,

    #[nwg_control(parent: window, text: "Load")]
    #[nwg_layout_item(layout: layout, col: 0, row: 8)]
    #[nwg_events(OnButtonClick: [IceIconSaverApp::on_load])]
    load_btn: nwg::Button,

    #[nwg_control(parent: window, text: "Save")]
    #[nwg_layout_item(layout: layout, col: 1, row: 8)]
    #[nwg_events(OnButtonClick: [IceIconSaverApp::on_save])]
    save_btn: nwg::Button,

    #[nwg_resource(
        title: "Save desktop icons",
        action: nwg::FileDialogAction::Save,
        filters: "Desktop Icons(*.deskicons)"
    )]
    save_dialog: nwg::FileDialog,

    #[nwg_resource(
        title: "Load desktop icons",
        action: nwg::FileDialogAction::Open,
        filters: "Desktop Icons(*.deskicons)"
    )]
    open_dialog: nwg::FileDialog,

    status_rows: RefCell<Vec<(IconEntry, String)>>,
    about_url_handler: RefCell<Option<nwg::RawEventHandler>>,
    columns_ready: RefCell<bool>,
}

impl IceIconSaverApp {
    fn on_init(&self) {
        let list = &self.icon_list;
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(0),
            fmt: None,
            width: Some(280),
            text: Some(String::new()),
        });
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(1),
            fmt: None,
            width: Some(80),
            text: Some(String::new()),
        });
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(2),
            fmt: None,
            width: Some(80),
            text: Some(String::new()),
        });
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(3),
            fmt: None,
            width: Some(120),
            text: Some(String::new()),
        });
        list.set_headers_enabled(true);
        *self.columns_ready.borrow_mut() = true;

        if let Ok(handler) = nwg::bind_raw_event_handler(
            &self.about_url.handle,
            0x10001,
            move |_hwnd, msg, _w, _l| {
                if msg == WM_SETCURSOR {
                    unsafe {
                        if let Ok(cursor) = LoadCursorW(None, IDC_HAND) {
                            let _ = SetCursor(cursor);
                        }
                    }
                    return Some(1);
                }
                None
            },
        ) {
            *self.about_url_handler.borrow_mut() = Some(handler);
        }

        if let Some(state) = window_state::load() {
            if let Some(hwnd) = window_state::hwnd_from_nwg(&self.window) {
                let _ = window_state::apply_to_hwnd(hwnd, &state);
            }
        }

        self.refresh_ui_strings();

        match desktop::list_desktop_icons() {
            Ok(mut icons) => {
                icons.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                *self.status_rows.borrow_mut() = icons
                    .into_iter()
                    .map(|entry| (entry, String::new()))
                    .collect();
                self.refresh_main_list();
            }
            Err(e) => {
                self.show_error(&e);
            }
        }
    }

    fn refresh_ui_strings(&self) {
        let monitor_count = unsafe { GetSystemMetrics(SM_CMONITORS) };
        let app = t!("app_name");
        let title = if monitor_count <= 1 {
            t!("title_single_monitor", app = app.as_ref()).to_string()
        } else {
            t!(
                "title_multi_monitor",
                app = app.as_ref(),
                count = monitor_count
            )
            .to_string()
        };
        self.window.set_text(&title);

        i18n::set_menu_bar_text(&self.file_menu, &t!("menu_file"));
        i18n::set_menu_item_text(&self.file_load_item, &t!("menu_load"));
        i18n::set_menu_item_text(&self.file_save_item, &t!("menu_save"));
        i18n::set_menu_item_text(&self.file_exit_item, &t!("menu_exit"));
        i18n::set_menu_bar_text(&self.help_menu, &t!("menu_help"));
        i18n::set_menu_bar_text(&self.language_menu, &t!("menu_language"));
        i18n::set_menu_item_text(&self.lang_en_item, &t!("menu_lang_en"));
        i18n::set_menu_item_text(&self.lang_pt_item, &t!("menu_lang_pt"));
        i18n::set_menu_item_text(&self.help_about_item, &t!("menu_about"));

        let locale = i18n::current_locale();
        self.lang_en_item.set_checked(locale == i18n::LOCALE_EN);
        self.lang_pt_item.set_checked(locale == i18n::LOCALE_PT_BR);

        self.load_btn.set_text(&t!("btn_load"));
        self.save_btn.set_text(&t!("btn_save"));

        if *self.columns_ready.borrow() {
            self.icon_list.update_column(
                0,
                nwg::InsertListViewColumn {
                    index: Some(0),
                    fmt: None,
                    width: None,
                    text: Some(t!("col_name").to_string()),
                },
            );
            self.icon_list.update_column(
                1,
                nwg::InsertListViewColumn {
                    index: Some(1),
                    fmt: None,
                    width: None,
                    text: Some(t!("col_x").to_string()),
                },
            );
            self.icon_list.update_column(
                2,
                nwg::InsertListViewColumn {
                    index: Some(2),
                    fmt: None,
                    width: None,
                    text: Some(t!("col_y").to_string()),
                },
            );
            self.icon_list.update_column(
                3,
                nwg::InsertListViewColumn {
                    index: Some(3),
                    fmt: None,
                    width: None,
                    text: Some(t!("col_status").to_string()),
                },
            );
        }

        self.save_dialog.set_title(&t!("dialog_save_title"));
        let _ = self.save_dialog.set_filters(&t!("dialog_save_filter"));
        self.open_dialog.set_title(&t!("dialog_load_title"));
        let _ = self.open_dialog.set_filters(&t!("dialog_load_filter"));

        self.about_window.set_text(&t!("about_title"));
        self.about_name.set_text(&t!("about_name"));
        self.about_version.set_text(&t!(
            "about_version",
            version = env!("CARGO_PKG_VERSION")
        ));
        self.about_author.set_text(&t!("about_author"));
        self.about_more_label.set_text(&t!("about_more"));
        self.about_ok.set_text(&t!("btn_ok"));

        self.refresh_main_list();
    }

    fn set_language(&self, locale: &str) {
        i18n::apply_locale(locale);
        let _ = i18n::save_locale(locale);
        self.refresh_ui_strings();
    }

    fn on_lang_en(&self) {
        self.set_language(i18n::LOCALE_EN);
    }

    fn on_lang_pt(&self) {
        self.set_language(i18n::LOCALE_PT_BR);
    }

    fn refresh_main_list(&self) {
        let list = &self.icon_list;
        list.clear();
        for (index, (entry, status)) in self.status_rows.borrow().iter().enumerate() {
            let status_text = i18n::display_status(status);
            list.insert_items_row(
                Some(index as i32),
                &[
                    entry.name.as_str(),
                    &entry.x.to_string(),
                    &entry.y.to_string(),
                    status_text.as_str(),
                ],
            );
        }
    }

    fn show_error(&self, message: &str) {
        nwg::modal_error_message(&self.window, &t!("app_name"), message);
    }

    fn show_info(&self, message: &str) {
        nwg::modal_info_message(&self.window, &t!("app_name"), message);
    }

    fn on_about(&self) {
        self.window.set_enabled(false);
        self.about_window.set_visible(true);
        self.about_window.set_focus();
    }

    fn hide_about(&self) {
        self.about_window.set_visible(false);
        self.window.set_enabled(true);
        self.window.set_focus();
    }

    fn close_about(&self, data: &nwg::EventData) {
        if let nwg::EventData::OnWindowClose(close) = data {
            close.close(false);
        }
        self.hide_about();
    }

    fn open_about_url(&self) {
        let wide: Vec<u16> = ABOUT_URL
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            ShellExecuteW(
                HWND::default(),
                w!("open"),
                PCWSTR(wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
    }

    fn on_save(&self) {
        let review_items: Vec<(IconEntry, bool)> = self
            .status_rows
            .borrow()
            .iter()
            .map(|(entry, _)| (entry.clone(), false))
            .collect();

        if review_items.is_empty() {
            self.show_info(&t!("msg_no_icons_to_save"));
            return;
        }

        let locale = i18n::current_locale();
        self.window.set_enabled(false);
        let confirmed = review::run_review_dialog(
            &t!("review_save_title"),
            review_items,
            false,
            &locale,
        );
        self.window.set_enabled(true);
        self.window.set_focus();

        let Some(confirmed) = confirmed else {
            return;
        };

        let confirmed_entries: Vec<IconEntry> =
            confirmed.into_iter().map(|(entry, _)| entry).collect();

        let dir = match store::storage_dir() {
            Ok(dir) => dir,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };

        let dir_str = dir.to_string_lossy().to_string();
        if let Err(e) = self.save_dialog.set_default_folder(&dir_str) {
            self.show_error(&t!("err_open_save_folder", error = e.to_string()));
            return;
        }
        let _ = self.save_dialog.clear_client_data();

        if !self.save_dialog.run(Some(&self.window)) {
            return;
        }

        let selected = match self.save_dialog.get_selected_item() {
            Ok(path) => path,
            Err(e) => {
                self.show_error(&t!("err_read_path", error = e.to_string()));
                return;
            }
        };

        let path = store::ensure_deskicons_extension(std::path::PathBuf::from(selected));
        if let Err(e) = store::save_layout(&path, &confirmed_entries) {
            self.show_error(&e);
            return;
        }

        {
            let mut rows = self.status_rows.borrow_mut();
            for (entry, status) in rows.iter_mut() {
                if confirmed_entries
                    .iter()
                    .any(|saved| desktop::entries_match(entry, saved))
                {
                    *status = "SAVED".to_string();
                }
            }
        }
        self.refresh_main_list();
    }

    fn on_load(&self) {
        let dir = match store::storage_dir() {
            Ok(dir) => dir,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };

        let dir_str = dir.to_string_lossy().to_string();
        if let Err(e) = self.open_dialog.set_default_folder(&dir_str) {
            self.show_error(&t!("err_open_load_folder", error = e.to_string()));
            return;
        }
        let _ = self.open_dialog.clear_client_data();

        if !self.open_dialog.run(Some(&self.window)) {
            return;
        }

        let selected = match self.open_dialog.get_selected_item() {
            Ok(path) => path,
            Err(e) => {
                self.show_error(&t!("err_read_path", error = e.to_string()));
                return;
            }
        };

        let entries = match store::load_layout(std::path::Path::new(&selected)) {
            Ok(entries) => entries,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };

        let review_items = match desktop::entries_needing_restore(&entries) {
            Ok(items) => items,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };

        if review_items.is_empty() {
            self.show_info(&t!("msg_all_match"));
            return;
        }

        let locale = i18n::current_locale();
        self.window.set_enabled(false);
        let confirmed = review::run_review_dialog(
            &t!("review_load_title"),
            review_items,
            true,
            &locale,
        );
        self.window.set_enabled(true);
        self.window.set_focus();

        let Some(confirmed) = confirmed else {
            return;
        };

        match desktop::disable_auto_arrange_if_needed() {
            Ok(true) => {
                self.show_info(&t!("msg_auto_arrange"));
            }
            Ok(false) => {}
            Err(e) => {
                self.show_error(&e);
                return;
            }
        }

        let results = match desktop::apply_positions(&confirmed) {
            Ok(results) => results,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };

        {
            let mut rows = self.status_rows.borrow_mut();
            for result in results {
                if result.status != "Moved" {
                    continue;
                }
                if let Some((entry, status)) = rows
                    .iter_mut()
                    .find(|(row, _)| desktop::entries_match(row, &result.entry))
                {
                    entry.x = result.entry.x;
                    entry.y = result.entry.y;
                    *status = "UPDATED".to_string();
                }
            }
        }
        self.refresh_main_list();
    }

    fn save_window_placement(&self) {
        if let Some(hwnd) = window_state::hwnd_from_nwg(&self.window) {
            if let Ok(state) = window_state::read_from_hwnd(hwnd) {
                let _ = window_state::save(&state);
            }
        }
    }

    fn exit(&self) {
        self.save_window_placement();
        nwg::stop_thread_dispatch();
    }
}

fn main() {
    nwg::init().expect("Failed to initialize native-windows-gui");
    nwg::Font::set_global_family("Segoe UI").ok();

    let locale = i18n::resolve_startup_locale();
    i18n::apply_locale(&locale);

    let _com = match ComGuard::init() {
        Ok(guard) => guard,
        Err(e) => {
            nwg::error_message(&t!("app_name"), &e);
            return;
        }
    };

    let _app = IceIconSaverApp::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
