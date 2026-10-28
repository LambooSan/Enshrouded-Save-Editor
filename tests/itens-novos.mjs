// Cada item guarda campos extras diferentes no save, e um byte de flags diz quais estão ali. Criar
// um item com os campos errados produz um item quebrado no jogo — joia sem nível, planador com
// durabilidade que ele não tem. Este teste confere que `newEntry` produz, para cada caso, as flags
// que os saves reais mostram.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const items = JSON.parse(fs.readFileSync(path.join(RAIZ, 'data', 'items.json'), 'utf8'));

// o gamedata.js importa JSON do jeito que o vite entende; injetamos os dados e carregamos o mesmo
// código-fonte, para o teste valer sobre o que roda de verdade. O i18n também é injetado: ele toca
// localStorage e navigator, que não existem aqui, e o que o teste mede não depende do idioma.
const fonte = fs.readFileSync(path.join(RAIZ, 'src', 'gamedata.js'), 'utf8')
  // o `[^}]*` cobre esse import crescer com mais nomes; preso em `{ campoDeNome }`, acrescentar um
  // nome ali matava este teste com ERR_MODULE_NOT_FOUND em vez de falhar dizendo o motivo
  .replace(/^import\s+\{[^}]*\}\s+from\s+'\.\/i18n\.js';$/gm,
    "const campoDeNome = () => 'pt';\nconst localeAtual = () => 'pt-BR';")
  .replace(/^import\s+(\w+)\s+from\s+'\.\.\/data\/([\w.-]+)\.json';$/gm,
    (_, nome, arq) => `const ${nome} = ${fs.readFileSync(path.join(RAIZ, 'data', `${arq}.json`), 'utf8')};`);
const tmp = path.join(RAIZ, `.gamedata-teste-${process.pid}.mjs`);
fs.writeFileSync(tmp, fonte);
let newEntry;
try {
  ({ newEntry } = await import(pathToFileURL(tmp).href));
} finally {
  fs.unlinkSync(tmp);
}

// mesma conta que o construtor do save faz: cada campo presente acende um bit
const flagsDe = (e) => (e.level !== undefined ? 0x01 : 0) | (e.durability !== undefined ? 0x02 : 0)
  | (e.liquid !== undefined ? 0x04 : 0) | (e.appearance !== undefined ? 0x08 : 0)
  | (e.upgrades !== undefined ? 0x10 : 0) | (e.socket !== undefined ? 0x20 : 0);

// nome do item => flags observadas em saves reais
const CASOS = {
  'Joia da Queimadura': 0x01,
  'Joia da Mania de Batalha': 0x01,
  'Joia das Correntezas': 0x01,
  'Picareta de Aço': 0x13,
  Ancinho: 0x03,
  'Baú Enorme': 0x00,
  'Poço de Água': 0x00,
  'Planador Fantasma': 0x00,
  'Arpéu Melhorado': 0x00,
  'Mochila grande': 0x00,
  "Bolsa d'Água": 0x04,
  'Flecha Ardente Eterna I': 0x11,
  'Cura em Cadeia Eterna II': 0x11,
  'Machado de Aço': 0x13,
  'Anel de Vigor Excepcional': 0x11,
  'Protetor Prismático': 0x13,
};

let ok = 0;
let ruim = 0;
for (const [nome, esperado] of Object.entries(CASOS)) {
  const id = Object.keys(items).find((k) => items[k].pt === nome);
  if (!id) {
    console.log(`ERRO ${nome}: nao esta no catalogo`);
    ruim += 1;
    continue;
  }
  const e = newEntry(Number(id), 0, 1);
  const f = flagsDe(e);
  const bate = f === esperado;
  if (bate) ok += 1; else ruim += 1;
  console.log(`${bate ? 'ok  ' : 'ERRO'} ${nome.padEnd(28)} flags 0x${f.toString(16).padStart(2, '0')}`
    + ` (esperado 0x${esperado.toString(16).padStart(2, '0')})`);
}

console.log(`itens novos: ${ok} certos, ${ruim} errados`);
process.exit(ruim ? 1 : 0);
