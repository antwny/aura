use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorOutput {
    pub name: String,
    pub description: String,
    pub resolution: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

impl MonitorOutput {
    #[allow(dead_code)]
    pub fn aspect_ratio(&self) -> f32 {
        if self.height > 0 {
            self.width as f32 / self.height as f32
        } else {
            16.0 / 9.0
        }
    }
}

pub fn detect_outputs() -> Vec<MonitorOutput> {
    let mut outputs = Vec::new();

    // 1. Try cosmic-randr (Official COSMIC Wayland display manager)
    if let Ok(output) = Command::new("cosmic-randr").arg("list").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_name = String::new();
            let mut current_res = String::new();
            let mut is_enabled = false;

            for line in stdout.lines() {
                let trimmed = line.trim();
                if line.contains(" (enabled)") {
                    if !current_name.is_empty() && is_enabled {
                        let (w, h) = parse_resolution(&current_res);
                        outputs.push(MonitorOutput {
                            name: current_name.clone(),
                            description: current_name.clone(),
                            resolution: if current_res.is_empty() { "Native".into() } else { current_res.clone() },
                            width: w,
                            height: h,
                            is_primary: outputs.is_empty(),
                        });
                    }
                    current_name = line.split_whitespace().next().unwrap_or("").to_string();
                    current_res.clear();
                    is_enabled = true;
                } else if trimmed.contains("(current)") {
                    if let Some(res) = trimmed.split_whitespace().next() {
                        current_res = res.to_string();
                    }
                }
            }

            if !current_name.is_empty() && is_enabled {
                let (w, h) = parse_resolution(&current_res);
                outputs.push(MonitorOutput {
                    name: current_name.clone(),
                    description: current_name.clone(),
                    resolution: if current_res.is_empty() { "Native".into() } else { current_res },
                    width: w,
                    height: h,
                    is_primary: outputs.is_empty(),
                });
            }
        }
    }

    // 2. Fallback to wlr-randr if cosmic-randr had no outputs
    if outputs.is_empty() {
        if let Ok(output) = Command::new("wlr-randr").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains(" connected") || (line.chars().next().map_or(false, |c| !c.is_whitespace()) && !line.starts_with(" ")) {
                        let name = line.split_whitespace().next().unwrap_or("");
                        if !name.is_empty() {
                            outputs.push(MonitorOutput {
                                name: name.to_string(),
                                description: name.to_string(),
                                resolution: "1920x1080".into(),
                                width: 1920,
                                height: 1080,
                                is_primary: outputs.is_empty(),
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Absolute fallback to wildcard (All monitors)
    if outputs.is_empty() {
        outputs.push(MonitorOutput {
            name: "*".to_string(),
            description: "All Monitors".to_string(),
            resolution: "1920x1080".to_string(),
            width: 1920,
            height: 1080,
            is_primary: true,
        });
    }

    outputs
}

fn parse_resolution(res: &str) -> (u32, u32) {
    if let Some((w, h)) = res.split_once('x') {
        let w_clean: String = w.chars().filter(|c| c.is_ascii_digit()).collect();
        let h_clean: String = h.chars().filter(|c| c.is_ascii_digit()).collect();
        let width = w_clean.parse::<u32>().unwrap_or(1920);
        let height = h_clean.parse::<u32>().unwrap_or(1080);
        (width, height)
    } else {
        (1920, 1080)
    }
}
