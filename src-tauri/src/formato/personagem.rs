// Bloco 'player_character' do save do personagem.
//
// 0x00..0x3a  aparência e escalas (preservado como está)
// 0x3a u32 nível, 0x3e u32 XP no nível atual, 0x42 u32 número de nós de habilidade
// nós: (u32 id, u8 nível) * n
// trecho opaco (buffs de comida ativos etc.) até o equipamento
// u32 máscara dos espaços de equipamento ocupados (4 bytes antes da lista)
// equipamento: lista densa, uma entrada por bit aceso da máscara, em ordem crescente;
//   o campo slot da entrada é o índice nessa lista (0, 1, 2...), não o espaço do boneco
// u16 espaços de equipamento livres (logo depois da lista)
// trecho opaco que termina em [05][qtd principal][04][qtd extra]
// entradas do inventário principal, depois as do contêiner extra, depois o resto (opaco)
//
// Medido em três saves do mesmo personagem (16, 15 e 14 peças): tirar a peça de índice k apaga
// o k-ésimo bit aceso da máscara e soma 1 aos espaços livres. A máscara tem 2 bits acesos a mais
// que a lista (as armas, guardadas em outra parte do bloco), e livres + peças deu 27 nos três.
//
// Entrada: u8 slot, u32 itemId, u16 quantidade, u8 flags e então, na ordem dos bits:
//   0x01 u8 nível | 0x02 u32 durabilidade, u32 durabilidade máx | 0x04 u32 líquido, u32 líquido máx
//   0x08 4 bytes (aparência/tingimento, preservado) | 0x10 u8 melhorias máx, u8 melhorias aplicadas
//   0x20 5 bytes (engaste, preservado)

use super::{fora, u16_em, u32_em, Dados};
use serde::{Deserialize, Serialize};

const FLAGS_CONHECIDAS: u8 = 0x01 | 0x02 | 0x04 | 0x08 | 0x10 | 0x20;
pub const EQUIP_MAX: u8 = 24;  // teto da lista densa (o personagem medido tem 16)
const MIN_EQUIP: usize = 4;    // menos que isso é coincidência, não a lista de equipamento
const EQUIP_BITS_EXTRAS: u32 = 4; // bits acesos fora da lista (as armas); medido = 2

/// Uma pilha de itens, no formato que a interface lê e devolve. Os campos opcionais existem
/// quando o bit de flag correspondente está aceso; `flags` é recalculado na escrita.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Entrada {
    pub slot: u8,
    pub item_id: u32,
    pub count: u16,
    #[serde(default)]
    pub flags: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durability: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_durability: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_liquid: Option<u32>,
    /// 4 bytes em hexadecimal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_upgrades: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrades: Option<u8>,
    /// 5 bytes em hexadecimal: itemId da joia (u32) e o nível dela (u8)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Habilidade {
    pub id: u32,
    pub level: u8,
}

/// Trechos que o editor não interpreta e devolve como estavam.
#[derive(Clone, Debug, Default)]
pub struct Brutos {
    pub head: Vec<u8>,
    /// os 4 últimos bytes são a máscara
    pub mid: Vec<u8>,
    /// os 2 primeiros são os espaços livres
    pub between: Vec<u8>,
    pub tail: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct Personagem {
    pub level: u32,
    pub xp: u32,
    pub skills: Vec<Habilidade>,
    pub equipment: Vec<Entrada>,
    pub main: Vec<Entrada>,
    pub extra: Vec<Entrada>,
    pub equip_count: usize,
    pub equip_mask: u32,
    pub equip_free: u16,
    pub brutos: Brutos,
}

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

fn de_hex(s: &str, tam: usize, campo: &str) -> Result<Vec<u8>, String> {
    let ruim = || format!("{campo} inválido: {s}");
    if s.len() != tam * 2 || !s.is_ascii() { return Err(ruim()); }
    (0..tam).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| ruim())).collect()
}

/// Lê uma entrada em `o`; devolve a entrada e onde ela termina, ou None se ali não há uma.
fn ler_entrada(p: &[u8], o: usize, d: &Dados) -> Option<(Entrada, usize)> {
    if o + 8 > p.len() { return None; }
    let item_id = u32_em(p, o + 1)?;
    if !d.eh_item(item_id) { return None; }
    let flags = p[o + 7];
    if flags & !FLAGS_CONHECIDAS != 0 { return None; }
    let mut e = Entrada { slot: p[o], item_id, count: u16_em(p, o + 5)?, flags, ..Default::default() };
    let tem = |b: u8, n: usize| if flags & b != 0 { n } else { 0 };
    let precisa = tem(0x01, 1) + tem(0x02, 8) + tem(0x04, 8) + tem(0x08, 4) + tem(0x10, 2) + tem(0x20, 5);
    let mut q = o + 8;
    if q + precisa > p.len() { return None; }
    if flags & 0x01 != 0 { e.level = Some(p[q]); q += 1; }
    if flags & 0x02 != 0 { e.durability = u32_em(p, q); e.max_durability = u32_em(p, q + 4); q += 8; }
    if flags & 0x04 != 0 { e.liquid = u32_em(p, q); e.max_liquid = u32_em(p, q + 4); q += 8; }
    if flags & 0x08 != 0 { e.appearance = Some(hex(&p[q..q + 4])); q += 4; }
    if flags & 0x10 != 0 { e.max_upgrades = Some(p[q]); e.upgrades = Some(p[q + 1]); q += 2; }
    if flags & 0x20 != 0 { e.socket = Some(hex(&p[q..q + 5])); q += 5; }
    Some((e, q))
}

fn escrever_entrada(out: &mut Vec<u8>, e: &Entrada) -> Result<(), String> {
    let mut flags = 0u8;
    if e.level.is_some() { flags |= 0x01; }
    if e.durability.is_some() { flags |= 0x02; }
    if e.liquid.is_some() { flags |= 0x04; }
    if e.appearance.is_some() { flags |= 0x08; }
    if e.max_upgrades.is_some() { flags |= 0x10; }
    if e.socket.is_some() { flags |= 0x20; }
    out.push(e.slot);
    out.extend_from_slice(&e.item_id.to_le_bytes());
    out.extend_from_slice(&e.count.to_le_bytes());
    out.push(flags);
    if let Some(v) = e.level { out.push(v); }
    if let Some(v) = e.durability {
        out.extend_from_slice(&v.to_le_bytes());
        out.extend_from_slice(&e.max_durability.unwrap_or(0).to_le_bytes());
    }
    if let Some(v) = e.liquid {
        out.extend_from_slice(&v.to_le_bytes());
        out.extend_from_slice(&e.max_liquid.unwrap_or(0).to_le_bytes());
    }
    if let Some(s) = &e.appearance { out.extend(de_hex(s, 4, "appearance")?); }
    if let Some(v) = e.max_upgrades { out.extend_from_slice(&[v, e.upgrades.unwrap_or(0)]); }
    if let Some(s) = &e.socket { out.extend(de_hex(s, 5, "socket")?); }
    Ok(())
}

struct Inventario { inicio: usize, qtd_main: usize, qtd_extra: usize }

// Trecho entre equipamento e inventário: termina em 05 <qtd> 04 <qtd extra>. O tamanho desse
// trecho varia muito de save para save (medido: de 4 a mais de 64 bytes), então procuramos até
// o fim do bloco e confiamos na conferência de que todas as pilhas depois do cabeçalho são lidas.
fn achar_inventario(p: &[u8], de: usize, d: &Dados) -> Option<Inventario> {
    for s in de + 4..p.len() {
        // 05 <qtd principal> <grupos do contêiner extra> <qtd extra>; sem mochila vestida os grupos são 0
        if p[s - 4] != 0x05 || p[s - 2] > 8 { continue; }
        let (mc, ec) = (p[s - 3] as usize, p[s - 1] as usize);
        let mut q = s;
        let ok = (0..mc + ec).all(|_| match ler_entrada(p, q, d) {
            Some((_, fim)) => { q = fim; true }
            None => false,
        });
        if ok { return Some(Inventario { inicio: s, qtd_main: mc, qtd_extra: ec }); }
    }
    None
}

pub fn ler(p: &[u8], d: &Dados) -> Result<Personagem, String> {
    let ler32 = |o| u32_em(p, o).ok_or_else(|| fora(o));
    let (level, xp, n) = (ler32(0x3a)?, ler32(0x3e)?, ler32(0x42)? as usize);
    let mut skills = Vec::new();
    let mut o = 0x46;
    for _ in 0..n {
        skills.push(Habilidade { id: ler32(o)?, level: *p.get(o + 4).ok_or_else(|| fora(o + 4))? });
        o += 5;
    }
    if skills.iter().any(|s| !d.eh_habilidade(s.id)) { return Err("Lista de habilidades não reconhecida".into()); }
    let fim_skills = o;

    // Equipamento: lista densa com os índices em ordem crescente (0, 1, 2…), precedida pela máscara
    // e seguida pelos espaços livres. A busca é por tentativa, então só aceitamos um candidato que
    // feche TODOS os confereres abaixo: adivinhar aqui grava um save corrompido. O equipamento vem
    // antes do inventário no bloco, então o primeiro candidato é o certo.
    let mut achado = None;
    for s in fim_skills + 4..p.len().saturating_sub(8) {
        let mut lista: Vec<Entrada> = Vec::new();
        let mut q = s;
        while let Some((e, fim)) = ler_entrada(p, q, d) {
            if e.slot >= EQUIP_MAX || e.slot as usize != lista.len() { break; }
            lista.push(e);
            q = fim;
        }
        if lista.len() < MIN_EQUIP { continue; }
        // só peças vestíveis: a mochila também é uma lista densa começando no 0, e sem isso o
        // editor pode confundir as duas e gravar um save corrompido
        if !lista.iter().all(|e| d.eh_equipavel(e.item_id)) { continue; }
        let Some(inv) = achar_inventario(p, q, d) else { continue };
        let mask = ler32(s - 4)?;
        let free = u16_em(p, q).ok_or_else(|| fora(q))?;
        // a máscara precisa ter um bit aceso para cada peça (mais os das armas, fora desta lista)
        let bits = mask.count_ones();
        if bits < lista.len() as u32 || bits > lista.len() as u32 + EQUIP_BITS_EXTRAS { continue; }
        achado = Some((s, lista, inv, mask, free, q));
        break;
    }
    let Some((ini_equip, equipment, inv, mask, free, fim_equip)) = achado else {
        return Err("Equipamento/inventário não encontrado no save".into());
    };

    let (mut main, mut extra) = (Vec::new(), Vec::new());
    let mut q = inv.inicio;
    for k in 0..inv.qtd_main + inv.qtd_extra {
        let (e, fim) = ler_entrada(p, q, d).ok_or_else(|| fora(q))?;
        if k < inv.qtd_main { main.push(e) } else { extra.push(e) }
        q = fim;
    }
    Ok(Personagem {
        level, xp, skills,
        equip_count: equipment.len(),
        equipment, main, extra,
        equip_mask: mask,
        equip_free: free,
        brutos: Brutos {
            head: p[..0x3a].to_vec(),
            mid: p[fim_skills..ini_equip].to_vec(),
            between: p[fim_equip..inv.inicio].to_vec(),
            tail: p[q..].to_vec(),
        },
    })
}

/// Máscara e espaços livres depois de tirar as peças cujos índices estão em `tirados`.
fn ajustar_equipamento(mask: u32, free: u16, tirados: &[usize]) -> Result<(u32, u16), String> {
    let bits: Vec<u32> = (0..32).filter(|i| mask & (1 << i) != 0).collect();
    let mut nova = mask;
    for &k in tirados {
        let b = bits.get(k).ok_or_else(|| format!("Não sei qual espaço de equipamento corresponde à peça {k}"))?;
        nova &= !(1 << b);
    }
    let free = u16::try_from(free as usize + tirados.len()).map_err(|_| "Espaços de equipamento livres demais".to_string())?;
    Ok((nova, free))
}

pub fn montar(c: &Personagem) -> Result<Vec<u8>, String> {
    let r = &c.brutos;
    if c.equipment.len() > c.equip_count {
        return Err("Não é possível adicionar peças de equipamento: não sei em que espaço do boneco elas entram".into());
    }
    if c.main.len() > 255 || c.extra.len() > 255 { return Err("Itens demais em um contêiner".into()); }

    // peças tiradas: os índices da lista original que não vieram de volta. A lista é densa,
    // então as que ficam são renumeradas de 0 em diante, e a máscara e os espaços livres acompanham.
    let mut mid = r.mid.clone();
    let mut between = r.between.clone();
    if between.len() < 3 { return Err("Não achei o cabeçalho do inventário neste save".into()); }
    let mut ficam = c.equipment.clone();
    ficam.sort_by_key(|e| e.slot);
    let tirados: Vec<usize> = (0..c.equip_count).filter(|&k| !ficam.iter().any(|e| e.slot as usize == k)).collect();
    if !tirados.is_empty() {
        if mid.len() < 4 { return Err("Não achei a máscara de equipamento neste save".into()); }
        let (mask, free) = ajustar_equipamento(c.equip_mask, c.equip_free, &tirados)?;
        let m = mid.len() - 4;
        mid[m..].copy_from_slice(&mask.to_le_bytes());
        between[..2].copy_from_slice(&free.to_le_bytes());
    }
    let n = between.len();
    between[n - 3] = c.main.len() as u8;
    between[n - 1] = c.extra.len() as u8;

    let mut out = r.head.clone();
    for v in [c.level, c.xp, c.skills.len() as u32] { out.extend_from_slice(&v.to_le_bytes()); }
    for s in &c.skills {
        out.extend_from_slice(&s.id.to_le_bytes());
        out.push(s.level);
    }
    out.extend_from_slice(&mid);
    for (i, e) in ficam.iter().enumerate() {
        escrever_entrada(&mut out, &Entrada { slot: i as u8, ..e.clone() })?;
    }
    out.extend_from_slice(&between);
    let mut main = c.main.clone();
    main.sort_by_key(|e| e.slot);
    for e in main.iter().chain(&c.extra) { escrever_entrada(&mut out, e)?; }
    out.extend_from_slice(&r.tail);
    Ok(out)
}

#[cfg(test)]
mod testes {
    // Interpreta um bloco de personagem e o remonta, exigindo que o resultado seja byte a byte
    // igual ao original. É o que garante que editar um campo não reescreve nada por acidente.
    //
    // O caso sintético monta um bloco com todas as combinações de campos extras que uma entrada
    // pode ter. Confere o leitor contra o escritor, que é onde quase todo erro aparece; não prova o
    // formato do jogo. Para isso, ENSHROUDED_SAVES aponta pastas com cópias A.bin do guardião ou
    // saves .ksd do jogo, e o bloco de personagem de cada um tem de remontar idêntico.
    use super::*;
    use crate::formato::sv2c::{self, CAB_BLOCO};
    use std::path::{Path, PathBuf};

    fn variacao(i: usize) -> Entrada {
        let s = |v: &str| Some(v.to_string());
        match i % 6 {
            0 => Entrada::default(),                                                       // flags 0x00
            1 => Entrada { level: Some(1), ..Default::default() },                         // 0x01
            2 => Entrada { level: Some(40), durability: Some(1200), max_durability: Some(1600), ..Default::default() }, // 0x03
            3 => Entrada { level: Some(55), appearance: s("deadbeef"), max_upgrades: Some(5), upgrades: Some(5), ..Default::default() }, // 0x19
            4 => Entrada { liquid: Some(3), max_liquid: Some(10), ..Default::default() },   // 0x04
            _ => Entrada {                                                                  // 0x3b
                level: Some(90), durability: Some(10), max_durability: Some(1600), appearance: s("01020304"),
                max_upgrades: Some(4), upgrades: Some(2), socket: s("c2b4bcda69"), ..Default::default()
            },
        }
    }

    fn com_flags(e: &Entrada) -> Entrada {
        let mut out = Vec::new();
        escrever_entrada(&mut out, e).unwrap();
        Entrada { flags: out[7], ..e.clone() }
    }

    fn sintetico(d: &Dados) -> Personagem {
        let equipaveis: Vec<u32> = d.ids_de_itens().into_iter().filter(|&id| d.eh_equipavel(id)).take(6).collect();
        let quaisquer: Vec<u32> = d.ids_de_itens().into_iter().take(9).collect();
        let nos: Vec<u32> = d.ids_de_habilidades().into_iter().take(3).collect();
        assert!(equipaveis.len() == 6 && quaisquer.len() == 9 && nos.len() == 3, "dados pequenos demais para montar o caso");
        let entrada = |slot: usize, item_id: u32, count: u16, v: usize| com_flags(&Entrada { slot: slot as u8, item_id, count, ..variacao(v) });
        let equipment: Vec<Entrada> = equipaveis.iter().enumerate().map(|(i, &id)| entrada(i, id, 1, i)).collect();
        let main: Vec<Entrada> = quaisquer[..5].iter().enumerate().map(|(i, &id)| entrada(i, id, 1 + i as u16 * 7, i + 2)).collect();
        let extra: Vec<Entrada> = quaisquer[5..8].iter().enumerate().map(|(i, &id)| entrada(i, id, 99, i + 4)).collect();
        // máscara com 2 bits acesos a mais que a lista (as armas ficam fora dela), como nos saves medidos
        let mask = 0xffu32;
        let free = 1u16; // os bytes 01 00 não podem ser confundidos com o 05 do inventário
        Personagem {
            level: 35, xp: 123_456,
            skills: nos.iter().enumerate().map(|(i, &id)| Habilidade { id, level: i as u8 + 1 }).collect(),
            equip_count: equipment.len(),
            equipment, extra,
            brutos: Brutos {
                head: vec![0xa7; 0x3a],
                mid: mask.to_le_bytes().to_vec(),
                // [livres u16][05][qtd principal][grupos][qtd extra]
                between: vec![free as u8, (free >> 8) as u8, 0x05, main.len() as u8, 0x04, 3],
                tail: vec![0x5c; 16],
            },
            main,
            equip_mask: mask,
            equip_free: free,
        }
    }

    #[test]
    fn sintetico_remonta_identico() {
        let d = Dados::do_jogo();
        let dentro = sintetico(d);
        let original = montar(&dentro).unwrap();
        let lido = ler(&original, d).expect("o leitor recusou o bloco que o escritor gerou");
        assert_eq!(montar(&lido).unwrap(), original);
        assert_eq!((lido.level, lido.xp), (dentro.level, dentro.xp));
        assert_eq!(lido.skills, dentro.skills);
        assert_eq!((lido.equip_mask, lido.equip_free), (dentro.equip_mask, dentro.equip_free));
        assert_eq!(lido.equipment, dentro.equipment);
        assert_eq!(lido.main, dentro.main);
        assert_eq!(lido.extra, dentro.extra);
    }

    #[test]
    fn tirar_uma_peca_acerta_mascara_e_livres() {
        let d = Dados::do_jogo();
        let lido = ler(&montar(&sintetico(d)).unwrap(), d).unwrap();
        let mut menos = lido.clone();
        menos.equipment.pop();
        let relido = ler(&montar(&menos).unwrap(), d).unwrap();
        assert_eq!(relido.equipment.len(), lido.equipment.len() - 1);
        assert_eq!(relido.equip_free, lido.equip_free + 1);
        assert_eq!(relido.equip_mask, lido.equip_mask & !(1 << 5));
    }

    #[test]
    fn nao_adiciona_peca() {
        let d = Dados::do_jogo();
        let mut c = ler(&montar(&sintetico(d)).unwrap(), d).unwrap();
        c.equipment.push(c.equipment[0].clone());
        assert!(montar(&c).is_err());
    }

    #[test]
    fn hex_invalido_e_erro() {
        let mut out = Vec::new();
        assert!(escrever_entrada(&mut out, &Entrada { socket: Some("zz".into()), ..Default::default() }).is_err());
    }

    fn achar(alvo: &Path, prof: u32, out: &mut Vec<PathBuf>) {
        if alvo.is_file() { out.push(alvo.into()); return; }
        if prof > 5 { return; }
        for e in std::fs::read_dir(alvo).into_iter().flatten().flatten() {
            let p = e.path();
            let nome = e.file_name().to_string_lossy().to_ascii_lowercase();
            if p.is_dir() { achar(&p, prof + 1, out) } else if nome == "a.bin" || (nome.starts_with("character") && nome.ends_with(".ksd")) { out.push(p) }
        }
    }

    #[test]
    fn saves_reais_remontam_identicos() {
        let Ok(pastas) = std::env::var("ENSHROUDED_SAVES") else {
            eprintln!("ENSHROUDED_SAVES não definido: só o caso sintético rodou");
            return;
        };
        let d = Dados::do_jogo();
        let mut arquivos = Vec::new();
        for p in std::env::split_paths(&pastas) { achar(&p, 0, &mut arquivos); }
        let mut ruins = Vec::new();
        for f in &arquivos {
            let a = std::fs::read(f).unwrap();
            let bloco = if a.starts_with(b"SV2C") {
                match sv2c::ler(&a, &["player_character"]) {
                    Ok(s) => s.bloco("player_character").and_then(|b| b.payload.clone()).unwrap_or_default(),
                    Err(e) => { ruins.push(format!("{}: {e}", f.display())); continue; }
                }
            } else if a.len() >= CAB_BLOCO && &a[..4] == b"S2BH" {
                a[CAB_BLOCO..].to_vec()
            } else {
                continue;
            };
            match ler(&bloco, d).and_then(|c| montar(&c)) {
                Ok(b) if b == bloco => {}
                Ok(b) => { ruins.push(format!("{}: remontou com {} bytes, original tem {}", f.display(), b.len(), bloco.len())); continue; }
                Err(e) => { ruins.push(format!("{}: {e}", f.display())); continue; }
            }
            // o caminho de gravar, sem gravar: bloco recompactado, .ksd remontado, relido e conferido
            if a.starts_with(b"SV2C") {
                let mut s = sv2c::ler(&a, &["player_character"]).unwrap();
                let b = s.blocos.iter_mut().find(|b| b.nome == "player_character").unwrap();
                b.mudou = true;
                let novo = sv2c::montar(&s).unwrap();
                let relido = sv2c::ler(&novo, &["player_character"]).map(|r| r.bloco("player_character").and_then(|b| b.payload.clone()));
                if relido != Ok(Some(bloco)) { ruins.push(format!("{}: o .ksd remontado não devolve o mesmo personagem", f.display())); }
            }
        }
        assert!(ruins.is_empty(), "{}", ruins.join("\n"));
        eprintln!("saves reais: {} idênticos", arquivos.len());
    }
}
