use std::path::Path;
use std::process::Command;

const OUI_URL: &str = "https://standards-oui.ieee.org/oui/oui.csv";

fn main() {
    println!("cargo:rerun-if-env-changed=SCANOPY_UPDATE_OUI");
    println!("cargo:rerun-if-changed=assets/oui.csv");

    if std::env::var("SCANOPY_UPDATE_OUI").is_ok() {
        let assets_dir = Path::new("assets");
        let oui_path = assets_dir.join("oui.csv");

        println!("cargo:warning=Downloading fresh OUI database from IEEE...");

        let status = Command::new("curl")
            .args([
                "-sf",
                "--max-time",
                "60",
                "-o",
                oui_path.to_str().unwrap(),
                OUI_URL,
            ])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=OUI database updated successfully");
            }
            _ => {
                println!("cargo:warning=Failed to download OUI database, using existing copy");
            }
        }
    }
}
