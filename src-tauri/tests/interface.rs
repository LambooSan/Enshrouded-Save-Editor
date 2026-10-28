// Testes do código da interface (src/*.js), rodados no Edge sem janela: o mesmo Chromium do
// WebView2 em que o editor roda, então o que passa aqui é o que roda de verdade — sem Node, sem
// reescrever import nenhum.
//
// Cada teste é um módulo em tests/navegador/ que exporta `default async (conferir) => {...}`.
// Um servidor HTTP mínimo serve a raiz do repositório (módulos ES não carregam de file://),
// o Edge abre tests/navegador/executar.html?t=<nome> e o resultado volta no DOM impresso.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn tipo(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "css" => "text/css",
        _ => "application/octet-stream",
    }
}

/// Sobe o servidor numa porta livre e devolve o endereço.
fn servir() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let porta = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for c in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut c = c;
                let mut linha = String::new();
                if BufReader::new(&c).read_line(&mut linha).is_err() { return; }
                let alvo = linha.split_whitespace().nth(1).unwrap_or("/");
                let alvo = alvo.split('?').next().unwrap_or("").trim_start_matches('/');
                let p = raiz().join(alvo.replace("%20", " "));
                let (status, corpo) = match std::fs::read(&p) {
                    Ok(b) => ("200 OK", b),
                    Err(_) => ("404 Not Found", Vec::new()),
                };
                let _ = write!(c, "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", tipo(&p), corpo.len());
                let _ = c.write_all(&corpo);
            });
        }
    });
    format!("http://127.0.0.1:{porta}")
}

fn edge() -> Option<PathBuf> {
    let mut candidatos = vec![];
    if let Ok(p) = std::env::var("EDGE") { candidatos.push(PathBuf::from(p)); }
    for base in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(b) = std::env::var(base) { candidatos.push(Path::new(&b).join("Microsoft/Edge/Application/msedge.exe")); }
    }
    candidatos.into_iter().find(|p| p.is_file())
}

fn desescapar(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

fn rodar(nome: &str) {
    let Some(edge) = edge() else {
        eprintln!("Edge não encontrado (defina EDGE): teste {nome} ignorado");
        return;
    };
    let perfil = std::env::temp_dir().join(format!("enshrouded-teste-{nome}-{}", std::process::id()));
    let url = format!("{}/tests/navegador/executar.html?t={nome}", servir());
    let saida = Command::new(edge)
        .args(["--headless=new", "--disable-gpu", "--no-first-run", "--virtual-time-budget=60000", "--dump-dom"])
        .arg(format!("--user-data-dir={}", perfil.display()))
        .arg(&url)
        .output()
        .expect("não consegui abrir o Edge");
    let _ = std::fs::remove_dir_all(&perfil);
    let dom = String::from_utf8_lossy(&saida.stdout);
    let Some(ini) = dom.find("<pre id=\"resultado\">") else {
        panic!("{nome}: o teste não terminou (sem resultado no DOM)\n{}", &dom[..dom.len().min(2000)]);
    };
    let corpo = &dom[ini + "<pre id=\"resultado\">".len()..];
    let corpo = desescapar(&corpo[..corpo.find("</pre>").unwrap_or(corpo.len())]);
    println!("{corpo}");
    let fim = corpo.lines().last().unwrap_or("");
    let ruins: usize = fim.strip_prefix("FIM ").and_then(|r| r.split(' ').nth(1)).and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{nome}: resultado sem a linha FIM"));
    let erros: Vec<&str> = corpo.lines().filter(|l| l.starts_with("ERRO")).collect();
    assert_eq!(ruins, 0, "{nome}: {ruins} conferências falharam:\n{}", erros.join("\n"));
}

#[test]
fn nomes_por_idioma() { rodar("nomes-por-idioma") }

#[test]
fn itens_novos() { rodar("itens-novos") }

#[test]
fn soquete() { rodar("soquete") }

#[test]
fn estatisticas() { rodar("estatisticas") }

#[test]
fn todas_as_armas() { rodar("todas-as-armas") }
