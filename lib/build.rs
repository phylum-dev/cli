use clap::{load_yaml, App};
use clap_generate::{generate_to, generators::Bash};

fn main() {
    println!("Running build");
    let yml = load_yaml!("src/bin/.conf/cli.yaml");
    let mut app = App::from(yml);

    generate_to::<Bash, _, _>(
        &mut app,   // We need to specify what generator to use
        "phylum",   // We need to specify the bin name manually
        "src/bin/", // We need to specify where to write to
    );
}
