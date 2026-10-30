//! zash-prompt: the shell prompt. Prints three lines for zsh: the first prompt line, the second (the arrow) and
//! the right prompt, coloured from tinct's terminal palette. No dependencies, so it starts in a millisecond.
use std::path::{Path, PathBuf};
use std::process::Command;

const HELP: &str = "zash-prompt: the shell prompt (zsh)

  zash-prompt [--status N] [--ms N] [--mode insert|normal] [--jobs N]

Prints three lines: line one, line two (the arrow) and the right prompt, with zsh's %{ %} around colours.
~/.config/zsh/utils/zz-zash-prompt.zsh wires it in.";

struct Pal { ansi: [(u8, u8, u8); 16], fg: (u8, u8, u8) }

/// tinct's ~/.config/wezterm/tinct_colors.lua: ansi, brights, foreground (zash reads the same file)
fn palette() -> Pal {
    let mut p = Pal { ansi: [(0x1d, 0x1f, 0x21), (0xcc, 0x66, 0x66), (0xb5, 0xbd, 0x68), (0xf0, 0xc6, 0x74), (0x81, 0xa2, 0xbe), (0xb2, 0x94, 0xbb), (0x8a, 0xbe, 0xb7), (0xc5, 0xc8, 0xc6),
        (0x66, 0x66, 0x66), (0xd5, 0x4e, 0x53), (0xb9, 0xca, 0x4a), (0xe7, 0xc5, 0x47), (0x7a, 0xa6, 0xda), (0xc3, 0x97, 0xd8), (0x70, 0xc0, 0xb1), (0xea, 0xea, 0xea)], fg: (0xc5, 0xc8, 0xc6) };
    let home = std::env::var("HOME").unwrap_or_default();
    let Ok(src) = std::fs::read_to_string(format!("{home}/.config/wezterm/tinct_colors.lua")) else { return p };
    let hex = |l: &str| l.split('"').nth(1).filter(|h| h.len() == 7 && h.starts_with('#')).and_then(|h| {
        let v = |i| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some((v(1)?, v(3)?, v(5)?))
    });
    let mut list: Option<(usize, usize)> = None;
    for raw in src.lines() {
        let line = raw.split("--").next().unwrap_or("").trim();
        if let Some((base, i)) = list.as_mut() {
            if line.starts_with('}') { list = None; continue }
            if let Some(c) = hex(line) { if *i < 8 { p.ansi[*base + *i] = c } *i += 1 }
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match k.trim() {
            "ansi" if v.contains('{') => list = Some((0, 0)),
            "brights" if v.contains('{') => list = Some((8, 0)),
            "foreground" => if let Some(c) = hex(line) { p.fg = c },
            _ => {}
        }
    }
    p
}

/// A piece of text in a colour: zsh's %{ %} around the escapes, % doubled.
fn paint(s: &str, c: (u8, u8, u8), bold: bool) -> String {
    format!("%{{\x1b[{}38;2;{};{};{}m%}}{}%{{\x1b[0m%}}", if bold { "1;" } else { "" }, c.0, c.1, c.2, s.replace('%', "%%"))
}
fn dim(c: (u8, u8, u8)) -> (u8, u8, u8) { ((c.0 as u16 * 55 / 100) as u8, (c.1 as u16 * 55 / 100) as u8, (c.2 as u16 * 55 / 100) as u8) }

/// The folder: ~ for home, the last three parts, the repository's own folder bold.
fn folder(cwd: &Path, repo: Option<&Path>, p: &Pal) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let shown = cwd.display().to_string();
    let shown = if !home.is_empty() && shown.starts_with(&home) { format!("~{}", &shown[home.len()..]) } else { shown };
    let parts: Vec<&str> = shown.split('/').filter(|s| !s.is_empty()).collect();
    let keep = 3;
    let lead = if parts.len() > keep { "…/".to_string() } else if shown.starts_with('/') { "/".into() } else { String::new() };
    let tail = &parts[parts.len().saturating_sub(keep)..];
    let root = repo.and_then(|r| r.file_name()).map(|n| n.to_string_lossy().to_string());
    let accent = p.ansi[12];
    let mut out = paint(&lead, dim(p.fg), false);
    for (i, part) in tail.iter().enumerate() {
        if i > 0 { out += &paint("/", dim(p.fg), false) }
        let is_root = root.as_deref() == Some(*part);
        out += &paint(part, if is_root { p.ansi[13] } else { accent }, is_root || i + 1 == tail.len());
    }
    out
}

/// The repository above cwd (a .git folder or file).
fn repo_of(cwd: &Path) -> Option<PathBuf> {
    let mut d = Some(cwd);
    while let Some(x) = d {
        if x.join(".git").exists() { return Some(x.to_path_buf()) }
        d = x.parent();
    }
    None
}

/// Branch, ahead/behind and change counts from one `git status`.
fn git(repo: &Path, p: &Pal) -> String {
    let Ok(o) = Command::new("git").args(["-C"]).arg(repo).args(["--no-optional-locks", "status", "--porcelain=v1", "-b", "--ignore-submodules=dirty"]).output() else { return String::new() };
    let s = String::from_utf8_lossy(&o.stdout);
    let (mut branch, mut ahead, mut behind) = (String::from("?"), 0, 0);
    let (mut staged, mut changed, mut new, mut conflict) = (0, 0, 0, 0);
    for l in s.lines() {
        if let Some(h) = l.strip_prefix("## ") {
            let (b, rest) = h.split_once("...").map_or((h, ""), |(b, r)| (b, r));
            branch = b.trim_start_matches("No commits yet on ").split(' ').next().unwrap_or(b).to_string();
            if let Some(a) = rest.split("ahead ").nth(1) { ahead = a.split([',', ']']).next().and_then(|n| n.trim().parse().ok()).unwrap_or(0) }
            if let Some(a) = rest.split("behind ").nth(1) { behind = a.split([',', ']']).next().and_then(|n| n.trim().parse().ok()).unwrap_or(0) }
            continue;
        }
        let b = l.as_bytes();
        if b.len() < 2 { continue }
        match (b[0], b[1]) {
            (b'?', b'?') => new += 1,
            (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => conflict += 1,
            (x, y) => { if x != b' ' { staged += 1 } if y != b' ' { changed += 1 } }
        }
    }
    let clean = staged + changed + new + conflict == 0;
    let mut out = paint(&format!("\u{e725} {branch}"), if clean { p.ansi[10] } else { p.ansi[11] }, true);
    let mut add = |n: i32, mark: &str, c| if n > 0 { out += &paint(&format!(" {mark}{n}"), c, false) };
    add(staged, "+", p.ansi[10]);
    add(changed, "~", p.ansi[11]);
    add(new, "?", p.ansi[14]);
    add(conflict, "!", p.ansi[9]);
    add(ahead, "⇡", p.ansi[12]);
    add(behind, "⇣", p.ansi[13]);
    out
}

/// The project's language, from the files at its root.
fn lang(dir: &Path) -> Option<&'static str> {
    [("Cargo.toml", "rust"), ("package.json", "node"), ("pyproject.toml", "python"), ("go.mod", "go"), ("CMakeLists.txt", "cmake"), ("build.zig", "zig"), ("flake.nix", "nix")]
        .into_iter().find(|(f, _)| dir.join(f).exists()).map(|(_, l)| l)
}

fn took(ms: u64) -> String {
    match ms {
        0..=999 => format!("{ms}ms"),
        1000..=59_999 => format!("{:.1}s", ms as f64 / 1000.0),
        _ => format!("{}m{}s", ms / 60_000, ms / 1000 % 60),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") { println!("{HELP}"); return }
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let status: i32 = get("--status").and_then(|s| s.parse().ok()).unwrap_or(0);
    let ms: Option<u64> = get("--ms").and_then(|s| s.parse().ok());
    let mode = get("--mode").unwrap_or_else(|| "insert".into());
    let jobs: u32 = get("--jobs").and_then(|s| s.parse().ok()).unwrap_or(0);
    let p = palette();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let repo = repo_of(&cwd);
    let frame = dim(p.fg);
    let sep = paint(" · ", frame, false);

    // line one: ╭─ [host over ssh] folder  git · language · time taken
    let mut one = paint("╭─ ", frame, false);
    if std::env::var_os("SSH_CONNECTION").is_some() {
        let host = std::fs::read_to_string("/etc/hostname").unwrap_or_default();
        one += &paint(&format!("{}@{} ", std::env::var("USER").unwrap_or_default(), host.trim()), p.ansi[14], true);
    }
    one += &folder(&cwd, repo.as_deref(), &p);
    if let Some(r) = &repo { one += "  "; one += &git(r, &p) }
    if let Some(l) = lang(repo.as_deref().unwrap_or(&cwd)) { one += &sep; one += &paint(l, p.ansi[6], false) }
    if let Some(ms) = ms.filter(|&m| m >= 2) { one += &sep; one += &paint(&took(ms), if ms > 5000 { p.ansi[11] } else { dim(p.fg) }, false) }
    if jobs > 0 { one += &sep; one += &paint(&format!("{jobs} job{}", if jobs == 1 { "" } else { "s" }), p.ansi[13], false) }

    // line two: ╰─❯, red after a failure, blue in normal mode
    let arrow = if status != 0 { p.ansi[9] } else if mode == "normal" { p.ansi[12] } else { p.ansi[10] };
    let two = paint("╰─", frame, false) + &paint(if mode == "normal" { "❮ " } else { "❯ " }, arrow, true);

    // right: the exit status when it failed, the time
    let mut right = String::new();
    if status != 0 { right += &paint(&format!("✗ {status} "), p.ansi[9], true) }
    // the time: zsh fills it in (%D), no process for it
    let d = dim(p.fg);
    right += &format!("%{{\x1b[38;2;{};{};{}m%}}%D{{%H:%M}}%{{\x1b[0m%}}", d.0, d.1, d.2);
    println!("{one}\n{two}\n{right}");
}
