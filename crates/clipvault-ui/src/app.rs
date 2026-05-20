use anyhow::Result;
use gtk::prelude::*;
use gtk4 as gtk;

const APP_ID: &str = "io.github.radhey.clipvault";

pub(crate) fn run() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("list") {
        return crate::cli::cmd_list(&args[1..]);
    }

    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        anyhow::bail!("Clipvault overlay requires Wayland");
    }

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        if let Err(err) = crate::window::build_ui(app) {
            eprintln!("clipvault: {err:#}");
            app.quit();
        }
    });
    app.run();
    Ok(())
}
