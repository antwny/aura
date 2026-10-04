use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GpuVendor {
    Amd,
    Intel,
    Nvidia,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GpuInfo {
    pub id: String,                 // e.g. "renderD128"
    pub name: String,               // Display name, e.g. "AMD Radeon Graphics (renderD128)"
    pub vendor: GpuVendor,
    pub render_node: Option<PathBuf>, // e.g. Some("/dev/dri/renderD128")
    pub is_boot_vga: bool,          // primary boot VGA card (typically iGPU)
}

/// Detect available GPUs via sysfs (/sys/class/drm)
pub fn detect_available_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let drm_path = Path::new("/sys/class/drm");

    if let Ok(entries) = std::fs::read_dir(drm_path) {
        let mut sorted_entries: Vec<_> = entries.flatten().collect();
        sorted_entries.sort_by_key(|e| e.file_name());

        for entry in sorted_entries {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str.starts_with("renderD") {
                let dev_path = entry.path().join("device");
                let vendor_id = std::fs::read_to_string(dev_path.join("vendor"))
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase();

                let boot_vga = std::fs::read_to_string(dev_path.join("boot_vga"))
                    .unwrap_or_default()
                    .trim() == "1";

                let vendor = match vendor_id.as_str() {
                    "0x1002" => GpuVendor::Amd,
                    "0x8086" => GpuVendor::Intel,
                    "0x10de" => GpuVendor::Nvidia,
                    _ => GpuVendor::Other,
                };

                let dev_node = Path::new("/dev/dri").join(&*name_str);
                let render_node = if dev_node.exists() {
                    Some(dev_node)
                } else {
                    None
                };

                let vendor_label = match vendor {
                    GpuVendor::Amd => "AMD Radeon",
                    GpuVendor::Intel => "Intel Graphics",
                    GpuVendor::Nvidia => "NVIDIA GeForce",
                    GpuVendor::Other => "GPU",
                };

                let name = format!("{} ({})", vendor_label, name_str);

                gpus.push(GpuInfo {
                    id: name_str.to_string(),
                    name,
                    vendor,
                    render_node,
                    is_boot_vga: boot_vga,
                });
            }
        }
    }

    gpus
}

/// Apply GPU offload environment variables to Command
pub fn apply_gpu_env(cmd: &mut Command, preference: &str) {
    match preference {
        "discrete" => {
            cmd.env("DRI_PRIME", "1");
            cmd.env("__NV_PRIME_RENDER_OFFLOAD", "1");
            cmd.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia");
            cmd.env("__VK_LAYER_NV_optimus", "NVIDIA_only");
        }
        "integrated" => {
            cmd.env("DRI_PRIME", "0");
        }
        _ => {}
    }
}

/// Returns argument list for systemd-run --setenv if discrete or integrated GPU is requested
pub fn systemd_run_gpu_args(preference: &str) -> Vec<String> {
    let mut args = Vec::new();
    match preference {
        "discrete" => {
            args.push("--setenv=DRI_PRIME=1".to_string());
            args.push("--setenv=__NV_PRIME_RENDER_OFFLOAD=1".to_string());
            args.push("--setenv=__GLX_VENDOR_LIBRARY_NAME=nvidia".to_string());
            args.push("--setenv=__VK_LAYER_NV_optimus=NVIDIA_only".to_string());
        }
        "integrated" => {
            args.push("--setenv=DRI_PRIME=0".to_string());
        }
        _ => {}
    }
    args
}

/// Resolves appropriate vaapi device path if discrete or specific GPU is chosen
pub fn resolve_vaapi_device(preference: &str, gpus: &[GpuInfo]) -> Option<String> {
    match preference {
        "discrete" => {
            // Find a GPU marked non-boot_vga or the second render node
            let discrete = gpus.iter().find(|g| !g.is_boot_vga)
                .or_else(|| if gpus.len() > 1 { gpus.get(1) } else { None });
            discrete.and_then(|g| g.render_node.as_ref().map(|p| p.to_string_lossy().to_string()))
        }
        "integrated" => {
            let integrated = gpus.iter().find(|g| g.is_boot_vga)
                .or_else(|| gpus.first());
            integrated.and_then(|g| g.render_node.as_ref().map(|p| p.to_string_lossy().to_string()))
        }
        custom if custom.starts_with("/dev/dri/") => Some(custom.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_run_gpu_args() {
        let disc = systemd_run_gpu_args("discrete");
        assert!(disc.iter().any(|a| a == "--setenv=DRI_PRIME=1"));
        assert!(disc.iter().any(|a| a == "--setenv=__NV_PRIME_RENDER_OFFLOAD=1"));

        let integ = systemd_run_gpu_args("integrated");
        assert_eq!(integ, vec!["--setenv=DRI_PRIME=0"]);

        let auto = systemd_run_gpu_args("auto");
        assert!(auto.is_empty());
    }

    #[test]
    fn test_resolve_vaapi_device() {
        let mock_gpus = vec![
            GpuInfo {
                id: "renderD128".into(),
                name: "Intel Graphics (renderD128)".into(),
                vendor: GpuVendor::Intel,
                render_node: Some(PathBuf::from("/dev/dri/renderD128")),
                is_boot_vga: true,
            },
            GpuInfo {
                id: "renderD129".into(),
                name: "AMD Radeon (renderD129)".into(),
                vendor: GpuVendor::Amd,
                render_node: Some(PathBuf::from("/dev/dri/renderD129")),
                is_boot_vga: false,
            },
        ];

        let disc_dev = resolve_vaapi_device("discrete", &mock_gpus);
        assert_eq!(disc_dev, Some("/dev/dri/renderD129".to_string()));

        let integ_dev = resolve_vaapi_device("integrated", &mock_gpus);
        assert_eq!(integ_dev, Some("/dev/dri/renderD128".to_string()));

        let auto_dev = resolve_vaapi_device("auto", &mock_gpus);
        assert_eq!(auto_dev, None);

        let custom_dev = resolve_vaapi_device("/dev/dri/renderD130", &mock_gpus);
        assert_eq!(custom_dev, Some("/dev/dri/renderD130".to_string()));
    }

    #[test]
    fn test_detect_available_gpus_non_panicking() {
        // Must never panic even if executed in test/container environments
        let gpus = detect_available_gpus();
        for gpu in &gpus {
            assert!(!gpu.id.is_empty());
            assert!(!gpu.name.is_empty());
        }
    }
}
