// Nome de item e de habilidade tem de acompanhar o idioma escolhido.
//
// Este teste existe porque a primeira versão da tradução não fazia isso: `itemName` devolvia
// `it.pt || it.en`, então trocar para inglês traduzia a interface e deixava todo item em português.
// Como o jogo já traz os dois nomes, o erro não era falta de tradução — era ler o campo errado.
import items from '../../src/data/items.json' with { type: 'json' };
import skills from '../../src/data/skills.json' with { type: 'json' };
import conhecimento from '../../src/data/conhecimento.json' with { type: 'json' };
import { definirIdioma } from '../../src/i18n.js';
import { itemName, skillName, descricaoDe, nomeConhecimento } from '../../src/gamedata.js';

// o gamedata lê o idioma na hora da chamada: cada função abaixo troca o idioma e chama, para medir
// os dois idiomas com o mesmo módulo
const noIdioma = (codigo) => {
  const em = (f) => (...a) => { definirIdioma(codigo); return f(...a); };
  return {
    itemName: em(itemName), skillName: em(skillName),
    descricaoDe: em(descricaoDe), nomeConhecimento: em(nomeConhecimento),
  };
};
const pt = noIdioma('pt-BR');
const en = noIdioma('en');

export default async function (conferir) {
  try {
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

    // ---------------------------------------------------------- guarda contra a reincidência
    //
    // A varredura das fontes atrás de `.pt` lido direto (sem `.en` na mesma linha) precisa listar
    // diretórios, o que o navegador não faz. Ela mora em src-tauri/tests/dados.rs.
  } finally {
    definirIdioma('pt-BR');
  }
}
