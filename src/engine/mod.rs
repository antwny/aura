pub mod gpu;
pub mod mpvpaper;
pub mod outputs;

#[allow(unused_imports)]
pub use gpu::{detect_available_gpus, GpuInfo};
pub use mpvpaper::WallpaperEngine;
pub use outputs::{detect_outputs, MonitorOutput};
