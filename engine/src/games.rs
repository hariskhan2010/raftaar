//! Finds installed games so Game Boost can offer them in one click.
//! Read-only: looks at launcher manifests, never launches or changes anything.

use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Game {
    pub name: String,
    /// Lowercase exe file name, the form Game Boost matches on.
    pub exe: String,
    pub launcher: &'static str,
    pub install_dir: String,
}

/// Quoted tokens of a Valve KeyValues (.vdf/.acf) file, with `\\` unescaped.
fn vdf_tokens(text: &str) -> Vec<String> {
    let mut out = vec![];
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut tok = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    if let Some(n) = chars.next() {
                        tok.push(n);
                    }
                }
                '"' => break,
                c => tok.push(c),
            }
        }
        out.push(tok);
    }
    out
}

fn vdf_values<'a>(tokens: &'a [String], key: &str) -> Vec<&'a str> {
    tokens.windows(2).filter(|w| w[0].eq_ignore_ascii_case(key)).map(|w| w[1].as_str()).collect()
}

/// Helper exes that ship next to games and are never the game itself.
fn is_helper(name: &str) -> bool {
    const HELPERS: &[&str] = &[
        "unins", "setup", "crash", "redist", "vcredist", "dxsetup", "uninstall", "updater", "report", "easyanticheat",
        "battleye", "be_service", "dotnet", "directx", "cefprocess", "helper", "installer", "prereq", "ue4prereq",
    ];
    let n = name.to_lowercase();
    HELPERS.iter().any(|h| n.contains(h))
}

/// The game's exe is almost always the biggest non-helper exe in its folder.
fn main_exe(dir: &Path, depth: u32) -> Option<(String, u64)> {
    let mut best: Option<(String, u64)> = None;
    let Ok(entries) = std::fs::read_dir(dir) else { return None };
    for e in entries.flatten() {
        let Ok(meta) = e.metadata() else { continue };
        let candidate = if meta.is_dir() {
            if depth == 0 {
                continue;
            }
            main_exe(&e.path(), depth - 1)
        } else {
            let name = e.file_name().to_string_lossy().to_lowercase();
            (name.ends_with(".exe") && !is_helper(&name)).then_some((name, meta.len()))
        };
        if let Some(c) = candidate
            && best.as_ref().is_none_or(|b| c.1 > b.1)
        {
            best = Some(c);
        }
    }
    best
}

pub fn steam_games(steam_root: &Path) -> Vec<Game> {
    let mut libraries = vec![steam_root.to_path_buf()];
    if let Ok(text) = std::fs::read_to_string(steam_root.join(r"steamapps\libraryfolders.vdf")) {
        for p in vdf_values(&vdf_tokens(&text), "path") {
            let p = PathBuf::from(p);
            if !libraries.contains(&p) {
                libraries.push(p);
            }
        }
    }
    let mut games = vec![];
    for lib in libraries {
        let apps = lib.join("steamapps");
        let Ok(entries) = std::fs::read_dir(&apps) else { continue };
        for e in entries.flatten() {
            let file = e.file_name().to_string_lossy().into_owned();
            if !(file.starts_with("appmanifest_") && file.ends_with(".acf")) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            let toks = vdf_tokens(&text);
            let (Some(name), Some(dir)) = (vdf_values(&toks, "name").first().copied(), vdf_values(&toks, "installdir").first().copied())
            else {
                continue;
            };
            if name.starts_with("Steamworks") || name.starts_with("Proton") || name.contains("Redistributable") {
                continue;
            }
            let install = apps.join("common").join(dir);
            if let Some((exe, _)) = main_exe(&install, 4) {
                games.push(Game { name: name.to_string(), exe, launcher: "Steam", install_dir: install.display().to_string() });
            }
        }
    }
    games
}

pub fn epic_games(manifests: &Path) -> Vec<Game> {
    let mut games = vec![];
    let Ok(entries) = std::fs::read_dir(manifests) else { return games };
    for e in entries.flatten() {
        if e.path().extension().is_none_or(|x| x != "item") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let (Some(name), Some(exe), Some(dir)) =
            (v["DisplayName"].as_str(), v["LaunchExecutable"].as_str(), v["InstallLocation"].as_str())
        else {
            continue;
        };
        let Some(exe) = Path::new(exe).file_name().map(|f| f.to_string_lossy().to_lowercase()) else { continue };
        if exe.is_empty() || !Path::new(dir).exists() {
            continue;
        }
        games.push(Game { name: name.into(), exe, launcher: "Epic", install_dir: dir.into() });
    }
    games
}

/// Launchers without a manifest format we can read: detect by folder.
fn known_games(program_files_drive: &Path, local_app_data: Option<&Path>) -> Vec<Game> {
    let mut games = vec![];
    let riot = program_files_drive.join("Riot Games");
    for (sub, name, exe) in [
        ("VALORANT", "VALORANT", "valorant-win64-shipping.exe"),
        ("League of Legends", "League of Legends", "league of legends.exe"),
    ] {
        let dir = riot.join(sub);
        if dir.exists() {
            games.push(Game { name: name.into(), exe: exe.into(), launcher: "Riot", install_dir: dir.display().to_string() });
        }
    }
    if let Some(l) = local_app_data {
        let dir = l.join(r"Roblox\Versions");
        if dir.exists() {
            games.push(Game {
                name: "Roblox".into(),
                exe: "robloxplayerbeta.exe".into(),
                launcher: "Roblox",
                install_dir: dir.display().to_string(),
            });
        }
    }
    games
}

#[cfg(windows)]
pub fn detect() -> Vec<Game> {
    use winreg::RegKey;
    use winreg::enums::*;
    let mut games = vec![];
    if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam")
        && let Ok(p) = k.get_value::<String, _>("SteamPath")
    {
        games.extend(steam_games(Path::new(&p.replace('/', "\\"))));
    }
    if let Some(pd) = std::env::var_os("ProgramData") {
        games.extend(epic_games(&Path::new(&pd).join(r"Epic\EpicGamesLauncher\Data\Manifests")));
    }
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    games.extend(known_games(Path::new(&format!(r"{drive}\")), local.as_deref()));
    games.sort_by_key(|g| g.name.to_lowercase());
    games.dedup_by(|a, b| a.exe == b.exe);
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    #[test]
    fn vdf_tokens_unescape_paths() {
        let t = vdf_tokens("\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t}\n}");
        assert_eq!(vdf_values(&t, "path"), vec!["D:\\SteamLibrary"]);
    }

    #[test]
    fn steam_games_found_across_libraries_with_biggest_exe() {
        let root = tempfile::tempdir().unwrap();
        let lib2 = tempfile::tempdir().unwrap();
        let vdf = format!("\"libraryfolders\" {{ \"1\" {{ \"path\" \"{}\" }} }}", lib2.path().display().to_string().replace('\\', "\\\\"));
        write(&root.path().join(r"steamapps\libraryfolders.vdf"), vdf.as_bytes());
        write(
            &lib2.path().join(r"steamapps\appmanifest_730.acf"),
            b"\"AppState\" { \"name\" \"Counter-Strike 2\" \"installdir\" \"Counter-Strike Global Offensive\" }",
        );
        let game = lib2.path().join(r"steamapps\common\Counter-Strike Global Offensive");
        write(&game.join(r"game\bin\win64\cs2.exe"), &[0; 500]);
        write(&game.join(r"game\bin\win64\crashhandler.exe"), &[0; 900]);
        write(&game.join("small.exe"), &[0; 10]);
        write(
            &root.path().join(r"steamapps\appmanifest_228980.acf"),
            b"\"AppState\" { \"name\" \"Steamworks Common Redistributables\" \"installdir\" \"Steamworks Shared\" }",
        );
        let games = steam_games(root.path());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "Counter-Strike 2");
        assert_eq!(games[0].exe, "cs2.exe");
    }

    #[test]
    fn epic_manifest_gives_exe_name() {
        let d = tempfile::tempdir().unwrap();
        let install = tempfile::tempdir().unwrap();
        let item = serde_json::json!({
            "DisplayName": "Fortnite",
            "LaunchExecutable": "FortniteGame/Binaries/Win64/FortniteLauncher.exe",
            "InstallLocation": install.path().display().to_string(),
        });
        write(&d.path().join("ABC.item"), item.to_string().as_bytes());
        write(&d.path().join("junk.txt"), b"x");
        let games = epic_games(d.path());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].exe, "fortnitelauncher.exe");
        assert_eq!(games[0].launcher, "Epic");
    }

    #[test]
    fn riot_and_roblox_detected_by_folder() {
        let drive = tempfile::tempdir().unwrap();
        let local = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(drive.path().join(r"Riot Games\VALORANT")).unwrap();
        std::fs::create_dir_all(local.path().join(r"Roblox\Versions")).unwrap();
        let games = known_games(drive.path(), Some(local.path()));
        let exes: Vec<_> = games.iter().map(|g| g.exe.as_str()).collect();
        assert_eq!(exes, vec!["valorant-win64-shipping.exe", "robloxplayerbeta.exe"]);
    }
}
