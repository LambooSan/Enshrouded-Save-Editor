// Versão Tauri do editor: o formato do save (formato/) e os saves abertos (sessao.rs) ficam aqui,
// junto com arquivos, backups, detecção do jogo e o protocolo game:// com os recursos embutidos no
// exe. O WebView (src/) só desenha e edita o modelo.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod formato;
mod servidor;
mod sessao;
mod sftp;
#[cfg(test)]
mod testes_e2e;

use include_dir::{include_dir, Dir};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;
use tauri::http::Response as HttpResponse;

// texturas, ícones e fontes extraídos do jogo (tools/pack_assets.py)
static GAME: Dir = include_dir!("$CARGO_MANIFEST_DIR/../build-assets/game");

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub(crate) fn modified_ms(p: &Path) -> f64 {
    fs::metadata(p).and_then(|m| m.modified()).ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

pub(crate) fn is_save(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    match p.extension().and_then(|e| e.to_str()) {
        Some(e) if e.eq_ignore_ascii_case("ksd") => Ok(p),
        _ => Err("Só arquivos .ksd podem ser lidos ou gravados".into()),
    }
}

fn backup_root() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    Path::new(&home).join("Documents").join("Enshrouded-Backups").join("editor")
}

#[derive(Serialize)]
pub(crate) struct SaveFile { pub path: String, file: String, modified: f64 }

/// `character0.ksd`, `character12.ksd`. Cópias do jogo são `character0.ksd.backup-N` e não entram.
fn is_character_ksd(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let Some(stem) = lower.strip_suffix(".ksd") else { return false };
    let Some(n) = stem.strip_prefix("character") else { return false };
    !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())
}

fn reg_sz(key: &str, value: &str) -> Option<String> {
    let mut cmd = Command::new("reg");
    cmd.args(["query", key, "/v", value]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().ok()?;
    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(value) else { continue };
        let mut parts = rest.split_whitespace();
        let ty = parts.next()?;
        if ty != "REG_SZ" && ty != "REG_EXPAND_SZ" { continue; }
        let data = parts.collect::<Vec<_>>().join(" ");
        if !data.is_empty() { return Some(data.to_string()); }
    }
    None
}

fn push_unique_dir(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if !dir.is_dir() { return; }
    let key = dir.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
    if dirs.iter().any(|d| d.to_string_lossy().replace('/', "\\").to_ascii_lowercase() == key) { return; }
    dirs.push(dir);
}

/// O Steam nem sempre está em Program Files. O caminho real fica no registro
/// (`SteamPath` / `InstallPath`); os dois caminhos antigos continuam como reserva.
fn steam_userdata_dirs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for (key, value) in [
        ("HKCU\\Software\\Valve\\Steam", "SteamPath"),
        ("HKLM\\SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath"),
        ("HKLM\\SOFTWARE\\Valve\\Steam", "InstallPath"),
    ] {
        if let Some(p) = reg_sz(key, value) {
            push_unique_dir(&mut roots, PathBuf::from(p).join("userdata"));
        }
    }
    for p in ["C:/Program Files (x86)/Steam/userdata", "C:/Program Files/Steam/userdata"] {
        push_unique_dir(&mut roots, PathBuf::from(p));
    }
    roots
}

fn push_characters_in(dir: &Path, out: &mut Vec<SaveFile>, seen: &mut Vec<String>) {
    let Ok(files) = fs::read_dir(dir) else { return };
    for f in files.flatten() {
        if !f.path().is_file() { continue; }
        let name = f.file_name().to_string_lossy().to_string();
        if !is_character_ksd(&name) { continue; }
        let path = f.path();
        let key = path.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
        if seen.iter().any(|s| s == &key) { continue; }
        seen.push(key);
        out.push(SaveFile { path: path.to_string_lossy().to_string(), file: name, modified: modified_ms(&path) });
    }
}

/// No 1.0 o jogo também grava o personagem em Saved Games, não só na nuvem da Steam.
/// A pasta `backup` é pulada: lá ficam as cópias automáticas, não o save atual.
fn collect_local_characters(dir: &Path, depth: u32, out: &mut Vec<SaveFile>, seen: &mut Vec<String>) {
    if depth > 4 || !dir.is_dir() { return; }
    push_characters_in(dir, out, seen);
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if !e.path().is_dir() { continue; }
        if e.file_name().to_string_lossy().eq_ignore_ascii_case("backup") { continue; }
        collect_local_characters(&e.path(), depth + 1, out, seen);
    }
}

pub(crate) fn list_saves() -> Vec<SaveFile> {
    let mut out = vec![];
    let mut seen = vec![];
    for root in steam_userdata_dirs() {
        let Ok(users) = fs::read_dir(&root) else { continue };
        for u in users.flatten() {
            push_characters_in(&u.path().join("1203620").join("remote"), &mut out, &mut seen);
        }
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let local = PathBuf::from(home).join("Saved Games").join("keen games").join("Enshrouded");
        collect_local_characters(&local, 0, &mut out, &mut seen);
    }
    out
}

#[tauri::command]
fn game_running() -> bool {
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FI", "IMAGENAME eq enshrouded.exe", "/NH"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("enshrouded.exe")).unwrap_or(false)
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)?.flatten() {
        let t = dst.join(e.file_name());
        if e.path().is_dir() { copy_dir(&e.path(), &t)?; } else { fs::copy(e.path(), t)?; }
    }
    Ok(())
}

fn make_backup(file: &Path) -> Result<PathBuf, String> {
    let secs = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let dir = backup_root().join(format!("tauri-{secs}"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::copy(file, dir.join(file.file_name().unwrap())).map_err(|e| e.to_string())?;
    if let Some(parent) = file.parent() {
        let b = parent.join("backup");
        if b.is_dir() { copy_dir(&b, &dir.join("backup")).map_err(|e| e.to_string())?; }
    }
    Ok(dir)
}

pub(crate) fn backup_file(path: String) -> Result<String, String> {
    make_backup(&is_save(&path)?).map(|d| d.to_string_lossy().to_string())
}

pub(crate) fn write_save(path: String, data: Vec<u8>) -> Result<(), String> {
    if game_running() { return Err("Feche o Enshrouded antes de salvar.".into()); }
    let p = is_save(&path)?;
    if data.len() < 0x30 || &data[..4] != b"SV2C" { return Err("Dados inválidos".into()); }
    fs::write(p, data).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct Backup { kind: String, label: String, path: String, modified: f64 }

// pasta com todos os backups (do editor e manuais): Documentos\Enshrouded-Backups
fn backups_home() -> PathBuf { backup_root().parent().map(Path::to_path_buf).unwrap_or_else(backup_root) }

fn collect(dir: &Path, base: &str, kind: &str, depth: u32, out: &mut Vec<Backup>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth > 0 { collect(&p, base, kind, depth - 1, out); }
        } else if e.file_name().to_string_lossy() == base {
            let label = p.parent().and_then(|d| d.strip_prefix(backups_home()).ok()).map(|r| r.to_string_lossy().to_string()).unwrap_or_default();
            out.push(Backup { kind: kind.into(), label, path: p.to_string_lossy().to_string(), modified: modified_ms(&p) });
        }
    }
}

// backups de um save: os 10 automáticos do jogo (remote\backup) e os do editor/manuais
#[tauri::command]
fn list_backups(target: String) -> Vec<Backup> {
    let t = PathBuf::from(&target);
    let base = t.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut out = vec![];
    if let Some(dir) = t.parent() {
        if let Ok(rd) = fs::read_dir(dir.join("backup")) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_string();
                if n.starts_with(&format!("{base}.backup-")) {
                    out.push(Backup { kind: "jogo".into(), label: n.clone(), path: e.path().to_string_lossy().to_string(), modified: modified_ms(&e.path()) });
                }
            }
        }
    }
    collect(&backups_home(), &base, "pasta", 3, &mut out);
    out.sort_by(|a, b| b.modified.partial_cmp(&a.modified).unwrap_or(std::cmp::Ordering::Equal));
    out
}

#[tauri::command]
fn restore_backup(src: String, target: String) -> Result<(), String> {
    if game_running() { return Err("Feche o Enshrouded antes de restaurar.".into()); }
    let t = is_save(&target)?;
    let s = PathBuf::from(&src);
    let game_backups = t.parent().map(|d| d.join("backup")).unwrap_or_default();
    if !(s.starts_with(backups_home()) || s.starts_with(&game_backups)) || !s.is_file() { return Err("Backup não encontrado".into()); }
    let data = fs::read(&s).map_err(|e| e.to_string())?;
    if data.len() < 4 || &data[..4] != b"SV2C" { return Err("O arquivo de backup não é um save válido".into()); }
    make_backup(&t)?;
    fs::write(t, data).map_err(|e| e.to_string())?;
    sessao::esquecer(&target);
    Ok(())
}

#[tauri::command]
fn open_backups() {
    let r = backups_home();
    let _ = fs::create_dir_all(&r);
    let _ = Command::new("explorer").arg(r).spawn();
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "webp" => "image/webp", "png" => "image/png", "otf" => "font/otf", "ttf" => "font/ttf", _ => "application/octet-stream",
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("game", |_ctx, req| {
            let path = req.uri().path().trim_start_matches('/');
            let path = percent_decode(path);
            match GAME.get_file(&path) {
                Some(f) => HttpResponse::builder().header("Content-Type", mime(&path)).header("Access-Control-Allow-Origin", "*")
                    .body(f.contents().to_vec()).unwrap(),
                None => HttpResponse::builder().status(404).body(Vec::new()).unwrap(),
            }
        })
        .invoke_handler(tauri::generate_handler![
            game_running, list_backups, restore_backup, open_backups,
            sessao::listar_saves, sessao::abrir_save, sessao::gravar_save,
            sessao::servidor_abrir, sessao::servidor_gravar_personagem, sessao::servidor_recusado_abrir,
            servidor::servidor_config, servidor::servidor_config_set, servidor::servidor_jogadores,
            servidor::servidor_ler, servidor::servidor_modo,
            servidor::servidor_registro, servidor::servidor_nomes,
            servidor::servidor_recusados, servidor::servidor_hostkey
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o editor");
}

#[cfg(test)]
mod testes_lista {
    use super::is_character_ksd;

    #[test]
    fn reconhece_character_ksd() {
        assert!(is_character_ksd("character0.ksd"));
        assert!(is_character_ksd("character12.ksd"));
        assert!(is_character_ksd("Character0.KSD"));
        assert!(!is_character_ksd("character.ksd"));
        assert!(!is_character_ksd("character0.ksd.backup-0"));
        assert!(!is_character_ksd("world0.ksd"));
        assert!(!is_character_ksd("global.ksd"));
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
