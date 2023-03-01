use eframe::wgpu;
use physics::App;

fn main() {
    eframe::run_native(
        "Physics",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            vsync: false,
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                power_preference: eframe::wgpu::PowerPreference::HighPerformance,
                device_descriptor: wgpu::DeviceDescriptor {
                    label: Some("Required Device Descriptor"),
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                },
                present_mode: wgpu::PresentMode::AutoNoVsync,
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .unwrap();
}
