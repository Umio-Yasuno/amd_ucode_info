use std::path::Path;

fn help_msg() {
    print!("\
        \x20usage: amd_ucode_info_rs [-h] [-e EXTRACT] container_file\n\
        \n\
        \x20Print information about an amd-ucode container\n\
        \n\
        \x20positional arguments:\n\
        \x20    container_file\n\
        \n\
        \x20options:\n\
        \x20    -h, --help      show this help message and exit\n\
        \x20    -e EXTRACT, --extract EXTRACT\n\
        \x20                    Dump each patch in container to the specified directory\n\
    \n")
}

pub(crate) struct MainOpt {
    pub(crate) ucode_path: String,
    pub(crate) extract_dir: String,
}

impl MainOpt {
    pub(crate) fn parse() -> Self {
        let mut opt = Self {
            ucode_path: String::new(),
            extract_dir: String::new(),
        };
        let mut skip = false;

        let args: Vec<String> = std::env::args().collect();

        for (idx, arg) in args[1..].iter().enumerate() {
            if skip {
                skip = false;
                continue;
            }

            match arg.as_str() {
                "-e" | "--extract" => {
                    skip = true;

                    if let Some(path) = args.get(idx+2) {
                        opt.extract_dir = path.to_string()
                    } else {
                        help_msg();
                        eprintln!("-e/--extract requires one argument (directory)");
                        std::process::exit(1);
                    }
                },
                "-h" | "--help" => {
                    help_msg();
                    std::process::exit(0);
                },
                _ => {
                    if Path::new(arg).exists() {
                        opt.ucode_path = arg.to_string()
                    } else {
                        eprintln!("{arg} dose not exist");
                        eprintln!("Unknown option: {arg}")
                    }
                },
            }
        }

        if opt.ucode_path.is_empty() {
            help_msg();
            eprintln!("the following arguments are required: container_file");
            std::process::exit(1);
        }

        opt
    }
}
