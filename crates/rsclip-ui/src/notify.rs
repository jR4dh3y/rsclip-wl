use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use rsclip_core::notify::CHANGE_EVENT;
use gtk4 as gtk;

use crate::actions::refresh::refresh_entries_if_changed;
use crate::actions::set_footer;
use crate::state::AppState;

pub(crate) fn install_change_listener(state: &Rc<AppState>, socket_path: &Path) -> Result<()> {
    match std::fs::remove_file(socket_path) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "removing stale notification socket {}",
                    socket_path.display()
                )
            });
        }
    }

    let socket = UnixDatagram::bind(socket_path)
        .with_context(|| format!("binding notification socket {}", socket_path.display()))?;
    socket
        .set_nonblocking(true)
        .context("setting notification socket nonblocking")?;
    let fd = socket.as_raw_fd();

    {
        let state = Rc::clone(state);
        gtk::glib::source::unix_fd_add_local(fd, gtk::glib::IOCondition::IN, move |_, _| {
            let mut buf = [0_u8; 64];
            let mut changed = false;
            loop {
                match socket.recv(&mut buf) {
                    Ok(size) => {
                        changed |= &buf[..size] == CHANGE_EVENT;
                    }
                    Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                    Err(err) => {
                        set_footer(&state, &format!("Notification listener failed: {err}"));
                        return gtk::glib::ControlFlow::Break;
                    }
                }
            }

            if changed && let Err(err) = refresh_entries_if_changed(&state) {
                set_footer(&state, &format!("Refresh failed: {err:#}"));
            }
            gtk::glib::ControlFlow::Continue
        });
    }

    Ok(())
}
