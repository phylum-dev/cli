use clap::{load_yaml, App};
use clap_generate::{
    generate_to,
    generators::{Bash, Fish, Zsh},
};

const BIN_NAME: &str = "phylum";
const OUT_DIR: &str = "src/bin/completions";

fn main() {
    println!("Running build");
    let yml = load_yaml!("src/cli.yaml");
    let mut app = App::from(yml);

    // Create tab completions files for some popular shells
    generate_to(
        Bash, &mut app, // We need to specify what generator to use
        BIN_NAME, // We need to specify the bin name manually
        OUT_DIR,  // We need to specify where to write to
    )
    .unwrap();

    generate_to(
        Zsh, &mut app, // We need to specify what generator to use
        BIN_NAME, // We need to specify the bin name manually
        OUT_DIR,  // We need to specify where to write to
    )
    .unwrap();

    generate_to(
        Fish, &mut app, // We need to specify what generator to use
        BIN_NAME, // We need to specify the bin name manually
        OUT_DIR,  // We need to specify where to write to
    )
    .unwrap();
}
