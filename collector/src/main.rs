use collector::CollectorApp;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = common::set_current_dir_to_exe_dir() {
        common::clog!("⚠ Failed to set working directory to executable directory: {}", e);
    }

    let mut app = match CollectorApp::new(1, None) {
        Ok(app) => app,
        Err(e) => {
            common::clog!("✗ Failed to create CollectorApp: {}", e);
            return;
        }
    };
    if let Err(e) = app.initialize() {
        common::clog!("✗ Failed to initialize CollectorApp: {}", e);
        return;
    }
    app.run().await;
}
