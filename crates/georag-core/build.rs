// Build script for GDAL integration
// This ensures GDAL is properly linked and configured

fn main() {
    // Print cargo instructions for GDAL linking
    println!("cargo:rerun-if-env-changed=GDAL_HOME");
    println!("cargo:rerun-if-env-changed=GDAL_DATA");
    println!("cargo:rerun-if-env-changed=GDAL_DRIVER_PATH");
    
    // Check if GDAL_HOME is set and provide helpful message
    if std::env::var("GDAL_HOME").is_err() {
        println!("cargo:warning=GDAL_HOME not set. GDAL will be detected from system paths.");
    }
    
    // The actual GDAL linking is handled by gdal-sys crate
    // This build script just provides additional configuration hints
}
