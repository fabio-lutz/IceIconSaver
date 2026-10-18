use crate::store::IconEntry;
use native_windows_gui as nwg;
use rust_i18n::t;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

const APP_ICON: &[u8] = include_bytes!("../monitor.ico");

#[derive(Clone)]
struct ReviewRow {
    entry: IconEntry,
    missing: bool,
}

enum ReviewAction {
    Confirm(Vec<(IconEntry, bool)>),
    Cancel,
}

struct ReviewState {
    rows: RefCell<Vec<ReviewRow>>,
    show_missing: bool,
    result_tx: RefCell<Option<mpsc::Sender<ReviewAction>>>,
}

struct ReviewControls {
    window: nwg::Window,
    list: nwg::ListView,
    remove_btn: nwg::Button,
    cancel_btn: nwg::Button,
    confirm_btn: nwg::Button,
    /// Kept alive for the lifetime of the dialog window.
    app_icon: nwg::Icon,
}

/// Shows a modal review dialog on a dedicated UI thread.
/// Returns `Some(rows)` on Confirm (including missing flags), `None` on Cancel.
pub fn run_review_dialog(
    title: &str,
    icons: Vec<(IconEntry, bool)>,
    show_missing: bool,
    locale: &str,
) -> Option<Vec<(IconEntry, bool)>> {
    let (tx, rx) = mpsc::channel::<ReviewAction>();
    let title = title.to_string();
    let locale = locale.to_string();

    thread::spawn(move || {
        rust_i18n::set_locale(&locale);

        if nwg::init().is_err() {
            let _ = tx.send(ReviewAction::Cancel);
            return;
        }

        let mut rows: Vec<ReviewRow> = icons
            .into_iter()
            .map(|(entry, missing)| ReviewRow { entry, missing })
            .collect();
        rows.sort_by(|a, b| a.entry.name.to_lowercase().cmp(&b.entry.name.to_lowercase()));

        let state = Rc::new(ReviewState {
            rows: RefCell::new(rows),
            show_missing,
            result_tx: RefCell::new(Some(tx)),
        });

        let mut app_icon = nwg::Icon::default();
        if nwg::Icon::builder()
            .source_bin(Some(APP_ICON))
            .build(&mut app_icon)
            .is_err()
        {
            finish(&state, ReviewAction::Cancel);
            return;
        }

        let mut controls = ReviewControls {
            window: Default::default(),
            list: Default::default(),
            remove_btn: Default::default(),
            cancel_btn: Default::default(),
            confirm_btn: Default::default(),
            app_icon,
        };

        if build_controls(&mut controls, &title).is_err() {
            finish(&state, ReviewAction::Cancel);
            return;
        }

        setup_list(&controls.list, show_missing);
        populate_list(&controls.list, &state.rows.borrow(), show_missing);

        let controls = Rc::new(controls);
        let state_handler = state.clone();
        let controls_handler = controls.clone();

        let handler = nwg::full_bind_event_handler(&controls.window.handle, move |evt, _evt_data, handle| {
            use nwg::Event as E;
            match evt {
                E::OnButtonClick => {
                    if handle == controls_handler.remove_btn.handle {
                        remove_selected(&controls_handler.list, &state_handler);
                    } else if handle == controls_handler.cancel_btn.handle {
                        finish(&state_handler, ReviewAction::Cancel);
                        nwg::stop_thread_dispatch();
                    } else if handle == controls_handler.confirm_btn.handle {
                        let icons: Vec<(IconEntry, bool)> = state_handler
                            .rows
                            .borrow()
                            .iter()
                            .map(|r| (r.entry.clone(), r.missing))
                            .collect();
                        finish(&state_handler, ReviewAction::Confirm(icons));
                        nwg::stop_thread_dispatch();
                    }
                }
                E::OnWindowClose => {
                    if handle == controls_handler.window.handle {
                        finish(&state_handler, ReviewAction::Cancel);
                        nwg::stop_thread_dispatch();
                    }
                }
                _ => {}
            }
        });

        nwg::dispatch_thread_events();
        nwg::unbind_event_handler(&handler);
        drop(controls);
    });

    match rx.recv() {
        Ok(ReviewAction::Confirm(icons)) => Some(icons),
        _ => None,
    }
}

fn build_controls(controls: &mut ReviewControls, title: &str) -> Result<(), nwg::NwgError> {
    nwg::Window::builder()
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .size((620, 460))
        .center(true)
        .title(title)
        .icon(Some(&controls.app_icon))
        .build(&mut controls.window)?;

    nwg::ListView::builder()
        .parent(&controls.window)
        .position((10, 10))
        .size((590, 360))
        .list_style(nwg::ListViewStyle::Detailed)
        .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)
        .build(&mut controls.list)?;

    nwg::Button::builder()
        .text(&t!("btn_remove_icon"))
        .parent(&controls.window)
        .position((10, 385))
        .size((140, 32))
        .build(&mut controls.remove_btn)?;

    nwg::Button::builder()
        .text(&t!("btn_cancel"))
        .parent(&controls.window)
        .position((380, 385))
        .size((100, 32))
        .build(&mut controls.cancel_btn)?;

    nwg::Button::builder()
        .text(&t!("btn_confirm"))
        .parent(&controls.window)
        .position((500, 385))
        .size((100, 32))
        .build(&mut controls.confirm_btn)?;

    Ok(())
}

fn setup_list(list: &nwg::ListView, show_missing: bool) {
    list.insert_column(nwg::InsertListViewColumn {
        index: Some(0),
        fmt: None,
        width: Some(260),
        text: Some(t!("col_name").to_string()),
    });
    list.insert_column(nwg::InsertListViewColumn {
        index: Some(1),
        fmt: None,
        width: Some(80),
        text: Some(t!("col_x").to_string()),
    });
    list.insert_column(nwg::InsertListViewColumn {
        index: Some(2),
        fmt: None,
        width: Some(80),
        text: Some(t!("col_y").to_string()),
    });
    if show_missing {
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(3),
            fmt: None,
            width: Some(100),
            text: Some(t!("col_missing").to_string()),
        });
    }
    list.set_headers_enabled(true);
}

fn finish(state: &ReviewState, action: ReviewAction) {
    if let Some(tx) = state.result_tx.borrow_mut().take() {
        let _ = tx.send(action);
    }
}

fn populate_list(list: &nwg::ListView, rows: &[ReviewRow], show_missing: bool) {
    list.clear();
    for (index, row) in rows.iter().enumerate() {
        list.insert_items_row(
            Some(index as i32),
            &[
                row.entry.name.as_str(),
                &row.entry.x.to_string(),
                &row.entry.y.to_string(),
            ],
        );
        if show_missing {
            let missing_text = if row.missing {
                t!("status_missing").to_string()
            } else {
                String::new()
            };
            list.insert_item(nwg::InsertListViewItem {
                index: Some(index as i32),
                column_index: 3,
                text: Some(missing_text),
                image: None,
            });
        }
    }
}

fn remove_selected(list: &nwg::ListView, state: &ReviewState) {
    let Some(selected) = list.selected_item() else {
        return;
    };
    {
        let mut rows = state.rows.borrow_mut();
        if selected < rows.len() {
            rows.remove(selected);
        }
    }
    populate_list(list, &state.rows.borrow(), state.show_missing);
}
