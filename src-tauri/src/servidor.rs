// Acesso às cópias de personagem que o guardião guarda no servidor dedicado.
//
// O guardião (dbghelp.dll no servidor) grava, por jogador, guardiao-dados/<steamid>/A.bin
// (bloco player_character), K.bin (player_knowledge) e meta.json. Editar essas cópias e
// expulsar o jogador faz o guardião trocar o save dele pela versão editada na próxima entrada.
//
// A conversa com o servidor é por SFTP (módulo sftp.rs), numa sessão só, reaproveitada entre
// chamadas. Era FTPS pelo curl, que abria duas conexões por arquivo e falhava com "response
// reading failed" quando o servidor cortava conexões simultâneas.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use crate::sftp;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Config {
    pub host: String,
    pub porta: u16,
    pub usuario: String,
    pub senha: String,
    /// impressão digital da chave do servidor ("SHA256:..."). Vazia = aceita e guarda na primeira conexão.
    pub hostkey: String,
    /// resquício da época do FTPS (chave do certificado TLS); não é mais usado
    pub pin: String,
    /// pasta das cópias do guardião na raiz do servidor
    pub pasta: String,
}

/// O que a interface precisa saber, sem devolver a senha.
#[derive(Serialize)]
pub struct ConfigPublica {
    pub host: String,
    pub porta: u16,
    pub usuario: String,
    pub hostkey: String,
    pub pin: String,
    pub pasta: String,
    pub tem_senha: bool,
    pub configurado: bool,
}

fn config_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("enshrouded-save-editor")
}

fn config_path() -> PathBuf {
    config_dir().join("servidor.json")
}

pub fn carregar() -> Config {
    let mut c: Config = fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // 21 era a porta do FTPS, que não falamos mais; quem tinha a configuração antiga vai para a
    // porta de SFTP do painel do servidor.
    if c.porta == 0 || c.porta == 21 {
        c.porta = 2022;
    }
    if c.pasta.is_empty() {
        c.pasta = "guardiao-dados".into();
    }
    c
}

/// Caminho remoto seguro: só nomes simples, sem subir de pasta.
fn remoto(c: &Config, partes: &[&str]) -> Result<String, String> {
    for p in partes {
        if p.is_empty() || p.contains('/') || p.contains('\\') || p.contains("..") {
            return Err(format!("nome inválido: {p}"));
        }
    }
    Ok(format!("/{}/{}", c.pasta, partes.join("/")))
}

/// Lista nomes dentro da pasta do guardião, ou de uma subpasta dela.
async fn listar(c: &Config, sub: Option<&str>) -> Result<Vec<String>, String> {
    let caminho = match sub {
        Some(s) => remoto(c, &[s])?,
        None => format!("/{}", c.pasta),
    };
    sftp::listar(c, &caminho).await
}

#[derive(Serialize)]
pub struct Jogador {
    pub steamid: String,
    pub meta: Option<String>,
}

#[tauri::command]
pub fn servidor_config() -> ConfigPublica {
    let c = carregar();
    ConfigPublica {
        configurado: !c.host.is_empty() && !c.usuario.is_empty(),
        tem_senha: !c.senha.is_empty(),
        host: c.host,
        porta: c.porta,
        usuario: c.usuario,
        hostkey: c.hostkey,
        pin: c.pin,
        pasta: c.pasta,
    }
}

/// Grava a configuração. Senha vazia mantém a que já estava salva.
/// Com a chave do servidor em branco, guardamos a que a última conexão viu.
#[tauri::command]
pub async fn servidor_config_set(
    host: String,
    porta: u16,
    usuario: String,
    senha: String,
    hostkey: String,
    pasta: String,
) -> Result<(), String> {
    let anterior = carregar();
    let c = Config {
        host: host.trim().to_string(),
        porta: if porta == 0 { 2022 } else { porta },
        usuario: usuario.trim().to_string(),
        senha: if senha.is_empty() { anterior.senha } else { senha },
        hostkey: hostkey.trim().to_string(),
        pin: String::new(),
        pasta: if pasta.trim().is_empty() { "guardiao-dados".into() } else { pasta.trim().to_string() },
    };
    fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
    fs::write(config_path(), serde_json::to_vec_pretty(&c).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    sftp::esquecer().await; // a próxima chamada abre a sessão com os dados novos
    Ok(())
}

/// A impressão digital da chave que a última conexão viu, para fixar na configuração.
#[tauri::command]
pub fn servidor_hostkey() -> Option<String> {
    sftp::hostkey_vista()
}

/// Jogadores com cópia guardada, com o meta.json de cada um (data, versão, se o save foi o final).
#[tauri::command]
pub async fn servidor_jogadores() -> Result<Vec<Jogador>, String> {
    let c = carregar();
    let mut out = vec![];
    for nome in listar(&c, None).await? {
        if !nome.chars().all(|ch| ch.is_ascii_digit()) {
            continue; // só as pastas por steamid
        }
        let meta = sftp::baixar(&c, &remoto(&c, &[&nome, "meta.json"])?)
            .await
            .ok()
            .map(|b| String::from_utf8_lossy(&b).to_string());
        out.push(Jogador { steamid: nome, meta });
    }
    Ok(out)
}

/// Lê um dos arquivos da cópia de um jogador (A.bin, K.bin, meta.json).
#[tauri::command]
pub async fn servidor_ler(steamid: String, arquivo: String) -> Result<tauri::ipc::Response, String> {
    let c = carregar();
    sftp::baixar(&c, &remoto(&c, &[&steamid, &arquivo])?).await.map(tauri::ipc::Response::new)
}

/// Grava um arquivo da cópia, guardando antes o conteúdo atual em Documentos\Enshrouded-Backups\servidor.
#[tauri::command]
pub async fn servidor_gravar(steamid: String, arquivo: String, data: Vec<u8>) -> Result<String, String> {
    let c = carregar();
    if !matches!(arquivo.as_str(), "A.bin" | "K.bin") {
        return Err("Só A.bin e K.bin podem ser gravados".into());
    }
    if data.is_empty() {
        return Err("Nada para gravar".into());
    }
    let caminho = remoto(&c, &[&steamid, &arquivo])?;
    let segs = std::time::SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let dir = PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into()))
        .join("Documents").join("Enshrouded-Backups").join("servidor")
        .join(format!("{steamid}-{segs}"));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // backup do que está no servidor antes de sobrescrever
    match sftp::baixar(&c, &caminho).await {
        Ok(atual) => fs::write(dir.join(&arquivo), atual).map_err(|e| e.to_string())?,
        Err(e) => return Err(format!("não consegui baixar a cópia atual para backup: {e}")),
    }
    sftp::enviar(&c, &caminho, &data).await?;
    if sftp::baixar(&c, &caminho).await? != data {
        return Err("o arquivo no servidor não confere com o enviado".into());
    }
    atualizar_meta(&c, &steamid, &arquivo, data.len()).await;
    Ok(dir.to_string_lossy().to_string())
}

/// Depois de gravar, deixa o meta.json de acordo com o tamanho novo: é por ele que o editor
/// percebe que a cópia mudou. O guardião não depende disso (ele lê A.bin e K.bin direto).
async fn atualizar_meta(c: &Config, steamid: &str, arquivo: &str, tam: usize) {
    let Ok(caminho) = remoto(c, &[steamid, "meta.json"]) else { return };
    let Ok(atual) = sftp::baixar(c, &caminho).await else { return };
    let txt = String::from_utf8_lossy(&atual).to_string();
    let campo = if arquivo == "A.bin" { "tamA" } else { "nK" };
    let valor = if arquivo == "A.bin" { tam } else { tam / 8 };
    // troca só o número do campo, preservando o resto do JSON como está
    let Some(ini) = txt.find(&format!("\"{campo}\":")) else { return };
    let depois = ini + campo.len() + 3;
    let fim = depois + txt[depois..].find(|ch: char| ch == ',' || ch == '}').unwrap_or(0);
    let novo = format!("{}{valor}{}", &txt[..depois], &txt[fim..]);
    let _ = sftp::enviar(c, &caminho, novo.as_bytes()).await;
}

/// Caminho de um save recusado. "recusados" é fixo; só o nome do arquivo vem de fora.
fn remoto_recusado(c: &Config, steamid: &str, nome: Option<&str>) -> Result<String, String> {
    let base = remoto(c, &[steamid])? + "/recusados/";
    match nome {
        None => Ok(base),
        Some(n) => {
            if n.is_empty() || n.contains('/') || n.contains('\\') || n.contains("..") {
                return Err(format!("nome inválido: {n}"));
            }
            Ok(base + n)
        }
    }
}

/// Nomes dos saves que o guardião recusou de um jogador, do mais antigo ao mais novo.
///
/// A pasta não existe para quem nunca trouxe save de fora. Esse caso é diferente de uma falha de
/// conexão, e confundir os dois esconde o erro: olhamos a pasta do jogador primeiro, e só
/// devolvemos lista vazia quando ela realmente não tem "recusados".
#[tauri::command]
pub async fn servidor_recusados(steamid: String) -> Result<Vec<String>, String> {
    let c = carregar();
    if !listar(&c, Some(&steamid)).await?.iter().any(|n| n == "recusados") {
        return Ok(vec![]);
    }
    let mut nomes: Vec<String> = sftp::listar(&c, &remoto_recusado(&c, &steamid, None)?)
        .await?
        .into_iter()
        .filter(|n| n.ends_with(".bin"))
        .collect();
    nomes.sort();
    Ok(nomes)
}

/// Um save recusado, cru: os bytes do personagem seguidos dos do conhecimento.
/// Onde um termina e o outro começa é o `tamA` que o registro anotou naquela entrada.
#[tauri::command]
pub async fn servidor_recusado_ler(steamid: String, nome: String) -> Result<tauri::ipc::Response, String> {
    let c = carregar();
    sftp::baixar(&c, &remoto_recusado(&c, &steamid, Some(&nome))?)
        .await
        .map(tauri::ipc::Response::new)
}

/// Modo do guardião, lido de guardiao.json na raiz do servidor ("aplicar" ou "observar").
#[tauri::command]
pub async fn servidor_modo() -> Result<String, String> {
    let c = carregar();
    let txt = sftp::baixar(&c, "/guardiao.json").await.map(|b| String::from_utf8_lossy(&b).to_string())?;
    Ok(if txt.replace(' ', "").contains("\"modo\":\"observar\"") { "observar".into() } else { "aplicar".into() })
}

/// Últimas linhas do registro do guardião, para mostrar o que ele decidiu em cada entrada.
#[tauri::command]
pub async fn servidor_registro() -> Result<String, String> {
    let c = carregar();
    sftp::baixar(&c, &format!("/{}/registro.jsonl", c.pasta))
        .await
        .map(|b| String::from_utf8_lossy(&b).to_string())
}

/// Caminhos de todos os logs do servidor: o atual e os rotacionados.
///
/// O servidor rotaciona o log a cada reinício e move os antigos para logs/backup. Quem entrou
/// antes do último reinício só aparece lá, então quem lê só o log atual fica sem o nome dele.
async fn caminhos_dos_logs(c: &Config) -> Vec<String> {
    let mut caminhos = vec!["/logs/enshrouded_server.log".to_string()];
    for sub in ["/logs", "/logs/backup"] {
        let Ok(nomes) = sftp::listar(c, sub).await else { continue };
        for nome in nomes {
            if nome.contains("enshrouded_server") && nome.ends_with(".log") {
                caminhos.push(format!("{sub}/{nome}"));
            }
        }
    }
    caminhos.sort();
    caminhos.dedup();
    caminhos
}

/// Acrescenta a `nomes` os pares steamid -> nome que este log revela.
///
/// O log encadeia três linhas: "Added peer P(x) (steamid:S)" dá a steamid do par P;
/// "Machine 'M': Player 'P(y)' logged in" identifica o par; e a linha seguinte,
/// "Player 'NOME' logged in with Permissions:", traz o nome.
fn nomes_do_log(log: &str, nomes: &mut serde_json::Map<String, serde_json::Value>) {
    let mut pares: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    let mut pendente: Option<u32> = None;
    for linha in log.lines() {
        if let Some(r) = linha.split("[online] Added peer ").nth(1) {
            // "P(x) (steamid:S)"
            let par = r.split('(').next().and_then(|s| s.trim().parse::<u32>().ok());
            let sid = r.split("steamid:").nth(1).and_then(|s| s.split(')').next()).map(str::to_string);
            if let (Some(p), Some(s)) = (par, sid) {
                pares.insert(p, s);
            }
        } else if let Some(r) = linha.split("[server] Machine '").nth(1) {
            // "M': Player 'P(y)' logged in" -- o par está na própria linha; o número da máquina
            // é só um a mais que ele, e serve de reserva quando a parte do Player não vem.
            let par = r.split("Player '").nth(1).and_then(|s| s.split('(').next()).and_then(|s| s.parse::<u32>().ok());
            pendente = par.or_else(|| {
                r.split('\'').next().and_then(|s| s.parse::<u32>().ok()).and_then(|m| m.checked_sub(1))
            });
        } else if let Some(r) = linha.split("[server] Player '").nth(1) {
            if !r.contains("logged in with Permissions") {
                continue;
            }
            let nome = r.rsplit_once("' logged in with Permissions").map(|(n, _)| n.to_string());
            if let (Some(p), Some(n)) = (pendente.take(), nome) {
                if let Some(sid) = pares.get(&p) {
                    nomes.insert(sid.clone(), serde_json::Value::String(n));
                }
            }
        }
    }
}

/// steamid -> nome, tirado dos logs do servidor.
///
/// Os nomes ficam em cache (nomes.json na pasta de configuração) porque os logs são grandes;
/// o cache acumula, então um nome aprendido sobrevive à rotação que apaga o log de origem.
#[tauri::command]
pub async fn servidor_nomes(atualizar: bool) -> Result<serde_json::Value, String> {
    let cache = config_dir().join("nomes.json");
    if !atualizar {
        if let Some(v) = fs::read_to_string(&cache).ok().and_then(|s| serde_json::from_str(&s).ok()) {
            return Ok(v);
        }
    }
    let c = carregar();
    let mut nomes: serde_json::Map<String, serde_json::Value> = fs::read_to_string(&cache)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut erro = None;
    for caminho in caminhos_dos_logs(&c).await {
        match sftp::baixar(&c, &caminho).await {
            Ok(b) => nomes_do_log(&String::from_utf8_lossy(&b), &mut nomes),
            Err(e) => erro = Some(e),
        }
    }
    if nomes.is_empty() {
        return Err(erro.unwrap_or_else(|| "nenhum nome encontrado nos logs do servidor".into()));
    }
    let v = serde_json::Value::Object(nomes);
    let _ = fs::create_dir_all(config_dir());
    let _ = fs::write(&cache, serde_json::to_vec_pretty(&v).unwrap_or_default());
    Ok(v)
}
