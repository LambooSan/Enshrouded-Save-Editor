// Testes de ponta a ponta contra o servidor dedicado de verdade, usando a configuração salva do
// editor (APPDATA\enshrouded-save-editor\servidor.json).
//
// Eles existem porque o transporte trocou de FTPS (curl) para SFTP: o que quebrava antes era ler
// vários arquivos em sequência, e isso só aparece conversando com o servidor real. Sem
// configuração salva, cada teste passa sem fazer nada, para a compilação não depender da rede.
//
// Nada de dado real é alterado: o único teste de escrita cria um arquivo temporário com nome
// próprio, confere e apaga. A.bin, K.bin e meta.json nunca são escritos aqui.

use crate::servidor::{self, Config};
use crate::sftp;

fn config() -> Option<Config> {
    let c = servidor::carregar();
    if c.host.is_empty() || c.usuario.is_empty() || c.senha.is_empty() {
        eprintln!("sem configuração de servidor salva: teste ignorado");
        return None;
    }
    Some(c)
}

/// Primeiro steamid com cópia guardada.
async fn algum_jogador(c: &Config) -> String {
    let nomes = sftp::listar(c, &format!("/{}", c.pasta)).await.expect("listar a pasta do guardião");
    nomes
        .into_iter()
        .find(|n| n.chars().all(|ch| ch.is_ascii_digit()) && !n.is_empty())
        .expect("nenhuma pasta de jogador no servidor")
}

#[tokio::test]
async fn lista_a_pasta_do_guardiao() {
    let Some(c) = config() else { return };
    let nomes = sftp::listar(&c, &format!("/{}", c.pasta)).await.expect("listar");
    assert!(nomes.iter().any(|n| n == "registro.jsonl"), "faltou registro.jsonl em {nomes:?}");
    assert!(
        nomes.iter().any(|n| n.chars().all(|ch| ch.is_ascii_digit())),
        "nenhuma pasta por steamid em {nomes:?}"
    );
}

#[tokio::test]
async fn le_a_copia_de_um_jogador() {
    let Some(c) = config() else { return };
    let sid = algum_jogador(&c).await;
    let base = format!("/{}/{}", c.pasta, sid);

    let meta: serde_json::Value = serde_json::from_slice(
        &sftp::baixar(&c, &format!("{base}/meta.json")).await.expect("baixar meta.json"),
    )
    .expect("meta.json não é JSON");
    assert_eq!(meta["steamid"].as_str(), Some(sid.as_str()), "o meta.json é de outro jogador");

    let a = sftp::baixar(&c, &format!("{base}/A.bin")).await.expect("baixar A.bin");
    assert_eq!(&a[..4], b"S2BH", "A.bin não começa com o cabeçalho de bloco");
    assert_eq!(a.len() as u64, meta["tamA"].as_u64().unwrap(), "tamanho de A.bin diferente do meta");

    let k = sftp::baixar(&c, &format!("{base}/K.bin")).await.expect("baixar K.bin");
    assert_eq!(k.len() % 8, 0, "K.bin não é múltiplo de 8 (id + valor por entrada)");
    assert_eq!(k.len() as u64 / 8, meta["nK"].as_u64().unwrap(), "nK diferente do meta");
}

/// A regressão que motivou a troca: com FTPS isso falhava no meio com
/// "curl: (56) response reading failed", porque cada arquivo abria conexões novas.
#[tokio::test]
async fn muitas_leituras_seguidas_na_mesma_sessao() {
    let Some(c) = config() else { return };
    let sid = algum_jogador(&c).await;
    let base = format!("/{}/{}", c.pasta, sid);
    let mut total = 0;
    for i in 0..12 {
        let arquivo = ["meta.json", "A.bin", "K.bin"][i % 3];
        let b = sftp::baixar(&c, &format!("{base}/{arquivo}"))
            .await
            .unwrap_or_else(|e| panic!("leitura {i} ({arquivo}) falhou: {e}"));
        assert!(!b.is_empty(), "leitura {i} ({arquivo}) veio vazia");
        total += b.len();
    }
    assert!(total > 0);
}

#[tokio::test]
async fn grava_confere_e_apaga_um_arquivo_temporario() {
    let Some(c) = config() else { return };
    let sid = algum_jogador(&c).await;
    let caminho = format!("/{}/{}/teste-e2e-editor.tmp", c.pasta, sid);
    let conteudo: Vec<u8> = (0u8..=255).cycle().take(5000).collect();

    sftp::enviar(&c, &caminho, &conteudo).await.expect("enviar o arquivo de teste");
    let lido = sftp::baixar(&c, &caminho).await.expect("baixar o arquivo de teste");
    assert_eq!(lido, conteudo, "o que voltou do servidor não é o que foi enviado");

    sftp::apagar(&c, &caminho).await.expect("apagar o arquivo de teste");
    assert!(!sftp::existe(&c, &caminho).await.expect("conferir se saiu"), "o arquivo de teste ficou no servidor");
}

/// Uma impressão digital errada tem que derrubar a conexão, senão a chave fixada não serve de nada.
#[tokio::test]
async fn chave_do_servidor_errada_e_recusada() {
    let Some(mut c) = config() else { return };
    sftp::esquecer().await;
    c.hostkey = "SHA256:chaveQueNaoExisteAAAAAAAAAAAAAAAAAAAAAAAAAAA".into();
    let r = sftp::listar(&c, &format!("/{}", c.pasta)).await;
    sftp::esquecer().await;
    assert!(r.is_err(), "conectou mesmo com a chave do servidor errada");
}

#[tokio::test]
async fn comandos_do_editor_respondem() {
    let Some(_) = config() else { return };

    let jogadores = servidor::servidor_jogadores().await.expect("servidor_jogadores");
    assert!(!jogadores.is_empty(), "nenhum jogador listado");
    assert!(jogadores.iter().all(|j| j.meta.is_some()), "algum jogador veio sem meta.json");

    let modo = servidor::servidor_modo().await.expect("servidor_modo");
    assert!(modo == "aplicar" || modo == "observar", "modo inesperado: {modo}");

    let registro = servidor::servidor_registro().await.expect("servidor_registro");
    assert!(registro.contains("\"kind\""), "o registro não parece jsonl do guardião");

    let sid = jogadores[0].steamid.clone();
    let a = servidor::servidor_ler(sid.clone(), "A.bin".into()).await;
    assert!(a.is_ok(), "servidor_ler falhou para {sid}");
}

/// Os saves recusados têm que casar com a linha do registro: tamA + nK*8 = tamanho do arquivo.
/// É dessa conta que o editor tira onde o personagem termina e o conhecimento começa.
#[tokio::test]
async fn recusados_casam_com_o_registro() {
    let Some(c) = config() else { return };
    let registro = servidor::servidor_registro().await.expect("registro");
    let entradas: Vec<serde_json::Value> = registro
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|d| d["kind"] == "entrada" && d["tamA"].is_u64())
        .collect();

    let mut conferidos = 0;
    for j in servidor::servidor_jogadores().await.expect("jogadores") {
        let nomes = servidor::servidor_recusados(j.steamid.clone()).await.expect("listar recusados");
        // a referência é do lado do servidor, e tem o tamanho dele: confere no teste seguinte
        let nomes: Vec<String> = nomes.into_iter().filter(|n| !n.ends_with("_referencia.bin")).collect();
        for nome in nomes.iter().rev().take(2) {
            let carimbo: String = nome.chars().filter(|ch| ch.is_ascii_digit()).take(14).collect();
            let Some(reg) = entradas.iter().find(|d| {
                d["steamid"] == j.steamid.as_str()
                    && d["quando"].as_str().map(|q| q.chars().filter(|ch| ch.is_ascii_digit()).take(14).collect::<String>())
                        == Some(carimbo.clone())
            }) else {
                continue; // o registro pode ter sido rotacionado
            };
            // pelo comando, como o editor faz, e pelo transporte, para poder medir o tamanho
            assert!(
                servidor::servidor_recusado_ler(j.steamid.clone(), nome.clone()).await.is_ok(),
                "servidor_recusado_ler falhou em {}/{nome}",
                j.steamid
            );
            let esperado = reg["tamA"].as_u64().unwrap() + reg["nK"].as_u64().unwrap() * 8;
            let real = sftp::baixar(&c, &format!("/{}/{}/recusados/{}", c.pasta, j.steamid, nome))
                .await
                .expect("baixar o recusado")
                .len() as u64;
            assert_eq!(real, esperado, "{}/{nome}: tamanho não casa com o registro", j.steamid);
            assert!(real > 14, "arquivo recusado muito pequeno");
            conferidos += 1;
        }
    }
    eprintln!("{conferidos} save(s) recusado(s) conferido(s) contra o registro");
}

/// Quando o guardião grava a cópia de referência ao lado do recusado, ela tem que ter o tamanho
/// que o registro anotou do lado do servidor: é disso que sai a comparação exata.
/// Sem nenhum arquivo de referência (guardião antigo), o teste passa sem conferir nada.
#[tokio::test]
async fn referencias_casam_com_o_tamanho_guardado() {
    let Some(c) = config() else { return };
    let registro = servidor::servidor_registro().await.expect("registro");
    let entradas: Vec<serde_json::Value> = registro
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|d| d["kind"] == "entrada" && d["guardadoTamA"].is_u64())
        .collect();

    let mut conferidos = 0;
    for j in servidor::servidor_jogadores().await.expect("jogadores") {
        for nome in servidor::servidor_recusados(j.steamid.clone()).await.expect("recusados") {
            if !nome.ends_with("_referencia.bin") {
                continue;
            }
            let carimbo: String = nome.chars().filter(|ch| ch.is_ascii_digit()).take(14).collect();
            let Some(reg) = entradas.iter().find(|d| {
                d["steamid"] == j.steamid.as_str()
                    && d["quando"].as_str().map(|q| q.chars().filter(|ch| ch.is_ascii_digit()).take(14).collect::<String>())
                        == Some(carimbo.clone())
            }) else {
                continue;
            };
            let real = sftp::baixar(&c, &format!("/{}/{}/recusados/{}", c.pasta, j.steamid, nome))
                .await
                .expect("baixar a referência")
                .len() as u64;
            let esperado = reg["guardadoTamA"].as_u64().unwrap() + reg["guardadoNK"].as_u64().unwrap() * 8;
            assert_eq!(real, esperado, "{}/{nome}: a referência não tem o tamanho que o registro anotou", j.steamid);
            conferidos += 1;
        }
    }
    eprintln!("{conferidos} referência(s) conferida(s) (0 = guardião ainda sem essa gravação)");
}
