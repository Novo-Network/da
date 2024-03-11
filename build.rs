use std::process::Command;

fn main() {
    if cfg!(feature = "greenfield") {
        // download gf server
        Command::new("git")
            .args(["submodule", "update", "--init", "--remote"])
            .status()
            .expect("Failed to initialize and update submodule");

        // build
        Command::new("make")
            .args(["build"])
            .current_dir("components/gf-sdk-server")
            .status()
            .expect("Failed to execute make in the submodule");
    }
}
