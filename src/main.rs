// Gupax - GUI Uniting P2Pool And XMRig
//
// Copyright (c) 2022 hinto-janaiyo
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
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//---------------------------------------------------------------------------------------------------- Imports
// egui/eframe
use egui::TextStyle::*;
use egui::color::Color32;
use egui::FontFamily::Proportional;
use egui::{FontId,Label,RichText,Stroke,Vec2};
use egui::special_emojis::GITHUB;
use egui::{Key,Modifiers};
use egui_extras::RetainedImage;
use eframe::{egui,NativeOptions};

// Logging
use log::*;
use env_logger::{Builder,WriteStyle};

// Regex
use regex::Regex;

// std
use std::io::Write;
use std::process::exit;
use std::sync::{Arc,Mutex};
use std::{thread,env};
use std::time::Instant;
use std::path::PathBuf;

// Modules
mod ferris;
mod constants;
mod node;
mod disk;
mod status;
mod gupax;
mod p2pool;
mod xmrig;
mod update;
use {ferris::*,constants::*,node::*,disk::*,status::*,update::*};

//---------------------------------------------------------------------------------------------------- Struct + Impl
// The state of the outer main [App].
// See the [State] struct in [state.rs] for the
// actual inner state of the tab settings.
pub struct App {
	// Misc state
	tab: Tab, // What tab are we on?
	width: f32, // Top-level width
	height: f32, // Top-level height
	// State
	og: Arc<Mutex<State>>, // og = Old state to compare against
	state: State, // state = Working state (current settings)
	update: Arc<Mutex<Update>>, // State for update data [update.rs]
	ping: Arc<Mutex<Ping>>, // Ping data found in [node.rs]
	og_node_vec: Vec<(String, Node)>, // Manual Node database
	node_vec: Vec<(String, Node)>, // Manual Node database
	diff: bool, // This bool indicates state changes
	// Error State
	// These values are essentially global variables that
	// indicate if an error message needs to be displayed
	// (it takes up the whole screen with [error_msg] and buttons for ok/quit/etc)
	error_state: ErrorState,
	// Process/update state:
	// Doesn't make sense to save this on disk
	// so it's represented as a bool here.
	p2pool: bool, // Is p2pool online?
	xmrig: bool, // Is xmrig online?
	// State from [--flags]
	no_startup: bool,
	reset: bool,
	// Static stuff
	now: Instant, // Internal timer
	exe: String, // Path for [Gupax] binary
	dir: String, // Directory [Gupax] binary is in
	resolution: Vec2, // Frame resolution
	os: &'static str, // OS
	os_data_path: PathBuf, // OS data path (e.g: ~/.local/share/gupax)
	version: &'static str, // Gupax version
	name_version: String, // [Gupax vX.X.X]
	img: Images, // Custom Struct holding pre-compiled bytes of [Images]
	regex: Regexes, // Custom Struct holding pre-made [Regex]'s
}

impl App {
	fn cc(cc: &eframe::CreationContext<'_>, app: Self) -> Self {
		let resolution = cc.integration_info.window_info.size;
		init_text_styles(&cc.egui_ctx, resolution[0] as f32);
		Self {
			resolution,
			..app
		}
	}

	fn new() -> Self {
		let app = Self {
			tab: Tab::default(),
			ping: Arc::new(Mutex::new(Ping::new())),
			width: 1280.0,
			height: 720.0,
			og: Arc::new(Mutex::new(State::new())),
			state: State::new(),
			update: Arc::new(Mutex::new(Update::new(String::new(), PathBuf::new(), PathBuf::new(), true))),
			og_node_vec: Node::new_vec(),
			node_vec: Node::new_vec(),
			diff: false,
			error_state: ErrorState::new(),
			p2pool: false,
			xmrig: false,
			no_startup: false,
			reset: false,
			now: Instant::now(),
			exe: "".to_string(),
			dir: "".to_string(),
			resolution: Vec2::new(1280.0, 720.0),
			os: OS,
			os_data_path: PathBuf::new(),
			version: GUPAX_VERSION,
			name_version: format!("Gupax {}", GUPAX_VERSION),
			img: Images::new(),
			regex: Regexes::new(),
		};
		// Apply arg state
		let mut app = parse_args(app);
		// Get exe path
		app.exe = match get_exe() {
			Ok(exe) => exe,
			Err(err) => { panic_main(err.to_string()); exit(1); },
		};
		// Get exe directory path
		app.dir = match get_exe_dir() {
			Ok(dir) => dir,
			Err(err) => { panic_main(err.to_string()); exit(1); },
		};
		// Get OS data path
		app.os_data_path = match get_os_data_path() {
			Ok(dir) => dir,
			Err(err) => { panic_main(err.to_string()); exit(1); },
		};
		// Read disk state if no [--reset] arg
		if app.reset == false {
			app.og = match State::get() {
				Ok(toml) => Arc::new(Mutex::new(toml)),
				Err(err) => { panic_main(err.to_string()); exit(1); },
			};
		}
		let mut og = app.og.lock().unwrap(); // Lock [og]
		app.state = og.clone();
		// Get node list
		app.node_vec = Node::get().unwrap();
		// Handle max threads
		og.xmrig.max_threads = num_cpus::get();
		let current = og.xmrig.current_threads;
		let max = og.xmrig.max_threads;
		if current > max {
			og.xmrig.current_threads = max;
		}
		// Apply TOML values to [Update]
		let p2pool_path = og.gupax.absolute_p2pool_path.clone();
		let xmrig_path = og.gupax.absolute_xmrig_path.clone();
		let tor = og.gupax.update_via_tor;
		app.update = Arc::new(Mutex::new(Update::new(app.exe.clone(), p2pool_path, xmrig_path, tor)));
		drop(og); // Unlock [og]
		app
	}
}

//---------------------------------------------------------------------------------------------------- [Tab] Enum + Impl
// The tabs inside [App].
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tab {
	About,
	Status,
	Gupax,
	P2pool,
	Xmrig,
}

impl Default for Tab {
    fn default() -> Self {
        Self::About
    }
}

//---------------------------------------------------------------------------------------------------- [ErrorState] struct
pub struct ErrorState {
	error: bool, // Is there an error?
	msg: &'static str, // What message to display?
	ferris: ErrorFerris, // Which ferris to display?
	buttons: ErrorButtons, // Which buttons to display?
}

impl ErrorState {
	pub fn new() -> Self {
		Self {
			error: false,
			msg: "Unknown Error",
			ferris: ErrorFerris::Oops,
			buttons: ErrorButtons::Okay,
		}
	}

	// Convenience function to set the [App] error state
	pub fn set(&mut self, error: bool, msg: &'static str, ferris: ErrorFerris, buttons: ErrorButtons) {
		*self = Self {
			error,
			msg,
			ferris,
			buttons,
		};
	}
}

//---------------------------------------------------------------------------------------------------- [ErrorButtons] enum
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorButtons {
	YesNo,
	StayQuit,
	Okay,
	Quit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ErrorFerris {
	Oops,
	Error,
	Panic,
}

//---------------------------------------------------------------------------------------------------- [Images] struct
struct Images {
	banner: RetainedImage,
	oops: RetainedImage,
	error: RetainedImage,
	panic: RetainedImage,
}

impl Images {
	fn new() -> Self {
		Self {
			banner: RetainedImage::from_image_bytes("banner.png", BYTES_BANNER).unwrap(),
			oops: RetainedImage::from_image_bytes("oops.png", FERRIS_OOPS).unwrap(),
			error: RetainedImage::from_image_bytes("error.png", FERRIS_ERROR).unwrap(),
			panic: RetainedImage::from_image_bytes("panic.png", FERRIS_PANIC).unwrap(),
		}
	}
}

//---------------------------------------------------------------------------------------------------- [Regexes] struct
#[derive(Clone, Debug)]
pub struct Regexes {
	pub name: Regex,
	pub address: Regex,
	pub ipv4: Regex,
	pub domain: Regex,
	pub port: Regex,
}

impl Regexes {
	fn new() -> Self {
		Regexes {
			name: Regex::new("^[A-Za-z0-9-_]+( [A-Za-z0-9-_]+)*$").unwrap(),
			address: Regex::new("^4[A-Za-z1-9]+$").unwrap(), // This still needs to check for (l, I, o, 0)
			ipv4: Regex::new(r#"^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$"#).unwrap(),
			domain: Regex::new(r#"^(([a-zA-Z]{1})|([a-zA-Z]{1}[a-zA-Z]{1})|([a-zA-Z]{1}[0-9]{1})|([0-9]{1}[a-zA-Z]{1})|([a-zA-Z0-9][a-zA-Z0-9-_]{1,61}[a-zA-Z0-9]))\.([a-zA-Z]{2,6}|[a-zA-Z0-9-]{2,30}\.[a-zA-Z]{2,3})$"#).unwrap(),
			port: Regex::new(r#"^([1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])$"#).unwrap(),
		}
	}
}

//---------------------------------------------------------------------------------------------------- Init functions
fn init_text_styles(ctx: &egui::Context, width: f32) {
	let scale = width / 26.666;
	let mut style = (*ctx.style()).clone();
	style.text_styles = [
		(Small, FontId::new(scale/3.0, Proportional)),
		(Body, FontId::new(scale/2.0, Proportional)),
		(Button, FontId::new(scale/2.0, Proportional)),
		(Monospace, FontId::new(scale/2.0, egui::FontFamily::Monospace)),
		(Heading, FontId::new(scale/1.5, Proportional)),
		(Name("Tab".into()), FontId::new(scale*1.2, Proportional)),
		(Name("Bottom".into()), FontId::new(scale/2.0, Proportional)),
		(Name("MonospaceSmall".into()), FontId::new(scale/2.5, egui::FontFamily::Monospace)),
		(Name("MonospaceLarge".into()), FontId::new(scale/1.5, egui::FontFamily::Monospace)),
	].into();
//	style.visuals.selection.stroke = Stroke { width: 5.0, color: Color32::from_rgb(255, 255, 255) };
//	style.spacing.slider_width = scale;
//	style.spacing.text_edit_width = scale;
//	style.spacing.button_padding = Vec2::new(scale/2.0, scale/2.0);
	ctx.set_style(style);
	ctx.set_pixels_per_point(1.0);
	ctx.request_repaint();
}

//fn init_color(ctx: &egui::Context) {
//	let mut style = (*ctx.style()).clone();
//	style.visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(100, 100, 100);
//	style.visuals.selection.bg_fill = Color32::from_rgb(255, 125, 50);
//	style.visuals.selection.stroke = Stroke { width: 5.0, color: Color32::from_rgb(255, 255, 255) };
//	ctx.set_style(style);
//}

fn init_logger() {
//	#[cfg(debug_assertions)]
	let filter = LevelFilter::Info;
//	#[cfg(not(debug_assertions))]
//	let filter = LevelFilter::Warn;
	use env_logger::fmt::Color;
	Builder::new().format(|buf, record| {
		let level;
		let mut style = buf.style();
		match record.level() {
			Level::Error => { style.set_color(Color::Red); level = "ERROR" },
			Level::Warn => { style.set_color(Color::Yellow); level = "WARN" },
			Level::Info => { style.set_color(Color::White); level = "INFO" },
			Level::Debug => { style.set_color(Color::Blue); level = "DEBUG" },
			Level::Trace => { style.set_color(Color::Magenta); level = "TRACE" },
		};
		writeln!(
			buf,
			"[{}] [{}] [{}:{}] {}",
			style.set_bold(true).value(level),
			buf.style().set_dimmed(true).value(chrono::Local::now().format("%F %T%.3f")),
			buf.style().set_dimmed(true).value(record.file().unwrap_or("???")),
			buf.style().set_dimmed(true).value(record.line().unwrap_or(0)),
			record.args(),
		)
	}).filter_level(filter).write_style(WriteStyle::Always).parse_default_env().format_timestamp_millis().init();
	info!("init_logger() ... OK");
}

fn init_options() -> NativeOptions {
	let mut options = eframe::NativeOptions::default();
	options.min_window_size = Option::from(Vec2::new(854.0, 480.0));
	options.max_window_size = Option::from(Vec2::new(3180.0, 2160.0));
	options.initial_window_size = Option::from(Vec2::new(1280.0, 720.0));
	options.follow_system_theme = false;
	options.default_theme = eframe::Theme::Dark;
	let icon = image::load_from_memory(BYTES_ICON).expect("Failed to read icon bytes").to_rgba8();
	let (icon_width, icon_height) = icon.dimensions();
	options.icon_data = Some(eframe::IconData {
		rgba: icon.into_raw(),
		width: icon_width,
		height: icon_height,
	});
	info!("init_options() ... OK");
	options
}

fn init_auto(app: &App) {
	if app.no_startup {
		info!("[--no-startup] flag passed, skipping init_auto()...");
		return
	} else {
		info!("Starting init_auto()...");
	}
	// [Auto-Update]
	if app.state.gupax.auto_update {
		let path_p2pool = app.og.lock().unwrap().gupax.absolute_p2pool_path.display().to_string();
		let path_xmrig = app.og.lock().unwrap().gupax.absolute_xmrig_path.display().to_string();
		let tor = app.og.lock().unwrap().gupax.update_via_tor;
		app.update.lock().unwrap().path_p2pool = path_p2pool;
		app.update.lock().unwrap().path_xmrig = path_xmrig;
		app.update.lock().unwrap().tor = tor;
		let og = Arc::clone(&app.og);
		let og_ver = Arc::clone(&app.og.lock().unwrap().version);
		let state_ver = Arc::clone(&app.state.version);
		let update = Arc::clone(&app.update);
		let update_thread = Arc::clone(&app.update);
		thread::spawn(move|| {
			info!("Spawning update thread...");
			match Update::start(update_thread, og_ver.clone(), state_ver.clone()) {
				Err(e) => {
					info!("Update ... FAIL ... {}", e);
					*update.lock().unwrap().msg.lock().unwrap() = format!("{} | {}", MSG_FAILED, e);
				},
				_ => {
					info!("Update | Saving state...");
					match State::save(&mut og.lock().unwrap()) {
						Ok(_) => info!("Update ... OK"),
						Err(e) => {
							warn!("Update | Saving state ... FAIL ... {}", e);
							*update.lock().unwrap().msg.lock().unwrap() = format!("Saving new versions into state failed");
						},
					};
				}
			};
			*update.lock().unwrap().updating.lock().unwrap() = false;
		});
	} else {
		info!("Skipping auto-update...");
	}

	// [Auto-Ping]
	let auto_node = app.og.lock().unwrap().p2pool.auto_node;
	let simple = app.og.lock().unwrap().p2pool.simple;
	if auto_node && simple {
		let ping = Arc::clone(&app.ping);
		let og = Arc::clone(&app.og);
		thread::spawn(move|| {
			info!("Spawning ping thread...");
			crate::node::ping(ping, og);
		});
	} else {
		info!("Skipping auto-ping...");
	}
}

fn reset() {
	let mut code = 0;
	info!("Resetting [state.toml]...");
	match State::create_new() {
		Ok(_)  => info!("Resetting [state.toml] ... OK"),
		Err(e) => { error!("Resetting [state.toml] ... FAIL ... {}", e); code = 1; },
	}
	info!("Resetting [node.toml]...");
	match Node::create_new() {
		Ok(_)  => info!("Resetting [node.toml] ... OK"),
		Err(e) => { error!("Resetting [node.toml] ... FAIL ... {}", e); code = 1; },
	}
	match code {
		0 => println!("\nGupax files were reset successfully."),
		_ => println!("\nGupax files reset FAILED."),
	}
	exit(code);
}

//---------------------------------------------------------------------------------------------------- Misc functions
fn parse_args(mut app: App) -> App {
	info!("Parsing CLI arguments...");
	let mut args: Vec<String> = env::args().collect();
	if args.len() == 1 { info!("No args ... OK"); return app } else { args.remove(0); info!("Args ... {:?}", args); }
	// [help/version], exit early
	for arg in &args {
		match arg.as_str() {
			"-h"|"--help"    => { println!("{}", ARG_HELP); exit(0); },
			"-v"|"--version" => {
				println!("Gupax {} [OS: {}, Commit: {}]\n\n{}", GUPAX_VERSION, OS_NAME, &COMMIT[..40], ARG_COPYRIGHT);
				exit(0);
			},
			"-f"|"--ferris" => { println!("{}", FERRIS_ANSI); exit(0); },
			_ => (),
		}
	}
	// Everything else
	for arg in args {
		match arg.as_str() {
			"-l"|"--node-list"  => { info!("Printing node list..."); print_disk_file(File::Node); }
			"-s"|"--state"      => { info!("Printing state..."); print_disk_file(File::State); }
			"-r"|"--reset"      => { reset(); }
			"-n"|"--no-startup" => { app.no_startup = true; }
			_                   => { eprintln!("[Gupax error] Invalid option: [{}]\nFor help, use: [--help]", arg); exit(1); },
		}
	}
	app
}

// Get absolute [Gupax] binary path
pub fn get_exe() -> Result<String, std::io::Error> {
	match std::env::current_exe() {
		Ok(path) => { Ok(path.display().to_string()) },
		Err(err) => { error!("Couldn't get exe basepath PATH"); return Err(err) },
	}
}

// Get absolute [Gupax] directory path
pub fn get_exe_dir() -> Result<String, std::io::Error> {
	match std::env::current_exe() {
		Ok(mut path) => { path.pop(); Ok(path.display().to_string()) },
		Err(err) => { error!("Couldn't get exe basepath PATH"); return Err(err) },
	}
}

// Clean any [gupax_update_.*] directories
// The trailing random bits must be exactly 10 alphanumeric characters
pub fn clean_dir() -> Result<(), anyhow::Error> {
	let regex = Regex::new("^gupax_update_[A-Za-z0-9]{10}$").unwrap();
	for entry in std::fs::read_dir(get_exe_dir()?)? {
		let entry = entry?;
		if ! entry.path().is_dir() { continue }
		if Regex::is_match(&regex, entry.file_name().to_str().ok_or(anyhow::Error::msg("Basename failed"))?) {
			let path = entry.path();
			match std::fs::remove_dir_all(&path) {
				Ok(_) => info!("Remove [{}] ... OK", path.display()),
				Err(e) => warn!("Remove [{}] ... FAIL ... {}", path.display(), e),
			}
		}
	}
	Ok(())
}

// Print disk files to console
fn print_disk_file(file: File) {
	let path = match get_file_path(file) {
		Ok(path) => path,
		Err(e) => { error!("{}", e); exit(1); },
	};
	match std::fs::read_to_string(&path) {
		Ok(string) => { println!("{}", string); exit(0); },
		Err(e) => { error!("{}", e); exit(1); },
	}
}

//---------------------------------------------------------------------------------------------------- [App] frame for [Panic] situations
fn panic_main(error: String) {
	error!("{}", error);
	let options = Panic::options();
	let name = format!("Gupax {}", GUPAX_VERSION);
	eframe::run_native(&name, options, Box::new(|cc| Box::new(Panic::new(cc, error))),);
	exit(1);
}

struct Panic { error_msg: String, }
impl Panic {
	fn options() -> NativeOptions {
		let mut options = eframe::NativeOptions::default();
		let frame = Option::from(Vec2::new(1280.0, 720.0));
		options.min_window_size = frame;
		options.max_window_size = frame;
		options.initial_window_size = frame;
		options.follow_system_theme = false;
		options.default_theme = eframe::Theme::Dark;
		let icon = image::load_from_memory(BYTES_ICON).expect("Failed to read icon bytes").to_rgba8();
		let (icon_width, icon_height) = icon.dimensions();
		options.icon_data = Some(eframe::IconData {
			rgba: icon.into_raw(),
			width: icon_width,
			height: icon_height,
		});
		info!("Panic::options() ... OK");
		options
	}
	fn new(cc: &eframe::CreationContext<'_>, error_msg: String) -> Self {
		let resolution = cc.integration_info.window_info.size;
		init_text_styles(&cc.egui_ctx, resolution[0] as f32);
		Self { error_msg }
	}
}

impl eframe::App for Panic {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::CentralPanel::default().show(ctx, |ui| {
			let width = ui.available_width();
			let height = ui.available_height();
			init_text_styles(ctx, width);
			ui.add_sized([width, height/8.0], Label::new("Gupax has encountered a fatal error:"));
			ui.add_sized([width, height/8.0], Label::new(&self.error_msg));
			ui.add_sized([width, height/3.0], Label::new("Please report to: https://github.com/hinto-janaiyo/gupax/issues"));
			ui.add_sized([width, height/3.0], egui::Button::new("Quit")).clicked() && exit(1)
		});
	}
}

//---------------------------------------------------------------------------------------------------- Main [App] frame
fn main() {
	let now = Instant::now();
	init_logger();
	let options = init_options();
	match clean_dir() {
		Ok(_) => info!("Temporary folder cleanup ... OK"),
		Err(e) => warn!("Could not cleanup [gupax_tmp] folders: {}", e),
	}
	let mut app = App::new();
	app.now = now;
	init_auto(&app);
	info!("Init ... DONE ... Took [{}] seconds", now.elapsed().as_secs_f32());
	eframe::run_native(&app.name_version.clone(), options, Box::new(|cc| Box::new(App::cc(cc, app))),);
}

impl eframe::App for App {
//	pub fn new() -> Self {
//		Self {
//			error: false,
//			msg: String::new(),
//			image: RetainedImage::from_image_bytes("banner.png", FERRIS_ERROR).unwrap(),
//			buttons: ErrorButtons::Okay,
//		}
//	}
	fn on_close_event(&mut self) -> bool {
		if self.state.gupax.ask_before_quit {
			self.error_state.set(true, "", ErrorFerris::Oops, ErrorButtons::StayQuit);
			false
		} else {
			true
		}
	}

	fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
		// *-------*
		// | DEBUG |
		// *-------*
//		crate::node::ping();
//		std::process::exit(0);
//		init_color(ctx);

		// This sets the top level Ui dimensions.
		// Used as a reference for other uis.
		egui::CentralPanel::default().show(ctx, |ui| { self.width = ui.available_width(); self.height = ui.available_height(); });
		// This sets fonts globally depending on the width.
		init_text_styles(ctx, self.width);

		// If [F11] was pressed, reverse [fullscreen] bool
        if ctx.input_mut().consume_key(Modifiers::NONE, Key::F11) {
            let info = frame.info();
            frame.set_fullscreen(!info.window_info.fullscreen);
        }

		// If there's an error, display [ErrorState] on the whole screen until user responds
		if self.error_state.error {
			egui::CentralPanel::default().show(ctx, |ui| {
			ui.vertical_centered(|ui| {
				// Set width/height/font
				let width = self.width;
				let height = self.height/4.0;
				ui.style_mut().override_text_style = Some(Name("MonospaceLarge".into()));

				// Display ferris
				use ErrorFerris::*;
				let ferris = match self.error_state.ferris {
					Oops => &self.img.oops,
					Error => &self.img.error,
					Panic => &self.img.panic,
				};
				ferris.show_max_size(ui, Vec2::new(width, height));

				// Error/Quit screen
				match self.error_state.buttons {
					StayQuit => {
						let mut text = "--- Are you sure you want to quit? ---".to_string();
						if *self.update.lock().unwrap().updating.lock().unwrap() { text = format!("{}\nUpdate is in progress...!", text); }
						if self.p2pool { text = format!("{}\nP2Pool is online...!", text); }
						if self.xmrig { text = format!("{}\nXMRig is online...!", text); }
						ui.add_sized([width, height], Label::new(text))
					},
					_ => ui.add_sized([width, height], Label::new("--- Gupax has encountered an error! ---")),
				};
				ui.add_sized([width, height], Label::new(self.error_state.msg));
				use ErrorButtons::*;
				let height = ui.available_height();

				// Capture [Esc] key
				let esc = ctx.input_mut().consume_key(Modifiers::NONE, Key::Escape);

				match self.error_state.buttons {
					YesNo   => {
						if ui.add_sized([width, height/2.0], egui::Button::new("Yes")).clicked() { self.error_state = ErrorState::new(); }
						// If [Esc] was pressed, assume [No]
				        if esc || ui.add_sized([width, height/2.0], egui::Button::new("No")).clicked() { exit(0); }
					},
					StayQuit => {
						// If [Esc] was pressed, assume [Stay]
				        if esc || ui.add_sized([width, height/2.0], egui::Button::new("Stay")).clicked() {
							self.error_state = ErrorState::new();
						}
						if ui.add_sized([width, height/2.0], egui::Button::new("Quit")).clicked() { exit(0); }
					},
					Okay => if esc || ui.add_sized([width, height], egui::Button::new("Okay")).clicked() { self.error_state = ErrorState::new(); },
					Quit => if ui.add_sized([width, height], egui::Button::new("Quit")).clicked() { exit(0); },
				}
			})});
			return
		}

		// Compare [og == state] and the [node_vec] and enable diff if found.
		// The struct fields are compared directly because [Version]
		// contains Arc<Mutex>'s that cannot be compared easily.
		// They don't need to be compared anyway.
		let og = self.og.lock().unwrap();
		if og.gupax != self.state.gupax || og.p2pool != self.state.p2pool || og.xmrig != self.state.xmrig || self.og_node_vec != self.node_vec {
			self.diff = true;
		} else {
			self.diff = false;
		}
		drop(og);

		// Top: Tabs
		egui::TopBottomPanel::top("top").show(ctx, |ui| {
			let width = (self.width - (SPACE*10.0))/5.0;
			let height = self.height/12.0;
			ui.group(|ui| {
			    ui.add_space(4.0);
				ui.horizontal(|ui| {
					let style = ui.style_mut();
					style.override_text_style = Some(Name("Tab".into()));
					style.visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(100, 100, 100);
					style.visuals.selection.bg_fill = Color32::from_rgb(255, 120, 120);
					style.visuals.selection.stroke = Stroke { width: 5.0, color: Color32::from_rgb(255, 255, 255) };
					if ui.add_sized([width, height], egui::SelectableLabel::new(self.tab == Tab::About, "About")).clicked() { self.tab = Tab::About; }
					ui.separator();
					if ui.add_sized([width, height], egui::SelectableLabel::new(self.tab == Tab::Status, "Status")).clicked() { self.tab = Tab::Status; }
					ui.separator();
					if ui.add_sized([width, height], egui::SelectableLabel::new(self.tab == Tab::Gupax, "Gupax")).clicked() { self.tab = Tab::Gupax; }
					ui.separator();
					if ui.add_sized([width, height], egui::SelectableLabel::new(self.tab == Tab::P2pool, "P2Pool")).clicked() { self.tab = Tab::P2pool; }
					ui.separator();
					if ui.add_sized([width, height], egui::SelectableLabel::new(self.tab == Tab::Xmrig, "XMRig")).clicked() { self.tab = Tab::Xmrig; }
				});
				ui.add_space(4.0);
			});
		});

		// Bottom: app info + state/process buttons
		egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
			let height = self.height/20.0;
			ui.style_mut().override_text_style = Some(Name("Bottom".into()));
			ui.horizontal(|ui| {
				ui.group(|ui| {
					// [Gupax Version] + [OS] + [P2Pool on/off] + [XMRig on/off]
					let width = ((self.width/2.0)/4.0)-(SPACE*2.0);
					ui.add_sized([width, height], Label::new(&*self.name_version));
					ui.separator();
					ui.add_sized([width, height], Label::new(self.os));
					ui.separator();
					if self.p2pool {
						ui.add_sized([width, height], Label::new(RichText::new("P2Pool  ⏺").color(Color32::from_rgb(100, 230, 100))));
					} else {
						ui.add_sized([width, height], Label::new(RichText::new("P2Pool  ⏺").color(Color32::from_rgb(230, 50, 50))));
					}
					ui.separator();
					if self.xmrig {
						ui.add_sized([width, height], Label::new(RichText::new("XMRig  ⏺").color(Color32::from_rgb(100, 230, 100))));
					} else {
						ui.add_sized([width, height], Label::new(RichText::new("XMRig  ⏺").color(Color32::from_rgb(230, 50, 50))));
					}
				});

				ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
				// [Start/Stop/Restart] + [Simple/Advanced] + [Save/Reset]
				let width = (ui.available_width()/3.0)-(SPACE*3.0);
				ui.group(|ui| {
					ui.set_enabled(self.diff);
					let width = width / 2.0;
					if ui.add_sized([width, height], egui::Button::new("Reset")).on_hover_text("Reset changes").clicked() {
						let og = self.og.lock().unwrap().clone();
						self.state.gupax = og.gupax;
						self.state.p2pool = og.p2pool;
						self.state.xmrig = og.xmrig;
						self.node_vec = self.og_node_vec.clone();
					}
					if ui.add_sized([width, height], egui::Button::new("Save")).on_hover_text("Save changes").clicked() {
						let mut og = self.og.lock().unwrap();
						og.gupax = self.state.gupax.clone();
						og.p2pool = self.state.p2pool.clone();
						og.xmrig = self.state.xmrig.clone();
						self.state.save();
						self.og_node_vec = self.node_vec.clone();
					}
				});

				match self.tab {
					Tab::P2pool => {
						ui.group(|ui| {
							let width = width / 1.5;
							if ui.add_sized([width, height], egui::SelectableLabel::new(!self.state.p2pool.simple, "Advanced")).on_hover_text(P2POOL_ADVANCED).clicked() {
								self.state.p2pool.simple = false;
							}
							ui.separator();
							if ui.add_sized([width, height], egui::SelectableLabel::new(self.state.p2pool.simple, "Simple")).on_hover_text(P2POOL_SIMPLE).clicked() {
								self.state.p2pool.simple = true;
							}
						});
						ui.group(|ui| {
							let width = (ui.available_width()/3.0)-5.0;
							if self.p2pool {
								if ui.add_sized([width, height], egui::Button::new("⟲")).on_hover_text("Restart P2Pool").clicked() { self.p2pool = false; }
								if ui.add_sized([width, height], egui::Button::new("⏹")).on_hover_text("Stop P2Pool").clicked() { self.p2pool = false; }
								ui.add_enabled_ui(false, |ui| {
									ui.add_sized([width, height], egui::Button::new("⏺")).on_hover_text("Start P2Pool");
								});
							} else {
								ui.add_enabled_ui(false, |ui| {
									ui.add_sized([width, height], egui::Button::new("⟲")).on_hover_text("Restart P2Pool");
									ui.add_sized([width, height], egui::Button::new("⏹")).on_hover_text("Stop P2Pool");
								});
								if ui.add_sized([width, height], egui::Button::new("⏺")).on_hover_text("Start P2Pool").clicked() { self.p2pool = true; }
							}
						});
					}
					Tab::Xmrig => {
						ui.group(|ui| {
							let width = width / 1.5;
							if ui.add_sized([width, height], egui::SelectableLabel::new(!self.state.xmrig.simple, "Advanced")).clicked() {
								self.state.xmrig.simple = false;
							}
							ui.separator();
							if ui.add_sized([width, height], egui::SelectableLabel::new(self.state.xmrig.simple, "Simple")).clicked() {
								self.state.xmrig.simple = true;
							}
						});
						ui.group(|ui| {
							let width = (ui.available_width()/3.0)-5.0;
							if self.xmrig {
								if ui.add_sized([width, height], egui::Button::new("⟲")).on_hover_text("Restart XMRig").clicked() { self.xmrig = false; }
								if ui.add_sized([width, height], egui::Button::new("⏹")).on_hover_text("Stop XMRig").clicked() { self.xmrig = false; }
								ui.add_enabled_ui(false, |ui| {
									ui.add_sized([width, height], egui::Button::new("⏺")).on_hover_text("Start XMRig");
								});
							} else {
								ui.add_enabled_ui(false, |ui| {
									ui.add_sized([width, height], egui::Button::new("⟲")).on_hover_text("Restart XMRig");
									ui.add_sized([width, height], egui::Button::new("⏹")).on_hover_text("Stop XMRig");
								});
								if ui.add_sized([width, height], egui::Button::new("⏺")).on_hover_text("Start XMRig").clicked() { self.xmrig = true; }
							}
						});
					}
					_ => (),
				}
			});
			});
		});

		// Middle panel, contents of the [Tab]
		egui::CentralPanel::default().show(ctx, |ui| {
			// This sets the Ui dimensions after Top/Bottom are filled
			self.width = ui.available_width();
			self.height = ui.available_height();
			ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
			match self.tab {
				Tab::About => {
					ui.add_space(10.0);
					ui.vertical_centered(|ui| {
						// Display [Gupax] banner at max, 1/4 the available length
						self.img.banner.show_max_size(ui, Vec2::new(self.width, self.height/4.0));
						ui.label("Gupax is a cross-platform GUI for mining");
						ui.hyperlink_to("[Monero]", "https://www.github.com/monero-project/monero");
						ui.label("on the decentralized");
						ui.hyperlink_to("[P2Pool]", "https://www.github.com/SChernykh/p2pool");
						ui.label("using the dedicated");
						ui.hyperlink_to("[XMRig]", "https://www.github.com/xmrig/xmrig");
						ui.label("miner for max hashrate");

						ui.add_space(ui.available_height()/2.0);
						ui.hyperlink_to("Powered by egui", "https://github.com/emilk/egui");
						ui.hyperlink_to(format!("{} {}", GITHUB, "Gupax made by hinto-janaiyo"), "https://www.github.com/hinto-janaiyo/gupax");
						ui.label("egui is licensed under MIT & Apache-2.0");
						ui.label("Gupax, P2Pool, and XMRig are licensed under GPLv3");
					});
				}
				Tab::Status => {
					Status::show(self, self.width, self.height, ctx, ui);
				}
				Tab::Gupax => {
					Gupax::show(&mut self.state.gupax, &self.og, &self.state.version, &self.update, self.width, self.height, ctx, ui);
				}
				Tab::P2pool => {
					P2pool::show(&mut self.state.p2pool, &mut self.node_vec, &self.og, self.p2pool, &self.ping, &self.regex, self.width, self.height, ctx, ui);
				}
				Tab::Xmrig => {
					Xmrig::show(&mut self.state.xmrig, self.width, self.height, ctx, ui);
				}
			}
		});
	}
}