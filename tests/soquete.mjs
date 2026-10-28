// A joia encaixada numa arma mora nos 5 bytes do soquete: itemId da joia (u32 little-endian) e o
// nível dela (u8). Este teste confere a leitura e a escrita contra soquetes tirados de saves reais,
// não contra a própria implementação — se o formato mudar, ele acusa.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const items = JSON.parse(fs.readFileSync(path.join(RAIZ, 'data', 'items.json'), 'utf8'));

// as funções moram no painel do item; extraímos o trecho para testar o código que roda de verdade
const fonte = fs.readFileSync(path.join(RAIZ, 'src', 'ItemEditor.jsx'), 'utf8');
const ini = fonte.indexOf('const SEM_JOIA');
const fim = fonte.indexOf('// as joias do catálogo');
if (ini < 0 || fim < 0) {
  console.log('ERRO: nao achei as funcoes do soquete no ItemEditor.jsx');
  process.exit(1);
}
const { leSoquete, escreveSoquete } = await import(
  'data:text/javascript,' + encodeURIComponent(fonte.slice(ini, fim) + '\nexport { leSoquete, escreveSoquete };')
);

// soquetes lidos de saves reais, com o que significam
const CASOS = [
  ['e945776a0f', 1786201577, 15],
  ['f0a1c7180b', 415736304, 11],
  ['97c2716127', 1634845335, 39],
  ['c2be5cda37', 3663511234, 55],
  ['c2be5cda69', 3663511234, 105],
  ['0000000000', 0, 0],
];

let ok = 0;
let ruim = 0;
for (const [hex, idEsperado, nvEsperado] of CASOS) {
  const lido = leSoquete(hex);
  const devolta = escreveSoquete(lido);
  const bate = lido.itemId === idEsperado && lido.level === nvEsperado && devolta === hex;
  if (bate) ok += 1; else ruim += 1;
  const nome = lido.itemId ? (items[lido.itemId]?.pt ?? '?') : 'sem joia';
  console.log(`${bate ? 'ok  ' : 'ERRO'} ${hex} -> id ${lido.itemId} nivel ${lido.level} (${nome}), volta ${devolta}`);
}

// montar do zero e ler de volta
for (const [id, nv] of [[3663511234, 255], [1460033731, 1], [63026718, 90]]) {
  const hex = escreveSoquete({ itemId: id, level: nv });
  const lido = leSoquete(hex);
  const bate = lido.itemId === id && lido.level === nv;
  if (bate) ok += 1; else ruim += 1;
  console.log(`${bate ? 'ok  ' : 'ERRO'} montar id ${id} nivel ${nv} -> ${hex}`);
}

console.log(`soquete: ${ok} certos, ${ruim} errados`);
process.exit(ruim ? 1 : 0);
