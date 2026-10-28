// Implementação de window.api para a versão Tauri. O formato do save e os saves abertos ficam no
// Rust (src-tauri/src/formato, sessao.rs): daqui vai e volta só o modelo (nível, habilidades,
// itens, conhecimento), nunca os bytes do save.
import { compararSaves, motivoEmTexto } from './shared/recusa.js';
import items from './data/items.json' with { type: 'json' };
import { itemName, nomeConhecimento } from './gamedata.js';
import skills from './data/skills.json' with { type: 'json' };
import mapaConhecimento from './data/conhecimento.json' with { type: 'json' };
import { t, dataHora } from './i18n.js';

// O invoke do Tauri rejeita com a string do Err do Rust, que não tem .message: quem fazia
// `catch (e) => e.message` mostrava "undefined" e perdia o motivo do erro. Embrulhamos aqui, numa
// vez só, para todas as chamadas. As mensagens do Rust estão em português, que é a chave do t().
const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args).catch((e) => {
  throw e instanceof Error ? e : new Error(typeof e === 'string' ? t(e) : JSON.stringify(e));
});

const isItem = (id) => Object.prototype.hasOwnProperty.call(items, id);
const isSkill = (id) => Object.prototype.hasOwnProperty.call(skills.nodes, id);

function checkModel(m) {
  const ints = (v, max) => Number.isInteger(v) && v >= 0 && v <= max;
  if (!ints(m.level, 1000) || !ints(m.xp, 0xffffffff)) throw new Error(t('Nível ou XP inválido'));
  for (const s of m.skills) if (!isSkill(s.id) || !ints(s.level, 255)) throw new Error(t('Habilidade inválida'));
  for (const list of [m.equipment, m.main, m.extra]) {
    for (const e of list) {
      if (!isItem(e.itemId)) throw new Error(t('Item desconhecido: {id}', { id: e.itemId }));
      if (!ints(e.slot, 255) || !ints(e.count, 65535)) throw new Error(t('Slot ou quantidade inválidos'));
    }
  }
  const slots = m.main.map((e) => e.slot);
  if (new Set(slots).size !== slots.length) throw new Error(t('Dois itens no mesmo espaço do inventário'));
  if (m.knowledge) {
    for (const k of m.knowledge) {
      if (!ints(k.id, 0xffffffff) || !ints(k.value, 0xffffffff)) throw new Error(t('Entrada de conhecimento inválida'));
    }
    const ids = m.knowledge.map((k) => k.id);
    if (new Set(ids).size !== ids.length) throw new Error(t('Conhecimento com id repetido'));
  }
}


// só o que a interface pode editar; o resto do modelo (nome, caminho, blocos) não volta ao Rust
const edicao = (m) => ({
  level: m.level, xp: m.xp, skills: m.skills, equipment: m.equipment, main: m.main, extra: m.extra,
  knowledge: m.knowledge ?? null,
});

// ---------------------------------------------------------------- cópias do servidor (guardião)
// O guardião guarda, por jogador, o bloco player_character (A.bin) e o player_knowledge (K.bin).
// Elas entram na mesma lista dos saves locais, com o caminho "servidor:<steamid>". Editar a cópia
// e expulsar o jogador faz o guardião trocar o save dele pela versão editada na próxima entrada.
const PREFIXO = 'servidor:';
const ehServidor = (p) => typeof p === 'string' && p.startsWith(PREFIXO);
const steamidDe = (p) => p.slice(PREFIXO.length);
const lerDoServidor = async (steamid, arquivo) => new Uint8Array(await invoke('servidor_ler', { steamid, arquivo }));
// `abrir` guarda a cópia no Rust para gravar depois; sem ele é só leitura
const loadServidor = (steamid, abrir = false) => invoke('servidor_abrir', { steamid, abrir });

// por que a lista do servidor veio vazia (mostrado na aba Servidor)
let estadoServidor = { configurado: false, erro: null };
// última lista boa e, por jogador, a linha já lida. Cada leitura abre uma conexão FTPS, então só
// relemos quem mudou (o meta.json diz a versão do save) e, se o servidor falhar, mantemos a lista.
let cacheJogadores = [];
const cachePorJogador = new Map();
const carimboDe = (meta) => `${meta?.quando ?? ''}|${meta?.versao ?? ''}|${meta?.tamA ?? ''}|${meta?.nK ?? ''}`;
// steamid -> nome, do log do servidor; fica em memória para não baixar o log a cada lista
let nomesCache = null;
const nomesTentados = new Set();
async function nomes(forcar = false) {
  if (nomesCache && !forcar) return nomesCache;
  try {
    const n = await invoke('servidor_nomes', { atualizar: forcar });
    // só guarda resultado útil: guardar o vazio de uma chamada que falhou (com o servidor
    // desligado, por exemplo) travaria os nomes pela sessão toda, porque {} é truthy
    if (n && Object.keys(n).length) nomesCache = n;
    return nomesCache ?? {};
  } catch {
    return nomesCache ?? {};
  }
}

function modeloServidor(s, nome) {
  const { meta, ...m } = s;
  return {
    ...m,
    path: PREFIXO + s.steamid, name: nome ? t('{nome} (servidor)', { nome }) : t('Servidor · {steamid}', { steamid: s.steamid }),
    jogador: nome ?? null, modified: meta?.quando ? Date.parse(meta.quando) : 0,
    servidor: true,
    online: meta?.final === 0, // último save periódico: o jogador provavelmente ainda está na sessão
    blocks: [{ name: 'player_character', size: (meta?.tamA ?? 0) }, { name: 'player_knowledge', size: (meta?.nK ?? 0) * 8 }],
  };
}

export default {
  async listSaves() {
    // cada save local já vem lido pelo Rust, com nome, nível e o erro de leitura, se houver
    const out = (await invoke('listar_saves')).map((f) => (f.error ? { ...f, error: t(f.error) } : f));
    try {
      const cfg = await invoke('servidor_config');
      estadoServidor = { configurado: cfg.configurado, erro: null };
      if (cfg.configurado) {
        const jogadores = await invoke('servidor_jogadores');
        let nn = await nomes();
        // jogador que entrou depois do último cache ainda não tem nome: relê o log do servidor.
        // A tentativa só conta quando trouxe nome: senão uma releitura com o servidor fora do ar
        // deixaria esse jogador como steamid até o editor ser reaberto.
        const faltam = jogadores.filter((j) => !nn[j.steamid] && !nomesTentados.has(j.steamid));
        if (faltam.length) {
          nn = await nomes(true);
          if (Object.keys(nn).length) faltam.forEach((j) => nomesTentados.add(j.steamid));
        }
        const novos = [];
        for (const j of jogadores) {
          const nome = nn[j.steamid] ?? null;
          const meta = j.meta ? JSON.parse(j.meta) : null;
          const carimbo = carimboDe(meta);
          const guardado = cachePorJogador.get(j.steamid);
          if (guardado && guardado.carimbo === carimbo) {
            novos.push({ ...guardado.linha, jogador: nome ?? guardado.linha.jogador });
            continue;
          }
          const base = { path: PREFIXO + j.steamid, file: t('servidor · {steamid}', { steamid: j.steamid }), servidor: true, steamid: j.steamid, jogador: nome };
          let linha;
          try {
            const m = modeloServidor(await loadServidor(j.steamid), nome);
            linha = { ...base, name: m.name, level: m.level, error: m.error, online: m.online, modified: m.modified };
          } catch (e) {
            linha = { ...base, name: nome ? t('{nome} (servidor)', { nome }) : t('Servidor · {steamid}', { steamid: j.steamid }), error: e.message, modified: 0 };
          }
          cachePorJogador.set(j.steamid, { carimbo, linha });
          novos.push(linha);
        }
        cacheJogadores = novos;
        out.push(...novos);
      }
    } catch (e) {
      // servidor fora do ar ou chamada que falhou: mantém a última lista boa em vez de apagá-la
      estadoServidor = { configurado: estadoServidor.configurado, erro: e.message };
      out.push(...cacheJogadores.map((l) => ({ ...l, desatualizado: true })));
    }
    return out;
  },
  serverState: () => estadoServidor,
  // data do último save guardado, para saber se a cópia no servidor mudou (o servidor grava a cada 30 s)
  async serverStamp(path) {
    if (!ehServidor(path)) return null;
    try {
      const m = JSON.parse(new TextDecoder().decode(await lerDoServidor(steamidDe(path), 'meta.json')));
      return `${m.quando ?? ''}|${m.versao ?? ''}|${m.tamA ?? ''}|${m.nK ?? ''}`;
    } catch { return null; }
  },
  serverConfig: () => invoke('servidor_config'),
  setServerConfig: (cfg) => invoke('servidor_config_set', cfg),
  serverMode: () => invoke('servidor_modo'),
  serverHostKey: () => invoke('servidor_hostkey'),
  serverLog: () => invoke('servidor_registro'),
  serverNames: (atualizar = false) => invoke('servidor_nomes', { atualizar }),
  // Por que o guardião recusou UMA entrada. Compara o save que o cliente mandou (guardado em
  // recusados/) com a cópia que o servidor tem. O ponto onde o personagem termina e o
  // conhecimento começa é o tamA que o registro anotou naquela entrada.
  //
  // Uma recusa por chamada, e não todas as do jogador: cada arquivo é uma conexão FTPS nova, e o
  // servidor recusa conexões simultâneas quando a atualização automática está no ar.
  async serverRejection(steamid, quando) {
    const lista = await this.serverRejections(steamid, { quando });
    return lista[0] ?? null;
  },
  async serverRejections(steamid, { quando = null, limite = 6 } = {}) {
    const arquivos = await invoke('servidor_recusados', { steamid });
    if (!arquivos.length) return [];
    const porCarimbo = new Map();
    for (const linha of (await invoke('servidor_registro')).trim().split('\n')) {
      let d;
      try { d = JSON.parse(linha); } catch { continue; }
      if (d.kind === 'entrada' && d.steamid === steamid && d.tamA) {
        porCarimbo.set(d.quando.replace(/[-:TZ]/g, ''), d);
      }
    }
    const achar = (carimbo) => {
      const base = Date.UTC(+carimbo.slice(0, 4), +carimbo.slice(4, 6) - 1, +carimbo.slice(6, 8),
        +carimbo.slice(8, 10), +carimbo.slice(10, 12), +carimbo.slice(12, 14));
      for (let d = 0; d <= 5; d++) {
        for (const s of [1, -1]) {
          const k = new Date(base + s * d * 1000).toISOString().slice(0, 19).replace(/[-:T]/g, '');
          if (porCarimbo.has(k)) return porCarimbo.get(k);
        }
      }
      return null;
    };
    // o nome do arquivo carrega o carimbo em UTC: 20261022_054601_substituido.bin
    const horaDo = (arquivo) => {
      const m = /^(\d{8})_(\d{6})_(\w+)\.bin$/.exec(arquivo);
      if (!m) return null;
      const c = m[1] + m[2];
      return {
        carimbo: c,
        tipo: m[3],
        quando: `${c.slice(0, 4)}-${c.slice(4, 6)}-${c.slice(6, 8)}T${c.slice(8, 10)}:${c.slice(10, 12)}:${c.slice(12, 14)}Z`,
      };
    };
    // o guardião grava, ao lado do save recusado, a cópia que ele tinha naquele instante
    const temReferencia = new Set(arquivos.filter((n) => n.endsWith('_referencia.bin')).map((n) => n.slice(0, 15)));
    let escolhidos = arquivos.filter((n) => !n.endsWith('_referencia.bin')).map(horaDo).filter(Boolean);
    if (quando) {
      const t = Date.parse(quando);
      escolhidos = escolhidos.filter((h) => Math.abs(Date.parse(h.quando) - t) <= 5000);
    }
    escolhidos = escolhidos.slice(-limite).reverse();
    if (!escolhidos.length) return [];
    const atual = await loadServidor(steamid);
    const servidor = { ...atual, knowledge: atual.knowledge ?? [] };
    const nomeItem = (id) => (items[id] ? itemName(id) : t('id {id}', { id }));
    const nomeDeConhecimento = (id) => (mapaConhecimento[id]
      ? nomeConhecimento(mapaConhecimento[id], id)
      : (items[id] ? itemName(id) : t('id {id}', { id })));
    const out = [];
    for (const { carimbo, tipo, quando: quandoArq } of escolhidos) {
      const arquivo = `${carimbo.slice(0, 8)}_${carimbo.slice(8)}_${tipo}.bin`;
      const reg = achar(carimbo);
      const item = { arquivo, quando: quandoArq, tipo, motivo: [], dif: null };
      if (!reg) {
        item.motivo = [{ texto: t('não achei essa entrada no registro, então não sei onde o personagem termina'), aviso: true }];
        out.push(item);
        continue;
      }
      try {
        const cliente = await invoke('servidor_recusado_abrir', { steamid, nome: arquivo, tamA: reg.tamA, nK: reg.nK });
        // A referência certa é a cópia do instante da recusa, que o guardião grava ao lado. Sem
        // ela (recusas anteriores a essa versão do guardião) sobra a cópia de agora, que já mudou:
        // aí a diferença inclui o que o jogador fez depois, e o motivo avisa isso.
        let referencia = servidor;
        let referenciaAntiga = reg.guardadoTamA !== atual.meta?.tamA || reg.guardadoNK !== atual.meta?.nK;
        if (temReferencia.has(carimbo.slice(0, 8) + '_' + carimbo.slice(8))) {
          try {
            referencia = await invoke('servidor_recusado_abrir', {
              steamid, nome: `${carimbo.slice(0, 8)}_${carimbo.slice(8)}_referencia.bin`, tamA: reg.guardadoTamA, nK: null,
            });
            referenciaAntiga = false;
          } catch { /* sem a referência, fica a cópia atual e o aviso */ }
        }
        item.referenciaAntiga = referenciaAntiga;
        item.dif = compararSaves(cliente, referencia, { nomeItem, nomeConhecimento: nomeDeConhecimento });
        item.motivo = motivoEmTexto(item.dif, {
          t,
          referenciaAntiga,
          referenciaQuando: atual.meta?.quando ? dataHora(atual.meta.quando) : null,
          registro: reg,
        });
        item.cliente = { level: cliente.level, xp: cliente.xp };
      } catch (e) {
        // o invoke do Tauri rejeita com a string do Err do Rust, que não tem .message
        item.motivo = [{ texto: t('não consegui ler esse save recusado: {erro}', { erro: e?.message ?? e }), aviso: true }];
      }
      out.push(item);
    }
    return out;
  },
  async openSave(path) {
    if (ehServidor(path)) {
      const s = await loadServidor(steamidDe(path), true);
      return modeloServidor(s, (await nomes())[s.steamid]);
    }
    return invoke('abrir_save', { path });
  },
  async browseSave() {
    const r = await invoke('plugin:dialog|open', {
      options: { filters: [{ name: t('Save do Enshrouded'), extensions: ['ksd'] }], multiple: false },
    });
    return r || null;
  },
  async writeSave(path, model) {
    if (ehServidor(path)) return this.writeServidor(path, model);
    if (await invoke('game_running')) throw new Error(t('Feche o Enshrouded antes de salvar: o jogo sobrescreveria a edição.'));
    checkModel(model);
    // o Rust remonta o bloco, confere que ele é lido de volta, valida o .ksd inteiro e faz o backup
    return invoke('gravar_save', { path, model: edicao(model) });
  },
  // grava a cópia do servidor: vale na próxima entrada do jogador (expulse-o para aplicar agora)
  async writeServidor(path, model) {
    checkModel(model);
    const r = await invoke('servidor_gravar_personagem', { steamid: steamidDe(path), model: edicao(model) });
    return { backupDir: r.backupDir, model: modeloServidor(r.model, (await nomes())[r.model.steamid]) };
  },
  gameRunning: () => invoke('game_running'),
  // cópias do servidor não têm backup automático do jogo; o editor guarda o anterior ao gravar
  listBackups: (target) => (ehServidor(target) ? Promise.resolve([]) : invoke('list_backups', { target })),
  restoreBackup: (src, target) => (ehServidor(target)
    ? Promise.reject(new Error(t('Para desfazer no servidor, grave de novo a cópia guardada em Documentos\\Enshrouded-Backups\\servidor')))
    : invoke('restore_backup', { src, target }).then(() => true)),
  openBackups: () => invoke('open_backups'),
};
