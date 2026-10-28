// Cada item guarda campos extras diferentes no save, e um byte de flags diz quais estão ali. Criar
// um item com os campos errados produz um item quebrado no jogo — joia sem nível, planador com
// durabilidade que ele não tem. Este teste confere que `newEntry` produz, para cada caso, as flags
// que os saves reais mostram.
//
// O que o teste mede não depende do idioma; mesmo assim fixa o português, como o teste antigo.
import items from '../../src/data/items.json' with { type: 'json' };
import { definirIdioma } from '../../src/i18n.js';
import { newEntry } from '../../src/gamedata.js';

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

export default async function (conferir) {
  definirIdioma('pt-BR');
  for (const [nome, esperado] of Object.entries(CASOS)) {
    const id = Object.keys(items).find((k) => items[k].pt === nome);
    if (!id) {
      conferir(false, `${nome}: nao esta no catalogo`);
      continue;
    }
    const e = newEntry(Number(id), 0, 1);
    const f = flagsDe(e);
    conferir(f === esperado, `${nome.padEnd(28)} flags 0x${f.toString(16).padStart(2, '0')}`
      + ` (esperado 0x${esperado.toString(16).padStart(2, '0')})`);
  }
}
