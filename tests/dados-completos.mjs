// Os arquivos de `data/` são gerados fora deste repositório por várias ferramentas em sequência,
// e cada uma acrescenta campos aos arquivos da anterior. Rodar só uma delas reescreve o arquivo e
// apaga o que as outras tinham posto.
//
// Foi o que aconteceu com os ícones da árvore de habilidades na 1.0.1: regerar o skills.json para
// acrescentar a descrição em inglês apagou `icon` e `iconOff` dos 43 nós que os têm, e a árvore
// ficou sem ícone nenhum para quem baixou. Nada falhou — o campo só sumiu.
//
// Este teste exige que cada arquivo tenha todos os campos que a interface usa, e que toda imagem
// citada exista mesmo em build-assets.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const RAIZ = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const ler = (nome) => JSON.parse(fs.readFileSync(path.join(RAIZ, 'data', nome), 'utf8'));
const items = ler('items.json');
const skills = ler('skills.json');
const conhecimento = ler('conhecimento.json');

let ok = 0;
let ruim = 0;
const conferir = (certo, descricao) => {
  if (certo) ok += 1; else ruim += 1;
  console.log(`${certo ? 'ok  ' : 'ERRO'} ${descricao}`);
};

// ----------------------------------------------------------------- árvore de habilidades
const nos = Object.values(skills.nodes);
conferir(nos.length === 243, `${nos.length} nós na árvore`);

for (const campo of ['pt', 'en', 'desc', 'descEn', 'type', 'class', 'cost', 'maxLevel', 'x', 'y']) {
  const faltam = nos.filter((n) => n[campo] === undefined);
  conferir(faltam.length === 0, `todo nó tem "${campo}" (${faltam.length} sem)`);
}

// os ícones vêm de build_ui.py, que roda DEPOIS do build_data.py sobre o mesmo arquivo
const comIcone = nos.filter((n) => n.icon);
conferir(comIcone.length === 43, `${comIcone.length} nós com ícone (build_ui.py já rodou)`);
conferir(comIcone.every((n) => n.iconOff), 'todo nó com ícone tem também o apagado');

// e o nome do ícone tem de existir como arquivo, senão a árvore mostra imagem quebrada
const faltaArquivo = [];
for (const n of comIcone) {
  for (const nome of [n.icon, n.iconOff]) {
    if (!fs.existsSync(path.join(RAIZ, 'build-assets', 'game', 'ui', `${nome}.webp`))) {
      faltaArquivo.push(`${n.pt}: ${nome}.webp`);
    }
  }
}
conferir(faltaArquivo.length === 0,
  faltaArquivo.length === 0
    ? `as ${comIcone.length * 2} imagens de ícone existem em build-assets`
    : `imagens de ícone faltando:\n     ${faltaArquivo.slice(0, 5).join('\n     ')}`);

conferir(skills.links.length > 0, `${skills.links.length} ligações entre nós`);

// ----------------------------------------------------------------- itens
const ids = Object.keys(items);
conferir(ids.length === 4262, `${ids.length} itens`);
for (const campo of ['debug', 'pt', 'en', 'category', 'rarity', 'maxStack', 'minLevel', 'maxLevel']) {
  const faltam = ids.filter((i) => items[i][campo] === undefined);
  conferir(faltam.length === 0, `todo item tem "${campo}" (${faltam.length} sem)`);
}

// o ícone do item é montado a partir do nome de depuração
const semIcone = ids.filter((i) => !fs.existsSync(path.join(RAIZ, 'build-assets', 'game', 'icons', `${items[i].debug}_icon.webp`)));
conferir(semIcone.length < ids.length * 0.1,
  `${ids.length - semIcone.length} de ${ids.length} itens têm ícone em build-assets`);

// ----------------------------------------------------------------- conhecimento
const ck = Object.keys(conhecimento);
conferir(ck.length === 8335, `${ck.length} entradas de conhecimento`);
const semNada = ck.filter((k) => conhecimento[k].i === undefined && conhecimento[k].n === undefined && conhecimento[k].t !== 'i');
conferir(semNada.length === 0, `${semNada.length} entradas sem item nem nome`);
const apontamFora = ck.filter((k) => conhecimento[k].i !== undefined && !items[conhecimento[k].i]);
conferir(apontamFora.length === 0, `${apontamFora.length} conhecimentos apontam para item inexistente`);

console.log(`dados completos: ${ok} certos, ${ruim} errados`);
process.exit(ruim ? 1 : 0);
