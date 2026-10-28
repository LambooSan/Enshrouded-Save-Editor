// Conhecimento do personagem: pares (knowledgeId -> estado), onde o estado é um conjunto de bits.
//
// Duas formas do mesmo conteúdo:
// - bloco 'player_knowledge' do save (.ksd): u32 estado do contêiner, u32 n, n ids u32, n valores u32
// - arquivo K.bin das cópias do guardião: só n ids u32 seguidos de n valores u32 (o n vem do tamanho)

use super::u32_em;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entrada {
    pub id: u32,
    pub value: u32,
}

pub struct Conhecimento {
    pub estado: u32,
    pub entradas: Vec<Entrada>,
}

const CAB: usize = 8; // u32 estado + u32 n

fn pares(p: &[u8], off: usize, n: usize) -> Vec<Entrada> {
    (0..n).map(|i| Entrada {
        id: u32_em(p, off + i * 4).unwrap_or(0),
        value: u32_em(p, off + n * 4 + i * 4).unwrap_or(0),
    }).collect()
}

fn escrever(out: &mut Vec<u8>, entradas: &[Entrada]) {
    for e in entradas { out.extend_from_slice(&e.id.to_le_bytes()); }
    for e in entradas { out.extend_from_slice(&e.value.to_le_bytes()); }
}

/// Lê o bloco player_knowledge de um .ksd.
pub fn ler_bloco(p: &[u8]) -> Result<Conhecimento, String> {
    let (Some(estado), Some(n)) = (u32_em(p, 0), u32_em(p, 4)) else { return Err("bloco player_knowledge truncado".into()) };
    let n = n as usize;
    if p.len() as u64 != CAB as u64 + 8 * n as u64 { return Err("bloco player_knowledge com tamanho inesperado".into()); }
    Ok(Conhecimento { estado, entradas: pares(p, CAB, n) })
}

/// Monta o bloco player_knowledge de volta.
pub fn montar_bloco(c: &Conhecimento) -> Vec<u8> {
    let mut out = Vec::with_capacity(CAB + 8 * c.entradas.len());
    out.extend_from_slice(&c.estado.to_le_bytes());
    out.extend_from_slice(&(c.entradas.len() as u32).to_le_bytes());
    escrever(&mut out, &c.entradas);
    out
}

/// Lê o K.bin das cópias do guardião.
pub fn ler_cru(p: &[u8]) -> Result<Conhecimento, String> {
    if p.len() % 8 != 0 { return Err("K.bin com tamanho inesperado".into()); }
    Ok(Conhecimento { estado: 1, entradas: pares(p, 0, p.len() / 8) })
}

/// Monta o K.bin de volta.
pub fn montar_cru(entradas: &[Entrada]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 * entradas.len());
    escrever(&mut out, entradas);
    out
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ida_e_volta() {
        let c = Conhecimento { estado: 3, entradas: vec![Entrada { id: 10, value: 1 }, Entrada { id: 0xdead_beef, value: 7 }] };
        let b = montar_bloco(&c);
        let l = ler_bloco(&b).unwrap();
        assert_eq!((l.estado, &l.entradas), (3, &c.entradas));
        assert_eq!(ler_cru(&montar_cru(&c.entradas)).unwrap().entradas, c.entradas);
        assert_eq!(&b[8..], &montar_cru(&c.entradas)[..]);
        assert!(ler_bloco(&b[..b.len() - 1]).is_err());
        assert!(ler_cru(&[0; 7]).is_err());
    }
}
