// Implementação de window.api para a versão Tauri: a lógica do save roda aqui (JS no WebView)
// e o Rust cuida de arquivos, zstd, backups e da detecção do jogo.
import { Buffer } from 'buffer';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import * as sv2c from '../shared/sv2c-web.mjs';
import { parseCharacter, buildCharacter } from '../shared/character.mjs';
import * as know from '../shared/knowledge.mjs';
import { compararSaves, motivoEmTexto } from '../shared/recusa.mjs';
import items from '../data/items.json';
import { itemName, nomeConhecimento } from './gamedata.js';
import skills from '../data/skills.json';
import mapaConhecimento from '../data/conhecimento.json';
import { t, dataHora } from './i18n.js';

// O invoke do Tauri rejeita com a string do Err do Rust, que não tem .message: quem fazia
// `catch (e) => e.message` mostrava "undefined" e perdia o motivo do erro. Embrulhamos aqui, numa
// vez só, para todas as chamadas.
const invoke = (cmd, args) => invokeTauri(cmd, args).catch((e) => {
  throw e instanceof Error ? e : new Error(typeof e === 'string' ? e : JSON.stringify(e));
});

const isItem = (id) => Object.prototype.hasOwnProperty.call(items, id);
const isSkill = (id) => Object.prototype.hasOwnProperty.call(skills.nodes, id);
// O que pode estar vestido, pelo campo slot do item: separa a lista de equipamento da mochila,
// que também é densa e começa no índice 0. As armas ficam fora desta lista.
const ESPACOS = new Set(['Armour_Head', 'Armour_UpperBody', 'Armour_Arms', 'Armour_LowerBody', 'Armour_Feet',
  'Armour_Ring', 'Shield', 'HookShot', 'Glider', 'Backpack']);
const ehEquipavel = (id) => ESPACOS.has(items[id]?.slot);
const sessions = new Map();

const readFile = async (path) => new Uint8Array(await invoke('read_file', { path }));
const decompress = async (bytes) => new Uint8Array(await invoke('zstd_decompress', { data: Array.from(bytes) }));
const compress = async (bytes) => new Uint8Array(await invoke('zstd_compress', { data: Array.from(bytes) }));

async function load(path) {
  await sv2c.init();
  const save = await sv2c.parse(await readFile(path), decompress, ['character_data', 'player_character', 'player_knowledge']);
  const pc = save.blocks.find((b) => b.name === 'player_character');
  let character = null, error = null;
  try { character = parseCharacter(pc.payload, isItem, isSkill, ehEquipavel); } catch (e) { error = e.message; }
  let knowledge = null;
  const pk = save.blocks.find((b) => b.name === 'player_knowledge');
  try { if (pk?.payload) knowledge = know.parseBlock(pk.payload); } catch (e) { knowledge = null; }
  return { save, character, knowledge, error };
}

function publicModel(path, modified, s) {
  const c = s.character;
  return {
    path, name: sv2c.characterName(s.save), modified, error: s.error, readOnly: !c,
    level: c?.level, xp: c?.xp, skills: c?.skills ?? [], equipment: c?.equipment ?? [], main: c?.main ?? [], extra: c?.extra ?? [],
    knowledge: s.knowledge?.entries ?? null,
    equipMask: c?._equipMask ?? 0, // posiciona cada peça no espaço certo do boneco
    blocks: s.save.blocks.map((b) => ({ name: b.name, size: b.payload ? b.payload.length : b.size })),
  };
}

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

// o conhecimento só é regravado se mudou: assim um save aberto e salvo sem mexer nele fica igual
function mudouConhecimento(s, model) {
  if (!s.knowledge || !model.knowledge) return false;
  return JSON.stringify(s.knowledge.entries) !== JSON.stringify(model.knowledge);
}

// ---------------------------------------------------------------- cópias do servidor (guardião)
// O guardião guarda, por jogador, o bloco player_character (A.bin) e o player_knowledge (K.bin).
// Elas entram na mesma lista dos saves locais, com o caminho "servidor:<steamid>". Editar a cópia
// e expulsar o jogador faz o guardião trocar o save dele pela versão editada na próxima entrada.
const PREFIXO = 'servidor:';
const CAB = 14; // cabeçalho S2BH de um bloco
const ehServidor = (p) => typeof p === 'string' && p.startsWith(PREFIXO);
const steamidDe = (p) => p.slice(PREFIXO.length);
const lerDoServidor = async (steamid, arquivo) => new Uint8Array(await invoke('servidor_ler', { steamid, arquivo }));

async function loadServidor(steamid) {
  // Buffer, e não Uint8Array: parseCharacter usa os métodos de leitura do Buffer
  const A = Buffer.from(await lerDoServidor(steamid, 'A.bin'));
  if (A.length < CAB || A.toString('latin1', 0, 4) !== 'S2BH') {
    throw new Error(t('a cópia do servidor não é um bloco de personagem'));
  }
  let character = null, error = null;
  try { character = parseCharacter(A.subarray(CAB), isItem, isSkill, ehEquipavel); } catch (e) { error = e.message; }
  let knowledge = null;
  try { knowledge = know.parseRaw(await lerDoServidor(steamid, 'K.bin')); } catch { knowledge = null; }
  let meta = {};
  try { meta = JSON.parse(new TextDecoder().decode(await lerDoServidor(steamid, 'meta.json'))); } catch { /* sem meta */ }
  return { servidor: true, steamid, cabecalho: A.slice(0, CAB), character, knowledge, error, meta };
}

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
  const c = s.character;
  return {
    path: PREFIXO + s.steamid, name: nome ? t('{nome} (servidor)', { nome }) : t('Servidor · {steamid}', { steamid: s.steamid }),
    jogador: nome ?? null, modified: s.meta?.quando ? Date.parse(s.meta.quando) : 0,
    error: s.error, readOnly: !c, servidor: true, steamid: s.steamid,
    online: s.meta?.final === 0, // último save periódico: o jogador provavelmente ainda está na sessão
    level: c?.level, xp: c?.xp, skills: c?.skills ?? [], equipment: c?.equipment ?? [], main: c?.main ?? [], extra: c?.extra ?? [],
    knowledge: s.knowledge?.entries ?? null,
    equipMask: c?._equipMask ?? 0,
    blocks: [{ name: 'player_character', size: (s.meta?.tamA ?? 0) }, { name: 'player_knowledge', size: (s.meta?.nK ?? 0) * 8 }],
  };
}

export default {
  async listSaves() {
    const files = await invoke('list_saves');
    const out = [];
    for (const f of files) {
      try {
        const s = await load(f.path);
        out.push({ ...f, name: sv2c.characterName(s.save), level: s.character?.level, error: s.error });
      } catch (e) {
        out.push({ ...f, name: '?', error: e.message });
      }
    }
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
    const servidor = { ...atual.character, knowledge: atual.knowledge?.entries ?? [] };
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
        const blob = Buffer.from(new Uint8Array(await invoke('servidor_recusado_ler', { steamid, nome: arquivo })));
        if (reg.tamA + reg.nK * 8 !== blob.length) throw new Error(t('os tamanhos do registro não fecham com o arquivo'));
        const c = parseCharacter(blob.subarray(CAB, reg.tamA), isItem, isSkill, ehEquipavel);
        const cliente = { ...c, knowledge: know.parseRaw(blob.subarray(reg.tamA)).entries };
        // A referência certa é a cópia do instante da recusa, que o guardião grava ao lado. Sem
        // ela (recusas anteriores a essa versão do guardião) sobra a cópia de agora, que já mudou:
        // aí a diferença inclui o que o jogador fez depois, e o motivo avisa isso.
        let referencia = servidor;
        let referenciaAntiga = reg.guardadoTamA !== atual.meta?.tamA || reg.guardadoNK !== atual.meta?.nK;
        if (temReferencia.has(carimbo.slice(0, 8) + '_' + carimbo.slice(8))) {
          try {
            const bref = Buffer.from(new Uint8Array(await invoke('servidor_recusado_ler', {
              steamid, nome: `${carimbo.slice(0, 8)}_${carimbo.slice(8)}_referencia.bin`,
            })));
            const cr = parseCharacter(bref.subarray(CAB, reg.guardadoTamA), isItem, isSkill, ehEquipavel);
            referencia = { ...cr, knowledge: know.parseRaw(bref.subarray(reg.guardadoTamA)).entries };
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
      const s = await loadServidor(steamidDe(path));
      sessions.set(path, s);
      return modeloServidor(s, (await nomes())[s.steamid]);
    }
    const s = await load(path);
    sessions.set(path, s);
    const { modified } = await invoke('file_info', { path });
    return publicModel(path, modified, s);
  },
  async browseSave() {
    const r = await openDialog({ filters: [{ name: t('Save do Enshrouded'), extensions: ['ksd'] }], multiple: false });
    return r || null;
  },
  async writeSave(path, model) {
    if (ehServidor(path)) return this.writeServidor(path, model);
    if (await invoke('game_running')) throw new Error(t('Feche o Enshrouded antes de salvar: o jogo sobrescreveria a edição.'));
    const s = sessions.get(path);
    if (!s?.character) throw new Error(t('Save não aberto para edição'));
    checkModel(model);
    const c = { ...s.character, level: model.level, xp: model.xp, skills: model.skills, equipment: model.equipment, main: model.main, extra: model.extra };
    const payload = buildCharacter(c);
    parseCharacter(payload, isItem, isSkill, ehEquipavel); // confere que o bloco novo é lido de volta
    const pk = mudouConhecimento(s, model) ? know.buildBlock({ ...s.knowledge, entries: model.knowledge }) : null;
    const blocks = s.save.blocks.map((b) => {
      if (b.name === 'player_character') return { ...b, payload, changed: true };
      if (b.name === 'player_knowledge' && pk) return { ...b, payload: pk, changed: true };
      return b;
    });
    const data = await sv2c.build({ header: s.save.header, blocks }, compress);
    await sv2c.parse(data, decompress, []); // valida hashes e estrutura
    const backupDir = await invoke('backup_file', { path });
    await invoke('write_save', { path, data: Array.from(data) });
    const fresh = await load(path);
    sessions.set(path, fresh);
    const { modified } = await invoke('file_info', { path });
    return { backupDir, model: publicModel(path, modified, fresh) };
  },
  // grava a cópia do servidor: vale na próxima entrada do jogador (expulse-o para aplicar agora)
  async writeServidor(path, model) {
    const s = sessions.get(path);
    if (!s?.character) throw new Error(t('Cópia do servidor não aberta para edição'));
    checkModel(model);
    const c = { ...s.character, level: model.level, xp: model.xp, skills: model.skills, equipment: model.equipment, main: model.main, extra: model.extra };
    const payload = buildCharacter(c);
    parseCharacter(payload, isItem, isSkill, ehEquipavel); // confere que o bloco novo é lido de volta
    const A = new Uint8Array(s.cabecalho.length + payload.length);
    A.set(s.cabecalho, 0);
    A.set(payload, s.cabecalho.length);
    const backupDir = await invoke('servidor_gravar', { steamid: s.steamid, arquivo: 'A.bin', data: Array.from(A) });
    if (mudouConhecimento(s, model)) {
      const K = know.buildRaw({ entries: model.knowledge });
      await invoke('servidor_gravar', { steamid: s.steamid, arquivo: 'K.bin', data: Array.from(K) });
    }
    const fresh = await loadServidor(s.steamid);
    sessions.set(path, fresh);
    return { backupDir, model: modeloServidor(fresh, (await nomes())[fresh.steamid]) };
  },
  gameRunning: () => invoke('game_running'),
  // cópias do servidor não têm backup automático do jogo; o editor guarda o anterior ao gravar
  listBackups: (target) => (ehServidor(target) ? Promise.resolve([]) : invoke('list_backups', { target })),
  restoreBackup: (src, target) => (ehServidor(target)
    ? Promise.reject(new Error(t('Para desfazer no servidor, grave de novo a cópia guardada em Documentos\\Enshrouded-Backups\\servidor')))
    : invoke('restore_backup', { src, target }).then(() => { sessions.delete(target); return true; })),
  openBackups: () => invoke('open_backups'),
};
