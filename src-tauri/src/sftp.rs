// Transporte SFTP para as cópias do guardião.
//
// Antes isso era FTPS pelo curl do Windows. O FTP abre uma conexão de controle e outra de dados
// por arquivo, cada uma com handshake TLS, e o servidor de jogo corta conexões simultâneas: dava
// "curl: (56) response reading failed" ao ler vários arquivos seguidos. O SFTP faz um login só e
// multiplexa tudo em canais dentro da mesma sessão, que aqui fica guardada e é reaproveitada.
//
// A chave do servidor é conferida por impressão digital fixada (hostkey na configuração). Na
// primeira conexão, com hostkey vazia, aceitamos e devolvemos a impressão para a interface
// guardar; das próximas vezes uma chave diferente derruba a conexão.

use russh::client;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::servidor::Config;

/// Sessão aberta, junto com os dados que a criaram: se a configuração mudar, abrimos outra.
struct Sessao {
    chave: String,
    sftp: SftpSession,
    /// mantém o handle vivo: se ele cai, a sessão SFTP morre junto
    _ssh: client::Handle<Verificador>,
}

static ATUAL: std::sync::OnceLock<Mutex<Option<Sessao>>> = std::sync::OnceLock::new();

fn guardada() -> &'static Mutex<Option<Sessao>> {
    ATUAL.get_or_init(|| Mutex::new(None))
}

/// Impressão digital da última chave aceita, para a interface fixar na primeira conexão.
static VISTA: std::sync::OnceLock<std::sync::Mutex<Option<String>>> = std::sync::OnceLock::new();

pub fn hostkey_vista() -> Option<String> {
    VISTA.get_or_init(|| std::sync::Mutex::new(None)).lock().ok()?.clone()
}

struct Verificador {
    esperada: String,
}

impl client::Handler for Verificador {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        chave: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let digital = match chave {
            russh::keys::PublicKeyOrCertificate::PublicKey { key, .. } => {
                key.fingerprint(Default::default()).to_string()
            }
            russh::keys::PublicKeyOrCertificate::Certificate(c) => {
                c.public_key().fingerprint(Default::default()).to_string()
            }
        };
        if let Ok(mut v) = VISTA.get_or_init(|| std::sync::Mutex::new(None)).lock() {
            *v = Some(digital.clone());
        }
        Ok(self.esperada.is_empty() || self.esperada == digital)
    }
}

fn chave_da(c: &Config) -> String {
    format!("{}|{}|{}|{}", c.host, c.porta, c.usuario, c.hostkey)
}

async fn abrir(c: &Config) -> Result<Sessao, String> {
    if c.host.is_empty() || c.usuario.is_empty() {
        return Err("Servidor não configurado".into());
    }
    let cfg = Arc::new(client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(300)),
        ..Default::default()
    });
    let verificador = Verificador { esperada: c.hostkey.clone() };
    let mut ssh = client::connect(cfg, (c.host.as_str(), c.porta), verificador)
        .await
        .map_err(|e| match e {
            russh::Error::UnknownKey => format!(
                "a chave do servidor não é a fixada na configuração. Se o servidor foi reinstalado, \
                 apague o campo da chave e conecte de novo (vista agora: {})",
                hostkey_vista().unwrap_or_else(|| "desconhecida".into())
            ),
            outro => format!("não consegui conectar em {}:{}: {outro}", c.host, c.porta),
        })?;
    if !ssh
        .authenticate_password(&c.usuario, &c.senha)
        .await
        .map_err(|e| format!("falha ao autenticar: {e}"))?
        .success()
    {
        return Err("usuário ou senha do servidor não foram aceitos".into());
    }
    let canal = ssh.channel_open_session().await.map_err(|e| format!("não consegui abrir o canal: {e}"))?;
    canal
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("o servidor não abriu o subsistema sftp: {e}"))?;
    let sftp = SftpSession::new(canal.into_stream())
        .await
        .map_err(|e| format!("não consegui iniciar o sftp: {e}"))?;
    Ok(Sessao { chave: chave_da(c), sftp, _ssh: ssh })
}

/// Roda `f` com a sessão aberta, reabrindo uma vez se a guardada tiver caído.
async fn com_sessao<T, F>(c: &Config, f: F) -> Result<T, String>
where
    F: for<'a> Fn(&'a SftpSession) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, String>> + Send + 'a>>,
{
    let mut guarda = guardada().lock().await;
    if guarda.as_ref().map(|s| s.chave != chave_da(c)).unwrap_or(false) {
        *guarda = None; // configuração mudou
    }
    if guarda.is_none() {
        *guarda = Some(abrir(c).await?);
    }
    match f(&guarda.as_ref().unwrap().sftp).await {
        Ok(v) => Ok(v),
        Err(primeiro) => {
            // sessão provavelmente caída (inatividade, reinício do servidor): uma segunda chance
            *guarda = None;
            let nova = abrir(c).await.map_err(|e| format!("{primeiro}; ao reconectar: {e}"))?;
            *guarda = Some(nova);
            f(&guarda.as_ref().unwrap().sftp).await
        }
    }
}

/// Fecha a sessão guardada. Usado quando a configuração muda.
pub async fn esquecer() {
    *guardada().lock().await = None;
}

pub async fn listar(c: &Config, caminho: &str) -> Result<Vec<String>, String> {
    let caminho = caminho.to_string();
    com_sessao(c, move |sftp| {
        let caminho = caminho.clone();
        Box::pin(async move {
            let mut out = vec![];
            let dir = sftp.read_dir(&caminho).await.map_err(|e| format!("não consegui listar {caminho}: {e}"))?;
            for entrada in dir {
                let n = entrada.file_name();
                if n != "." && n != ".." {
                    out.push(n);
                }
            }
            Ok(out)
        })
    })
    .await
}

/// Usada pelos testes de ponta a ponta.
#[allow(dead_code)]
pub async fn existe(c: &Config, caminho: &str) -> Result<bool, String> {
    let caminho = caminho.to_string();
    com_sessao(c, move |sftp| {
        let caminho = caminho.clone();
        Box::pin(async move { Ok(sftp.metadata(&caminho).await.is_ok()) })
    })
    .await
}

pub async fn baixar(c: &Config, caminho: &str) -> Result<Vec<u8>, String> {
    let caminho = caminho.to_string();
    com_sessao(c, move |sftp| {
        let caminho = caminho.clone();
        Box::pin(async move {
            let mut arq = sftp.open(&caminho).await.map_err(|e| format!("não consegui abrir {caminho}: {e}"))?;
            let mut buf = vec![];
            arq.read_to_end(&mut buf).await.map_err(|e| format!("não consegui ler {caminho}: {e}"))?;
            Ok(buf)
        })
    })
    .await
}

pub async fn enviar(c: &Config, caminho: &str, dados: &[u8]) -> Result<(), String> {
    let caminho = caminho.to_string();
    let dados = dados.to_vec();
    com_sessao(c, move |sftp| {
        let caminho = caminho.clone();
        let dados = dados.clone();
        Box::pin(async move {
            let mut arq = sftp
                .open_with_flags(&caminho, OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE)
                .await
                .map_err(|e| format!("não consegui gravar {caminho}: {e}"))?;
            arq.write_all(&dados).await.map_err(|e| format!("não consegui escrever {caminho}: {e}"))?;
            arq.shutdown().await.map_err(|e| format!("não consegui fechar {caminho}: {e}"))?;
            Ok(())
        })
    })
    .await
}

/// Usada pelos testes de ponta a ponta.
#[allow(dead_code)]
pub async fn apagar(c: &Config, caminho: &str) -> Result<(), String> {
    let caminho = caminho.to_string();
    com_sessao(c, move |sftp| {
        let caminho = caminho.clone();
        Box::pin(async move {
            sftp.remove_file(&caminho).await.map_err(|e| format!("não consegui apagar {caminho}: {e}"))?;
            Ok(())
        })
    })
    .await
}
