// Saves abertos no editor. A interface recebe o modelo (nível, habilidades, itens, conhecimento)
// e devolve o modelo editado; os bytes que o editor não interpreta ficam aqui, na sessão, e são
// remontados em volta do que mudou.
//
// Dois tipos de save: o .ksd local (contêiner SV2C inteiro) e a cópia do guardião no servidor,
// com o caminho "servidor:<steamid>" (A.bin = bloco player_character, K.bin = player_knowledge).

use crate::formato::conhecimento::{self, Conhecimento};
use crate::formato::personagem::{self, Entrada, Habilidade, Personagem};
use crate::formato::sv2c::{self, Save, CAB_BLOCO};
use crate::formato::Dados;
use crate::servidor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

const PEDIDOS: [&str; 3] = ["character_data", "player_character", "player_knowledge"];

enum Origem {
    Local(Save),
    Servidor { cabecalho: Vec<u8> },
}

struct Sessao {
    origem: Origem,
    personagem: Option<Personagem>,
    conhecimento: Option<Conhecimento>,
}

static SESSOES: LazyLock<Mutex<HashMap<String, Sessao>>> = LazyLock::new(Default::default);

fn guardar(path: String, s: Sessao) {
    SESSOES.lock().unwrap_or_else(|e| e.into_inner()).insert(path, s);
}

/// O que a interface vê de um save aberto.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Modelo {
    error: Option<String>,
    read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xp: Option<u32>,
    skills: Vec<Habilidade>,
    equipment: Vec<Entrada>,
    main: Vec<Entrada>,
    extra: Vec<Entrada>,
    knowledge: Option<Vec<conhecimento::Entrada>>,
    /// posiciona cada peça no espaço certo do boneco
    equip_mask: u32,
}

impl Modelo {
    fn de(p: &Result<Personagem, String>, k: Option<&Conhecimento>) -> Modelo {
        let c = p.as_ref().ok();
        Modelo {
            error: p.as_ref().err().cloned(),
            read_only: c.is_none(),
            level: c.map(|c| c.level),
            xp: c.map(|c| c.xp),
            skills: c.map(|c| c.skills.clone()).unwrap_or_default(),
            equipment: c.map(|c| c.equipment.clone()).unwrap_or_default(),
            main: c.map(|c| c.main.clone()).unwrap_or_default(),
            extra: c.map(|c| c.extra.clone()).unwrap_or_default(),
            knowledge: k.map(|k| k.entradas.clone()),
            equip_mask: c.map(|c| c.equip_mask).unwrap_or(0),
        }
    }
}

/// O modelo que a interface devolve para gravar.
#[derive(Deserialize)]
pub struct Edicao {
    level: u32,
    xp: u32,
    skills: Vec<Habilidade>,
    equipment: Vec<Entrada>,
    main: Vec<Entrada>,
    extra: Vec<Entrada>,
    knowledge: Option<Vec<conhecimento::Entrada>>,
}

impl Edicao {
    /// Aplica a edição sobre o personagem lido e confere que o bloco novo é lido de volta.
    fn montar(self, original: &Personagem) -> Result<Vec<u8>, String> {
        let c = Personagem {
            level: self.level, xp: self.xp, skills: self.skills,
            equipment: self.equipment, main: self.main, extra: self.extra,
            ..original.clone()
        };
        let payload = personagem::montar(&c)?;
        personagem::ler(&payload, Dados::do_jogo()).map_err(|e| format!("o bloco montado não é lido de volta: {e}"))?;
        Ok(payload)
    }
}

/// O conhecimento só é regravado se mudou: assim um save aberto e salvo sem mexer nele fica igual.
fn conhecimento_novo(s: &Sessao, edicao: &Option<Vec<conhecimento::Entrada>>) -> Option<Vec<conhecimento::Entrada>> {
    match (&s.conhecimento, edicao) {
        (Some(k), Some(novo)) if &k.entradas != novo => Some(novo.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------- saves locais (.ksd)

struct Lido {
    save: Save,
    personagem: Result<Personagem, String>,
    conhecimento: Option<Conhecimento>,
}

fn ler_local(path: &str) -> Result<Lido, String> {
    let data = std::fs::read(crate::is_save(path)?).map_err(|e| e.to_string())?;
    let save = sv2c::ler(&data, &PEDIDOS)?;
    let pc = save.bloco("player_character").and_then(|b| b.payload.as_deref()).ok_or("Save sem bloco player_character")?;
    let personagem = personagem::ler(pc, Dados::do_jogo());
    let conhecimento = save.bloco("player_knowledge").and_then(|b| b.payload.as_deref())
        .and_then(|p| conhecimento::ler_bloco(p).ok());
    Ok(Lido { save, personagem, conhecimento })
}

#[derive(Serialize)]
pub struct Tamanho { name: String, size: usize }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeloLocal {
    path: String,
    name: String,
    modified: f64,
    blocks: Vec<Tamanho>,
    #[serde(flatten)]
    modelo: Modelo,
}

fn abrir_local(path: &str) -> Result<ModeloLocal, String> {
    let l = ler_local(path)?;
    let out = ModeloLocal {
        path: path.into(),
        name: l.save.nome_do_personagem(),
        modified: crate::modified_ms(Path::new(path)),
        blocks: l.save.blocos.iter().map(|b| Tamanho { name: b.nome.clone(), size: b.payload.as_ref().map_or(b.raw.len(), Vec::len) }).collect(),
        modelo: Modelo::de(&l.personagem, l.conhecimento.as_ref()),
    };
    guardar(path.into(), Sessao { origem: Origem::Local(l.save), personagem: l.personagem.ok(), conhecimento: l.conhecimento });
    Ok(out)
}

#[tauri::command]
pub fn abrir_save(path: String) -> Result<ModeloLocal, String> {
    abrir_local(&path)
}

/// Linha da lista de saves: nome e nível, sem abrir sessão.
#[derive(Serialize)]
pub struct Resumo {
    #[serde(flatten)]
    arquivo: crate::SaveFile,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<u32>,
    error: Option<String>,
}

#[tauri::command]
pub fn listar_saves() -> Vec<Resumo> {
    crate::list_saves().into_iter().map(|arquivo| match ler_local(&arquivo.path) {
        Ok(l) => Resumo {
            name: l.save.nome_do_personagem(),
            level: l.personagem.as_ref().ok().map(|c| c.level),
            error: l.personagem.err(),
            arquivo,
        },
        Err(e) => Resumo { arquivo, name: "?".into(), level: None, error: Some(e) },
    }).collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gravado<M> {
    backup_dir: String,
    model: M,
}

#[tauri::command]
pub fn gravar_save(path: String, model: Edicao) -> Result<Gravado<ModeloLocal>, String> {
    if crate::game_running() { return Err("Feche o Enshrouded antes de salvar: o jogo sobrescreveria a edição.".into()); }
    let data = {
        let sessoes = SESSOES.lock().unwrap_or_else(|e| e.into_inner());
        let s = sessoes.get(&path).ok_or("Save não aberto para edição")?;
        let (Origem::Local(save), Some(original)) = (&s.origem, &s.personagem) else { return Err("Save não aberto para edição".into()) };
        let pk = conhecimento_novo(s, &model.knowledge)
            .map(|entradas| conhecimento::montar_bloco(&Conhecimento { estado: s.conhecimento.as_ref().map_or(1, |k| k.estado), entradas }));
        let payload = model.montar(original)?;
        let novo = Save {
            cabecalho: save.cabecalho.clone(),
            blocos: save.blocos.iter().map(|b| {
                let trocar = match b.nome.as_str() {
                    "player_character" => Some(payload.clone()),
                    "player_knowledge" => pk.clone(),
                    _ => None,
                };
                sv2c::Bloco {
                    nome: b.nome.clone(), raw: b.raw.clone(), compactado: b.compactado,
                    mudou: trocar.is_some(),
                    payload: trocar.or_else(|| b.payload.clone()),
                }
            }).collect(),
        };
        let data = sv2c::montar(&novo)?;
        sv2c::ler(&data, &[])?; // valida hashes e estrutura
        data
    };
    let backup_dir = crate::backup_file(path.clone())?;
    crate::write_save(path.clone(), data)?;
    Ok(Gravado { backup_dir, model: abrir_local(&path)? })
}

/// Esquece a sessão de um save restaurado de backup: o que estava aberto não vale mais.
pub fn esquecer(path: &str) {
    SESSOES.lock().unwrap_or_else(|e| e.into_inner()).remove(path);
}

// ---------------------------------------------------------------- cópias do servidor (guardião)

fn chave_servidor(steamid: &str) -> String { format!("servidor:{steamid}") }

#[derive(Serialize)]
pub struct ModeloServidor {
    steamid: String,
    meta: serde_json::Value,
    #[serde(flatten)]
    modelo: Modelo,
}

fn bloco_de_personagem(a: &[u8]) -> Result<(), String> {
    if a.len() < CAB_BLOCO || &a[..4] != b"S2BH" {
        return Err("a cópia do servidor não é um bloco de personagem".into());
    }
    Ok(())
}

/// Lê a cópia de um jogador. `abrir` guarda a sessão para gravar depois; sem ele é só leitura
/// (lista de saves, comparação das recusas) e não mexe numa sessão aberta.
#[tauri::command]
pub async fn servidor_abrir(steamid: String, abrir: bool) -> Result<ModeloServidor, String> {
    let a = servidor::baixar_da_copia(&steamid, "A.bin").await?;
    bloco_de_personagem(&a)?;
    let personagem = personagem::ler(&a[CAB_BLOCO..], Dados::do_jogo());
    let conhecimento = match servidor::baixar_da_copia(&steamid, "K.bin").await {
        Ok(k) => conhecimento::ler_cru(&k).ok(),
        Err(_) => None,
    };
    let meta = match servidor::baixar_da_copia(&steamid, "meta.json").await {
        Ok(m) => serde_json::from_slice(&m).unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };
    let out = ModeloServidor { steamid: steamid.clone(), meta, modelo: Modelo::de(&personagem, conhecimento.as_ref()) };
    if abrir {
        guardar(chave_servidor(&steamid), Sessao {
            origem: Origem::Servidor { cabecalho: a[..CAB_BLOCO].to_vec() },
            personagem: personagem.ok(),
            conhecimento,
        });
    }
    Ok(out)
}

/// Grava a cópia do servidor: vale na próxima entrada do jogador (expulse-o para aplicar agora).
#[tauri::command]
pub async fn servidor_gravar_personagem(steamid: String, model: Edicao) -> Result<Gravado<ModeloServidor>, String> {
    let (a, k) = {
        let sessoes = SESSOES.lock().unwrap_or_else(|e| e.into_inner());
        let s = sessoes.get(&chave_servidor(&steamid)).ok_or("Cópia do servidor não aberta para edição")?;
        let (Origem::Servidor { cabecalho }, Some(original)) = (&s.origem, &s.personagem) else {
            return Err("Cópia do servidor não aberta para edição".into());
        };
        let k = conhecimento_novo(s, &model.knowledge).map(|e| conhecimento::montar_cru(&e));
        ([&cabecalho[..], &model.montar(original)?].concat(), k)
    };
    let backup_dir = servidor::gravar(&steamid, "A.bin", &a).await?;
    if let Some(k) = k { servidor::gravar(&steamid, "K.bin", &k).await?; }
    Ok(Gravado { backup_dir, model: servidor_abrir(steamid, true).await? })
}

/// O que a comparação das recusas precisa de um save: nível, xp, itens e conhecimento.
#[derive(Serialize)]
pub struct Comparavel {
    level: u32,
    xp: u32,
    equipment: Vec<Entrada>,
    main: Vec<Entrada>,
    extra: Vec<Entrada>,
    knowledge: Vec<conhecimento::Entrada>,
}

/// Um save recusado (ou a cópia de referência gravada ao lado dele): o bloco do personagem até
/// `tam_a`, depois o conhecimento. `n_k`, quando dado, confere o tamanho com o que o registro anotou.
#[tauri::command]
pub async fn servidor_recusado_abrir(steamid: String, nome: String, tam_a: usize, n_k: Option<usize>) -> Result<Comparavel, String> {
    let blob = servidor::baixar_recusado(&steamid, &nome).await?;
    if n_k.is_some_and(|n| tam_a + n * 8 != blob.len()) {
        return Err("os tamanhos do registro não fecham com o arquivo".into());
    }
    let a = blob.get(..tam_a).filter(|a| a.len() >= CAB_BLOCO).ok_or("os tamanhos do registro não fecham com o arquivo")?;
    let c = personagem::ler(&a[CAB_BLOCO..], Dados::do_jogo())?;
    let k = conhecimento::ler_cru(&blob[tam_a..])?;
    Ok(Comparavel { level: c.level, xp: c.xp, equipment: c.equipment, main: c.main, extra: c.extra, knowledge: k.entradas })
}
