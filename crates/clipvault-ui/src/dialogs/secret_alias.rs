use std::rc::Rc;

use anyhow::Result;
use gtk::gdk;
use gtk::prelude::*;
use gtk4 as gtk;

use crate::actions::set_footer;
use crate::state::AppState;

pub(crate) fn prompt_secret_alias<F>(
    state: &Rc<AppState>,
    parent: &gtk::Window,
    title: &str,
    default_alias: &str,
    on_accept: F,
) where
    F: Fn(&Rc<AppState>, String) -> Result<()> + 'static,
{
    let Some(root) = parent
        .child()
        .and_then(|child| child.downcast::<gtk::Overlay>().ok())
    else {
        set_footer(state, "Secret prompt failed: overlay root is unavailable");
        return;
    };

    let scrim = gtk::Box::new(gtk::Orientation::Vertical, 0);
    scrim.add_css_class("inline-dialog-scrim");
    scrim.set_halign(gtk::Align::Fill);
    scrim.set_valign(gtk::Align::Fill);
    scrim.set_hexpand(true);
    scrim.set_vexpand(true);

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.add_css_class("inline-dialog");
    panel.set_halign(gtk::Align::Center);
    panel.set_valign(gtk::Align::Center);
    panel.set_width_request(320);

    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("inline-dialog-title");
    title_label.set_xalign(0.0);
    panel.append(&title_label);

    let alias = gtk::Entry::new();
    alias.add_css_class("inline-dialog-entry");
    alias.set_text(default_alias);
    panel.append(&alias);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.add_css_class("inline-dialog-actions");
    actions.set_halign(gtk::Align::End);

    let cancel = gtk::Button::with_label("Cancel");
    let save = gtk::Button::with_label("Save");
    save.add_css_class("primary-button");
    actions.append(&cancel);
    actions.append(&save);
    panel.append(&actions);

    root.add_overlay(&scrim);
    root.add_overlay(&panel);
    *state.prompt_active.borrow_mut() = true;

    let on_accept = Rc::new(on_accept);
    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        cancel.connect_clicked(move |_| {
            *state.prompt_active.borrow_mut() = false;
            root.remove_overlay(&panel);
            root.remove_overlay(&scrim);
            state.search_entry.grab_focus();
        });
    }

    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        let alias = alias.clone();
        let on_accept = Rc::clone(&on_accept);
        save.connect_clicked(move |_| {
            let alias = alias.text().to_string();
            if let Err(err) = on_accept(&state, alias) {
                set_footer(&state, &format!("Secret failed: {err:#}"));
                return;
            }
            *state.prompt_active.borrow_mut() = false;
            root.remove_overlay(&panel);
            root.remove_overlay(&scrim);
            state.search_entry.grab_focus();
        });
    }

    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        let on_accept = Rc::clone(&on_accept);
        alias.connect_activate(move |entry| {
            let alias = entry.text().to_string();
            if let Err(err) = on_accept(&state, alias) {
                set_footer(&state, &format!("Secret failed: {err:#}"));
                return;
            }
            *state.prompt_active.borrow_mut() = false;
            root.remove_overlay(&panel);
            root.remove_overlay(&scrim);
            state.search_entry.grab_focus();
        });
    }

    let controller = gtk::EventControllerKey::new();
    {
        let root = root.clone();
        let panel = panel.clone();
        let scrim = scrim.clone();
        let state = Rc::clone(state);
        controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                *state.prompt_active.borrow_mut() = false;
                root.remove_overlay(&panel);
                root.remove_overlay(&scrim);
                state.search_entry.grab_focus();
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
    }
    alias.add_controller(controller);

    alias.grab_focus();
}
