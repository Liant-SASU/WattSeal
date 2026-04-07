fn main() {
    common::clog!("\u{2139} Starting UI with wgpu renderer");
    if let Err(err) = ui::run() {
        common::clog!("✗ UI failed to start with the wgpu renderer: {err}");

        common::clog!("\u{2139} Retrying UI with tiny-skia renderer");

        unsafe {
            std::env::set_var("ICED_BACKEND", "tiny-skia");
        }

        if let Err(err) = ui::run() {
            common::clog!("✗ UI failed to start with tiny-skia fallback: {err}");
        }
    }
}
