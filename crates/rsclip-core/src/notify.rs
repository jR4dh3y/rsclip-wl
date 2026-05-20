use std::os::unix::net::UnixDatagram;

use crate::config::RsclipPaths;

pub const CHANGE_EVENT: &[u8] = b"changed";

pub fn notify_changed(paths: &RsclipPaths) {
    if let Ok(socket) = UnixDatagram::unbound() {
        let _ = socket.send_to(CHANGE_EVENT, &paths.socket_path);
    }
}
