use std::os::unix::net::UnixDatagram;

use crate::config::RsclipPaths;

pub const CHANGE_EVENT: &[u8] = b"changed";
pub const FAVICON_EVENT: &[u8] = b"favicons";

pub fn notify_changed(paths: &RsclipPaths) {
    notify(paths, CHANGE_EVENT);
}

pub fn notify_favicons_changed(paths: &RsclipPaths) {
    notify(paths, FAVICON_EVENT);
}

fn notify(paths: &RsclipPaths, event: &[u8]) {
    if let Ok(socket) = UnixDatagram::unbound() {
        let _ = socket.send_to(event, &paths.socket_path);
    }
}
