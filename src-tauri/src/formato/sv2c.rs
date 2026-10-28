// Contêiner SV2C (.ksd): cabeçalho de 0x30 bytes, os blocos S2BH em sequência e, no fim, o
// diretório (20 bytes por bloco) seguido dos nomes. Cada bloco tem 14 bytes de cabeçalho e o
// conteúdo, compactado em zstd quando o byte 9 é 1. O hash de cada bloco são os 32 bits baixos do
// xxh3-64 dos bytes do bloco inteiro.
//
// Só os blocos pedidos são descompactados; os não alterados são regravados com os bytes originais.

use super::u32_em;
use xxhash_rust::xxh3::xxh3_64;

pub const CAB_BLOCO: usize = 14;

pub struct Bloco {
    pub nome: String,
    pub raw: Vec<u8>,
    pub compactado: bool,
    /// conteúdo descompactado; só existe nos blocos pedidos na leitura
    pub payload: Option<Vec<u8>>,
    /// payload novo, recompactado na montagem
    pub mudou: bool,
}

pub struct Save {
    pub cabecalho: Vec<u8>,
    pub blocos: Vec<Bloco>,
}

fn hash32(b: &[u8]) -> u32 { xxh3_64(b) as u32 }

pub fn ler(data: &[u8], pedidos: &[&str]) -> Result<Save, String> {
    if data.get(..4) != Some(b"SV2C") { return Err("Arquivo não é um save SV2C".into()); }
    let ler32 = |o: usize| u32_em(data, o).map(|v| v as usize).ok_or_else(|| "Save truncado".to_string());
    let cabecalho = data.get(..0x30).ok_or("Save truncado")?.to_vec();
    let dir = ler32(0x28)? + 40;
    let n = ler32(0x2c)?;
    let mut blocos = Vec::with_capacity(n.min(256));
    for i in 0..n {
        let e = dir + 20 * i;
        let (nome_rel, nome_len, off, tam, h) = (ler32(e)?, ler32(e + 4)?, ler32(e + 8)?, ler32(e + 12)?, ler32(e + 16)? as u32);
        let nome = data.get(e + nome_rel..e + nome_rel + nome_len).ok_or("Diretório do save truncado")?;
        let nome = String::from_utf8_lossy(nome).to_string();
        let raw = data.get(off..off + tam).ok_or_else(|| format!("Bloco {nome} fora do arquivo"))?.to_vec();
        if raw.get(..4) != Some(b"S2BH") || raw.len() < CAB_BLOCO { return Err(format!("Bloco {nome} sem assinatura S2BH")); }
        if hash32(&raw) != h { return Err(format!("Hash do bloco {nome} não confere")); }
        let compactado = raw[9] == 1;
        let payload = if pedidos.contains(&nome.as_str()) {
            let corpo = &raw[CAB_BLOCO..];
            Some(if compactado { zstd::stream::decode_all(corpo).map_err(|e| format!("Bloco {nome}: {e}"))? } else { corpo.to_vec() })
        } else {
            None
        };
        blocos.push(Bloco { nome, raw, compactado, payload, mudou: false });
    }
    Ok(Save { cabecalho, blocos })
}

/// Blocos com `mudou` são recompactados a partir do payload; os demais usam os bytes originais.
pub fn montar(save: &Save) -> Result<Vec<u8>, String> {
    let mut out = save.cabecalho.clone();
    let mut entradas = Vec::with_capacity(save.blocos.len());
    for b in &save.blocos {
        let novo;
        let raw: &[u8] = if b.mudou {
            let payload = b.payload.as_deref().ok_or_else(|| format!("Bloco {} alterado sem conteúdo", b.nome))?;
            let corpo = if b.compactado { zstd::bulk::compress(payload, 19).map_err(|e| e.to_string())? } else { payload.to_vec() };
            novo = [&b.raw[..CAB_BLOCO], &corpo[..]].concat();
            &novo
        } else {
            &b.raw
        };
        entradas.push((out.len() as u32, raw.len() as u32, hash32(raw), b.nome.as_bytes()));
        out.extend_from_slice(raw);
    }
    out.resize(out.len().div_ceil(8) * 8, 0);
    let dir = out.len();
    let n = entradas.len();
    let mut nome_pos = 0;
    for (i, (off, tam, h, nome)) in entradas.iter().enumerate() {
        for v in [(20 * (n - i) + nome_pos) as u32, nome.len() as u32, *off, *tam, *h] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        nome_pos += nome.len();
    }
    for (.., nome) in &entradas { out.extend_from_slice(nome); }
    let total = out.len() as u32;
    out[0x08..0x0c].copy_from_slice(&total.to_le_bytes());
    out[0x28..0x2c].copy_from_slice(&((dir - 40) as u32).to_le_bytes());
    out[0x2c..0x30].copy_from_slice(&(n as u32).to_le_bytes());
    Ok(out)
}

impl Save {
    pub fn bloco(&self, nome: &str) -> Option<&Bloco> { self.blocos.iter().find(|b| b.nome == nome) }

    pub fn nome_do_personagem(&self) -> String {
        let Some(p) = self.bloco("character_data").and_then(|b| b.payload.as_deref()) else { return "?".into() };
        let Some(len) = u32_em(p, 5).filter(|_| p.len() >= 9) else { return "?".into() };
        let fim = (9 + (len as usize).min(64)).min(p.len());
        String::from_utf8_lossy(&p[9..fim]).to_string()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn bloco(nome: &str, compactado: bool, conteudo: &[u8]) -> Bloco {
        let mut cab = b"S2BH".to_vec();
        cab.extend_from_slice(&[0; 10]);
        cab[9] = compactado as u8;
        let corpo = if compactado { zstd::bulk::compress(conteudo, 3).unwrap() } else { conteudo.to_vec() };
        Bloco { nome: nome.into(), raw: [cab, corpo].concat(), compactado, payload: None, mudou: false }
    }

    fn save_sintetico() -> Vec<u8> {
        let mut cabecalho = b"SV2C".to_vec();
        cabecalho.resize(0x30, 0x11);
        let mut nome = vec![0u8; 9];
        nome[5..9].copy_from_slice(&6u32.to_le_bytes());
        nome.extend_from_slice(b"Viktor");
        let save = Save {
            cabecalho,
            blocos: vec![bloco("character_data", true, &nome), bloco("player_character", true, &[7; 300]), bloco("outro", false, b"abc")],
        };
        montar(&save).unwrap()
    }

    #[test]
    fn le_o_que_monta() {
        let data = save_sintetico();
        let s = ler(&data, &["character_data", "player_character"]).unwrap();
        assert_eq!(s.blocos.len(), 3);
        assert_eq!(s.nome_do_personagem(), "Viktor");
        assert_eq!(s.bloco("player_character").unwrap().payload.as_deref(), Some(&[7u8; 300][..]));
        assert!(s.bloco("outro").unwrap().payload.is_none());
        // sem nada alterado, remonta idêntico
        assert_eq!(montar(&s).unwrap(), data);
    }

    #[test]
    fn recompacta_o_bloco_alterado() {
        let mut s = ler(&save_sintetico(), &["player_character"]).unwrap();
        let b = s.blocos.iter_mut().find(|b| b.nome == "player_character").unwrap();
        b.payload = Some(vec![9; 500]);
        b.mudou = true;
        let novo = montar(&s).unwrap();
        let relido = ler(&novo, &["player_character", "outro"]).unwrap();
        assert_eq!(relido.bloco("player_character").unwrap().payload.as_deref(), Some(&[9u8; 500][..]));
        assert_eq!(relido.bloco("outro").unwrap().payload.as_deref(), Some(&b"abc"[..]));
        assert_eq!(u32_em(&novo, 8), Some(novo.len() as u32));
    }

    #[test]
    fn recusa_hash_errado() {
        let mut data = save_sintetico();
        data[0x30 + 20] ^= 0xff; // um byte dentro do primeiro bloco
        assert!(ler(&data, &[]).err().unwrap().contains("Hash"));
    }
}
