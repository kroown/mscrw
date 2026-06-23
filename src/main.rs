mod scanner;
mod image;
mod text;
mod strip;
mod json;

use std::path::PathBuf;

struct Options {
    paths: Vec<PathBuf>,
    pretty: bool,
    strip: bool,
    verbose: bool,
    threads: usize,
}

fn print_usage() {
    println!("mcsrw v{} - metadata scraper for windows", env!("CARGO_PKG_VERSION"));
    println!();
    println!("usage: mcsrw [options] <path...>");
    println!();
    println!("options:");
    println!("  --strip            strip metadata from files in-place");
    println!("  --pretty           pretty-print json");
    println!("  -v, --verbose      verbose output to stderr");
    println!("  -t, --threads <n>  worker threads");
    println!("  --help             show this help");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = Options {
        paths: Vec::new(),
        pretty: false,
        strip: false,
        verbose: false,
        threads: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => { print_usage(); return; }
            "--pretty" => opts.pretty = true,
            "--strip" => opts.strip = true,
            "-v" | "--verbose" => opts.verbose = true,
            "-t" | "--threads" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("mcsrw: --threads needs a number");
                    std::process::exit(1);
                }
                opts.threads = args[i].parse().unwrap_or(1).max(1);
            }
            _ => opts.paths.push(PathBuf::from(&args[i])),
        }
        i += 1;
    }

    if opts.paths.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let files = scanner::scan_all(&opts.paths, opts.verbose);

    if opts.strip {
        let mut stripped = 0;
        for f in &files {
            if f.is_dir { continue; }
            let ext = f.extension.to_lowercase();
            if strip::strip_file(&f.path, &ext) {
                stripped += 1;
                if opts.verbose {
                    eprintln!("  stripped: {}", f.path.display());
                }
            }
        }
        if opts.verbose {
            eprintln!("mcsrw: stripped {}/{} files", stripped, files.len());
        }
        return;
    }

    let results = scanner::collect_all(files, opts.threads, opts.verbose);
    let output = json::to_json(&results, opts.pretty);
    println!("{}", output);
}
