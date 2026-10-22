use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorOutput {
    pub name: String,
    pub description: String,
    pub resolution: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    /// Logical X position from the compositor layout, if known.
    pub x: i32,
    /// Logical Y position from the compositor layout, if known.
    pub y: i32,
    /// Output scale in percent (100 = 1.0).
    pub scale_percent: u32,
    /// Rotation in degrees: 0, 90, 180 or 270.
    pub rotation: u16,
}

impl Default for MonitorOutput {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            resolution: "Native".into(),
            width: 1920,
            height: 1080,
            is_primary: false,
            x: 0,
            y: 0,
            scale_percent: 100,
            rotation: 0,
        }
    }
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

/// Returns the monitor that currently has keyboard focus on Niri. Layer
/// surfaces do not expose their output name to the client, so this is the most
/// reliable way to know which monitor a HUD invocation belongs to. Returns
/// `None` on compositors or Niri versions without this IPC command.
pub fn focused_output_name() -> Option<String> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-output"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    value
        .get("name")
        .and_then(|name| name.as_str())
        .map(str::to_string)
}

pub fn detect_outputs() -> Vec<MonitorOutput> {
    let mut outputs = Vec::new();

    let is_sandboxed = std::path::Path::new("/.flatpak-info").exists()
        || std::env::var_os("FLATPAK_ID").is_some();

    // 0. Niri IPC is the most accurate source on Niri: it reports the real
    // layout, scale and transform used by the compositor.
    if !is_sandboxed {
        if let Some(niri_outputs) = detect_niri_outputs() {
            if !niri_outputs.is_empty() {
                return niri_outputs;
            }
        }
    }

    // 1. Try cosmic-randr (Official COSMIC Wayland display manager, native only)
    if !is_sandboxed {
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
                            ..MonitorOutput::default()
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
                    ..MonitorOutput::default()
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
                                    ..MonitorOutput::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback to Linux sysfs DRM (/sys/class/drm) for native Wayland outputs and exact resolutions
    if outputs.is_empty() && !is_sandboxed {
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            let mut drm_outputs = Vec::new();
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let status_file = path.join("status");
                if let Ok(status) = std::fs::read_to_string(&status_file) {
                    if status.trim() == "connected" {
                        let modes_file = path.join("modes");
                        let mut res = "1920x1080".to_string();
                        if let Ok(modes) = std::fs::read_to_string(&modes_file) {
                            if let Some(first_mode) = modes.lines().next() {
                                if !first_mode.trim().is_empty() {
                                    res = first_mode.trim().to_string();
                                }
                            }
                        }
                        let (w, h) = parse_resolution(&res);
                        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        let name = file_name.split_once('-').map(|(_, rest)| rest).unwrap_or(file_name).to_string();
                        if !name.is_empty() {
                            drm_outputs.push(MonitorOutput {
                                name: name.clone(),
                                description: name,
                                resolution: res,
                                width: w,
                                height: h,
                                is_primary: drm_outputs.is_empty(),
                                ..MonitorOutput::default()
                            });
                        }
                    }
                }
            }
            if !drm_outputs.is_empty() {
                outputs = drm_outputs;
            }
        }
    }

    // 4. Fallback to mpvpaper -d (or primary for sandboxed Flatpak)
    if outputs.is_empty() {
        if let Ok(output) = Command::new(crate::engine::mpvpaper::resolve_mpvpaper_binary()).arg("-d").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Some(rest) = line.strip_prefix("[*] Output") {
                        let name = rest.trim_start_matches(':').split_whitespace().next().unwrap_or("");
                        if !name.is_empty() && name != "found" {
                            outputs.push(MonitorOutput {
                                name: name.to_string(),
                                description: name.to_string(),
                                resolution: "Native".into(),
                                width: 1920,
                                height: 1080,
                                is_primary: outputs.is_empty(),
                                ..MonitorOutput::default()
                            });
                        }
                    }
                }
            }
        }
    }

    // 4. Absolute fallback to wildcard (All monitors)
    if outputs.is_empty() {
        outputs.push(MonitorOutput {
            name: "*".to_string(),
            description: "All Monitors".to_string(),
            resolution: "1920x1080".to_string(),
            width: 1920,
            height: 1080,
            is_primary: true,
            ..MonitorOutput::default()
        });
    }

    outputs
}

fn detect_niri_outputs() -> Option<Vec<MonitorOutput>> {
    let output = Command::new("niri")
        .args(["msg", "--json", "outputs"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let map = value.as_object()?;
    let mut outputs = Vec::new();

    for (name, info) in map {
        let Some(logical) = info.get("logical") else {
            continue;
        };
        if logical.is_null() {
            continue;
        }
        if info.get("active").and_then(|v| v.as_bool()) == Some(false) {
            continue;
        }

        let width = logical
            .get("width")
            .and_then(|v| v.as_u64())
            .unwrap_or(1920) as u32;
        let height = logical
            .get("height")
            .and_then(|v| v.as_u64())
            .unwrap_or(1080) as u32;
        let x = logical
            .get("x")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let y = logical
            .get("y")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let scale_percent = info
            .get("scale")
            .and_then(|v| v.as_f64())
            .map(|scale| (scale * 100.0).round() as u32)
            .unwrap_or(100)
            .max(1);
        let rotation = info
            .get("transform")
            .and_then(|v| v.as_str())
            .map(transform_degrees)
            .unwrap_or(0);

        let mode = info
            .get("current_mode")
            .and_then(|v| v.as_u64())
            .and_then(|idx| {
                info.get("modes")
                    .and_then(|v| v.as_array())
                    .and_then(|modes| modes.get(idx as usize))
            });
        let (physical_w, physical_h) = mode
            .map(|m| {
                (
                    m.get("width").and_then(|v| v.as_u64()).unwrap_or(width as u64),
                    m.get("height").and_then(|v| v.as_u64()).unwrap_or(height as u64),
                )
            })
            .unwrap_or((width as u64, height as u64));
        let refresh = mode
            .and_then(|m| m.get("refresh_rate").and_then(|v| v.as_f64()))
            .map(|r| format!(" @{:.0}Hz", r))
            .unwrap_or_default();

        let make = info.get("make").and_then(|v| v.as_str()).unwrap_or("");
        let model = info.get("model").and_then(|v| v.as_str()).unwrap_or("");
        let description = match (make.trim(), model.trim()) {
            ("", "") => name.clone(),
            (make, "") => make.to_string(),
            ("", model) => model.to_string(),
            (make, model) => format!("{} {}", make, model),
        };

        outputs.push(MonitorOutput {
            name: name.clone(),
            description,
            resolution: format!("{}x{}{}", physical_w, physical_h, refresh),
            width,
            height,
            is_primary: outputs.is_empty(),
            x,
            y,
            scale_percent,
            rotation,
        });
    }

    outputs.sort_by(|a, b| (a.x, a.y, &a.name).cmp(&(b.x, b.y, &b.name)));
    for (idx, output) in outputs.iter_mut().enumerate() {
        output.is_primary = idx == 0;
    }
    Some(outputs)
}

fn transform_degrees(transform: &str) -> u16 {
    let lower = transform.to_lowercase();
    if lower.contains("90") {
        90
    } else if lower.contains("180") {
        180
    } else if lower.contains("270") {
        270
    } else {
        0
    }
}

fn parse_resolution(res: &str) -> (u32, u32) {
    if let Some((w, h)) = res.split_once('x') {
        let w_digits: String = w.trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
        let h_digits: String = h.trim_start().chars().take_while(|c| c.is_ascii_digit()).collect();
        let width = w_digits.parse::<u32>().unwrap_or(1920);
        let height = h_digits.parse::<u32>().unwrap_or(1080);
        (
            if width == 0 { 1920 } else { width },
            if height == 0 { 1080 } else { height },
        )
    } else {
        (1920, 1080)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resolution_standard() {
        assert_eq!(parse_resolution("1920x1080"), (1920, 1080));
        assert_eq!(parse_resolution("2560x1440"), (2560, 1440));
        assert_eq!(parse_resolution("3840x2160"), (3840, 2160));
    }

    #[test]
    fn test_parse_resolution_with_refresh_rate() {
        assert_eq!(parse_resolution("1920x1080@60Hz"), (1920, 1080));
        assert_eq!(parse_resolution("2560x1440@144.00"), (2560, 1440));
        assert_eq!(parse_resolution(" 1920x1080 (current)"), (1920, 1080));
    }

    #[test]
    fn test_parse_resolution_invalid_or_zero() {
        assert_eq!(parse_resolution("invalid"), (1920, 1080));
        assert_eq!(parse_resolution("0x0"), (1920, 1080));
        assert_eq!(parse_resolution(""), (1920, 1080));
    }

    #[test]
    fn test_aspect_ratio() {
        let m = MonitorOutput {
            name: "DP-1".into(),
            description: "DisplayPort 1".into(),
            resolution: "1920x1080".into(),
            width: 1920,
            height: 1080,
            is_primary: true,
            ..MonitorOutput::default()
        };
        let ratio = m.aspect_ratio();
        assert!((ratio - (16.0 / 9.0)).abs() < 0.01);

        let m_zero = MonitorOutput {
            name: "DP-1".into(),
            description: "DisplayPort 1".into(),
            resolution: "0x0".into(),
            width: 0,
            height: 0,
            is_primary: false,
            ..MonitorOutput::default()
        };
        assert_eq!(m_zero.aspect_ratio(), 16.0 / 9.0);
    }

    #[test]
    fn test_hotplug_topology_diff() {
        let old_outputs = vec![
            MonitorOutput {
                name: "eDP-1".into(),
                description: "Built-in".into(),
                resolution: "1920x1080".into(),
                width: 1920,
                height: 1080,
                is_primary: true,
                ..MonitorOutput::default()
            },
        ];

        let docked_outputs = vec![
            MonitorOutput {
                name: "DP-1".into(),
                description: "Dock Screen 1".into(),
                resolution: "2560x1440".into(),
                width: 2560,
                height: 1440,
                is_primary: true,
                ..MonitorOutput::default()
            },
            MonitorOutput {
                name: "DP-2".into(),
                description: "Dock Screen 2".into(),
                resolution: "1920x1080".into(),
                width: 1920,
                height: 1080,
                is_primary: false,
                ..MonitorOutput::default()
            },
        ];

        let disconnected: Vec<_> = old_outputs.iter()
            .filter(|old| !docked_outputs.iter().any(|d| d.name == old.name))
            .collect();
        assert_eq!(disconnected.len(), 1);
        assert_eq!(disconnected[0].name, "eDP-1");

        let added: Vec<_> = docked_outputs.iter()
            .filter(|d| !old_outputs.iter().any(|old| old.name == d.name))
            .collect();
        assert_eq!(added.len(), 2);
        assert_eq!(added[0].name, "DP-1");
        assert_eq!(added[1].name, "DP-2");
    }
}
