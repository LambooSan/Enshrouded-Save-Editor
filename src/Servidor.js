import { html, useEffect, useState } from './react.js';
import { api } from './platform.js';
import { GameButton } from './components.js';
import { t } from './i18n.js';

// Configuração do acesso ao servidor dedicado e situação do guardião.
// O editor lê as cópias que o guardião guarda (guardiao-dados/<steamid>) por FTPS.
export default function Servidor({ onClose, onChanged }) {
  const [cfg, setCfg] = useState(null);
  const [senha, setSenha] = useState('');
  const [msg, setMsg] = useState(null);
  const [modo, setModo] = useState(null);

  useEffect(() => { api.serverConfig().then((c) => setCfg({ ...c })).catch((e) => setMsg({ kind: 'error', text: e.message })); }, []);
  useEffect(() => {
    const k = (e) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', k); return () => window.removeEventListener('keydown', k);
  }, [onClose]);

  const campo = (rotulo, chave, dica, tipo = 'text') => html`
    <label className="srv-campo">
      <span className="caps small">${t(rotulo)}</span>
      <input type=${tipo} value=${cfg[chave] ?? ''} placeholder=${t(dica)}
        onChange=${(e) => setCfg({ ...cfg, [chave]: tipo === 'number' ? Number(e.target.value) : e.target.value })} />
    </label>
  `;

  const salvar = async () => {
    try {
      await api.setServerConfig({ host: cfg.host, porta: Number(cfg.porta) || 2022, usuario: cfg.usuario, senha, hostkey: cfg.hostkey ?? '', pasta: cfg.pasta });
      setSenha('');
      const c = await api.serverConfig();
      setCfg({ ...c });
      setMsg({ kind: 'ok', text: t('Salvo. Atualizando a lista de jogadores…') });
      onChanged();
    } catch (e) { setMsg({ kind: 'error', text: e.message }); }
  };

  // depois de uma conexão bem-sucedida, grava a impressão digital que o servidor apresentou:
  // daí em diante uma chave diferente (servidor trocado, ou alguém no meio) derruba a conexão
  const fixarChave = async () => {
    try {
      const k = await api.serverHostKey?.();
      if (!k) {
        setMsg({ kind: 'warn', text: t('Teste a conexão primeiro: a chave só aparece depois de conectar uma vez.') });
        return;
      }
      setCfg({ ...cfg, hostkey: k });
      setMsg({ kind: 'ok', text: t('Chave {chave} preenchida. Clique em Salvar para fixar.', { chave: k }) });
    } catch (e) { setMsg({ kind: 'error', text: `${e?.message ?? e}` }); }
  };

  const testar = async () => {
    setMsg({ kind: 'warn', text: t('Testando…') });
    try {
      const m = await api.serverMode();
      setModo(m);
      setMsg({ kind: 'ok', text: t('Conectado. Guardião em modo "{modo}".', { modo: m }) });
      onChanged();
    } catch (e) { setMsg({ kind: 'error', text: e.message }); }
  };

  if (!cfg) return null;
  return html`
    <div className="modal" onMouseDown=${(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="modal-box">
        <div className="detail-title">${t('Servidor dedicado')}</div>
        <div className="hint">
          ${t('O editor lê as cópias de personagem que o guardião guarda no servidor. Editar uma cópia e expulsar o jogador faz o guardião trocar o save dele pela sua versão na próxima entrada.')}
        </div>
        ${msg && html`<div className=${`hint ${msg.kind}`}>${msg.text}</div>`}
        <div className="srv-form">
          ${campo('Endereço', 'host', 'endereço do servidor')}
          ${campo('Porta (SFTP)', 'porta', '2022', 'number')}
          ${campo('Usuário', 'usuario', 'usuário do SFTP do painel')}
          <label className="srv-campo">
            <span className="caps small">${t('Senha')}</span>
            <input type="password" value=${senha} placeholder=${cfg.tem_senha ? t('(guardada; deixe vazio para manter)') : t('senha do SFTP')}
              onChange=${(e) => setSenha(e.target.value)} />
          </label>
          ${campo('Chave do servidor', 'hostkey', 'SHA256:… (vazio = aceita e guarda na primeira conexão)')}
          ${campo('Pasta do guardião', 'pasta', 'guardiao-dados')}
        </div>
        <div className="hint">
          ${t('A chave fixada autentica o servidor. Deixando vazia, a primeira conexão aceita a chave que vier; clique em "Fixar chave vista" depois de testar para gravá-la, e daí em diante uma chave diferente derruba a conexão.')}
          ${modo === 'observar' && ` ${t('O guardião está em modo observar: ele registra as divergências, mas não troca save nenhum.')}`}
        </div>
        <div className="row-gap">
          <${GameButton} onClick=${salvar}>${t('Salvar')}<//>
          <${GameButton} onClick=${testar} disabled=${!cfg.host || !cfg.usuario}>${t('Testar conexão')}<//>
          <${GameButton} onClick=${fixarChave} disabled=${!!cfg.hostkey}>${t('Fixar chave vista')}<//>
          <${GameButton} onClick=${onClose}>${t('Fechar')}<//>
        </div>
      </div>
    </div>
  `;
}
