mod graph;

use eframe::egui::{self, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, pos2, vec2};
use egui_phosphor::regular as icon;
use graph::{Graph, Kind};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

const BG: Color32 = Color32::from_rgb(27, 28, 31);
const PANEL: Color32 = Color32::from_rgb(36, 37, 41);
const LINE: Color32 = Color32::from_rgb(61, 62, 70);
const MUTED: Color32 = Color32::from_rgb(169, 170, 182);
const ACCENT: Color32 = Color32::from_rgb(172, 157, 255);
const NODE_SIZE: egui::Vec2 = vec2(164.0, 134.0);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    format: String,
    version: u32,
    graph: Graph,
}

fn save_graph(path: &Path, graph: &Graph) -> Result<(), String> {
    graph.validate()?;
    let project = Project {
        format: "touchcraft".into(),
        version: 1,
        graph: graph.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&project).map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
fn load_graph(path: &Path) -> Result<Graph, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 4_000_000 {
        return Err("Project exceeds the 4 MB limit.".into());
    }
    let project: Project = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if project.format != "touchcraft" || project.version != 1 {
        return Err("Unsupported Touchcraft project format.".into());
    }
    project.graph.validate()?;
    Ok(project.graph)
}

#[derive(Clone, Copy)]
enum Pending {
    New,
    Open,
    Quit,
}

struct Touchcraft {
    graph: Graph,
    selected: Option<u64>,
    path: Option<PathBuf>,
    dirty: bool,
    history: Vec<Graph>,
    future: Vec<Graph>,
    playing: bool,
    time: f32,
    last: Instant,
    textures: HashMap<u64, egui::TextureHandle>,
    preview: Option<egui::TextureHandle>,
    pan: egui::Vec2,
    zoom: f32,
    wire: Option<u64>,
    search: String,
    palette: bool,
    palette_focus: bool,
    pending: Option<Pending>,
    close_allowed: bool,
    status: String,
    screenshot: Option<PathBuf>,
    frames: usize,
    fit_requested: bool,
    canvas_size: egui::Vec2,
    name_edit: String,
    name_edit_id: Option<u64>,
}
impl Touchcraft {
    fn new(ctx: &egui::Context, screenshot: Option<PathBuf>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        ctx.set_fonts(fonts);
        let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = PANEL;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = BG;
        style.visuals.faint_bg_color = Color32::from_rgb(43, 44, 49);
        style.visuals.selection.bg_fill = Color32::from_rgb(78, 66, 133);
        style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(48, 49, 56);
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(64, 62, 80);
        style.spacing.item_spacing = vec2(8.0, 8.0);
        style.spacing.button_padding = vec2(10.0, 6.0);
        ctx.set_theme(egui::Theme::Dark);
        ctx.set_style_of(egui::Theme::Dark, style);
        Self {
            graph: Graph::default(),
            selected: Some(5),
            path: None,
            dirty: false,
            history: vec![],
            future: vec![],
            playing: true,
            time: 0.0,
            last: Instant::now(),
            textures: HashMap::new(),
            preview: None,
            pan: vec2(32.0, 65.0),
            zoom: 0.85,
            wire: None,
            search: String::new(),
            palette: false,
            palette_focus: false,
            pending: None,
            close_allowed: false,
            status: "Ready · select an operator to inspect its image".into(),
            screenshot,
            frames: 0,
            fit_requested: true,
            canvas_size: egui::Vec2::ZERO,
            name_edit: String::new(),
            name_edit_id: None,
        }
    }
    fn modified(&self) -> bool {
        self.dirty
            || self
                .name_edit_id
                .and_then(|id| self.graph.node(id))
                .is_some_and(|n| self.name_edit != n.name)
    }
    fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.close_allowed {
            self.commit_name();
            if self.modified() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                self.pending = Some(Pending::Quit);
            } else {
                self.close_allowed = true;
            }
        }
    }
    fn checkpoint(&mut self) {
        self.history.push(self.graph.clone());
        if self.history.len() > 64 {
            self.history.remove(0);
        }
        self.future.clear();
        self.dirty = true;
    }
    fn undo(&mut self) {
        if let Some(graph) = self.history.pop() {
            self.future.push(std::mem::replace(&mut self.graph, graph));
            self.dirty = true;
            self.reconcile();
        }
    }
    fn redo(&mut self) {
        if let Some(graph) = self.future.pop() {
            self.history.push(std::mem::replace(&mut self.graph, graph));
            self.dirty = true;
            self.reconcile();
        }
    }
    fn reconcile(&mut self) {
        self.name_edit_id = None;
        if self
            .selected
            .is_some_and(|id| self.graph.node(id).is_none())
        {
            self.selected = self.graph.nodes.first().map(|n| n.id);
        }
        self.wire = None;
        self.textures.retain(|id, _| self.graph.node(*id).is_some());
    }
    fn add(&mut self, kind: Kind) {
        let offset = (self.graph.nodes.len() % 6) as f32 * 26.0;
        let mut updated = self.graph.clone();
        match updated.add(kind, 90.0 + offset, 280.0 + offset) {
            Ok(id) => {
                self.checkpoint();
                self.graph = updated;
                self.selected = Some(id);
            }
            Err(error) => {
                self.status = error;
                return;
            }
        }
        self.palette = false;
        self.search.clear();
        self.status = "Operator added · connect its output to an input port".into();
    }
    fn commit_name(&mut self) -> bool {
        let Some(id) = self.name_edit_id else {
            return true;
        };
        let Some(node) = self.graph.node(id) else {
            return true;
        };
        let name = self.name_edit.trim().to_string();
        if name == node.name {
            self.name_edit = name;
            return true;
        }
        if name.is_empty() || name.len() > 64 || name.chars().any(char::is_control) {
            self.status = "Use a nonempty name of up to 64 UTF-8 bytes.".into();
            return false;
        }
        self.checkpoint();
        self.name_edit = name.clone();
        self.graph.node_mut(id).unwrap().name = name;
        true
    }
    fn save(&mut self, save_as: bool) -> bool {
        if !self.commit_name() {
            return false;
        }
        let path = if save_as || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("Touchcraft project", &["touchcraft"])
                .set_file_name("Untitled.touchcraft")
                .save_file()
        } else {
            self.path.clone()
        };
        let Some(path) = path else {
            return false;
        };
        match save_graph(&path, &self.graph) {
            Ok(()) => {
                self.path = Some(path);
                self.dirty = false;
                self.status = "Project saved".into();
                true
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
                false
            }
        }
    }
    fn request(&mut self, action: Pending, ctx: &egui::Context) {
        self.commit_name();
        if self.modified() {
            self.pending = Some(action);
        } else {
            self.perform(action, ctx);
        }
    }
    fn perform(&mut self, action: Pending, ctx: &egui::Context) {
        match action {
            Pending::Quit => {
                self.close_allowed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Pending::New => {
                self.graph = Graph::default();
                self.path = None;
                self.reset();
            }
            Pending::Open => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Touchcraft project", &["touchcraft"])
                    .pick_file()
                {
                    match load_graph(&path) {
                        Ok(graph) => {
                            self.graph = graph;
                            self.path = Some(path);
                            self.reset();
                            self.status = "Project opened".into();
                        }
                        Err(e) => {
                            self.status = format!("Open failed: {e}. Current project retained.")
                        }
                    }
                }
            }
        }
    }
    fn reset(&mut self) {
        self.selected = self.graph.nodes.first().map(|n| n.id);
        self.name_edit_id = None;
        self.history.clear();
        self.future.clear();
        self.dirty = false;
        self.wire = None;
        self.textures.clear();
        self.time = 0.0;
        self.pan = vec2(32.0, 65.0);
        self.zoom = 0.85;
        self.fit_requested = true;
    }
    fn connect(&mut self, source: u64, target: u64, slot: usize) {
        let before = self.graph.clone();
        match self.graph.connect(source, target, slot) {
            Ok(()) => {
                self.history.push(before);
                if self.history.len() > 64 {
                    self.history.remove(0);
                }
                self.future.clear();
                self.dirty = true;
                self.status = "Connection created".into();
            }
            Err(e) => self.status = e,
        }
        self.wire = None;
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.pending = None;
            }
            return;
        }
        let command = ctx.input(|i| i.modifiers.command);
        if command {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
                self.save(false);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
                self.request(Pending::Open, ctx);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
                self.request(Pending::New, ctx);
            }
            if !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Z)) {
                if ctx.input(|i| i.modifiers.shift) {
                    self.redo();
                } else {
                    self.undo();
                }
            }
        }
        if !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.playing = !self.playing;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
                self.palette = true;
                self.palette_focus = true;
            }
            if ctx
                .input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
                && let Some(id) = self.selected
            {
                self.checkpoint();
                self.graph.remove(id);
                self.selected = None;
                self.wire = None;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.wire = None;
            self.palette = false;
        }
    }
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(RichText::new(icon::GRAPH).size(23.0).color(ACCENT));
            ui.label(RichText::new("Touchcraft").size(17.0).strong());
            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);
            let name = self
                .path
                .as_ref()
                .and_then(|p| p.file_stem())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or("Untitled".into());
            ui.label(
                RichText::new(format!(
                    "{name}{}",
                    if self.modified() { " · Edited" } else { "" }
                ))
                .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(format!("{}  Save", icon::FLOPPY_DISK))
                    .on_hover_text("Save project · ⌘S")
                    .clicked()
                {
                    self.save(false);
                }
                if ui
                    .button(format!("{}  Open", icon::FOLDER_OPEN))
                    .on_hover_text("Open project · ⌘O")
                    .clicked()
                {
                    self.request(Pending::Open, ui.ctx());
                }
                if ui
                    .button(format!("{}  New", icon::FILE_PLUS))
                    .on_hover_text("New project · ⌘N")
                    .clicked()
                {
                    self.request(Pending::New, ui.ctx());
                }
            });
        });
    }
    fn library(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.heading("Operators");
        ui.add_space(7.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search operators…")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("TOP").color(ACCENT).strong());
            ui.label(RichText::new("Image operators").color(MUTED).small());
        });
        ui.separator();
        let mut found = false;
        for kind in Kind::ALL {
            if !kind
                .label()
                .to_lowercase()
                .contains(&self.search.to_lowercase())
            {
                continue;
            }
            found = true;
            let symbol = match kind {
                Kind::Noise => icon::WAVE_SINE,
                Kind::Circle => icon::CIRCLE,
                Kind::Constant => icon::SQUARE,
                Kind::Transform => icon::ARROWS_OUT_CARDINAL,
                Kind::Level => icon::SLIDERS_HORIZONTAL,
                Kind::Composite => icon::STACK,
                Kind::Output => icon::MONITOR_PLAY,
            };
            if ui
                .add_sized(
                    [ui.available_width(), 34.0],
                    egui::Button::new(format!("{symbol}   {}", kind.label())).frame(false),
                )
                .on_hover_text(description(kind))
                .clicked()
            {
                self.add(kind);
            }
        }
        if !found {
            ui.label(RichText::new("No matching operators").color(MUTED));
        }
        ui.add_space(12.0);
        ui.separator();
        ui.strong("In this network");
        for node in &self.graph.nodes {
            if ui
                .selectable_label(self.selected == Some(node.id), &node.name)
                .clicked()
            {
                self.selected = Some(node.id);
            }
        }
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new("Build a signal path").strong());
        ui.label(
            RichText::new("Add operators, then connect an output port to an input.").color(MUTED),
        );
        ui.add_space(12.0);
        for (key, action) in [
            ("Tab", "Add operator"),
            ("Space", "Play / pause"),
            ("Delete", "Remove selected"),
            ("⌘Z", "Undo"),
            ("Scroll", "Zoom network"),
            ("Middle drag", "Pan network"),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(key).size(11.0).color(ACCENT));
                ui.label(RichText::new(action).size(11.0).color(MUTED));
            });
        }
    }
    fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        let Some(id) = self.selected else {
            ui.heading("Parameters");
            ui.label("Select an operator in the network.");
            return;
        };
        let Some(node) = self.graph.node(id).cloned() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.heading(&node.name);
            ui.label(RichText::new("TOP").small().color(ACCENT));
        });
        ui.label(RichText::new(description(node.kind)).small().color(MUTED));
        ui.add_space(12.0);
        let pixels = graph::render(&self.graph, id, 384, 240, self.time);
        let img = egui::ColorImage::new([384, 240], pixels);
        if let Some(texture) = &mut self.preview {
            texture.set(img, egui::TextureOptions::LINEAR);
        } else {
            self.preview = Some(ui.ctx().load_texture(
                "selected-preview",
                img,
                egui::TextureOptions::LINEAR,
            ));
        }
        if let Some(texture) = &self.preview {
            ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(vec2(ui.available_width(), ui.available_width() * 0.625)),
            );
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("LIVE VIEWER").size(10.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("384 × 240").size(10.0).color(MUTED));
            });
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);
        if self.name_edit_id != Some(id) {
            self.commit_name();
            self.name_edit = node.name.clone();
            self.name_edit_id = Some(id);
        }
        ui.label(RichText::new("Name").small().color(MUTED));
        let rename = ui.text_edit_singleline(&mut self.name_edit);
        if rename.lost_focus() && !self.commit_name() {
            self.name_edit = node.name.clone();
        }
        ui.add_space(12.0);
        ui.label(RichText::new("Parameters").strong());
        for (i, &(label, min, max)) in graph::parameter_specs(node.kind).iter().enumerate() {
            let mut value = self.graph.node(id).unwrap().params[i];
            ui.label(RichText::new(label).small().color(MUTED));
            let slider = ui.add(egui::Slider::new(&mut value, min..=max).max_decimals(3));
            if slider.drag_started() {
                self.checkpoint();
            }
            if slider.changed() {
                if !slider.dragged() && !slider.drag_started() {
                    self.checkpoint();
                }
                self.graph.node_mut(id).unwrap().params[i] = value;
            }
        }
        if graph::parameter_specs(node.kind).is_empty() {
            ui.label(RichText::new("Passes the input image through.").color(MUTED));
        }
        if !node.inputs.is_empty() {
            ui.add_space(12.0);
            ui.separator();
            ui.label(RichText::new("Connections").strong());
            for slot in 0..node.inputs.len() {
                let mut source = node.inputs[slot];
                egui::ComboBox::from_id_salt(("input", id, slot))
                    .selected_text(
                        source
                            .and_then(|s| self.graph.node(s))
                            .map(|n| n.name.as_str())
                            .unwrap_or("Disconnected"),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut source, None, "Disconnected");
                        for candidate in &self.graph.nodes {
                            if candidate.id != id {
                                ui.selectable_value(
                                    &mut source,
                                    Some(candidate.id),
                                    &candidate.name,
                                );
                            }
                        }
                    });
                if source != node.inputs[slot] {
                    if let Some(source) = source {
                        self.connect(source, id, slot);
                    } else {
                        self.checkpoint();
                        self.graph.node_mut(id).unwrap().inputs[slot] = None;
                    }
                }
            }
        }
        ui.add_space(16.0);
        if ui
            .button(format!(
                "{}  Reset parameters",
                icon::ARROW_COUNTER_CLOCKWISE
            ))
            .clicked()
        {
            let mut defaults = Graph {
                nodes: vec![],
                next_id: 1,
            };
            let default = defaults
                .add(node.kind, 0.0, 0.0)
                .expect("Valid empty graph");
            self.checkpoint();
            self.graph.node_mut(id).unwrap().params = defaults.node(default).unwrap().params;
        }
    }
    fn network(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon::FLOW_ARROW).color(ACCENT));
            ui.strong("Network");
            ui.label(RichText::new("/project1").color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("{}  Add operator", icon::PLUS)).clicked() {
                    self.palette = true;
                    self.palette_focus = true;
                }
                if ui
                    .button(icon::CORNERS_OUT)
                    .on_hover_text("Fit network")
                    .clicked()
                {
                    self.fit_requested = true;
                }
                if ui
                    .add_enabled(
                        !self.future.is_empty(),
                        egui::Button::new(icon::ARROW_U_UP_RIGHT),
                    )
                    .on_hover_text("Redo · ⇧⌘Z")
                    .clicked()
                {
                    self.redo();
                }
                if ui
                    .add_enabled(
                        !self.history.is_empty(),
                        egui::Button::new(icon::ARROW_U_UP_LEFT),
                    )
                    .on_hover_text("Undo · ⌘Z")
                    .clicked()
                {
                    self.undo();
                }
            });
        });
        ui.separator();
        let rect = ui.available_rect_before_wrap();
        if self.fit_requested || (rect.size() - self.canvas_size).length_sq() > 1.0 {
            self.fit(rect);
            self.fit_requested = false;
            self.canvas_size = rect.size();
        }
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, BG);
        let spacing = 24.0 * self.zoom;
        let offset = vec2(
            self.pan.x.rem_euclid(spacing),
            self.pan.y.rem_euclid(spacing),
        );
        let mut x = rect.left() + offset.x;
        while x < rect.right() {
            let mut y = rect.top() + offset.y;
            while y < rect.bottom() {
                painter.circle_filled(pos2(x, y), 0.8, Color32::from_rgb(54, 55, 62));
                y += spacing;
            }
            x += spacing;
        }
        let panning = ui.input(|i| {
            i.pointer.hover_pos().is_some_and(|p| rect.contains(p))
                && (i.pointer.middle_down() || (i.modifiers.alt && i.pointer.primary_down()))
        });
        if panning {
            self.pan += ui.input(|i| i.pointer.delta());
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let old = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.35, 1.8);
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let pivot = pos - rect.min;
                    self.pan = pivot - (pivot - self.pan) * (self.zoom / old);
                }
            }
        }
        if response.double_clicked() {
            self.palette = true;
            self.palette_focus = true;
        }
        let origin = rect.min + self.pan;
        let nodes = self.graph.nodes.clone();
        let zoom = self.zoom;
        let node_rect =
            |n: &graph::Node| Rect::from_min_size(origin + vec2(n.x, n.y) * zoom, NODE_SIZE * zoom);
        for node in &nodes {
            let end = node_rect(node);
            for (slot, source) in node.inputs.iter().enumerate() {
                if let Some(source) = source.and_then(|id| nodes.iter().find(|n| n.id == id)) {
                    let from = node_rect(source).right_center();
                    let to = input_pos(end, slot, node.inputs.len());
                    curve(
                        &painter,
                        from,
                        to,
                        Stroke::new(2.0, ACCENT.gamma_multiply(0.7)),
                    );
                }
            }
        }
        if let Some(source) = self.wire.and_then(|id| nodes.iter().find(|n| n.id == id))
            && let Some(pointer) = ui.input(|i| i.pointer.hover_pos())
        {
            curve(
                &painter,
                node_rect(source).right_center(),
                pointer,
                Stroke::new(2.0, ACCENT),
            );
        }
        for node in &nodes {
            let r = node_rect(node);
            if !r.intersects(rect) {
                continue;
            }
            let selected = self.selected == Some(node.id);
            painter.rect_filled(r, 5.0, Color32::from_rgb(46, 46, 54));
            painter.rect_stroke(
                r,
                5.0,
                Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    if selected { ACCENT } else { LINE },
                ),
                egui::StrokeKind::Inside,
            );
            let head = Rect::from_min_size(r.min, vec2(r.width(), 27.0 * self.zoom));
            painter.rect_filled(
                head.shrink(2.0),
                3.0,
                if selected {
                    Color32::from_rgb(80, 66, 126)
                } else {
                    Color32::from_rgb(57, 51, 76)
                },
            );
            painter.text(
                head.left_center() + vec2(10.0, 0.0) * self.zoom,
                Align2::LEFT_CENTER,
                &node.name,
                FontId::proportional(12.0 * self.zoom),
                Color32::from_gray(239),
            );
            let thumb = Rect::from_min_max(
                r.min + vec2(8.0, 34.0) * self.zoom,
                r.max - vec2(8.0, 24.0) * self.zoom,
            );
            let img = egui::ColorImage::new(
                [128, 64],
                graph::render(&self.graph, node.id, 128, 64, self.time),
            );
            let texture = self.textures.entry(node.id).or_insert_with(|| {
                ui.ctx().load_texture(
                    format!("node-{}", node.id),
                    img.clone(),
                    egui::TextureOptions::LINEAR,
                )
            });
            texture.set(img, egui::TextureOptions::LINEAR);
            painter.image(
                texture.id(),
                thumb,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            painter.text(
                r.left_bottom() + vec2(9.0, -10.0) * self.zoom,
                Align2::LEFT_CENTER,
                node.kind.label(),
                FontId::proportional(10.0 * self.zoom),
                MUTED,
            );
            let resp = ui.interact(r, ui.id().with(("node", node.id)), Sense::click_and_drag());
            if resp.clicked() || resp.drag_started_by(egui::PointerButton::Primary) {
                self.selected = Some(node.id);
            }
            if resp.drag_started_by(egui::PointerButton::Primary) && !panning {
                self.checkpoint();
            }
            if resp.dragged_by(egui::PointerButton::Primary) && !panning {
                let delta = resp.drag_delta() / self.zoom;
                if let Some(n) = self.graph.node_mut(node.id) {
                    n.x += delta.x;
                    n.y += delta.y;
                }
            }
            resp.on_hover_text(format!("{} · {}", node.name, description(node.kind)));
            for slot in 0..node.inputs.len() {
                let center = input_pos(r, slot, node.inputs.len());
                let port = ui.interact(
                    Rect::from_center_size(center, vec2(22.0, 22.0)),
                    ui.id().with(("in", node.id, slot)),
                    Sense::click(),
                );
                painter.circle_filled(
                    center,
                    4.5,
                    if node.inputs[slot].is_some() {
                        ACCENT
                    } else {
                        MUTED
                    },
                );
                if (port.clicked()
                    || (port.hovered() && ui.input(|i| i.pointer.primary_released())))
                    && self.wire.is_some()
                {
                    self.connect(self.wire.unwrap(), node.id, slot);
                }
                port.on_hover_text(format!(
                    "Input {} · click to connect the selected output",
                    slot + 1
                ));
            }
            let port = ui.interact(
                Rect::from_center_size(r.right_center(), vec2(22.0, 22.0)),
                ui.id().with(("out", node.id)),
                Sense::click_and_drag(),
            );
            painter.circle_filled(
                r.right_center(),
                4.5,
                if self.wire == Some(node.id) {
                    Color32::WHITE
                } else {
                    ACCENT
                },
            );
            if port.clicked() || port.drag_started() {
                self.wire = Some(node.id);
                self.status = "Choose an input port to connect · Escape cancels".into();
            }
            port.on_hover_text("Output · click or drag to connect");
        }
        if nodes.is_empty() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Add an operator to start a network",
                FontId::proportional(18.0),
                MUTED,
            );
        }
        painter.text(
            rect.left_bottom() + vec2(16.0, -16.0),
            Align2::LEFT_BOTTOM,
            format!(
                "{} operators    ·    {}%",
                nodes.len(),
                (self.zoom * 100.0) as u32
            ),
            FontId::proportional(11.0),
            MUTED,
        );
    }
    fn fit(&mut self, rect: Rect) {
        if self.graph.nodes.is_empty() {
            return;
        }
        let mut bounds = Rect::NOTHING;
        for n in &self.graph.nodes {
            bounds = bounds.union(Rect::from_min_size(pos2(n.x, n.y), NODE_SIZE));
        }
        self.zoom = ((rect.width() - 80.0) / bounds.width())
            .min((rect.height() - 100.0) / bounds.height())
            .clamp(0.35, 1.3);
        self.pan = vec2(40.0, 50.0) - bounds.min.to_vec2() * self.zoom;
    }
    fn transport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button(icon::SKIP_BACK)
                .on_hover_text("Rewind to frame 1")
                .clicked()
            {
                self.time = 0.0;
            }
            if ui
                .button(if self.playing {
                    icon::PAUSE
                } else {
                    icon::PLAY
                })
                .on_hover_text("Play / pause · Space")
                .clicked()
            {
                self.playing = !self.playing;
            }
            ui.label(
                RichText::new(format!("{:04}", (self.time * 60.0) as u32 + 1))
                    .monospace()
                    .color(ACCENT),
            );
            ui.add_space(10.0);
            let max = self.time.max(20.0);
            ui.add(
                egui::Slider::new(&mut self.time, 0.0..=max)
                    .show_value(false)
                    .text("Time"),
            );
            ui.label(
                RichText::new(format!("{:.2} s", self.time))
                    .monospace()
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("60 fps timeline · CPU preview")
                        .size(11.0)
                        .color(MUTED),
                );
            });
        });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.palette {
            let mut open = true;
            egui::Window::new("Add operator")
                .collapsible(false)
                .resizable(false)
                .default_width(360.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    let search = ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search image operators…")
                            .desired_width(f32::INFINITY),
                    );
                    if self.palette_focus {
                        search.request_focus();
                        self.palette_focus = false;
                    }
                    let matches: Vec<_> = Kind::ALL
                        .into_iter()
                        .filter(|k| {
                            k.label()
                                .to_lowercase()
                                .contains(&self.search.to_lowercase())
                        })
                        .collect();
                    if ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && let Some(kind) = matches.first()
                    {
                        self.add(*kind);
                    }
                    for kind in matches {
                        if ui
                            .button(format!("{}   {}", icon::PLUS, kind.label()))
                            .on_hover_text(description(kind))
                            .clicked()
                        {
                            self.add(kind);
                        }
                    }
                });
            if !open {
                self.palette = false;
            }
        }
        if let Some(action) = self.pending {
            egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Save project changes?");
                ui.label("Save the current network before continuing.");
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.pending = None;
                    }
                    if ui.button("Discard changes").clicked() {
                        self.pending = None;
                        self.perform(action, ctx);
                    }
                    if ui.button("Save").clicked() && self.save(false) {
                        self.pending = None;
                        self.perform(action, ctx);
                    }
                });
            });
        }
    }
}
fn description(kind: Kind) -> &'static str {
    match kind {
        Kind::Noise => "Generate an animated procedural texture",
        Kind::Circle => "Create a soft circular mask",
        Kind::Constant => "Generate a solid RGBA image",
        Kind::Transform => "Translate, scale, and rotate an image",
        Kind::Level => "Adjust brightness, contrast, and gamma",
        Kind::Composite => "Blend two input images",
        Kind::Output => "Pass an image to the end of the network",
    }
}
fn input_pos(rect: Rect, slot: usize, count: usize) -> egui::Pos2 {
    pos2(
        rect.left(),
        rect.top() + rect.height() * (slot + 1) as f32 / (count + 1) as f32,
    )
}
fn curve(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, stroke: Stroke) {
    let bend = ((to.x - from.x).abs() * 0.5).max(45.0);
    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
        [from, from + vec2(bend, 0.0), to - vec2(bend, 0.0), to],
        false,
        Color32::TRANSPARENT,
        stroke,
    ));
}
impl eframe::App for Touchcraft {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.screenshot.is_some() {
            input
                .events
                .retain(|event| matches!(event, egui::Event::Screenshot { .. }));
        }
    }
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_close(ctx);
        #[cfg(target_os = "macos")]
        craft_quit::on_close_request(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        let now = Instant::now();
        if self.playing && self.screenshot.is_none() {
            self.time += now.duration_since(self.last).as_secs_f32().min(0.1);
        }
        self.last = now;
        egui::Panel::top("header")
            .exact_size(54.0)
            .show(ui, |ui| self.header(ui));
        egui::Panel::bottom("status")
            .exact_size(28.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).size(11.0).color(MUTED));
                });
            });
        egui::Panel::bottom("transport")
            .exact_size(46.0)
            .show(ui, |ui| self.transport(ui));
        egui::Panel::left("library")
            .exact_size(198.0)
            .resizable(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.library(ui));
            });
        egui::Panel::right("inspector")
            .default_size(310.0)
            .size_range(260.0..=430.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.inspector(ui));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(12.0))
            .show(ui, |ui| self.network(ui));
        self.dialogs(&ctx);
        self.frames += 1;
        if let Some(path) = &self.screenshot {
            if self.frames == 5 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            let capture = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = capture {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                image::save_buffer(
                    path,
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .expect("Cannot save native screenshot");
                println!(
                    "Native screenshot: {} (canvas {:?}, zoom {:.2})",
                    path.display(),
                    self.canvas_size,
                    self.zoom
                );
                self.close_allowed = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            assert!(self.frames < 120, "Native screenshot was not returned");
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{}Touchcraft",
            if self.modified() { "● " } else { "" }
        )));
        if self.playing || self.screenshot.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
}
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let screenshot = args
        .windows(2)
        .find(|a| a[0] == "--screenshot")
        .map(|a| PathBuf::from(&a[1]));
    let compact = args.iter().any(|a| a == "--compact");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(if compact {
                [1100.0, 720.0]
            } else {
                [1480.0, 900.0]
            })
            .with_min_inner_size([1000.0, 680.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/touchcraft-icon.png"))
                    .expect("Invalid bundled icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Touchcraft",
        options,
        Box::new(move |cc| {
            #[cfg(target_os = "macos")]
            craft_quit::install_app_menu(&cc.egui_ctx, "Touchcraft")?;
            Ok(Box::new(Touchcraft::new(&cc.egui_ctx, screenshot)))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fit_contains_the_entire_starter_network_at_supported_sizes() {
        let mut app = Touchcraft::new(&egui::Context::default(), None);
        for size in [vec2(550.0, 560.0), vec2(930.0, 740.0)] {
            app.fit(Rect::from_min_size(egui::Pos2::ZERO, size));
            for node in &app.graph.nodes {
                let bounds = Rect::from_min_size(
                    pos2(node.x, node.y) * app.zoom + app.pan,
                    NODE_SIZE * app.zoom,
                );
                assert!(Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(bounds));
            }
        }
    }
    #[test]
    fn native_close_cancels_only_when_changes_need_a_decision() {
        for dirty in [false, true] {
            let ctx = egui::Context::default();
            let mut app = Touchcraft::new(&ctx, None);
            app.dirty = dirty;
            let mut input = egui::RawInput::default();
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .events
                .push(egui::ViewportEvent::Close);
            let mut result = ctx.run_logic(&input, |ctx| app.handle_close(ctx));
            let commands = result
                .viewport_commands
                .remove(&egui::ViewportId::ROOT)
                .unwrap_or_default();
            assert_eq!(
                commands.contains(&egui::ViewportCommand::CancelClose),
                dirty
            );
            assert_eq!(app.pending.is_some(), dirty);
            assert_eq!(app.close_allowed, !dirty);
        }
    }
    #[test]
    fn unsaved_dialog_blocks_document_keyboard_changes() {
        let ctx = egui::Context::default();
        let mut app = Touchcraft::new(&ctx, None);
        let before = serde_json::to_value(&app.graph).unwrap();
        let delete = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        app.pending = Some(Pending::New);
        let mut result = ctx.run_ui(delete.clone(), |ui| app.shortcuts(ui.ctx()));
        result.textures_delta.clear();
        assert_eq!(serde_json::to_value(&app.graph).unwrap(), before);
        assert!(app.pending.is_some());
        let ctx = egui::Context::default();
        app.pending = None;
        let mut result = ctx.run_ui(delete, |ui| app.shortcuts(ui.ctx()));
        result.textures_delta.clear();
        assert_eq!(app.graph.nodes.len(), 4);
        app.undo();
        assert_eq!(serde_json::to_value(&app.graph).unwrap(), before);
    }
    #[test]
    fn undo_does_not_reapply_a_stale_name_on_save() {
        let mut app = Touchcraft::new(&egui::Context::default(), None);
        let id = app.selected.unwrap();
        let original = app.graph.node(id).unwrap().name.clone();
        app.name_edit_id = Some(id);
        app.name_edit = "renamed_output".into();
        assert!(app.commit_name());
        assert_eq!(app.graph.node(id).unwrap().name, "renamed_output");
        app.undo();
        assert!(app.commit_name());
        assert_eq!(app.graph.node(id).unwrap().name, original);
        assert_eq!(app.future.len(), 1);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("undo.touchcraft");
        save_graph(&path, &app.graph).unwrap();
        assert_eq!(load_graph(&path).unwrap().node(id).unwrap().name, original);
    }
    #[test]
    fn project_roundtrip_and_invalid_load_preserve_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.touchcraft");
        let graph = Graph::default();
        save_graph(&path, &graph).unwrap();
        assert_eq!(
            serde_json::to_value(load_graph(&path).unwrap()).unwrap(),
            serde_json::to_value(&graph).unwrap()
        );
        let before = std::fs::read(&path).unwrap();
        let mut invalid = graph.clone();
        invalid.nodes[0].x = f32::NAN;
        assert!(save_graph(&path, &invalid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let bad = dir.path().join("bad.touchcraft");
        std::fs::write(
            &bad,
            "{\"format\":\"other\",\"version\":1,\"graph\":{\"nodes\":[],\"next_id\":1}}",
        )
        .unwrap();
        assert!(load_graph(&bad).is_err());
    }
}
