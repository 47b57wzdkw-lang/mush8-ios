// Mush8 - shfind.rs
// Constantin Clerc (c22dev) - 0.1
// Maps guest shader keys (the "vs/fs" hashes the runtime logs) back to the files
// they live in. Feed it the same game data you would give shextract, plus either
// keys on the command line or the log lines pasted into a file.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, base: &Path, found: &mut BTreeMap<u64, Vec<String>>, files: &mut usize) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, base, found, files);
            continue;
        }
        let Ok(d) = std::fs::read(&p) else { continue };
        *files += 1;
        let rel = p.strip_prefix(base).map(|r| r.to_path_buf()).unwrap_or_else(|_| p.clone());
        nxshader::corpus::scan_paths(&d, &rel.to_string_lossy(), found);
    }
}

fn keys_in(token: &str) -> Vec<u64> {
    let t = token.trim().trim_end_matches(',');
    let mut out = Vec::new();
    for side in t.split('/') {
        let side = side.split(" x").next().unwrap_or(side);
        let k = side.split(':').next().unwrap_or(side);
        if k.len() == 16 && k.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Ok(v) = u64::from_str_radix(k, 16) {
                out.push(v);
            }
        }
    }
    out
}

fn usage() -> ! {
    eprintln!("usage: shfind <game data dir> [hex key | vs/fs pair | --log <file>]...");
    eprintln!("  keys are the 16 hex digit hashes from \"[gfx] prewarm coverage\" / \"top missing shader pairs\"");
    eprintln!("  with no keys, prints every shader found (first 200 keys)");
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(dir) = args.first() else { usage() };
    let base = PathBuf::from(dir);
    if !base.is_dir() {
        eprintln!("{dir}: not a directory (point shfind at extracted game data, like shextract)");
        std::process::exit(2);
    }
    let mut want: Vec<u64> = Vec::new();
    let mut rest = args[1..].iter();
    while let Some(a) = rest.next() {
        if a.as_str() == "--log" {
            match rest.next().map(|f| std::fs::read_to_string(f)) {
                Some(Ok(text)) => {
                    for tok in text.split_whitespace() {
                        want.extend(keys_in(tok));
                    }
                }
                Some(Err(e)) => eprintln!("--log: {e}"),
                None => usage(),
            }
        } else {
            want.extend(keys_in(a));
        }
    }
    want.sort_unstable();
    want.dedup();

    let t0 = std::time::Instant::now();
    let mut found = BTreeMap::new();
    let mut files = 0;
    walk(&base, &base, &mut found, &mut files);
    eprintln!("{} files scanned, {} unique shader programs in {:.1} s", files, found.len(), t0.elapsed().as_secs_f32());

    if want.is_empty() {
        for (k, v) in found.iter().take(200) {
            for o in v {
                println!("{k:016x}\t{o}");
            }
        }
        return;
    }
    let mut hit = 0;
    for k in &want {
        match found.get(k) {
            Some(v) => {
                hit += 1;
                for o in v {
                    println!("{k:016x}\t{o}");
                }
            }
            None => println!("{k:016x}\tMISSING"),
        }
    }
    eprintln!("{hit}/{} keys found in the scanned data", want.len());
}
