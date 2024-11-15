// Gupax - GUI Uniting P2Pool And XMRig
//
// Copyright (c) 2022-2023 hinto-janai
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// Hide console in Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Only (windows|macos|linux) + (x64|arm64) are supported.
#[cfg(not(target_pointer_width = "64"))]
compile_error!("gupaxx is only compatible with 64-bit CPUs");

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux",)))]
compile_error!("gupaxx is only built for windows/macos/linux");

use crate::app::App;
use crate::cli::Cli;
//---------------------------------------------------------------------------------------------------- Imports
use crate::constants::*;
use crate::inits::init_auto;
use crate::inits::init_logger;
use crate::inits::init_options;
use crate::miscs::clean_dir;
use crate::utils::*;
use clap::Parser;
use egui::Vec2;
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow};
use gtk::Button;
use log::info;
use log::warn;
use std::time::Instant;

mod app;
mod cli;
mod components;
mod disk;
mod helper;
mod inits;
mod miscs;
mod utils;

// Sudo (dummy values for Windows)
#[cfg(target_family = "unix")]
extern crate sudo as sudo_check;

// //---------------------------------------------------------------------------------------------------- Main [App] frame
// fn main() {
//     let args = Cli::parse();
//     let now = Instant::now();

//     // Set custom panic hook.
//     crate::panic::set_panic_hook(now);

//     // Init logger.
//     init_logger(now, args.logfile);
//     let mut app = App::new(now, args);
//     init_auto(&mut app);

//     // Init GUI stuff.
//     let selected_width = app.state.gupax.selected_width as f32;
//     let selected_height = app.state.gupax.selected_height as f32;
//     let initial_window_size = if selected_width > APP_MAX_WIDTH || selected_height > APP_MAX_HEIGHT
//     {
//         warn!("App | Set width or height was greater than the maximum! Starting with the default resolution...");
//         Some(Vec2::new(APP_DEFAULT_WIDTH, APP_DEFAULT_HEIGHT))
//     } else {
//         Some(Vec2::new(
//             app.state.gupax.selected_width as f32,
//             app.state.gupax.selected_height as f32,
//         ))
//     };
//     let options = init_options(initial_window_size);

//     // Gupax folder cleanup.
//     match clean_dir() {
//         Ok(_) => info!("Temporary folder cleanup ... OK"),
//         Err(e) => warn!("Could not cleanup [gupax_tmp] folders: {}", e),
//     }

//     let resolution = Vec2::new(selected_width, selected_height);

//     // Run Gupax.
//     info!("/*************************************/ Init ... OK /*************************************/");
//     eframe::run_native(
//         &app.name_version.clone(),
//         options,
//         Box::new(move |cc| {
//             egui_extras::install_image_loaders(&cc.egui_ctx);
//             Ok(Box::new(App::cc(cc, resolution, app)))
//         }),
//     )
//     .unwrap();
// }

const APP_ID: &str = "com.github.karce.gupaxtk";

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of app.
    app.connect_activate(build_ui);
    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    // Create a button with label and margins
    let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Connect to "clicked" signal of `button`
    button.connect_clicked(|button| {
        // Set the label to "Hello World!" after the button has been clicked on
        button.set_label("Hello World!");
    });

    // Create the main window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Gupaxtk")
        .child(&button)
        .build();

    window.present();
}
