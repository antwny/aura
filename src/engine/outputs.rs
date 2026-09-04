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
        };
        assert_eq!(m_zero.aspect_ratio(), 16.0 / 9.0);
    }
}
