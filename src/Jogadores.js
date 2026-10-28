import { html, Fragment, useEffect, useState } from './react.js';
import { api } from './platform.js';
import { GameButton } from './components.js';
import { t, dataHora } from './i18n.js';

// Aba Servidor: os jogadores com cópia guardada pelo guardião, com o que o registro dele mostra.
const DECISAO_PT = {
  primeira_entrada: 'primeira entrada (aceito e guardado)',
  igual: 'entrou com o mesmo save',
  substituido: 'trouxe save de fora: trocado pela cópia do servidor',
  divergente_observado: 'trouxe save de fora (só registrado: modo observar)',
  substituicao_falhou: 'a troca falhou',
  sem_steamid: 'entrada sem steamid',
};

const RECUSA = new Set(['substituido', 'divergente_observado', 'substituicao_falhou']);

export default function Jogadores({ saves, atual, onOpen, onConfig, onRefresh }) {
  const [modo, setModo] = useState(null);
  const [registro, setRegistro] = useState(null);
  const [erro, setErro] = useState(null);
  const [nomes, setNomes] = useState({});
  const [estado, setEstado] = useState(null);
  // "steamid|quando" -> o motivo daquela recusa, ou 'lendo', ou a mensagem de erro
  const [recusas, setRecusas] = useState({});
  const jogadores = saves.filter((s) => s.servidor);
  const nomeDe = (s) => s.jogador ?? nomes[s.steamid] ?? null;

  // O motivo vem de comparar o save recusado com a cópia do servidor. Buscamos uma recusa por
  // clique: cada arquivo é uma conexão FTPS nova, e o servidor recusa conexões simultâneas.
  const chaveDa = (l) => `${l.steamid}|${l.quando}`;
  const verMotivo = async (linha) => {
    const k = chaveDa(linha);
    setRecusas((r) => ({ ...r, [k]: 'lendo' }));
    try {
      const m = await api.serverRejection?.(linha.steamid, linha.quando);
      setRecusas((r) => ({ ...r, [k]: m ?? t('o guardião não guardou o save dessa entrada') }));
    } catch (e) {
      // o invoke do Tauri rejeita com a string do Err do Rust, que não tem .message
      setRecusas((r) => ({ ...r, [k]: `${e?.message ?? e}` }));
    }
  };

  // ao abrir a aba, relê a lista (com o cache por jogador isso custa pouco) e o estado do guardião
  const carregar = () => {
    Promise.resolve(onRefresh?.()).finally(() => setEstado(api.serverState?.() ?? null));
    api.serverNames().then(setNomes).catch(() => {});
    api.serverMode().then(setModo).catch((e) => setErro(e.message));
    api.serverLog().then((texto) => {
      const linhas = texto.trim().split('\n').map((l) => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);
      setRegistro(linhas.filter((l) => l.kind === 'entrada').slice(-12).reverse());
    }).catch(() => setRegistro([]));
  };
  useEffect(carregar, []);

  return html`
    <div className="jog">
      <div className="jog-cab">
        <div>
          <div className="detail-title">${t('Jogadores no servidor')}</div>
          <div className="hint">
            ${t('O guardião guarda o último save que o servidor mandou a cada jogador. Edite a cópia aqui e expulse o jogador: ele volta com a sua versão.')}
            ${modo === 'observar' && ` ${t('Atenção: o guardião está em modo observar, então ele ainda não troca save nenhum.')}`}
            ${modo === 'aplicar' && ` ${t('O guardião está em modo aplicar.')}`}
          </div>
          ${erro && html`<div className="hint error">${erro}</div>`}
        </div>
        <div className="row-gap">
          <${GameButton} small onClick=${() => { onRefresh(); carregar(); }}>${t('Atualizar')}<//>
          <${GameButton} small onClick=${() => api.serverNames(true).then((n) => { setNomes(n); onRefresh(); }).catch((e) => setErro(e.message))}>
            ${t('Reler nomes')}
          <//>
          <${GameButton} small onClick=${onConfig}>${t('Configurar servidor')}<//>
        </div>
      </div>

      ${!jogadores.length && html`
        <div className=${`hint ${estado?.erro ? 'error' : ''}`}>
          ${!estado?.configurado
            ? t('Servidor ainda não configurado: clique em "Configurar servidor", preencha endereço, usuário e senha e clique em Salvar.')
            : estado?.erro
              ? t('Não consegui falar com o servidor: {erro}', { erro: estado.erro })
              : t('Nenhum jogador com cópia guardada. Peça para cada um entrar uma vez: o guardião guarda a cópia no primeiro save.')}
        </div>
      `}
      <div className="jog-lista">
        ${jogadores.map((s) => html`
          <button key=${s.path} className=${`jog-card ${atual === s.path ? 'on' : ''}`} onClick=${() => onOpen(s.path)}>
            <b>${nomeDe(s) ?? s.steamid ?? s.name}</b>
            <span>
              ${nomeDe(s) ? `${s.steamid} · ` : ''}
              ${s.level !== undefined ? t('nível {n}', { n: s.level }) : t('não consegui ler')}
              ${s.modified ? ` · ${t('save de {quando}', { quando: dataHora(s.modified) })}` : ''}
            </span>
            ${s.online && html`<span className="srv-tag on">${t('na sessão')}</span>`}
            ${s.desatualizado && html`<span className="warn">${t('sem resposta do servidor: dados da última leitura')}</span>`}
            ${s.error && html`<span className="warn">${s.error}</span>`}
          </button>
        `)}
      </div>

      ${registro?.length > 0 && html`
        <${Fragment}>
          <div className="caps small">${t('Últimas entradas')}</div>
          <div className="jog-reg">
            ${registro.map((l, i) => {
              const m = RECUSA.has(l.decisao) ? recusas[chaveDa(l)] : null;
              return html`
                <div key=${i} className="jog-reg-row">
                  <span className="bk-date">${dataHora(l.quando)}</span>
                  <span className="bk-label">${nomes[l.steamid] ?? l.steamid ?? '—'}</span>
                  <span className=${RECUSA.has(l.decisao) ? 'warn' : ''}>${t(DECISAO_PT[l.decisao] ?? l.decisao)}</span>
                  ${RECUSA.has(l.decisao) && l.steamid && (
                    m === 'lendo' ? html`<span className="hint">${t('lendo o save recusado…')}</span>`
                      : typeof m === 'string' ? html`
                        <span className="hint error">
                          ${m} <${GameButton} small onClick=${() => verMotivo(l)}>${t('Tentar de novo')}<//>
                        </span>
                      ` : m ? html`
                        <div className="jog-motivo">
                          <b className="caps small">${t('Motivo')}</b>
                          <ul>
                            ${m.motivo.map((linha, j) => html`
                              <li key=${j} className=${linha.aviso ? 'error' : undefined}>${linha.texto}</li>
                            `)}
                          </ul>
                        </div>
                      ` : html`<${GameButton} small onClick=${() => verMotivo(l)}>${t('Ver motivo')}<//>`
                  )}
                </div>
              `;
            })}
          </div>
        <//>
      `}
    </div>
  `;
}
