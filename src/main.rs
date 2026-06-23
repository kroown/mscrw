mod scanner;
mod image;
mod text;
mod strip;
mod json;

#[cfg_attr(windows, allow(unused_imports))]
use std::io::Write;
use std::path::PathBuf;

struct Options {
    paths: Vec<PathBuf>,
    pretty: bool,
    strip: bool,
    verbose: bool,
    threads: usize,
}

fn print_usage() {
    println!("mscrw v{} - metadata scraper for windows", env!("CARGO_PKG_VERSION"));
    println!();
    println!("usage: mscrw [options] <path...>");
    println!();
    println!("options:");
    println!("  --strip            strip metadata from files in-place");
    println!("  --pretty           pretty-print json");
    println!("  -v, --verbose      verbose output to stderr");
    println!("  -t, --threads <n>  worker threads");
    println!("  --help             show this help");
}

fn install_self(verbose: bool) {
    let exe = std::env::current_exe().expect("mscrw: could not determine executable path");

    #[cfg(windows)]
    {
        let local = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| {
                let user = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".into());
                format!("{}\\AppData\\Local", user)
            });
        let dir = format!("{}\\mscrw", local);
        let bin = format!("{}\\mscrw.exe", dir);
        _ = std::fs::create_dir_all(&dir);
        _ = std::fs::copy(&exe, &bin);

        let output = std::process::Command::new("setx")
            .args(["PATH", &format!("%PATH%;{}", dir)])
            .output();

        if verbose {
            match output {
                Ok(o) if o.status.success() => {
                    println!("installed to {}", bin);
                    println!("restart your terminal for PATH changes to take effect");
                }
                _ => {
                    eprintln!("mscrw: failed to add to PATH (try running as admin?)");
                    println!("installed to {}", bin);
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/kroown".into());
        let dir = format!("{}/.local/bin", home);
        let bin = format!("{}/mscrw", dir);
        _ = std::fs::create_dir_all(&dir);
        _ = std::fs::copy(&exe, &bin);

        let shell = std::env::var("SHELL").unwrap_or_default();
        let rc = if shell.ends_with("zsh") { ".zshrc" } else { ".bashrc" };
        let rc_path = format!("{}/{}", home, rc);

        let path_line = format!("\nexport PATH=\"$PATH:{}\"\n", dir);
        if !std::fs::read_to_string(&rc_path).unwrap_or_default().contains(&dir) {
            _ = std::fs::OpenOptions::new()
                .append(true)
                .open(&rc_path)
                .and_then(|mut f| f.write_all(path_line.as_bytes()));
        }

        if verbose {
            println!("installed to {}", bin);
            println!("run `source ~/{}` or restart your shell", rc);
        }
    }
}

fn main() {
    install_self(std::env::args().any(|a| a == "-v" || a == "--verbose"));

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
                    eprintln!("mscrw: --threads needs a number");
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
            eprintln!("mscrw: stripped {}/{} files", stripped, files.len());
        }
        return;
    }

    let results = scanner::collect_all(files, opts.threads, opts.verbose);
    let output = json::to_json(&results, opts.pretty);
    println!("{}", output);
}
