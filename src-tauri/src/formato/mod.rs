// Formato do save do Enshrouded: o contêiner SV2C (.ksd), o bloco player_character e o
// player_knowledge. Era JavaScript no WebView (shared/*.mjs); aqui o mesmo formato, byte a byte.

pub mod conhecimento;
pub mod personagem;
pub mod sv2c;

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Leitura little-endian que falha em vez de ler fora do buffer.
pub(crate) fn u16_em(p: &[u8], o: usize) -> Option<u16> {
    p.get(o..o.checked_add(2)?).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

pub(crate) fn u32_em(p: &[u8], o: usize) -> Option<u32> {
    p.get(o..o.checked_add(4)?).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub(crate) fn fora(o: usize) -> String {
    format!("leitura fora do bloco na posição {o}")
}

#[derive(Deserialize)]
struct ItemInfo {
    #[serde(default)]
    slot: String,
}

#[derive(Deserialize)]
struct Arvore {
    nodes: HashMap<String, serde_json::Value>,
}

/// O que o leitor precisa saber do jogo: quais ids são itens, quais são nós de habilidade e quais
/// itens se vestem. Sem isso a busca pelo equipamento aceitaria lixo como entrada.
pub struct Dados {
    itens: HashMap<u32, String>,
    habilidades: HashSet<u32>,
}

// O que pode estar vestido, pelo campo slot do item: separa a lista de equipamento da mochila,
// que também é densa e começa no índice 0. As armas ficam fora desta lista.
const ESPACOS: [&str; 10] = ["Armour_Head", "Armour_UpperBody", "Armour_Arms", "Armour_LowerBody", "Armour_Feet",
    "Armour_Ring", "Shield", "HookShot", "Glider", "Backpack"];

impl Dados {
    pub fn de_json(itens: &str, habilidades: &str) -> Result<Dados, String> {
        let itens: HashMap<String, ItemInfo> = serde_json::from_str(itens).map_err(|e| format!("items.json: {e}"))?;
        let arvore: Arvore = serde_json::from_str(habilidades).map_err(|e| format!("skills.json: {e}"))?;
        Ok(Dados {
            itens: itens.into_iter().filter_map(|(id, i)| Some((id.parse().ok()?, i.slot))).collect(),
            habilidades: arvore.nodes.keys().filter_map(|id| id.parse().ok()).collect(),
        })
    }

    /// Os dados embutidos no exe, os mesmos que a interface usa.
    pub fn do_jogo() -> &'static Dados {
        static D: OnceLock<Dados> = OnceLock::new();
        D.get_or_init(|| {
            Dados::de_json(include_str!("../../../src/data/items.json"), include_str!("../../../src/data/skills.json"))
                .expect("dados do jogo embutidos inválidos")
        })
    }

    pub fn eh_item(&self, id: u32) -> bool { self.itens.contains_key(&id) }
    pub fn eh_habilidade(&self, id: u32) -> bool { self.habilidades.contains(&id) }
    pub fn eh_equipavel(&self, id: u32) -> bool {
        self.itens.get(&id).is_some_and(|s| ESPACOS.contains(&s.as_str()))
    }

    /// Ids de itens em ordem crescente (os testes montam saves sintéticos com eles).
    #[cfg(test)]
    pub fn ids_de_itens(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.itens.keys().copied().collect();
        v.sort_unstable();
        v
    }

    #[cfg(test)]
    pub fn ids_de_habilidades(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.habilidades.iter().copied().collect();
        v.sort_unstable();
        v
    }
}
