use arboard::{Clipboard, ImageData};
use chrono::{Datelike, Local, Timelike};
use gif;
use gtk::prelude::*;
use gtk::{
    glib, Align, Application, ApplicationWindow, Box, Button, ContentFit, DropDown, GestureDrag,
    Grid, Label, Orientation, Picture, Window,
};
use image::{open, ImageFormat};
use livesplit_hotkey::{Hook, Hotkey, KeyCode, Modifiers};
use native_dialog::FileDialog;
use screenshots::Screen;
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::{fs, time};
use std::{fs::File, thread};

#[derive(Clone)]
struct Coordinates {
    start_x: f64,
    start_y: f64,
    offset_x: f64,
    offset_y: f64,
}

impl Default for Coordinates {
    fn default() -> Self {
        Coordinates {
            start_x: f64::NAN,
            start_y: f64::NAN,
            offset_x: f64::NAN,
            offset_y: f64::NAN,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JSONStruct {
    new_shortcut_modif: String,
    new_shortcut_key: String,
    save_shortcut_modif: String,
    save_shortcut_key: String,
    undo_shortcut_modif: String,
    undo_shortcut_key: String,
    redo_shortcut_modif: String,
    redo_shortcut_key: String,
    cancel_shortcut_modif: String,
    cancel_shortcut_key: String,
    default_location: String,
}

const APP_ID: &str = "org.gtk_rs.Screen-PDS";
const TMP_FOLDER_NAME: &str = "screenshots";
const TMP_IMAGE_NAME: &str = "tmp";
const TMP_IMAGE_EXTENSION: &str = "png";
const SETTINGS_FILENAME: &str = "settings.json";
const DEFAULT_IMAGE_NAME: &str = "capture";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    let output = app.run();

    let _ = clean_tmp();

    output
}

fn build_ui(app: &Application) {
    let label = build_label("  HOW TO\n1) Click <+ New> to capture screen with chosen delay.\n2) Then, draw rectangles to crop the capture.\n3) Click <Undo> and <Redo> to cycle through cropped images timeline.\n4) When you are done, press OS key, select the main window and click <Save> with the chosen format.\n5) Or click <Cancel> if you want to quit the cropping procedure.".to_string());
    let button_new = build_button("+ New".to_string());
    let button_save = build_button("Save".to_string());
    let button_settings = build_button("Settings".to_string());
    let extension_list = build_dropdown(&["PNG", "JPG", "GIF"]);
    let timer_list = build_dropdown(&[
        "No delay",
        "3 seconds delay",
        "5 seconds delay",
        "10 seconds delay",
    ]);
    let button_undo = build_button("Undo".to_string());
    let button_redo = build_button("Redo".to_string());
    let button_cancel = build_button("Cancel".to_string());

    let content = Grid::new();
    content.attach(&label, 0, 0, 7, 1);
    content.attach(&button_new, 0, 1, 1, 1);
    content.attach(&timer_list, 1, 1, 1, 1);
    content.attach(&button_save, 2, 1, 1, 1);
    content.attach(&extension_list, 3, 1, 1, 1);
    content.attach(&button_settings, 4, 1, 1, 1);
    content.attach(&button_undo, 0, 2, 1, 1);
    content.attach(&button_redo, 1, 2, 1, 1);
    content.attach(&button_cancel, 2, 2, 1, 1);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Screen-PDS")
        .child(&content)
        .build();
    window.present();

    let (tx_thread_function_number, rx_thread_function_number) = mpsc::channel::<u32>();
    let tx_thread_function_number_hotkey = tx_thread_function_number.clone();
    let tx_thread_function_number_new = tx_thread_function_number.clone();
    let tx_thread_function_number_save = tx_thread_function_number.clone();
    let tx_thread_function_number_undo = tx_thread_function_number.clone();
    let tx_thread_function_number_redo = tx_thread_function_number.clone();
    let tx_thread_function_number_cancel = tx_thread_function_number.clone();

    let flag_function_selector = Arc::new(Mutex::new(0 as u32));
    let flag_function_selector_worker_thread = Arc::clone(&flag_function_selector);

    button_new.connect_clicked(move |_| {
        let result = tx_thread_function_number_new.send(1);
        match result {
            Ok(_) => {}
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    });

    button_save.connect_clicked(move |_| {
        let result = tx_thread_function_number_save.send(2);
        match result {
            Ok(_) => {}
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    });

    button_settings.connect_clicked(move |_| {
        build_settings_window();
    });

    button_undo.connect_clicked(move |_| {
        let result = tx_thread_function_number_undo.send(3);
        match result {
            Ok(_) => {}
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    });

    button_redo.connect_clicked(move |_| {
        let result = tx_thread_function_number_redo.send(4);
        match result {
            Ok(_) => {}
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    });

    button_cancel.connect_clicked(move |_| {
        let result = tx_thread_function_number_cancel.send(5);
        match result {
            Ok(_) => {}
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    });

    let _hotkey_thread = thread::spawn(move || loop {
        let shortcuts = retrieve_data_from_json();

        let new_hotkey =
            retrieve_hotkey(&shortcuts.new_shortcut_modif, &shortcuts.new_shortcut_key);
        let save_hotkey =
            retrieve_hotkey(&shortcuts.save_shortcut_modif, &shortcuts.save_shortcut_key);
        let undo_hotkey =
            retrieve_hotkey(&shortcuts.undo_shortcut_modif, &shortcuts.undo_shortcut_key);
        let redo_hotkey =
            retrieve_hotkey(&shortcuts.redo_shortcut_modif, &shortcuts.redo_shortcut_key);
        let cancel_hotkey = retrieve_hotkey(
            &shortcuts.cancel_shortcut_modif,
            &shortcuts.cancel_shortcut_key,
        );

        let tx_thread_function_number_hotkey_new = tx_thread_function_number_hotkey.clone();
        let tx_thread_function_number_hotkey_save = tx_thread_function_number_hotkey.clone();
        let tx_thread_function_number_hotkey_undo = tx_thread_function_number_hotkey.clone();
        let tx_thread_function_number_hotkey_redo = tx_thread_function_number_hotkey.clone();
        let tx_thread_function_number_hotkey_cancel = tx_thread_function_number_hotkey.clone();

        let hook = Hook::new().unwrap();
        hook.register(new_hotkey, move || {
            let result = tx_thread_function_number_hotkey_new.send(1);
            match result {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{}", error);
                }
            }
        })
        .unwrap();

        hook.register(save_hotkey, move || {
            let result = tx_thread_function_number_hotkey_save.send(2);
            match result {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{}", error);
                }
            }
        })
        .unwrap();

        hook.register(undo_hotkey, move || {
            let result = tx_thread_function_number_hotkey_undo.send(3);
            match result {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{}", error);
                }
            }
        })
        .unwrap();

        hook.register(redo_hotkey, move || {
            let result = tx_thread_function_number_hotkey_redo.send(4);
            match result {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{}", error);
                }
            }
        })
        .unwrap();

        hook.register(cancel_hotkey, move || {
            let result = tx_thread_function_number_hotkey_cancel.send(5);
            match result {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{}", error);
                }
            }
        })
        .unwrap();

        std::thread::sleep(time::Duration::from_millis(5000));
    });

    let _worker_thread = thread::spawn(move || loop {
        let output = rx_thread_function_number.recv();

        match output {
            Ok(result) => match result {
                1 => {
                    let mut flag = flag_function_selector_worker_thread.lock().unwrap();
                    *flag = 1;
                }
                2 => {
                    let mut flag = flag_function_selector_worker_thread.lock().unwrap();
                    *flag = 2;
                }
                3 => {
                    let mut flag = flag_function_selector_worker_thread.lock().unwrap();
                    *flag = 3;
                }
                4 => {
                    let mut flag = flag_function_selector_worker_thread.lock().unwrap();
                    *flag = 4;
                }
                5 => {
                    let mut flag = flag_function_selector_worker_thread.lock().unwrap();
                    *flag = 5;
                }
                _ => eprintln!("Other"),
            },
            Err(error) => {
                eprintln!("Error: {}", error);
            }
        }
    });

    set_default_json();
    let tmp_path_file = create_starting_tmp_path_file();
    let mut full_window = Window::builder().build();
    let mut screen_image = Picture::builder().build();
    let mut activate_check_coor = false;
    let mut coor_outer: Arc<Mutex<Coordinates>> = Arc::new(Mutex::new(Coordinates::default()));
    let mut timeline_current_index: u32 = 0;
    let mut timeline_last_index: u32 = 0;
    let mut condvar = Arc::new(Condvar::new());

    let tick = move || {
        let mut flag = flag_function_selector.lock().unwrap();
        match *flag {
            1 => {
                // new
                *flag = 0;
                if !activate_check_coor {
                    timeline_current_index = 0;
                    timeline_last_index = 0;
                    full_window = Window::builder().build();
                    window.minimize();
                    capture_screenshot_with_delay(timer_list.selected(), &tmp_path_file);
                    set_image_to_clipboard(&tmp_path_file);
                    screen_image = Picture::for_filename(&tmp_path_file);
                    build_fullscreen_window(&screen_image, &full_window);
                    (coor_outer, condvar) = draw_area(&full_window);
                    activate_check_coor = true;
                }
            }
            2 => {
                // save
                *flag = 0;
                if activate_check_coor {
                    let current_path = create_new_path(&tmp_path_file, timeline_current_index);
                    if save_image(extension_list.selected(), &current_path) {
                        full_window.close();
                        window.present();
                        activate_check_coor = false;
                    }
                }
            }
            3 => {
                // undo
                *flag = 0;
                if timeline_current_index > 0 && activate_check_coor {
                    timeline_current_index -= 1;
                    let current_path = create_new_path(&tmp_path_file, timeline_current_index);
                    full_window
                        .child()
                        .unwrap()
                        .downcast::<gtk::Box>()
                        .unwrap()
                        .remove(&screen_image);
                    screen_image = Picture::for_filename(&current_path);
                    screen_image.set_content_fit(ContentFit::ScaleDown);
                    screen_image.set_halign(Align::Start);
                    screen_image.set_valign(Align::Start);
                    full_window
                        .child()
                        .unwrap()
                        .downcast::<gtk::Box>()
                        .unwrap()
                        .append(&screen_image);
                    set_image_to_clipboard(&current_path);
                }
            }
            4 => {
                // redo
                *flag = 0;
                if timeline_current_index < timeline_last_index && activate_check_coor {
                    timeline_current_index += 1;
                    let current_path = create_new_path(&tmp_path_file, timeline_current_index);
                    full_window
                        .child()
                        .unwrap()
                        .downcast::<gtk::Box>()
                        .unwrap()
                        .remove(&screen_image);
                    screen_image = Picture::for_filename(&current_path);
                    screen_image.set_content_fit(ContentFit::ScaleDown);
                    screen_image.set_halign(Align::Start);
                    screen_image.set_valign(Align::Start);
                    full_window
                        .child()
                        .unwrap()
                        .downcast::<gtk::Box>()
                        .unwrap()
                        .append(&screen_image);
                    set_image_to_clipboard(&current_path);
                }
            }
            5 => {
                // cancel
                *flag = 0;
                if activate_check_coor {
                    full_window.close();
                    window.present();
                    activate_check_coor = false;
                }
            }
            _ => {}
        }
        if activate_check_coor {
            let mut coor = coor_outer.lock().unwrap();
            if !coor.offset_x.is_nan() && (coor.offset_x != 0.0 || coor.offset_y != 0.0) {
                let mut local_coor = Coordinates {
                    start_x: coor.start_x,
                    start_y: coor.start_y,
                    offset_x: coor.offset_x,
                    offset_y: coor.offset_y,
                };

                if local_coor.offset_x.is_sign_negative() {
                    local_coor.start_x += local_coor.offset_x;
                    local_coor.offset_x = -local_coor.offset_x;
                }
                if local_coor.offset_y.is_sign_negative() {
                    local_coor.start_y += local_coor.offset_y;
                    local_coor.offset_y = -local_coor.offset_y;
                }

                let prev_path = create_new_path(&tmp_path_file, timeline_current_index);
                let mut img = open(prev_path.as_path()).unwrap();
                if (local_coor.start_x as u32) < img.width()
                    && (local_coor.start_y as u32) < img.height()
                {
                    timeline_current_index += 1;
                    timeline_last_index = timeline_current_index;
                    let new_path = create_new_path(&tmp_path_file, timeline_current_index);
                    img.crop(
                        local_coor.start_x as u32,
                        local_coor.start_y as u32,
                        local_coor.offset_x as u32,
                        local_coor.offset_y as u32,
                    )
                    .save(&new_path)
                    .unwrap();
                    set_image_to_clipboard(&new_path);
                    full_window
                        .child()
                        .unwrap()
                        .downcast::<gtk::Box>()
                        .unwrap()
                        .remove(&screen_image);
                    screen_image = Picture::for_filename(&new_path);
                    screen_image.set_content_fit(ContentFit::ScaleDown);
                    screen_image.set_halign(Align::Start);
                    screen_image.set_valign(Align::Start);
                    full_window
                        .child()
                        .unwrap()
                        .downcast::<gtk::Box>()
                        .unwrap()
                        .append(&screen_image);
                    coor.start_x = f64::NAN;
                    coor.start_y = f64::NAN;
                    coor.offset_x = f64::NAN;
                    coor.offset_y = f64::NAN;
                }
            }
            condvar.notify_one();
        }

        glib::ControlFlow::Continue
    };

    glib::timeout_add_seconds_local(1, tick);
}

fn capture_screenshot_with_delay(delay: u32, path: &PathBuf) {
    match delay {
        0 => {}
        1 => {
            std::thread::sleep(time::Duration::from_millis(3000));
        }
        2 => {
            std::thread::sleep(time::Duration::from_millis(5000));
        }
        3 => {
            std::thread::sleep(time::Duration::from_millis(10000));
        }
        _ => eprintln!("Error"),
    }

    capture_fullscreen(path);
}

/* captures fullscreen screenshot */
fn capture_fullscreen(path: &PathBuf) {
    let screen = Screen::from_point(0, 0).unwrap();
    let image = screen.capture().unwrap();
    let result = image.save(path.as_path());
    match result {
        Ok(_v) => {}
        Err(error) => eprintln!("Error: {}", error),
    }
}

/* save the image in a chosen extension in a chosen path */
fn save_image(current_selected: u32, tmp_path: &PathBuf) -> bool {
    let tmp = open(tmp_path.as_path());

    match tmp {
        Ok(tmp_image) => match current_selected {
            0 => {
                let result = choose_path(".png");
                match result {
                    Some(path) => {
                        path.clone().set_extension("png");
                        let _ = tmp_image.save_with_format(path, ImageFormat::Png);
                        true
                    }
                    None => false,
                }
            }
            1 => {
                let result = choose_path(".jpg");
                match result {
                    Some(path) => {
                        path.clone().set_extension("jpg");
                        let _ = tmp_image.save_with_format(path, ImageFormat::Jpeg);
                        true
                    }
                    None => false,
                }
            }
            2 => {
                let result = choose_path(".gif");
                match result {
                    Some(path) => {
                        let mut pixels = tmp_image.to_rgb8().into_raw();
                        let frame = gif::Frame::from_rgb(
                            tmp_image.width() as u16,
                            tmp_image.height() as u16,
                            &mut *pixels,
                        );
                        let mut image = File::create(path).unwrap();
                        let mut encoder =
                            gif::Encoder::new(&mut image, frame.width, frame.height, &[]).unwrap();
                        encoder.write_frame(&frame).unwrap();
                        true
                    }
                    None => false,
                }
            }
            _ => {
                eprintln!("Error");
                false
            }
        },
        Err(_) => false,
    }
}

/* build the fullscreen window with the new acquisition */
fn build_fullscreen_window(image: &Picture, window_full: &Window) {
    let content = Box::new(Orientation::Horizontal, 0);
    content.append(image);

    window_full.set_child(Some(&content));
    window_full.present();
    window_full.fullscreen();
}

fn build_button(label: String) -> Button {
    Button::builder()
        .label(label)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build()
}

fn build_label(label: String) -> Label {
    Label::builder()
        .label(label)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build()
}

fn build_dropdown(params: &[&str]) -> DropDown {
    let tmp: DropDown = DropDown::from_strings(params);
    tmp.set_margin_bottom(12);
    tmp.set_margin_end(12);
    tmp.set_margin_start(12);
    tmp.set_margin_top(12);
    tmp
}

fn build_settings_window() {
    let current_n_shortcut = build_label("New screenshot:".to_string());
    let current_s_shortcut = build_label("Save screenshot:".to_string());
    let current_u_shortcut = build_label("Undo action:".to_string());
    let current_r_shortcut = build_label("Redo action:".to_string());
    let current_c_shortcut: Label = build_label("Cancel :".to_string());

    let button_change_shortcut = build_button("Change Shortcuts".to_string());
    let button_go_back = build_button("<-".to_string().to_string());

    let file_path = SETTINGS_FILENAME;
    let mut file = File::open(file_path).expect("Failed to open file");

    // Read the file contents into a string
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)
        .expect("Failed to read file");

    // Parse JSON
    let parsed_data: Result<JSONStruct, serde_json::Error> = serde_json::from_str(&file_contents);
    let n_shortcut = build_label("ERR".to_string());
    let s_shortcut = build_label("ERR".to_string());
    let u_shortcut = build_label("ERR".to_string());
    let r_shortcut = build_label("ERR".to_string());
    let c_shortcut: Label = build_label("ERR".to_string());

    let mut json_data: JSONStruct = JSONStruct {
        new_shortcut_modif: "CONTROL".to_string(),
        new_shortcut_key: "N".to_string(),
        save_shortcut_modif: "CONTROL".to_string(),
        save_shortcut_key: "S".to_string(),
        undo_shortcut_modif: "CONTROL".to_string(),
        undo_shortcut_key: "X".to_string(),
        redo_shortcut_modif: "CONTROL".to_string(),
        redo_shortcut_key: "Y".to_string(),
        cancel_shortcut_modif: "CONTROL".to_string(),
        cancel_shortcut_key: "E".to_string(),
        default_location: "./".to_string(),
    };

    match parsed_data {
        Ok(data) => {
            // Access the values from the parsed data
            let data_clone = data.clone();
            json_data.new_shortcut_modif = data.new_shortcut_modif;
            json_data.new_shortcut_key = data.new_shortcut_key;
            json_data.save_shortcut_modif = data.save_shortcut_modif;
            json_data.save_shortcut_key = data.save_shortcut_key;
            json_data.undo_shortcut_modif = data.undo_shortcut_modif;
            json_data.undo_shortcut_key = data.undo_shortcut_key;
            json_data.redo_shortcut_modif = data.redo_shortcut_modif;
            json_data.redo_shortcut_key = data.redo_shortcut_key;
            json_data.cancel_shortcut_modif = data.cancel_shortcut_modif;
            json_data.cancel_shortcut_key = data.cancel_shortcut_key;

            json_data.default_location = data.default_location;

            n_shortcut
                .set_label(&(data_clone.new_shortcut_modif + " + " + &json_data.new_shortcut_key));
            s_shortcut.set_label(
                &(data_clone.save_shortcut_modif + " + " + &json_data.save_shortcut_key),
            );
            u_shortcut.set_label(
                &(data_clone.undo_shortcut_modif + " + " + &json_data.undo_shortcut_key),
            );
            r_shortcut.set_label(
                &(data_clone.redo_shortcut_modif + " + " + &json_data.redo_shortcut_key),
            );
            c_shortcut.set_label(
                &(data_clone.cancel_shortcut_modif + " + " + &json_data.cancel_shortcut_key),
            );
        }
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
        }
    }

    let settings_grid = Grid::new();
    let current_def_loc = build_label("Default location:".to_string());
    let button_change_location = build_button("Change def loc".to_string());

    settings_grid.attach(&current_def_loc, 0, 6, 1, 1);
    let json_data_clone = json_data.clone();

    settings_grid.attach(&build_label(json_data_clone.default_location), 1, 6, 1, 1);
    settings_grid.attach(&button_change_location, 2, 6, 1, 1);

    settings_grid.attach(&current_n_shortcut, 0, 1, 1, 1);
    settings_grid.attach(&n_shortcut, 1, 1, 1, 1);
    settings_grid.attach(&button_change_shortcut, 2, 5, 1, 1);

    settings_grid.attach(&current_s_shortcut, 0, 2, 1, 1);
    settings_grid.attach(&s_shortcut, 1, 2, 1, 1);

    settings_grid.attach(&current_u_shortcut, 0, 3, 1, 1);
    settings_grid.attach(&u_shortcut, 1, 3, 1, 1);

    settings_grid.attach(&current_r_shortcut, 0, 4, 1, 1);
    settings_grid.attach(&r_shortcut, 1, 4, 1, 1);

    settings_grid.attach(&current_c_shortcut, 0, 5, 1, 1);
    settings_grid.attach(&c_shortcut, 1, 5, 1, 1);

    let settings_window = ApplicationWindow::builder()
        .title("Settings-PDS")
        .child(&settings_grid)
        .build();

    let settings_window_clone = settings_window.clone();
    let settings_window_clone2 = settings_window.clone();

    button_go_back.connect_clicked(move |_| {
        settings_window_clone.close();
    });

    button_change_shortcut.connect_clicked(move |_| {
        settings_window_clone2.close();
        let change_settings_window = build_change_settings_window();
        change_settings_window.present();
    });

    button_change_location.connect_clicked(move |_| {
        let mut settings = retrieve_data_from_json();

        match fs::metadata(&settings.default_location) {
            Ok(_) => {}
            Err(_) => {
                std::fs::create_dir_all(&settings.default_location).unwrap();
            }
        }
        let result = FileDialog::new()
            .set_location(&settings.default_location)
            .show_open_single_dir()
            .unwrap();
        match result {
            Some(path) => {
                settings.default_location = path.clone().into_os_string().into_string().unwrap();
                let json_data = serde_json::to_string(&settings).unwrap();
                let _ = std::fs::write(SETTINGS_FILENAME, json_data);
                let label = settings_grid.child_at(1, 6).unwrap();
                settings_grid.remove(&label);
                settings_grid.attach(
                    &build_label(path.into_os_string().into_string().unwrap()),
                    1,
                    6,
                    1,
                    1,
                );
            }
            None => {}
        }
    });

    settings_window.present();
}

fn build_change_settings_window() -> ApplicationWindow {
    /* change shortcuts */
    let change_settings_grid = Grid::new();

    let change_settings_grid_clone = change_settings_grid.clone();
    let json_data: JSONStruct = retrieve_data_from_json();
    let button_save_changes = build_button("Save changes".to_string());
    let curr_n_label = build_label("New acquisition".to_string());
    let curr_s_label = build_label("Save image".to_string());
    let curr_u_label = build_label("Undo action".to_string());
    let curr_r_label = build_label("Redo action".to_string());
    let curr_c_label = build_label("Cancel action".to_string());

    change_settings_grid.attach(&curr_n_label, 0, 1, 1, 1);
    change_settings_grid.attach(&curr_s_label, 0, 2, 1, 1);
    change_settings_grid.attach(&curr_u_label, 0, 3, 1, 1);
    change_settings_grid.attach(&curr_r_label, 0, 4, 1, 1);
    change_settings_grid.attach(&curr_c_label, 0, 5, 1, 1);

    let modif_new_drop = build_dropdown(&["CTRL", "SHIFT", "ALT"]);
    let key_new_drop = build_dropdown(&[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ]);
    let modif_save_drop = build_dropdown(&["CTRL", "SHIFT", "ALT"]);
    let key_save_drop: DropDown = build_dropdown(&[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ]);
    let modif_undo_drop = build_dropdown(&["CTRL", "SHIFT", "ALT"]);
    let key_undo_drop: DropDown = build_dropdown(&[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ]);
    let modif_redo_drop = build_dropdown(&["CTRL", "SHIFT", "ALT"]);
    let key_redo_drop: DropDown = build_dropdown(&[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ]);
    let modif_cancel_drop = build_dropdown(&["CTRL", "SHIFT", "ALT"]);
    let key_cancel_drop: DropDown = build_dropdown(&[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ]);

    let modif_new_drop_clone = modif_new_drop.clone();
    let key_new_drop_clone = key_new_drop.clone();
    let modif_save_drop_clone = modif_save_drop.clone();
    let key_save_drop_clone = key_save_drop.clone();
    let modif_undo_drop_clone = modif_undo_drop.clone();
    let key_undo_drop_clone = key_undo_drop.clone();
    let modif_redo_drop_clone = modif_redo_drop.clone();
    let key_redo_drop_clone = key_redo_drop.clone();
    let modif_cancel_drop_clone = modif_cancel_drop.clone();
    let key_cancel_drop_clone = key_cancel_drop.clone();

    let modif_new_index = json_modif_to_index(&json_data.new_shortcut_modif);
    let key_new_index = json_key_to_index(&json_data.new_shortcut_key);
    let modif_save_index = json_modif_to_index(&json_data.save_shortcut_modif);
    let key_save_index = json_key_to_index(&json_data.save_shortcut_key);
    let modif_undo_index = json_modif_to_index(&json_data.undo_shortcut_modif);
    let key_undo_index = json_key_to_index(&json_data.undo_shortcut_key);
    let modif_redo_index = json_modif_to_index(&json_data.redo_shortcut_modif);
    let key_redo_index = json_key_to_index(&json_data.redo_shortcut_key);
    let modif_cancel_index = json_modif_to_index(&json_data.cancel_shortcut_modif);
    let key_cancel_index = json_key_to_index(&json_data.cancel_shortcut_key);

    modif_new_drop.set_selected(modif_new_index);
    key_new_drop.set_selected(key_new_index);
    modif_save_drop.set_selected(modif_save_index);
    key_save_drop.set_selected(key_save_index);
    modif_undo_drop.set_selected(modif_undo_index);
    key_undo_drop.set_selected(key_undo_index);
    modif_redo_drop.set_selected(modif_redo_index);
    key_redo_drop.set_selected(key_redo_index);
    modif_cancel_drop.set_selected(modif_cancel_index);
    key_cancel_drop.set_selected(key_cancel_index);

    change_settings_grid.attach(&modif_new_drop, 2, 1, 1, 1);
    change_settings_grid.attach(&key_new_drop, 3, 1, 1, 1);
    change_settings_grid.attach(&modif_save_drop, 2, 2, 1, 1);
    change_settings_grid.attach(&key_save_drop, 3, 2, 1, 1);
    change_settings_grid.attach(&modif_undo_drop, 2, 3, 1, 1);
    change_settings_grid.attach(&key_undo_drop, 3, 3, 1, 1);
    change_settings_grid.attach(&modif_redo_drop, 2, 4, 1, 1);
    change_settings_grid.attach(&key_redo_drop, 3, 4, 1, 1);
    change_settings_grid.attach(&modif_cancel_drop, 2, 5, 1, 1);
    change_settings_grid.attach(&key_cancel_drop, 3, 5, 1, 1);
    change_settings_grid.attach(&button_save_changes, 2, 6, 1, 1);

    let change_shortcut_window = ApplicationWindow::builder()
        .title("Change shortcut")
        .child(&change_settings_grid)
        .build();

    let change_shortcut_window_save: ApplicationWindow = change_shortcut_window.clone();

    button_save_changes.connect_clicked(move |_| {
        let mn = modif_new_drop_clone.selected();
        let kn = key_new_drop_clone.selected();
        let ms = modif_save_drop_clone.selected();
        let ks = key_save_drop_clone.selected();
        let mu = modif_undo_drop_clone.selected();
        let ku = key_undo_drop_clone.selected();
        let kr = key_redo_drop_clone.selected();
        let mr = modif_redo_drop_clone.selected();
        let mc = modif_cancel_drop_clone.selected();
        let kc = key_cancel_drop_clone.selected();

        if ((mn == ms) && (kn == ks))
            || ((mn == mu) && (kn == ku))
            || ((ms == mu) && (ku == ks))
            || ((mr == mu) && (kr == ku))
            || ((mr == mn) && (kr == kn))
            || ((mr == ms) && (kr == ks))
            || ((mc == mn) && (kc == kn))
            || ((mc == ms) && (kc == ks))
            || ((mc == mu) && (kc == ku))
            || ((mc == mr) && (kc == kr))
        {
            let err_label = build_label("The shortcuts must be different!".to_string());
            change_settings_grid_clone.attach(&err_label, 0, 0, 4, 1);
        } else {
            let mn_json = index_to_json_modif(mn);
            let kn_json = index_to_json_key(kn);
            let ms_json = index_to_json_modif(ms);
            let ks_json = index_to_json_key(ks);
            let mu_json = index_to_json_modif(mu);
            let ku_json = index_to_json_key(ku);
            let mr_json = index_to_json_modif(mr);
            let kr_json = index_to_json_key(kr);
            let kc_json = index_to_json_key(kc);
            let mc_json = index_to_json_modif(mc);
            let x = retrieve_data_from_json().default_location;
            let new_json = JSONStruct {
                new_shortcut_modif: mn_json,
                new_shortcut_key: kn_json,
                save_shortcut_modif: ms_json,
                save_shortcut_key: ks_json,
                undo_shortcut_modif: mu_json,
                undo_shortcut_key: ku_json,
                redo_shortcut_modif: mr_json,
                redo_shortcut_key: kr_json,
                cancel_shortcut_modif: mc_json,
                cancel_shortcut_key: kc_json,
                default_location: x,
            };
            let json_data = serde_json::to_string(&new_json).unwrap();

            // Write the JSON string to a file
            let _ = std::fs::write(SETTINGS_FILENAME, json_data);

            change_shortcut_window_save.close();
        }
    });

    change_shortcut_window
}

/* draw rectangle area to crop the screenshot */
fn draw_area(window: &Window) -> (Arc<Mutex<Coordinates>>, Arc<Condvar>) {
    let draw_ctrl = GestureDrag::new();
    let coor: Arc<Mutex<Coordinates>> = Arc::new(Mutex::new(Coordinates::default()));
    let condvar = Arc::new(Condvar::new());

    let thread_condvar = Arc::clone(&condvar);
    let thread_coor_begin = Arc::clone(&coor);
    draw_ctrl.connect_drag_begin(move |_, x, y| {
        let mut coor = thread_coor_begin.lock().unwrap();
        coor.start_x = x;
        coor.start_y = y;
        thread_condvar.notify_one();
    });

    let thread_condvar = Arc::clone(&condvar);
    let thread_coor_end = Arc::clone(&coor);
    draw_ctrl.connect_drag_end(move |_, x, y| {
        let mut coor = thread_coor_end.lock().unwrap();
        coor.offset_x = x;
        coor.offset_y = y;
        thread_condvar.notify_one();
    });

    window.child().unwrap().add_controller(draw_ctrl);
    (coor, condvar)
}

fn create_starting_tmp_path_file() -> PathBuf {
    let mut path = std::env::current_dir().unwrap();
    path.push(TMP_FOLDER_NAME);
    std::fs::create_dir_all(&path).unwrap();
    path.push(TMP_IMAGE_NAME.to_owned() + "0");
    path.set_extension(TMP_IMAGE_EXTENSION);
    path
}

/* copies the new screenshot to clipboard */
fn set_image_to_clipboard(path: &PathBuf) {
    let result = Clipboard::new();
    match result {
        Ok(value) => {
            let mut ctx = value;
            let img = open(path.as_path()).unwrap();
            let img_data = ImageData {
                width: img.width() as usize,
                height: img.height() as usize,
                bytes: img.to_rgba8().into_raw().into(),
            };
            let result = ctx.set_image(img_data);
            match result {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{}", error);
                }
            }
        }
        Err(error) => {
            eprintln!("{}", error);
        }
    }
}

fn choose_path(extension: &str) -> Option<PathBuf> {
    let mut default_path = std::env::current_dir().unwrap();
    default_path.push(retrieve_data_from_json().default_location);
    std::fs::create_dir_all(&default_path).unwrap();
    let current_datetime = Local::now();
    let filename = DEFAULT_IMAGE_NAME.to_owned()
        + &current_datetime.year().to_string()
        + "-"
        + &current_datetime.month().to_string()
        + "-"
        + &current_datetime.day().to_string()
        + "-"
        + &current_datetime.hour().to_string()
        + "_"
        + &current_datetime.minute().to_string()
        + "_"
        + &current_datetime.second().to_string()
        + extension;
    FileDialog::new()
        .set_location(default_path.as_path())
        .set_filename(&filename)
        .show_save_single_file()
        .unwrap()
}

fn json_modif_to_index(modif: &str) -> u32 {
    match modif {
        "CONTROL" => 0,
        "SHIFT" => 1,
        "ALT" => 2,
        _ => 0,
    }
}

fn index_to_json_modif(modif: u32) -> String {
    match modif {
        0 => "CONTROL".to_string(),
        1 => "SHIFT".to_string(),
        2 => "ALT".to_string(),
        _ => "CONTROL".to_string(),
    }
}

fn json_key_to_index(key: &str) -> u32 {
    match key {
        "A" => 0,
        "B" => 1,
        "C" => 2,
        "D" => 3,
        "E" => 4,
        "F" => 5,
        "G" => 6,
        "H" => 7,
        "I" => 8,
        "J" => 9,
        "K" => 10,
        "L" => 11,
        "M" => 12,
        "N" => 13,
        "O" => 14,
        "P" => 15,
        "Q" => 16,
        "R" => 17,
        "S" => 18,
        "T" => 19,
        "U" => 20,
        "V" => 21,
        "W" => 22,
        "X" => 23,
        "Y" => 24,
        "Z" => 25,
        _ => 0, // Default case for any other input
    }
}

fn index_to_json_key(index: u32) -> String {
    match index {
        0 => "A".to_string(),
        1 => "B".to_string(),
        2 => "C".to_string(),
        3 => "D".to_string(),
        4 => "E".to_string(),
        5 => "F".to_string(),
        6 => "G".to_string(),
        7 => "H".to_string(),
        8 => "I".to_string(),
        9 => "J".to_string(),
        10 => "K".to_string(),
        11 => "L".to_string(),
        12 => "M".to_string(),
        13 => "N".to_string(),
        14 => "O".to_string(),
        15 => "P".to_string(),
        16 => "Q".to_string(),
        17 => "R".to_string(),
        18 => "S".to_string(),
        19 => "T".to_string(),
        20 => "U".to_string(),
        21 => "V".to_string(),
        22 => "W".to_string(),
        23 => "X".to_string(),
        24 => "Y".to_string(),
        25 => "Z".to_string(),
        _ => "ERR".to_string(), // Default case for any other input
    }
}

/* reads json file "settings.json" to update hotkeys and default location */
fn retrieve_data_from_json() -> JSONStruct {
    let mut file = File::open(SETTINGS_FILENAME).expect("Failed to open file");

    // Read the file contents into a string
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)
        .expect("Failed to read file");

    // Parse JSON
    let parsed_data: Result<JSONStruct, serde_json::Error> = serde_json::from_str(&file_contents);
    let mut nm = "CONTROL".to_string();
    let mut nk = "N".to_string();
    let mut sm = "CONTROL".to_string();
    let mut sk = "S".to_string();
    let mut um = "CONTROL".to_string();
    let mut uk = "Z".to_string();
    let mut rm = "CONTROL".to_string();
    let mut rk = "Y".to_string();
    let mut cm = "CONTROL".to_string();
    let mut ck = "E".to_string();
    let mut dl = "/.".to_string();
    match parsed_data {
        Ok(data) => {
            // Access the values from the parsed data
            nm = data.new_shortcut_modif;
            nk = data.new_shortcut_key;
            sm = data.save_shortcut_modif;
            sk = data.save_shortcut_key;
            um = data.undo_shortcut_modif;
            uk = data.undo_shortcut_key;
            rm = data.redo_shortcut_modif;
            rk = data.redo_shortcut_key;
            cm = data.cancel_shortcut_modif;
            ck = data.cancel_shortcut_key;
            dl = data.default_location;
        }
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
        }
    }

    JSONStruct {
        new_shortcut_modif: nm,
        new_shortcut_key: nk,
        save_shortcut_modif: sm,
        save_shortcut_key: sk,
        undo_shortcut_modif: um,
        undo_shortcut_key: uk,
        redo_shortcut_modif: rm,
        redo_shortcut_key: rk,
        cancel_shortcut_modif: cm,
        cancel_shortcut_key: ck,
        default_location: dl,
    }
}

/* retrieve hotkeys from parameters */
fn retrieve_hotkey(modif: &str, key: &str) -> Hotkey {
    let mut m = Modifiers::CONTROL;
    match modif {
        "CONTROL" => m = Modifiers::CONTROL,
        "SHIFT" => m = Modifiers::SHIFT,
        "ALT" => m = Modifiers::ALT,
        _ => {}
    }

    let k: KeyCode = match key {
        "A" => KeyCode::KeyA,
        "B" => KeyCode::KeyB,
        "C" => KeyCode::KeyC,
        "D" => KeyCode::KeyD,
        "E" => KeyCode::KeyE,
        "F" => KeyCode::KeyF,
        "G" => KeyCode::KeyG,
        "H" => KeyCode::KeyH,
        "I" => KeyCode::KeyI,
        "J" => KeyCode::KeyJ,
        "K" => KeyCode::KeyK,
        "L" => KeyCode::KeyL,
        "M" => KeyCode::KeyM,
        "N" => KeyCode::KeyN,
        "O" => KeyCode::KeyO,
        "P" => KeyCode::KeyP,
        "Q" => KeyCode::KeyQ,
        "R" => KeyCode::KeyR,
        "S" => KeyCode::KeyS,
        "T" => KeyCode::KeyT,
        "U" => KeyCode::KeyU,
        "V" => KeyCode::KeyV,
        "W" => KeyCode::KeyW,
        "X" => KeyCode::KeyX,
        "Y" => KeyCode::KeyY,
        "Z" => KeyCode::KeyZ,
        _ => KeyCode::KeyA,
    };
    let hotkey = Hotkey {
        key_code: k,
        modifiers: m,
    };
    hotkey
}

fn set_default_json() {
    let mut settings = retrieve_data_from_json();
    if settings.default_location.is_empty() {
        let mut path = std::env::current_dir().unwrap();
        path.push(TMP_FOLDER_NAME);
        settings.default_location = path.into_os_string().into_string().unwrap();
        let json_data = serde_json::to_string(&settings);
        match json_data {
            Ok(json_data) => {
                let _ = std::fs::write(SETTINGS_FILENAME, json_data);
            }
            Err(err) => {
                eprintln!("{}", err);
            }
        }
    }
}

fn create_new_path(path: &PathBuf, index: u32) -> PathBuf {
    let mut output = path.clone();
    output.pop();
    output.push(TMP_IMAGE_NAME.to_owned() + &index.to_string());
    output.set_extension(TMP_IMAGE_EXTENSION);

    output
}

fn clean_tmp() -> std::io::Result<()> {
    let mut path = std::env::current_dir().unwrap();
    path.push(TMP_FOLDER_NAME);
    std::fs::remove_dir_all(path)?;
    Ok(())
}
