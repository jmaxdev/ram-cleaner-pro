use crate::monitor::SharedMonitorState;
use crate::ui::tray::TrayManager;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tray_icon::menu::MenuEvent;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CurrentScreen {
    Home,
    Menu,
    Settings,
    History,
    Stats,
    About,
}

#[allow(dead_code)]
pub struct PurgeRecord {
    pub time_str: String,
    pub bytes_freed: u64,
    pub mb_freed: u64,
    pub processes: usize,
    pub levels: String,
}

pub struct RamPurgerApp {
    pub monitor_state: SharedMonitorState,
    pub is_visible: Arc<AtomicBool>,
    pub status_message: Option<String>,
    pub tray_mgr: Option<TrayManager>,
    pub current_screen: CurrentScreen,
    pub purge_history: Vec<PurgeRecord>,
    pub last_purge_anim: Option<Instant>,
}

impl RamPurgerApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        monitor_state: SharedMonitorState,
        is_visible: Arc<AtomicBool>,
    ) -> Self {
        let tray_mgr = match TrayManager::new() {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("Warning creating tray icon: {}", e);
                None
            }
        };

        Self {
            monitor_state,
            is_visible,
            status_message: None,
            tray_mgr,
            current_screen: CurrentScreen::Home,
            purge_history: Vec::new(),
            last_purge_anim: None,
        }
    }
}

fn draw_hamburger_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let center = rect.center();
    let width = 14.0;
    let stroke = egui::Stroke::new(2.0, color);
    painter.line_segment([center + egui::vec2(-width / 2.0, -4.5), center + egui::vec2(width / 2.0, -4.5)], stroke);
    painter.line_segment([center + egui::vec2(-width / 2.0, 0.0), center + egui::vec2(width / 2.0, 0.0)], stroke);
    painter.line_segment([center + egui::vec2(-width / 2.0, 4.5), center + egui::vec2(width / 2.0, 4.5)], stroke);
}

fn draw_back_chevron(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let center = rect.center();
    let stroke = egui::Stroke::new(2.2, color);
    painter.line_segment([center + egui::vec2(3.0, -5.5), center + egui::vec2(-3.5, 0.0)], stroke);
    painter.line_segment([center + egui::vec2(-3.5, 0.0), center + egui::vec2(3.0, 5.5)], stroke);
}

fn draw_right_chevron(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let center = rect.center();
    let stroke = egui::Stroke::new(2.0, color);
    painter.line_segment([center + egui::vec2(-3.0, -5.5), center + egui::vec2(3.5, 0.0)], stroke);
    painter.line_segment([center + egui::vec2(3.5, 0.0), center + egui::vec2(-3.0, 5.5)], stroke);
}

fn draw_shield_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let center = rect.center();
    let stroke = egui::Stroke::new(1.8, color);
    let p1 = center + egui::vec2(-5.0, -6.0);
    let p2 = center + egui::vec2(5.0, -6.0);
    let p3 = center + egui::vec2(5.0, -1.0);
    let p4 = center + egui::vec2(0.0, 5.5);
    let p5 = center + egui::vec2(-5.0, -1.0);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p4], stroke);
    painter.line_segment([p4, p5], stroke);
    painter.line_segment([p5, p1], stroke);
}

impl eframe::App for RamPurgerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(200));

        let mut monitor = self.monitor_state.write();

        if let Some(ref tray) = self.tray_mgr {
            let stats = monitor.last_stats;
            tray.update_tooltip(&format!(
                "RAM Purger - Usage: {:.1}% ({} / {})",
                stats.usage_percent,
                crate::purger::format_bytes(stats.used_bytes),
                crate::purger::format_bytes(stats.total_bytes)
            ));

            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id == tray.item_purge_now.id() {
                    let res = monitor.force_purge();
                    self.purge_history.push(PurgeRecord {
                        time_str: current_time_string(),
                        bytes_freed: res.bytes_freed,
                        mb_freed: res.mb_freed,
                        processes: res.processes_trimmed,
                        levels: res.levels_executed.join(", "),
                    });
                    self.status_message = Some(format!(
                        "Freed {} (Processes: {})",
                        crate::purger::format_bytes(res.bytes_freed),
                        res.processes_trimmed
                    ));
                    self.last_purge_anim = Some(Instant::now());
                } else if event.id == tray.item_toggle_gui.id() {
                    let curr = self.is_visible.load(Ordering::Relaxed);
                    let new_vis = !curr;
                    self.is_visible.store(new_vis, Ordering::Relaxed);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(new_vis));
                } else if event.id == tray.item_toggle_auto.id() {
                    monitor.config.auto_purge_enabled = !monitor.config.auto_purge_enabled;
                    let _ = monitor.config.save();
                } else if event.id == tray.item_quit.id() {
                    std::process::exit(0);
                }
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.is_visible.store(false, Ordering::Relaxed);
        }

        if let Some(res) = monitor.update() {
            self.purge_history.push(PurgeRecord {
                time_str: current_time_string(),
                bytes_freed: res.bytes_freed,
                mb_freed: res.mb_freed,
                processes: res.processes_trimmed,
                levels: res.levels_executed.join(", "),
            });
            self.status_message = Some(format!(
                "Auto-purge: {} freed",
                crate::purger::format_bytes(res.bytes_freed)
            ));
            self.last_purge_anim = Some(Instant::now());
        }

        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.window_fill = egui::Color32::from_rgb(11, 14, 23);
        style.visuals.panel_fill = egui::Color32::from_rgb(11, 14, 23);

        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.open.bg_stroke = egui::Stroke::NONE;

        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(22, 28, 44);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(30, 40, 65);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 229, 255);
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(0, 229, 255);

        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.8, egui::Color32::from_rgb(180, 190, 210));
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 229, 255));
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 208, 0));

        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

            match self.current_screen {
                CurrentScreen::Home => {
                    let is_admin = crate::purger::is_admin();
                    ui.horizontal(|ui| {
                        let badge_bg = if is_admin {
                            egui::Color32::from_rgb(15, 30, 45)
                        } else {
                            egui::Color32::from_rgb(45, 20, 20)
                        };
                        let badge_color = if is_admin {
                            egui::Color32::from_rgb(0, 229, 255)
                        } else {
                            egui::Color32::from_rgb(239, 68, 68)
                        };
                        let badge_text = if is_admin { "ADMIN" } else { "NO ADMIN" };
                        let badge_width = if is_admin { 66.0 } else { 82.0 };

                        let (badge_rect, _) = ui.allocate_exact_size(egui::vec2(badge_width, 24.0), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(badge_rect, 6.0, badge_bg);
                        draw_shield_icon(painter, egui::Rect::from_min_size(badge_rect.min + egui::vec2(4.0, 2.0), egui::vec2(14.0, 20.0)), badge_color);
                        painter.text(
                            badge_rect.min + egui::vec2(22.0, 12.0),
                            egui::Align2::LEFT_CENTER,
                            badge_text,
                            egui::FontId::proportional(11.0),
                            badge_color,
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (menu_rect, menu_resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                            let menu_fill = if menu_resp.hovered() {
                                egui::Color32::from_rgb(25, 33, 52)
                            } else {
                                egui::Color32::from_rgb(18, 24, 38)
                            };
                            let painter = ui.painter();
                            painter.rect_filled(menu_rect, 6.0, menu_fill);
                            draw_hamburger_icon(painter, menu_rect, egui::Color32::WHITE);

                            if menu_resp.clicked() {
                                self.current_screen = CurrentScreen::Menu;
                            }

                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.label(
                                    egui::RichText::new("RAM PURGER")
                                        .size(17.0)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                );
                            });
                        });
                    });

                    ui.add_space(12.0);

                    if !is_admin {
                        ui.vertical_centered(|ui| {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(42, 18, 22))
                                .stroke(egui::Stroke::new(1.2, egui::Color32::from_rgb(239, 68, 68)))
                                .rounding(8.0)
                                .inner_margin(egui::vec2(12.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("⚠ WARNING: ELEVATED PRIVILEGES REQUIRED")
                                            .size(12.0)
                                            .strong()
                                            .color(egui::Color32::from_rgb(239, 68, 68)),
                                    );
                                    ui.add_space(3.0);
                                    ui.label(
                                        egui::RichText::new("Native NT Kernel purge functions require elevation. Please restart using \"Run as Administrator\".")
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(220, 220, 220)),
                                    );
                                });
                        });
                    }

                    ui.add_space(20.0);

                    let stats = monitor.last_stats;
                    let usage_pct = stats.usage_percent;

                    ui.vertical_centered(|ui| {
                        let dial_size = egui::vec2(220.0, 220.0);
                        let (rect, response) = ui.allocate_exact_size(dial_size, egui::Sense::click());

                        let is_animating = self.last_purge_anim
                            .map(|t| t.elapsed() < Duration::from_millis(800))
                            .unwrap_or(false);

                        let stroke_color = if is_animating {
                            egui::Color32::from_rgb(0, 229, 255)
                        } else {
                            egui::Color32::from_rgb(255, 208, 0)
                        };

                        let painter = ui.painter();
                        let center = rect.center();
                        let radius = rect.width() / 2.0 - 4.0;

                        painter.circle_stroke(
                            center,
                            radius,
                            egui::Stroke::new(2.5, stroke_color.linear_multiply(0.4)),
                        );
                        painter.circle_stroke(
                            center,
                            radius - 8.0,
                            egui::Stroke::new(3.0, stroke_color),
                        );
                        let fill_color = if response.hovered() {
                            egui::Color32::from_rgb(22, 28, 46)
                        } else {
                            egui::Color32::from_rgb(16, 21, 35)
                        };
                        painter.circle_filled(center, radius - 12.0, fill_color);

                        let text_color = if is_animating {
                            egui::Color32::from_rgb(0, 229, 255)
                        } else {
                            egui::Color32::from_rgb(255, 208, 0)
                        };

                        let button_text = if is_animating { "PURGING..." } else { "PURGE" };

                        painter.text(
                            center - egui::vec2(0.0, 14.0),
                            egui::Align2::CENTER_CENTER,
                            button_text,
                            egui::FontId::proportional(28.0),
                            text_color,
                        );

                        painter.text(
                            center + egui::vec2(0.0, 24.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{:.1}% RAM", usage_pct),
                            egui::FontId::proportional(15.0),
                            egui::Color32::from_rgb(180, 190, 210),
                        );

                        if response.clicked() {
                            let res = monitor.force_purge();
                            self.purge_history.push(PurgeRecord {
                                time_str: current_time_string(),
                                bytes_freed: res.bytes_freed,
                                mb_freed: res.mb_freed,
                                processes: res.processes_trimmed,
                                levels: res.levels_executed.join(", "),
                            });
                            self.status_message = Some(format!(
                                "Freed {} (Processes: {})",
                                crate::purger::format_bytes(res.bytes_freed),
                                res.processes_trimmed
                            ));
                            self.last_purge_anim = Some(Instant::now());
                        }
                    });

                    ui.add_space(30.0);

                    if let Some(ref msg) = self.status_message {
                        ui.vertical_centered(|ui| {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(19, 24, 38))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 50, 75)))
                                .rounding(16.0)
                                .inner_margin(egui::vec2(16.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("OK  {}", msg))
                                            .size(13.0)
                                            .strong()
                                            .color(egui::Color32::from_rgb(0, 229, 255)),
                                    );
                                });
                        });
                    }
                }

                CurrentScreen::Menu => {
                    ui.horizontal(|ui| {
                        let (back_rect, back_resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                        let back_fill = if back_resp.hovered() {
                            egui::Color32::from_rgb(25, 33, 52)
                        } else {
                            egui::Color32::from_rgb(18, 24, 38)
                        };
                        let painter = ui.painter();
                        painter.rect_filled(back_rect, 6.0, back_fill);
                        draw_back_chevron(painter, back_rect, egui::Color32::WHITE);

                        if back_resp.clicked() {
                            self.current_screen = CurrentScreen::Home;
                        }

                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(
                                egui::RichText::new("MAIN MENU")
                                    .size(16.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                    });

                    ui.separator();
                    ui.add_space(8.0);

                    let total_count = self.purge_history.len();
                    let max_freed = self.purge_history.iter().map(|r| r.bytes_freed).max().unwrap_or(0);
                    let avg_freed = if total_count > 0 {
                        self.purge_history.iter().map(|r| r.bytes_freed).sum::<u64>() / total_count as u64
                    } else {
                        0
                    };

                    ui.columns(3, |cols| {
                        cols[0].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("PURGES").small().color(egui::Color32::from_rgb(130, 140, 160)));
                            ui.label(egui::RichText::new(format!("{}", total_count)).size(24.0).strong().color(egui::Color32::WHITE));
                        });
                        cols[1].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("MAX FREED").small().color(egui::Color32::from_rgb(130, 140, 160)));
                            ui.label(egui::RichText::new(crate::purger::format_bytes(max_freed)).size(24.0).strong().color(egui::Color32::from_rgb(0, 229, 255)));
                        });
                        cols[2].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("AVERAGE").small().color(egui::Color32::from_rgb(130, 140, 160)));
                            ui.label(egui::RichText::new(crate::purger::format_bytes(avg_freed)).size(24.0).strong().color(egui::Color32::from_rgb(255, 208, 0)));
                        });
                    });

                    ui.add_space(14.0);

                    let stats = monitor.last_stats;
                    let (rect_ram, resp_ram) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), egui::Sense::click());
                    let fill_ram = if resp_ram.hovered() {
                        egui::Color32::from_rgb(22, 28, 44)
                    } else {
                        egui::Color32::from_rgb(15, 19, 32)
                    };

                    let painter = ui.painter();
                    painter.rect_filled(rect_ram, 10.0, fill_ram);
                    painter.rect_stroke(rect_ram, 10.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 38, 58)));

                    painter.text(
                        rect_ram.left_center() + egui::vec2(16.0, -10.0),
                        egui::Align2::LEFT_CENTER,
                        format!("RAM Used: {} / {}", crate::purger::format_bytes(stats.used_bytes), crate::purger::format_bytes(stats.total_bytes)),
                        egui::FontId::proportional(14.0),
                        egui::Color32::from_rgb(0, 229, 255),
                    );
                    painter.text(
                        rect_ram.left_center() + egui::vec2(16.0, 10.0),
                        egui::Align2::LEFT_CENTER,
                        format!("RAM Free: {}", crate::purger::format_bytes(stats.free_bytes)),
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_rgb(130, 140, 160),
                    );
                    draw_right_chevron(painter, egui::Rect::from_min_size(rect_ram.right_center() - egui::vec2(24.0, 12.0), egui::vec2(16.0, 24.0)), egui::Color32::from_rgb(0, 229, 255));

                    if resp_ram.clicked() {
                        self.current_screen = CurrentScreen::Stats;
                    }

                    ui.add_space(14.0);

                    let nav_items = [
                        ("PURGE CONFIGURATION", CurrentScreen::Settings),
                        ("RESULT HISTORY", CurrentScreen::History),
                        ("MEMORY STATISTICS", CurrentScreen::Stats),
                        ("ABOUT RAM PURGER", CurrentScreen::About),
                    ];

                    for (label, target_screen) in nav_items {
                        let (rect_item, resp_item) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), egui::Sense::click());
                        let fill_item = if resp_item.hovered() {
                            egui::Color32::from_rgb(22, 28, 44)
                        } else {
                            egui::Color32::from_rgb(15, 19, 32)
                        };

                        let painter = ui.painter();
                        painter.rect_filled(rect_item, 8.0, fill_item);
                        painter.rect_stroke(rect_item, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 38, 58)));

                        painter.text(
                            rect_item.left_center() + egui::vec2(16.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(13.5),
                            egui::Color32::WHITE,
                        );

                        draw_right_chevron(painter, egui::Rect::from_min_size(rect_item.right_center() - egui::vec2(24.0, 12.0), egui::vec2(16.0, 24.0)), egui::Color32::from_rgb(180, 190, 210));

                        if resp_item.clicked() {
                            self.current_screen = target_screen;
                        }

                        ui.add_space(6.0);
                    }
                }

                CurrentScreen::Settings => {
                    ui.horizontal(|ui| {
                        let (back_rect, back_resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                        let back_fill = if back_resp.hovered() {
                            egui::Color32::from_rgb(25, 33, 52)
                        } else {
                            egui::Color32::from_rgb(18, 24, 38)
                        };
                        let painter = ui.painter();
                        painter.rect_filled(back_rect, 6.0, back_fill);
                        draw_back_chevron(painter, back_rect, egui::Color32::WHITE);

                        if back_resp.clicked() {
                            self.current_screen = CurrentScreen::Menu;
                        }

                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(
                                egui::RichText::new("PURGE CONFIGURATION")
                                    .size(15.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                    });

                    ui.separator();
                    ui.add_space(10.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(15, 19, 32))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 38, 58)))
                        .rounding(10.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            let mut config_changed = false;

                            if ui.checkbox(&mut monitor.config.auto_purge_enabled, "Enable Automatic Purge").changed() {
                                config_changed = true;
                            }

                            ui.add_space(12.0);

                            ui.add_enabled_ui(monitor.config.auto_purge_enabled, |ui| {
                                ui.label(egui::RichText::new("RAM THRESHOLD (%):").small().strong().color(egui::Color32::from_rgb(0, 229, 255)));
                                if ui.add(egui::Slider::new(&mut monitor.config.threshold_percent, 20.0..=98.0).suffix("%")).changed() {
                                    config_changed = true;
                                }

                                ui.add_space(10.0);

                                ui.label(egui::RichText::new("TIME INTERVAL:").small().strong().color(egui::Color32::from_rgb(0, 229, 255)));
                                if ui.add(egui::Slider::new(&mut monitor.config.interval_minutes, 7..=120).suffix(" min")).changed() {
                                    config_changed = true;
                                }

                                ui.add_space(10.0);

                                ui.label(egui::RichText::new("SAFETY COOLDOWN:").small().strong().color(egui::Color32::from_rgb(0, 229, 255)));
                                if ui.add(egui::Slider::new(&mut monitor.config.cooldown_seconds, 30..=300).suffix(" sec")).changed() {
                                    config_changed = true;
                                }
                            });

                            ui.add_space(18.0);
                            ui.separator();
                            ui.add_space(10.0);

                            ui.label(egui::RichText::new("ENABLED PURGE LEVELS").small().strong().color(egui::Color32::WHITE));
                            ui.add_space(8.0);

                            if ui.checkbox(&mut monitor.config.purge_working_sets, "1. Working Sets (Processes)").changed() {
                                config_changed = true;
                            }
                            ui.add_space(4.0);
                            if ui.checkbox(&mut monitor.config.purge_standby_list, "2. Standby Memory List").changed() {
                                config_changed = true;
                            }
                            ui.add_space(4.0);
                            if ui.checkbox(&mut monitor.config.purge_modified_list, "3. Modified Page List").changed() {
                                config_changed = true;
                            }
                            ui.add_space(4.0);
                            if ui.checkbox(&mut monitor.config.purge_system_cache, "4. System File Cache").changed() {
                                config_changed = true;
                            }

                            ui.add_space(18.0);
                            ui.separator();
                            ui.add_space(10.0);

                            ui.label(egui::RichText::new("SOFTWARE UPDATES").small().strong().color(egui::Color32::WHITE));
                            ui.add_space(8.0);

                            if ui.checkbox(&mut monitor.config.check_updates_enabled, "Buscar actualizaciones automáticamente (cada 3 hs)").changed() {
                                config_changed = true;
                            }
                            ui.add_space(8.0);

                            let check_btn_text = if monitor.is_checking_update {
                                "Buscando actualizaciones..."
                            } else {
                                "Comprobar actualizaciones ahora"
                            };

                            if ui.add_enabled(!monitor.is_checking_update, egui::Button::new(egui::RichText::new(check_btn_text).small().color(egui::Color32::from_rgb(0, 229, 255)))).clicked() {
                                monitor.check_update_async();
                            }

                            if let Some(ref err) = monitor.update_error {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(format!("⚠ {}", err)).small().color(egui::Color32::from_rgb(239, 68, 68)));
                            }

                            if config_changed {
                                let _ = monitor.config.save();
                            }
                        });
                }

                CurrentScreen::History => {
                    ui.horizontal(|ui| {
                        let (back_rect, back_resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                        let back_fill = if back_resp.hovered() {
                            egui::Color32::from_rgb(25, 33, 52)
                        } else {
                            egui::Color32::from_rgb(18, 24, 38)
                        };
                        let painter = ui.painter();
                        painter.rect_filled(back_rect, 6.0, back_fill);
                        draw_back_chevron(painter, back_rect, egui::Color32::WHITE);

                        if back_resp.clicked() {
                            self.current_screen = CurrentScreen::Menu;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("CLEAR").small().strong().color(egui::Color32::from_rgb(239, 68, 68))).clicked() {
                                self.purge_history.clear();
                            }
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.label(
                                    egui::RichText::new("RESULT HISTORY")
                                        .size(15.0)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                );
                            });
                        });
                    });

                    ui.separator();
                    ui.add_space(8.0);

                    let total_count = self.purge_history.len();
                    let max_freed = self.purge_history.iter().map(|r| r.bytes_freed).max().unwrap_or(0);
                    let avg_freed = if total_count > 0 {
                        self.purge_history.iter().map(|r| r.bytes_freed).sum::<u64>() / total_count as u64
                    } else {
                        0
                    };

                    ui.columns(3, |cols| {
                        cols[0].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("PURGES").small().color(egui::Color32::from_rgb(130, 140, 160)));
                            ui.label(egui::RichText::new(format!("{}", total_count)).size(24.0).strong().color(egui::Color32::WHITE));
                        });
                        cols[1].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("MAX FREED").small().color(egui::Color32::from_rgb(130, 140, 160)));
                            ui.label(egui::RichText::new(crate::purger::format_bytes(max_freed)).size(24.0).strong().color(egui::Color32::from_rgb(0, 229, 255)));
                        });
                        cols[2].vertical_centered(|ui| {
                            ui.label(egui::RichText::new("AVERAGE").small().color(egui::Color32::from_rgb(130, 140, 160)));
                            ui.label(egui::RichText::new(crate::purger::format_bytes(avg_freed)).size(24.0).strong().color(egui::Color32::from_rgb(255, 208, 0)));
                        });
                    });

                    ui.add_space(12.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(19, 24, 38))
                        .inner_margin(egui::vec2(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.columns(3, |cols| {
                                cols[0].label(egui::RichText::new("Time").small().strong().color(egui::Color32::WHITE));
                                cols[1].label(egui::RichText::new("Freed").small().strong().color(egui::Color32::from_rgb(0, 229, 255)));
                                cols[2].label(egui::RichText::new("Processes").small().strong().color(egui::Color32::from_rgb(255, 208, 0)));
                            });
                        });

                    ui.add_space(4.0);

                    if self.purge_history.is_empty() {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("No purge records available yet").color(egui::Color32::GRAY));
                        });
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(340.0)
                            .show(ui, |ui| {
                                for rec in self.purge_history.iter().rev() {
                                    ui.horizontal(|ui| {
                                        ui.columns(3, |cols| {
                                            cols[0].label(egui::RichText::new(&rec.time_str).size(13.0).color(egui::Color32::from_rgb(180, 190, 210)));
                                            cols[1].label(egui::RichText::new(format!("+{}", crate::purger::format_bytes(rec.bytes_freed))).size(16.0).strong().color(egui::Color32::from_rgb(0, 229, 255)));
                                            cols[2].label(egui::RichText::new(format!("{} proc", rec.processes)).size(13.0).color(egui::Color32::from_rgb(255, 208, 0)));
                                        });
                                    });
                                    ui.separator();
                                }
                            });
                    }
                }

                CurrentScreen::Stats => {
                    ui.horizontal(|ui| {
                        let (back_rect, back_resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                        let back_fill = if back_resp.hovered() {
                            egui::Color32::from_rgb(25, 33, 52)
                        } else {
                            egui::Color32::from_rgb(18, 24, 38)
                        };
                        let painter = ui.painter();
                        painter.rect_filled(back_rect, 6.0, back_fill);
                        draw_back_chevron(painter, back_rect, egui::Color32::WHITE);

                        if back_resp.clicked() {
                            self.current_screen = CurrentScreen::Menu;
                        }

                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(
                                egui::RichText::new("MEMORY STATISTICS")
                                    .size(15.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                    });

                    ui.separator();
                    ui.add_space(16.0);

                    let stats = monitor.last_stats;

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(15, 19, 32))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 38, 58)))
                        .rounding(10.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("PHYSICAL MEMORY STATUS").small().strong().color(egui::Color32::from_rgb(0, 229, 255)));
                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                ui.label("Total RAM Installed:");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(crate::purger::format_bytes(stats.total_bytes)).strong().color(egui::Color32::WHITE));
                                });
                            });
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Active RAM Used:");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(crate::purger::format_bytes(stats.used_bytes)).strong().color(egui::Color32::from_rgb(239, 68, 68)));
                                });
                            });
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Free RAM Available:");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(crate::purger::format_bytes(stats.free_bytes)).strong().color(egui::Color32::from_rgb(16, 185, 129)));
                                });
                            });
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label("Current Usage Percentage:");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format!("{:.1}%", stats.usage_percent)).strong().color(egui::Color32::from_rgb(255, 208, 0)));
                                });
                            });
                        });

                    ui.add_space(16.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(15, 19, 32))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 38, 58)))
                        .rounding(10.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("SESSION SUMMARY").small().strong().color(egui::Color32::from_rgb(0, 229, 255)));
                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                ui.label("Total Freed in Session:");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(crate::purger::format_bytes(monitor.total_freed_bytes_session)).strong().color(egui::Color32::from_rgb(0, 229, 255)));
                                });
                            });
                        });
                }

                CurrentScreen::About => {
                    ui.horizontal(|ui| {
                        let (back_rect, back_resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                        let back_fill = if back_resp.hovered() {
                            egui::Color32::from_rgb(25, 33, 52)
                        } else {
                            egui::Color32::from_rgb(18, 24, 38)
                        };
                        let painter = ui.painter();
                        painter.rect_filled(back_rect, 6.0, back_fill);
                        draw_back_chevron(painter, back_rect, egui::Color32::WHITE);

                        if back_resp.clicked() {
                            self.current_screen = CurrentScreen::Menu;
                        }

                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(
                                egui::RichText::new("ABOUT")
                                    .size(15.0)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                    });

                    ui.separator();
                    ui.add_space(16.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(15, 19, 32))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 38, 58)))
                        .rounding(10.0)
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("RAM Purger Pro")
                                    .size(18.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 208, 0)),
                            );
                            ui.add_space(8.0);
                            ui.label("High-performance utility written in Rust for complete RAM memory purging on Windows.");
                            ui.add_space(12.0);
                            ui.label("Uses native NT Kernel API calls (NtSetSystemInformation) to flush Working Sets, Standby Lists, Modified Page Lists, and System Cache.");
                            ui.add_space(16.0);
                            ui.label(
                                egui::RichText::new(format!("Version {} | jmaxdev", env!("CARGO_PKG_VERSION")))
                                    .small()
                                    .color(egui::Color32::from_rgb(0, 229, 255)),
                            );
                        });
                }
            }

            if let Some(info) = monitor.pending_update.clone() {
                let mut close_modal = false;
                let mut skip_version = false;
                let mut download_update = false;

                egui::Window::new("⚠ NUEVA VERSIÓN DISPONIBLE")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .fixed_size([340.0, 260.0])
                    .show(ctx, |ui| {
                        ui.add_space(6.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(format!("RAM Purger Pro v{} está disponible", info.version))
                                    .size(15.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 229, 255)),
                            );
                            ui.label(
                                egui::RichText::new(format!("Versión instalada: v{}", env!("CARGO_PKG_VERSION")))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(180, 190, 210)),
                            );
                        });

                        ui.add_space(10.0);

                        if !info.release_notes.is_empty() {
                            ui.label(egui::RichText::new("Notas de la versión:").small().strong().color(egui::Color32::WHITE));
                            egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                                ui.label(egui::RichText::new(&info.release_notes).size(11.0).color(egui::Color32::from_rgb(200, 210, 225)));
                            });
                            ui.add_space(10.0);
                        }

                        ui.vertical_centered(|ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("⬇ DESCARGAR E INSTALAR").strong().color(egui::Color32::BLACK)).fill(egui::Color32::from_rgb(0, 229, 255))).clicked() {
                                download_update = true;
                            }

                            ui.add_space(6.0);

                            ui.columns(2, |cols| {
                                cols[0].vertical_centered(|ui| {
                                    if ui.button(egui::RichText::new("Saltar versión").small().color(egui::Color32::from_rgb(239, 68, 68))).clicked() {
                                        skip_version = true;
                                    }
                                });
                                cols[1].vertical_centered(|ui| {
                                    if ui.button(egui::RichText::new("Recordar luego").small().color(egui::Color32::from_rgb(180, 190, 210))).clicked() {
                                        close_modal = true;
                                    }
                                });
                            });
                        });
                    });

                if download_update {
                    self.status_message = Some("Descargando e instalando actualización...".to_string());
                    let url = info.download_url.clone();
                    std::thread::spawn(move || {
                        let _ = crate::updater::download_and_apply_update(&url);
                    });
                    monitor.pending_update = None;
                } else if skip_version {
                    monitor.config.skipped_version = Some(info.version);
                    let _ = monitor.config.save();
                    monitor.pending_update = None;
                } else if close_modal {
                    monitor.pending_update = None;
                }
            }
        });
    }
}

fn current_time_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let mins = (now / 60) % 60;
    let secs = now % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}
