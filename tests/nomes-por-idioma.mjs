// Nome de item e de habilidade tem de acompanhar o idioma escolhido.
//
// Este teste existe porque a primeira versão da tradução não fazia isso: `itemName` devolvia
// `it.pt || it.en`, então trocar para inglês traduzia a interface e deixava todo item em português.
// Como o jogo já traz os dois nomes, o erro não era falta de tradução — era ler o campo errado.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const items = JSON.parse(fs.readFileSync(path.join(RAIZ, 'data', 'items.json'), 'utf8'));
const skills = JSON.parse(fs.readFileSync(path.join(RAIZ, 'data', 'skills.json'), 'utf8'));

// carrega o gamedata com o campo de nome fixado, para medir os dois idiomas sem tocar na interface
async function carregar(campo) {
  const fonte = fs.readFileSync(path.join(RAIZ, 'src', 'gamedata.js'), 'utf8')
    // troca o import do i18n por versões fixas: o teste mede um idioma por vez, e o módulo real
    // depende de localStorage e navigator. O `[^}]*` cobre o import crescer com mais nomes.
    .replace(/^import\s+\{[^}]*\}\s+from\s+'\.\/i18n\.js';$/gm,
      `const campoDeNome = () => '${campo}';\nconst localeAtual = () => '${campo === 'pt' ? 'pt-BR' : 'en-US'}';`)
    .replace(/^import\s+(\w+)\s+from\s+'\.\.\/data\/([\w.-]+)\.json';$/gm,
      (_, nome, arq) => `const ${nome} = ${fs.readFileSync(path.join(RAIZ, 'data', `${arq}.json`), 'utf8')};`);
  const tmp = path.join(RAIZ, `.gamedata-${campo}-${process.pid}.mjs`);
  fs.writeFileSync(tmp, fonte);
  try {
    return await import(pathToFileURL(tmp).href);
  } finally {
    fs.unlinkSync(tmp);
  }
}

const pt = await carregar('pt');
const en = await carregar('en');

let ok = 0;
let ruim = 0;
const conferir = (certo, descricao) => {
  if (certo) ok += 1; else ruim += 1;
  console.log(`${certo ? 'ok  ' : 'ERRO'} ${descricao}`);
};

// itens conhecidos, com o nome que a localização do jogo traz
for (const [id, nomePt, nomeEn] of [
  [2818493303, 'Chá de Tomilho', 'Thyme Tea'],
  [1457307757, 'Arco Aceso', 'Ignited Bow'],
  [3663511234, 'Joia da Queimadura', 'Gem of Burning'],
  [3905532315, 'Picareta de Aço', 'Steel Pickaxe'],
  [3131144042, 'Mochila grande', 'Large Backpack'],
]) {
  conferir(pt.itemName(id) === nomePt, `item ${id} em português: ${pt.itemName(id)}`);
  conferir(en.itemName(id) === nomeEn, `item ${id} em inglês:    ${en.itemName(id)}`);
}

// habilidade também
conferir(pt.skillName(2398930034) === 'Fagulha', `habilidade em português: ${pt.skillName(2398930034)}`);
conferir(en.skillName(2398930034) === 'Spark', `habilidade em inglês:    ${en.skillName(2398930034)}`);

// item sem nome nenhum cai no nome de depuração, nos dois idiomas
const semNome = Object.keys(items).find((i) => !items[i].pt && !items[i].en && items[i].debug);
if (semNome) {
  conferir(pt.itemName(semNome) === items[semNome].debug, `item sem nome cai no debug: ${pt.itemName(semNome)}`);
}
conferir(en.itemName(999999999) === '#999999999', `id desconhecido: ${en.itemName(999999999)}`);

// e a premissa que sustenta tudo isto: todo nome tem os dois idiomas
const faltando = Object.keys(items).filter((i) => {
  const v = items[i];
  return (v.pt || '').trim() && !(v.en || '').trim();
});
conferir(faltando.length === 0, `itens com português e sem inglês: ${faltando.length}`);
const nosFaltando = Object.keys(skills.nodes).filter((k) => {
  const n = skills.nodes[k];
  return (n.pt || '').trim() && !(n.en || '').trim();
});
conferir(nosFaltando.length === 0, `nós de habilidade com português e sem inglês: ${nosFaltando.length}`);

// a descrição do nó também acompanha o idioma, e vem num campo diferente do nome
const semDescEn = Object.values(skills.nodes).filter((n) => (n.desc || '').trim() && !(n.descEn || '').trim());
conferir(semDescEn.length === 0, `nós com descrição em português e sem inglês: ${semDescEn.length}`);
const comArpeu = Object.values(skills.nodes).find((n) => (n.desc || '').includes('Arpéu'));
for (const [campo, mod, trecho] of [['pt', pt, 'Arpéu'], ['en', en, 'Grappling Hook']]) {
  conferir(mod.descricaoDe(comArpeu).includes(trecho), `${campo}: descrição do nó em ${campo}`);
}

// conhecimento: a maioria é item ou receita de item, e aí o nome segue o idioma
const conhecimento = JSON.parse(fs.readFileSync(path.join(RAIZ, 'data', 'conhecimento.json'), 'utf8'));
const comItem = Object.keys(conhecimento).filter((k) => conhecimento[k].i !== undefined);
conferir(comItem.length > 6000, `${comItem.length} conhecimentos resolvem o nome por item`);
const umId = comItem.find((k) => items[conhecimento[k].i]?.pt !== items[conhecimento[k].i]?.en);
conferir(pt.nomeConhecimento(conhecimento[umId], umId) === items[conhecimento[umId].i].pt
  && en.nomeConhecimento(conhecimento[umId], umId) === items[conhecimento[umId].i].en,
  `conhecimento ${umId}: "${pt.nomeConhecimento(conhecimento[umId], umId)}" / "${en.nomeConhecimento(conhecimento[umId], umId)}"`);
// os que não têm item por trás continuam no idioma único em que o jogo os traz
const semItem = Object.keys(conhecimento).filter((k) => conhecimento[k].n !== undefined);
conferir(semItem.every((k) => pt.nomeConhecimento(conhecimento[k], k) === conhecimento[k].n),
  `${semItem.length} sem item por trás caem no nome gravado`);

// ------------------------------------------------------------------ guarda contra a reincidência
//
// Este bug já voltou duas vezes, sempre num arquivo novo: alguém escreve `item.pt` para pôr um nome
// na tela e aquele ponto fica em português para sempre, com a interface em inglês. Os testes acima
// não pegavam, porque só olham `gamedata.js`.
//
// A regra: uma linha que lê `.pt` sem citar `.en` na mesma linha está escolhendo um idioma, e isso
// é errado — o nome tem de sair de `itemName`, `skillName` ou `nomeDe`. Linhas que citam os dois
// estão tratando ambos (busca, reserva, o nome no outro idioma) e passam.
//
// Exceção: `attrs[...]` usa o nome em português como CHAVE da tabela de atributos, não como texto;
// trocar ali quebraria a contagem. A exibição desses rótulos passa por t().
const FONTES = ['src'];
const EXCECAO = /attrs\s*\[/;
const ehComentario = (l) => /^\s*(\/\/|\/\*|\*)/.test(l);

function* arquivos(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) yield* arquivos(p);
    else if (/\.(js|jsx)$/.test(e.name) && !p.endsWith(`${path.sep}i18n.js`)) yield p;
  }
}

const suspeitas = [];
for (const raiz of FONTES) {
  for (const arq of arquivos(path.join(RAIZ, raiz))) {
    fs.readFileSync(arq, 'utf8').split('\n').forEach((linha, i) => {
      if (ehComentario(linha) || EXCECAO.test(linha)) return;
      if (!/\.pt\b|\['pt'\]|\["pt"\]/.test(linha)) return;
      if (/\.en\b|\['en'\]|\["en"\]/.test(linha)) return; // trata os dois idiomas
      suspeitas.push(`${path.relative(RAIZ, arq)}:${i + 1}  ${linha.trim()}`);
    });
  }
}
conferir(suspeitas.length === 0, suspeitas.length === 0
  ? 'nenhum ponto do código escolhe o português direto'
  : `pontos lendo .pt direto (use itemName/skillName/nomeDe):\n     ${suspeitas.join('\n     ')}`);

console.log(`nomes por idioma: ${ok} certos, ${ruim} errados`);
process.exit(ruim ? 1 : 0);
