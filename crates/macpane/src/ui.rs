//! The two screens: a connect form with discovered Macs, and the viewer.

use std::sync::mpsc::{self, Receiver};
use std::thread;

use egui::{Color32, ColorImage, Event, TextureHandle, TextureOptions, Vec2};
use macpane_discovery::{Discovery, DiscoveryEvent, Host};
use macpane_rfb::{keysym, Credentials, ServerEvent};

use crate::input;
use crate::session::{self, Session, SessionEvent};

pub struct App {
    screen: Screen,
    form: ConnectForm,
    discovery: Option<Discovery>,
    hosts: Vec<Host>,
}

enum Screen {
    Connect,
    Connecting(Receiver<Result<Session, String>>),
    Viewer(Viewer),
}

#[derive(Default)]
struct ConnectForm {
    host: String,
    username: String,
    password: String,
    error: Option<String>,
}

struct Viewer {
    session: Session,
    texture: Option<TextureHandle>,
    button_mask: u8,
    status: Option<String>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let discovery = match Discovery::start() {
            Ok(d) => Some(d),
            Err(e) => {
                log::warn!("Bonjour discovery unavailable: {e}");
                None
            }
        };
        Self {
            screen: Screen::Connect,
            form: ConnectForm::default(),
            discovery,
            hosts: Vec::new(),
        }
    }

    fn drain_discovery(&mut self) {
        let Some(d) = &self.discovery else { return };
        for ev in d.poll() {
            match ev {
                DiscoveryEvent::Found(h) => {
                    self.hosts.retain(|x| x.name != h.name);
                    self.hosts.push(h);
                    self.hosts.sort();
                }
                DiscoveryEvent::Lost { name } => self.hosts.retain(|x| x.name != name),
            }
        }
    }

    fn start_connect(&mut self, ctx: &egui::Context) {
        let host = self.form.host.trim().to_string();
        if host.is_empty() {
            self.form.error = Some("Enter a host name or address.".into());
            return;
        }
        let creds = Credentials {
            username: Some(self.form.username.clone()).filter(|s| !s.is_empty()),
            password: Some(self.form.password.clone()).filter(|s| !s.is_empty()),
        };
        let (tx, rx) = mpsc::channel();
        let ctx_for_repaint = ctx.clone();
        let ctx_done = ctx.clone();
        thread::spawn(move || {
            let result = session::open(&host, &creds, move || ctx_for_repaint.request_repaint());
            let _ = tx.send(result);
            ctx_done.request_repaint();
        });
        self.form.error = None;
        self.screen = Screen::Connecting(rx);
    }

    fn connect_screen(&mut self, ctx: &egui::Context) {
        self.drain_discovery();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.heading("MacPane");
                ui.label("Connect to a Mac on your local network");
                ui.add_space(16.0);
            });

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.heading("Nearby Macs");
                    if self.discovery.is_none() {
                        ui.colored_label(Color32::YELLOW, "Bonjour discovery is unavailable.");
                    } else if self.hosts.is_empty() {
                        ui.label("Searching...");
                        ui.spinner();
                    }
                    let hosts = self.hosts.clone();
                    for h in hosts {
                        let addr = h.addresses.first().map(|a| a.to_string()).unwrap_or_default();
                        let label = format!("{}\n{} : {}", h.name, addr, h.port);
                        if ui.selectable_label(self.form.host == addr, label).clicked() {
                            self.form.host = if h.port == macpane_rfb::DEFAULT_PORT {
                                addr
                            } else {
                                format!("{addr}:{}", h.port)
                            };
                        }
                    }
                });

                cols[1].group(|ui| {
                    ui.heading("Connection");
                    egui::Grid::new("form").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
                        ui.label("Host");
                        ui.text_edit_singleline(&mut self.form.host);
                        ui.end_row();
                        ui.label("Mac username");
                        ui.text_edit_singleline(&mut self.form.username);
                        ui.end_row();
                        ui.label("Password");
                        ui.add(egui::TextEdit::singleline(&mut self.form.password).password(true));
                        ui.end_row();
                    });
                    ui.small("Username is only needed if the Mac has not enabled the VNC password option.");
                    ui.add_space(8.0);
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Connect").clicked() || enter {
                        self.start_connect(ctx);
                    }
                    if let Some(err) = &self.form.error {
                        ui.colored_label(Color32::LIGHT_RED, err);
                    }
                });
            });
        });
    }

    fn connecting_screen(&mut self, ctx: &egui::Context) {
        let result = match &self.screen {
            Screen::Connecting(rx) => rx.try_recv().ok(),
            _ => None,
        };
        match result {
            Some(Ok(session)) => {
                self.screen = Screen::Viewer(Viewer {
                    session,
                    texture: None,
                    button_mask: 0,
                    status: None,
                });
                ctx.request_repaint();
            }
            Some(Err(e)) => {
                self.form.error = Some(e);
                self.screen = Screen::Connect;
            }
            None => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(64.0);
                        ui.spinner();
                        ui.label(format!("Connecting to {}...", self.form.host));
                    });
                });
            }
        }
    }

    fn viewer_screen(&mut self, ctx: &egui::Context) {
        let Screen::Viewer(v) = &mut self.screen else {
            return;
        };

        // Drain session events.
        let mut disconnected = None;
        let mut dirty = false;
        for ev in v.session.events.try_iter() {
            match ev {
                SessionEvent::Server(ServerEvent::FramebufferUpdated { .. })
                | SessionEvent::Server(ServerEvent::DesktopResized { .. }) => dirty = true,
                SessionEvent::Server(ServerEvent::Bell) => v.status = Some("Bell".into()),
                SessionEvent::Server(ServerEvent::CutText(t)) => ctx.copy_text(t),
                SessionEvent::Disconnected(why) => disconnected = Some(why),
            }
        }
        if let Some(why) = disconnected {
            self.form.error = Some(format!("Disconnected: {why}"));
            self.screen = Screen::Connect;
            return;
        }

        // Upload the framebuffer when it changed. Whole-frame upload for now;
        // partial uploads are tracked as a roadmap item.
        if dirty || v.texture.is_none() {
            let fb = v.session.framebuffer.lock().unwrap();
            let image = ColorImage::from_rgba_unmultiplied([fb.width(), fb.height()], fb.pixels());
            drop(fb);
            match &mut v.texture {
                Some(t) => t.set(image, TextureOptions::LINEAR),
                None => {
                    v.texture = Some(ctx.load_texture("framebuffer", image, TextureOptions::LINEAR))
                }
            }
        }

        let name = v.session.info.name.clone();
        let peer = v.session.peer;
        let mut disconnect_clicked = false;
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{name}  ({peer})"));
                if let Some(s) = &v.status {
                    ui.separator();
                    ui.label(s);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Disconnect").clicked() {
                        disconnect_clicked = true;
                    }
                });
            });
        });
        if disconnect_clicked {
            self.screen = Screen::Connect;
            return;
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::BLACK))
            .show(ctx, |ui| {
                let Some(tex) = &v.texture else { return };
                let fb_size = tex.size_vec2();
                let avail = ui.available_size();
                let scale = (avail.x / fb_size.x).min(avail.y / fb_size.y).min(1.0);
                let shown = fb_size * scale;
                let offset = (avail - shown) * 0.5;

                let (rect, response) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());
                let image_rect = egui::Rect::from_min_size(rect.min + offset, shown);
                ui.painter().image(
                    tex.id(),
                    image_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );

                // Pointer.
                let to_fb = |p: egui::Pos2| -> (u16, u16) {
                    let rel = (p - image_rect.min) / scale;
                    (
                        rel.x.clamp(0.0, fb_size.x - 1.0) as u16,
                        rel.y.clamp(0.0, fb_size.y - 1.0) as u16,
                    )
                };
                if let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) {
                    if image_rect.contains(pos) || v.button_mask != 0 {
                        let mut mask = v.button_mask;
                        ui.input(|i| {
                            for b in [
                                egui::PointerButton::Primary,
                                egui::PointerButton::Secondary,
                                egui::PointerButton::Middle,
                            ] {
                                let bit = input::button_bit(b);
                                if i.pointer.button_down(b) {
                                    mask |= bit;
                                } else {
                                    mask &= !bit;
                                }
                            }
                        });
                        let (x, y) = to_fb(pos);
                        let mut w = v.session.writer.lock().unwrap();
                        let _ = w.pointer_event(mask, x, y);
                        v.button_mask = mask;

                        let scroll: Vec2 = ui.input(|i| i.raw_scroll_delta);
                        for bit in input::scroll_pulses(scroll.y, scroll.x) {
                            let _ = w.pointer_event(mask | bit, x, y);
                            let _ = w.pointer_event(mask, x, y);
                        }
                    }
                }

                // Keyboard. Capture focus so egui does not eat keys.
                if response.clicked() || response.hovered() {
                    response.request_focus();
                }
                if response.has_focus() {
                    let events: Vec<Event> = ui.input(|i| i.events.clone());
                    let mut w = v.session.writer.lock().unwrap();
                    for ev in events {
                        match ev {
                            Event::Text(text) => {
                                for c in text.chars() {
                                    let ks = keysym::from_char(c);
                                    let _ = w.key_event(ks, true);
                                    let _ = w.key_event(ks, false);
                                }
                            }
                            Event::Key {
                                key,
                                pressed,
                                modifiers,
                                ..
                            } => {
                                // Shortcuts (Cmd/Ctrl/Alt + letter) do not arrive as
                                // Text, so synthesise them here from the key name.
                                let has_shortcut_mod =
                                    modifiers.command || modifiers.ctrl || modifiers.alt;
                                let ks = input::keysym_for(key).or_else(|| {
                                    if has_shortcut_mod {
                                        key.symbol_or_name()
                                            .chars()
                                            .next()
                                            .map(|c| keysym::from_char(c.to_ascii_lowercase()))
                                    } else {
                                        None
                                    }
                                });
                                let Some(ks) = ks else { continue };
                                let mods = input::modifier_keysyms(modifiers);
                                if pressed {
                                    for m in &mods {
                                        let _ = w.key_event(*m, true);
                                    }
                                    let _ = w.key_event(ks, true);
                                } else {
                                    let _ = w.key_event(ks, false);
                                    for m in mods.iter().rev() {
                                        let _ = w.key_event(*m, false);
                                    }
                                }
                            }
                            Event::Paste(text) => {
                                let _ = w.cut_text(&text);
                            }
                            _ => {}
                        }
                    }
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::Connect => {
                self.connect_screen(ctx);
                // Poll discovery a few times a second while on this screen.
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
            Screen::Connecting(_) => {
                self.connecting_screen(ctx);
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Screen::Viewer(_) => self.viewer_screen(ctx),
        }
    }
}
