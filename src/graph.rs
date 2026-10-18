use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Noise,
    Circle,
    Constant,
    Transform,
    Level,
    Composite,
    Output,
}

impl Kind {
    pub const ALL: [Self; 7] = [
        Self::Noise,
        Self::Circle,
        Self::Constant,
        Self::Transform,
        Self::Level,
        Self::Composite,
        Self::Output,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Noise => "Noise",
            Self::Circle => "Circle",
            Self::Constant => "Constant",
            Self::Transform => "Transform",
            Self::Level => "Level",
            Self::Composite => "Composite",
            Self::Output => "Output",
        }
    }
    pub fn input_count(self) -> usize {
        match self {
            Self::Noise | Self::Circle | Self::Constant => 0,
            Self::Composite => 2,
            _ => 1,
        }
    }
}

pub fn parameter_specs(kind: Kind) -> &'static [(&'static str, f32, f32)] {
    match kind {
        Kind::Noise => &[
            ("Scale", 0.2, 12.0),
            ("Speed", 0.0, 2.0),
            ("Detail", 0.0, 1.0),
            ("Hue", 0.0, 1.0),
            ("Seed", 0.0, 100.0),
        ],
        Kind::Circle => &[
            ("Radius", 0.02, 0.7),
            ("Softness", 0.001, 0.2),
            ("Center X", 0.0, 1.0),
            ("Center Y", 0.0, 1.0),
            ("Hue", 0.0, 1.0),
        ],
        Kind::Constant => &[
            ("Red", 0.0, 1.0),
            ("Green", 0.0, 1.0),
            ("Blue", 0.0, 1.0),
            ("Alpha", 0.0, 1.0),
        ],
        Kind::Transform => &[
            ("Translate X", -1.0, 1.0),
            ("Translate Y", -1.0, 1.0),
            ("Scale", 0.1, 4.0),
            ("Rotate", -180.0, 180.0),
        ],
        Kind::Level => &[
            ("Brightness", 0.0, 3.0),
            ("Contrast", 0.0, 3.0),
            ("Gamma", 0.1, 3.0),
        ],
        Kind::Composite => &[("Mix", 0.0, 1.0)],
        Kind::Output => &[],
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: u64,
    pub name: String,
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    pub inputs: Vec<Option<u64>>,
    pub params: [f32; 8],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub next_id: u64,
}

impl Default for Graph {
    fn default() -> Self {
        let mut graph = Self {
            nodes: Vec::new(),
            next_id: 1,
        };
        let noise = graph
            .add(Kind::Noise, 40.0, 80.0)
            .expect("starter operator");
        let circle = graph
            .add(Kind::Circle, 40.0, 300.0)
            .expect("starter operator");
        let composite = graph
            .add(Kind::Composite, 305.0, 150.0)
            .expect("starter operator");
        let level = graph
            .add(Kind::Level, 570.0, 150.0)
            .expect("starter operator");
        let output = graph
            .add(Kind::Output, 835.0, 150.0)
            .expect("starter operator");
        for (source, target, slot) in [
            (noise, composite, 0),
            (circle, composite, 1),
            (composite, level, 0),
            (level, output, 0),
        ] {
            graph
                .connect(source, target, slot)
                .expect("valid starter network");
        }
        graph
    }
}

impl Graph {
    pub fn add(&mut self, kind: Kind, x: f32, y: f32) -> Result<u64, String> {
        if self.nodes.len() >= 256 {
            return Err("Projects support at most 256 operators.".into());
        }
        let id = self.next_id;
        if id == 0 || self.nodes.iter().any(|node| node.id >= id) {
            return Err("Next operator identifier is invalid.".into());
        }
        let next_id = id
            .checked_add(1)
            .filter(|&next| next < u64::MAX)
            .ok_or("No operator identifiers remain.")?;
        let defaults: &[f32] = match kind {
            Kind::Noise => &[3.2, 0.3, 0.68, 0.05, 4.0],
            Kind::Circle => &[0.29, 0.065, 0.56, 0.49, 0.04],
            Kind::Constant => &[0.15, 0.4, 0.95, 1.0],
            Kind::Transform => &[0.0, 0.0, 1.0, 0.0],
            Kind::Level => &[1.06, 1.18, 1.0],
            Kind::Composite => &[0.7],
            Kind::Output => &[],
        };
        let mut params = [0.0; 8];
        params[..defaults.len()].copy_from_slice(defaults);
        self.nodes.push(Node {
            id,
            name: format!("{}{}", kind.label().to_lowercase(), id),
            kind,
            x,
            y,
            inputs: vec![None; kind.input_count()],
            params,
        });
        self.next_id = next_id;
        Ok(id)
    }

    pub fn node(&self, id: u64) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }
    pub fn node_mut(&mut self, id: u64) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    fn reaches(&self, from: u64, target: u64, visited: &mut HashSet<u64>) -> bool {
        if from == target {
            return true;
        }
        if !visited.insert(from) {
            return false;
        }
        self.node(from).is_some_and(|node| {
            node.inputs
                .iter()
                .flatten()
                .any(|&input| self.reaches(input, target, visited))
        })
    }

    pub fn connect(&mut self, source: u64, target: u64, slot: usize) -> Result<(), String> {
        if self.node(source).is_none() {
            return Err("Source operator does not exist.".into());
        }
        let node = self.node(target).ok_or("Target operator does not exist.")?;
        if slot >= node.kind.input_count() || slot >= node.inputs.len() {
            return Err("Input connector does not exist.".into());
        }
        if self.reaches(source, target, &mut HashSet::new()) {
            return Err("Connection would create a cycle.".into());
        }
        self.node_mut(target).unwrap().inputs[slot] = Some(source);
        Ok(())
    }

    pub fn remove(&mut self, id: u64) {
        self.nodes.retain(|node| node.id != id);
        for node in &mut self.nodes {
            for input in &mut node.inputs {
                if *input == Some(id) {
                    *input = None;
                }
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.nodes.len() > 256 {
            return Err("Projects support at most 256 operators.".into());
        }
        let mut ids = HashSet::new();
        for node in &self.nodes {
            if node.id == 0 || !ids.insert(node.id) {
                return Err("Operator identifiers must be unique and nonzero.".into());
            }
            if node.name.trim().is_empty()
                || node.name.len() > 64
                || node.name.chars().any(char::is_control)
            {
                return Err("Operator names must contain 1–64 printable bytes.".into());
            }
            if !node.x.is_finite()
                || !node.y.is_finite()
                || node.x.abs() > 100_000.0
                || node.y.abs() > 100_000.0
            {
                return Err("Operator positions are invalid.".into());
            }
            if node.inputs.len() != node.kind.input_count() {
                return Err("Operator input count is invalid.".into());
            }
            if node
                .params
                .iter()
                .any(|value| !value.is_finite() || value.abs() > 100_000.0)
            {
                return Err("Operator parameters are invalid.".into());
            }
            for (index, &(_, min, max)) in parameter_specs(node.kind).iter().enumerate() {
                if !(min..=max).contains(&node.params[index]) {
                    return Err(format!("{} parameter is outside its range.", node.name));
                }
            }
        }
        if self.next_id == 0
            || self.next_id == u64::MAX
            || self.nodes.iter().any(|node| node.id >= self.next_id)
        {
            return Err("Next operator identifier is invalid.".into());
        }
        for node in &self.nodes {
            for &input in node.inputs.iter().flatten() {
                if !ids.contains(&input) {
                    return Err("A connection references a missing operator.".into());
                }
                if self.reaches(input, node.id, &mut HashSet::new()) {
                    return Err("Project contains a connection cycle.".into());
                }
            }
        }
        Ok(())
    }
}

type Pixel = [f32; 4];

fn palette(value: f32) -> [f32; 3] {
    [0.0, 0.33, 0.67].map(|phase| 0.52 + 0.48 * (std::f32::consts::TAU * (value + phase)).cos())
}

fn noise(x: f32, y: f32, seed: f32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let hash = |x: i32, y: i32| {
        let mut value = (x as u32).wrapping_mul(374_761_393)
            ^ (y as u32).wrapping_mul(668_265_263)
            ^ seed.to_bits();
        value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);
        (value ^ (value >> 16)) as f32 / u32::MAX as f32
    };
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let top = hash(ix, iy) * (1.0 - sx) + hash(ix.wrapping_add(1), iy) * sx;
    let bottom = hash(ix, iy.wrapping_add(1)) * (1.0 - sx)
        + hash(ix.wrapping_add(1), iy.wrapping_add(1)) * sx;
    top * (1.0 - sy) + bottom * sy
}

fn sample(buffer: &[Pixel], width: usize, height: usize, u: f32, v: f32) -> Pixel {
    if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
        return [0.0; 4];
    }
    let x = (u * width as f32 - 0.5).clamp(0.0, (width - 1) as f32);
    let y = (v * height as f32 - 0.5).clamp(0.0, (height - 1) as f32);
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
    let (fx, fy) = (x.fract(), y.fract());
    std::array::from_fn(|c| {
        (buffer[y0 * width + x0][c] * (1.0 - fx) + buffer[y0 * width + x1][c] * fx) * (1.0 - fy)
            + (buffer[y1 * width + x0][c] * (1.0 - fx) + buffer[y1 * width + x1][c] * fx) * fy
    })
}

struct Renderer<'a> {
    graph: &'a Graph,
    width: usize,
    height: usize,
    time: f32,
    cache: HashMap<u64, Arc<Vec<Pixel>>>,
    active: HashSet<u64>,
    blank: Arc<Vec<Pixel>>,
}

impl Renderer<'_> {
    fn evaluate(&mut self, id: u64) -> Arc<Vec<Pixel>> {
        if let Some(buffer) = self.cache.get(&id) {
            return buffer.clone();
        }
        if !self.active.insert(id) {
            return self.blank.clone();
        }
        let Some(node) = self.graph.node(id).cloned() else {
            self.active.remove(&id);
            return self.blank.clone();
        };
        let inputs: Vec<_> = (0..node.kind.input_count())
            .map(|slot| {
                node.inputs
                    .get(slot)
                    .copied()
                    .flatten()
                    .map(|input| self.evaluate(input))
                    .unwrap_or_else(|| self.blank.clone())
            })
            .collect();
        let p = node.params;
        let mut output = Vec::with_capacity(self.width * self.height);
        let angle = -p[3].to_radians();
        let (sin, cos) = angle.sin_cos();
        for y in 0..self.height {
            for x in 0..self.width {
                let index = y * self.width + x;
                let (u, v) = (
                    (x as f32 + 0.5) / self.width as f32,
                    (y as f32 + 0.5) / self.height as f32,
                );
                let pixel = match node.kind {
                    Kind::Noise => {
                        let t = self.time * p[1];
                        let (px, py) = (
                            (u - 0.5) * p[0] * self.width as f32 / self.height as f32,
                            (v - 0.5) * p[0],
                        );
                        let warp = noise(px + t * 0.3, py - t * 0.2, p[4]);
                        let detail = noise(px * 2.6 + warp + t * 0.2, py * 2.6 - warp, p[4] + 1.0);
                        let field = warp + (detail - 0.5) * p[2] * 0.7;
                        let ribbons = (field * 12.0 - t * 0.9 + (px + py + t).sin() * 0.6).sin();
                        let color = palette(field * 0.9 + p[3] + t * 0.035);
                        let light = 0.32 + 0.68 * (ribbons * 0.5 + 0.5).powf(0.65);
                        [color[0] * light, color[1] * light, color[2] * light, 1.0]
                    }
                    Kind::Circle => {
                        let distance = ((u - p[2]).powi(2)
                            * (self.width as f32 / self.height as f32).powi(2)
                            + (v - p[3]).powi(2))
                        .sqrt();
                        let edge = p[1].max(1.0 / self.height as f32);
                        let a = ((p[0] - distance) / edge + 0.5).clamp(0.0, 1.0);
                        let color = palette(p[4]);
                        [color[0], color[1], color[2], a * a * (3.0 - 2.0 * a)]
                    }
                    Kind::Constant => [p[0], p[1], p[2], p[3]],
                    Kind::Transform => {
                        let (px, py) = (
                            (u - 0.5 - p[0]) * self.width as f32 / self.height as f32,
                            v - 0.5 - p[1],
                        );
                        sample(
                            &inputs[0],
                            self.width,
                            self.height,
                            (px * cos - py * sin) / p[2].max(0.1) * self.height as f32
                                / self.width as f32
                                + 0.5,
                            (px * sin + py * cos) / p[2].max(0.1) + 0.5,
                        )
                    }
                    Kind::Level => {
                        let mut color = inputs[0][index];
                        for channel in &mut color[..3] {
                            *channel = (((*channel - 0.5) * p[1] + 0.5) * p[0])
                                .clamp(0.0, 1.0)
                                .powf(1.0 / p[2].max(0.1));
                        }
                        color
                    }
                    Kind::Composite => {
                        let (a, b) = (inputs[0][index], inputs[1][index]);
                        let alpha = b[3] * p[0];
                        let mut color = [0.0, 0.0, 0.0, a[3] + alpha * (1.0 - a[3])];
                        for c in 0..3 {
                            color[c] = (1.0 - (1.0 - a[c] * a[3]) * (1.0 - b[c] * alpha))
                                / color[3].max(0.00001);
                        }
                        color
                    }
                    Kind::Output => inputs[0][index],
                };
                output.push(pixel);
            }
        }
        let output = Arc::new(output);
        self.active.remove(&id);
        self.cache.insert(id, output.clone());
        output
    }
}

pub fn render(graph: &Graph, node_id: u64, width: usize, height: usize, time: f32) -> Vec<Color32> {
    // shortcut: CPU preview is limited to one megapixel; use GPU textures for production-resolution output.
    let Some(count) = width
        .checked_mul(height)
        .filter(|&count| count > 0 && count <= 1_048_576)
    else {
        return Vec::new();
    };
    if graph.nodes.len() > 256 {
        return vec![Color32::TRANSPARENT; count];
    }
    let mut renderer = Renderer {
        graph,
        width,
        height,
        time: if time.is_finite() { time } else { 0.0 },
        cache: HashMap::new(),
        active: HashSet::new(),
        blank: Arc::new(vec![[0.0; 4]; count]),
    };
    renderer
        .evaluate(node_id)
        .iter()
        .map(|pixel| {
            let rgba = pixel.map(|channel| (channel.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
            Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_allocations_preserve_the_graph() {
        let mut graph = Graph::default();
        for next_id in [0, 1, u64::MAX - 1, u64::MAX] {
            graph.next_id = next_id;
            let before = serde_json::to_string(&graph).unwrap();
            assert!(graph.add(Kind::Constant, 0.0, 0.0).is_err());
            assert_eq!(before, serde_json::to_string(&graph).unwrap());
        }
        graph.next_id = 6;
        while graph.nodes.len() < 256 {
            graph.add(Kind::Constant, 0.0, 0.0).unwrap();
        }
        let before = serde_json::to_string(&graph).unwrap();
        assert!(graph.add(Kind::Constant, 0.0, 0.0).is_err());
        assert_eq!(before, serde_json::to_string(&graph).unwrap());
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn connections_reject_cycles_without_changes() {
        let mut graph = Graph::default();
        let before = serde_json::to_string(&graph).unwrap();
        assert!(graph.connect(5, 3, 0).is_err());
        assert!(graph.connect(1, 3, 9).is_err());
        assert!(graph.connect(99, 3, 0).is_err());
        assert_eq!(before, serde_json::to_string(&graph).unwrap());
        graph.remove(1);
        assert_eq!(graph.node(3).unwrap().inputs[0], None);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn validation_rejects_malformed_projects() {
        let mut graph = Graph::default();
        graph.node_mut(3).unwrap().inputs[0] = Some(5);
        assert!(graph.validate().is_err());
        let mut graph = Graph::default();
        graph.node_mut(1).unwrap().params[0] = f32::NAN;
        assert!(graph.validate().is_err());
        let mut graph = Graph::default();
        graph.node_mut(5).unwrap().inputs[0] = Some(100);
        assert!(graph.validate().is_err());
        let graph = Graph {
            next_id: 1,
            ..Graph::default()
        };
        assert!(graph.validate().is_err());
    }

    #[test]
    fn renders_animate_and_parameters_change_pixels() {
        let mut graph = Graph::default();
        let initial = render(&graph, 5, 48, 32, 0.0);
        assert_eq!(initial.len(), 48 * 32);
        assert_ne!(initial, render(&graph, 5, 48, 32, 2.0));
        graph.node_mut(1).unwrap().params[0] = 7.0;
        assert_ne!(initial, render(&graph, 5, 48, 32, 0.0));
        assert!(render(&graph, 5, 0, 32, 0.0).is_empty());
    }

    #[test]
    fn projects_roundtrip() {
        let graph = Graph::default();
        let json = serde_json::to_string(&graph).unwrap();
        let restored: Graph = serde_json::from_str(&json).unwrap();
        assert!(restored.validate().is_ok());
        assert_eq!(json, serde_json::to_string(&restored).unwrap());
        assert_eq!(
            render(&graph, 5, 16, 16, 1.0),
            render(&restored, 5, 16, 16, 1.0)
        );
    }
}
