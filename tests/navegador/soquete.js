// A joia encaixada numa arma mora nos 5 bytes do soquete: itemId da joia (u32 little-endian) e o
// nível dela (u8). Este teste confere a leitura e a escrita contra soquetes tirados de saves reais,
// não contra a própria implementação — se o formato mudar, ele acusa.
import items from '../../src/data/items.json' with { type: 'json' };
import { leSoquete, escreveSoquete } from '../../src/soquete.js';

// soquetes lidos de saves reais, com o que significam
const CASOS = [
  ['e945776a0f', 1786201577, 15],
  ['f0a1c7180b', 415736304, 11],
  ['97c2716127', 1634845335, 39],
  ['c2be5cda37', 3663511234, 55],
  ['c2be5cda69', 3663511234, 105],
  ['0000000000', 0, 0],
];

export default async function (conferir) {
  for (const [hex, idEsperado, nvEsperado] of CASOS) {
    const lido = leSoquete(hex);
    const devolta = escreveSoquete(lido);
    const bate = lido.itemId === idEsperado && lido.level === nvEsperado && devolta === hex;
    const nome = lido.itemId ? (items[lido.itemId]?.pt ?? '?') : 'sem joia';
    conferir(bate, `${hex} -> id ${lido.itemId} nivel ${lido.level} (${nome}), volta ${devolta}`);
  }

  // montar do zero e ler de volta
  for (const [id, nv] of [[3663511234, 255], [1460033731, 1], [63026718, 90]]) {
    const hex = escreveSoquete({ itemId: id, level: nv });
    const lido = leSoquete(hex);
    conferir(lido.itemId === id && lido.level === nv, `montar id ${id} nivel ${nv} -> ${hex}`);
  }
}
