// Os arquivos de `src/data/` são gerados fora deste repositório por várias ferramentas em
// sequência, e cada uma acrescenta campos aos arquivos da anterior. Rodar só uma delas reescreve o
// arquivo e apaga o que as outras tinham posto.
//
// Foi o que aconteceu com os ícones da árvore de habilidades na 1.0.1: regerar o skills.json para
// acrescentar a descrição em inglês apagou `icon` e `iconOff` dos 43 nós que os têm, e a árvore
// ficou sem ícone nenhum para quem baixou. Nada falhou — o campo só sumiu.
//
// Estes testes exigem que cada arquivo tenha todos os campos que a interface usa, e que toda
// imagem citada exista mesmo em build-assets.

use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn ler(nome: &str) -> Map<String, Value> {
    let txt = std::fs::read_to_string(raiz().join("src/data").join(nome)).unwrap();
    serde_json::from_str::<Value>(&txt).unwrap().as_object().unwrap().clone()
}

fn texto<'a>(v: &'a Value, campo: &str) -> &'a str { v.get(campo).and_then(Value::as_str).unwrap_or("") }

#[test]
fn arvore_de_habilidades() {
    let skills = ler("skills.json");
    let nos: Vec<&Value> = skills["nodes"].as_object().unwrap().values().collect();
    assert_eq!(nos.len(), 243, "nós na árvore");
    for campo in ["pt", "en", "desc", "descEn", "type", "class", "cost", "maxLevel", "x", "y"] {
        let faltam = nos.iter().filter(|n| n.get(campo).is_none()).count();
        assert_eq!(faltam, 0, "todo nó tem \"{campo}\"");
    }

    // os ícones vêm de build_ui.py, que roda DEPOIS do build_data.py sobre o mesmo arquivo
    let com_icone: Vec<&&Value> = nos.iter().filter(|n| !texto(n, "icon").is_empty()).collect();
    assert_eq!(com_icone.len(), 43, "nós com ícone (build_ui.py já rodou)");
    assert!(com_icone.iter().all(|n| !texto(n, "iconOff").is_empty()), "todo nó com ícone tem também o apagado");

    // e o nome do ícone tem de existir como arquivo, senão a árvore mostra imagem quebrada
    let falta: Vec<String> = com_icone.iter()
        .flat_map(|n| [texto(n, "icon"), texto(n, "iconOff")].map(|nome| (texto(n, "pt"), nome)))
        .filter(|(_, nome)| !raiz().join("build-assets/game/ui").join(format!("{nome}.webp")).is_file())
        .map(|(pt, nome)| format!("{pt}: {nome}.webp"))
        .collect();
    assert!(falta.is_empty(), "imagens de ícone faltando:\n{}", falta.join("\n"));

    assert!(!skills["links"].as_array().unwrap().is_empty(), "ligações entre nós");
}

#[test]
fn itens() {
    let items = ler("items.json");
    assert_eq!(items.len(), 4262, "itens");
    for campo in ["debug", "pt", "en", "category", "rarity", "maxStack", "minLevel", "maxLevel"] {
        let faltam = items.values().filter(|i| i.get(campo).is_none()).count();
        assert_eq!(faltam, 0, "todo item tem \"{campo}\"");
    }
    // o ícone do item é montado a partir do nome de depuração
    let icones = raiz().join("build-assets/game/icons");
    let sem = items.values().filter(|i| !icones.join(format!("{}_icon.webp", texto(i, "debug"))).is_file()).count();
    assert!(sem * 10 < items.len(), "{} de {} itens têm ícone em build-assets", items.len() - sem, items.len());
}

#[test]
fn conhecimento() {
    let items = ler("items.json");
    let k = ler("conhecimento.json");
    assert_eq!(k.len(), 8335, "entradas de conhecimento");
    let sem_nada = k.values().filter(|c| c.get("i").is_none() && c.get("n").is_none() && texto(c, "t") != "i").count();
    assert_eq!(sem_nada, 0, "entradas sem item nem nome");
    let fora = k.values().filter_map(|c| c.get("i")).filter(|i| {
        let id = i.as_u64().map(|n| n.to_string()).or_else(|| i.as_str().map(String::from)).unwrap_or_default();
        !items.contains_key(&id)
    }).count();
    assert_eq!(fora, 0, "conhecimentos apontam para item inexistente");
}

// Guarda contra a reincidência (vinha de tests/nomes-por-idioma.mjs; o navegador não lista pastas).
//
// Este bug já voltou duas vezes, sempre num arquivo novo: alguém escreve `item.pt` para pôr um nome
// na tela e aquele ponto fica em português para sempre, com a interface em inglês.
//
// A regra: uma linha que lê `.pt` sem citar `.en` na mesma linha está escolhendo um idioma, e isso
// é errado — o nome tem de sair de `itemName`, `skillName` ou `nomeDe`. Linhas que citam os dois
// estão tratando ambos (busca, reserva, o nome no outro idioma) e passam.
//
// Exceção: `attrs[...]` usa o nome em português como CHAVE da tabela de atributos, não como texto;
// trocar ali quebraria a contagem. A exibição desses rótulos passa por t().
#[test]
fn nenhum_ponto_escolhe_o_portugues_direto() {
    fn cita(l: &str, idioma: &str) -> bool {
        let campo = format!(".{idioma}");
        l.match_indices(&campo).any(|(i, _)| {
            !l[i + campo.len()..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_')
        }) || l.contains(&format!("['{idioma}']")) || l.contains(&format!("[\"{idioma}\"]"))
    }
    fn arquivos(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                if e.file_name() != "vendor" && e.file_name() != "data" { arquivos(&p, out); }
            } else if p.extension().is_some_and(|x| x == "js") && e.file_name() != "i18n.js" {
                out.push(p);
            }
        }
    }
    let mut fontes = vec![];
    arquivos(&raiz().join("src"), &mut fontes);
    let mut suspeitas = vec![];
    for arq in &fontes {
        for (i, linha) in std::fs::read_to_string(arq).unwrap().lines().enumerate() {
            let t = linha.trim_start();
            if t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') || linha.replace(char::is_whitespace, "").contains("attrs[") { continue; }
            if cita(linha, "pt") && !cita(linha, "en") {
                suspeitas.push(format!("{}:{}  {}", arq.strip_prefix(raiz()).unwrap().display(), i + 1, linha.trim()));
            }
        }
    }
    assert!(suspeitas.is_empty(), "pontos lendo .pt direto (use itemName/skillName/nomeDe):\n{}", suspeitas.join("\n"));
}
