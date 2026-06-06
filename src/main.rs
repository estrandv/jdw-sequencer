#![feature(result_flattening, proc_macro_hygiene, decl_macro)]

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut config_path = "config.toml".to_string();
    let mut quiet = false;
    while let Some(arg) = args.next() {
        if arg == "-q" || arg == "--quiet" {
            quiet = true;
        } else {
            config_path = arg;
            break;
        }
    }

    jdw_sequencer::run(&config_path, quiet);
}
