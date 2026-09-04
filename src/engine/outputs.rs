use std::process::Command;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MonitorOutput {
    pub name: String,
    pub description: String,
    pub resolution: String,
    pub is_primary: bool,
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
                        outputs.push(MonitorOutput {
                            name: current_name.clone(),
                            description: current_name.clone(),
                            resolution: if current_res.is_empty() { "Native".into() } else { current_res.clone() },
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
                outputs.push(MonitorOutput {
                    name: current_name.clone(),
                    description: current_name.clone(),
                    resolution: if current_res.is_empty() { "Native".into() } else { current_res },
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
                                resolution: "Unknown".into(),
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
            resolution: "All Displays".to_string(),
            is_primary: true,
        });
    }

    outputs
}
