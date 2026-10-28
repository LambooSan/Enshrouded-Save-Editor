// Varre TODOS os itens do jogo e exige que nada apareça quebrado na tela.
//
// Existe porque um bug passou pelos outros testes: o gerador extraía as melhorias de dano
// (`damageModifier` do Perk) mas o editor não as montava, e três das cinco linhas do Arco da
// Fênix saíam em branco. Os testes olhavam itens escolhidos a dedo; este olha todos.
//
// O que é possível conferir sem o jogo aberto é consistência, não valor: nenhuma linha sem
// rótulo, nenhum número inválido, nenhuma melhoria vazia. Os valores de verdade estão travados
// em tests/navegador/estatisticas.js, contra capturas do jogo.
import items from '../../src/data/items.json' with { type: 'json' };
import stats from '../../src/data/item-stats.json' with { type: 'json' };
import { definirIdioma } from '../../src/i18n.js';
import * as itemstats from '../../src/itemstats.js';

// o itemstats.js lê o idioma na hora da chamada: isto devolve o módulo com cada função trocando
// para o idioma pedido antes de rodar, para medir os dois idiomas com o mesmo código
function carregar(campo) {
  const codigo = campo === 'pt' ? 'pt-BR' : 'en';
  return Object.fromEntries(Object.entries(itemstats).map(([k, v]) => [k,
    typeof v === 'function' ? (...a) => { definirIdioma(codigo); return v(...a); } : v]));
}

const naoResolvido = (s) => !s || /^#\d+$/.test(s) || /^g?\d+$/.test(s);
const numeroRuim = (v) => v !== null && (v === '' || /NaN|undefined|Infinity/.test(String(v)));

export default async function (conferir) {
  try {
    for (const campo of ['pt', 'en']) {
      const m = carregar(campo);
      const ids = Object.keys(stats.itens).map(Number);

      // cada item é medido em três níveis: o mínimo dele, o nominal e um exagero que o editor permite
      const problemas = { semRotulo: [], numero: [], melhoriaVazia: [], danoNulo: [] };

      for (const id of ids) {
        const reg = stats.itens[id];
        const [minNivel] = reg.nivel || [1];
        for (const nivel of [Math.max(1, minNivel), minNivel + 5, 255]) {
          for (const l of m.atributosDe(id, nivel)) {
            if (naoResolvido(l.nome)) problemas.semRotulo.push(`${id} atributo "${l.nome}"`);
            if (numeroRuim(l.valor)) problemas.numero.push(`${id} atributo ${l.valor}`);
          }

          // a arma tem de produzir dano, e o dano tem de ter nome e número
          if (reg.d) {
            const soma = Object.entries(reg.d.dist || {})
              .filter(([, v]) => v > 0 && v <= 1).reduce((s, [, v]) => s + v, 0);
            const d = m.danoDe(id, nivel);
            if (soma > 0 && !d) problemas.danoNulo.push(`${id} ${items[id]?.debug}`);
            for (const x of d || []) {
              if (naoResolvido(x.nome)) problemas.semRotulo.push(`${id} dano "${x.nome}"`);
              if (!Number.isFinite(x.valor)) problemas.numero.push(`${id} dano ${x.valor}`);
            }
          }

          // nenhuma melhoria pode sair sem nada a mostrar: foi o bug do Arco da Fênix
          for (const mel of m.melhoriasDe(id, nivel, 5)) {
            const temAlgo = mel.linhas.length || mel.armadura || mel.dano || mel.no !== null;
            if (!temAlgo) problemas.melhoriaVazia.push(`${id} ${items[id]?.debug} nível ${mel.nivel}`);
            for (const l of mel.linhas) {
              if (naoResolvido(l.nome)) problemas.semRotulo.push(`${id} melhoria "${l.nome}"`);
              if (numeroRuim(l.valor)) problemas.numero.push(`${id} melhoria ${l.valor}`);
            }
            if (mel.armadura && naoResolvido(mel.armadura.nome)) problemas.semRotulo.push(`${id} armadura sem rótulo`);
            if (mel.dano && naoResolvido(mel.dano.nome)) problemas.semRotulo.push(`${id} dano de melhoria sem rótulo`);
          }
        }
      }

      const amostra = (l) => (l.length ? `\n     ${[...new Set(l)].slice(0, 5).join('\n     ')}` : '');
      conferir(problemas.semRotulo.length === 0, `${campo}: ${problemas.semRotulo.length} linhas sem rótulo${amostra(problemas.semRotulo)}`);
      conferir(problemas.numero.length === 0, `${campo}: ${problemas.numero.length} números inválidos${amostra(problemas.numero)}`);
      conferir(problemas.melhoriaVazia.length === 0, `${campo}: ${problemas.melhoriaVazia.length} melhorias sem nada a mostrar${amostra(problemas.melhoriaVazia)}`);
      conferir(problemas.danoNulo.length === 0, `${campo}: ${problemas.danoNulo.length} armas com distribuição e sem dano${amostra(problemas.danoNulo)}`);
    }

    // --------------------------------------------------- o que se espera do conjunto
    const m = carregar('pt');
    const armas = Object.keys(stats.itens).map(Number).filter((id) => stats.itens[id].d);
    conferir(armas.length === 675, `${armas.length} itens com dano`);

    // A distribuição de dano separa os itens em três famílias, e isso não é detalhe: é o que
    // explica o arco. Quem entrega o golpe soma 1; arco, cajado e varinha somam 0 porque o dano vem
    // do projétil; e a munição soma de 2 a 4, que é ela carregando o dano.
    const soma = (id) => Object.entries(stats.itens[id].d.dist || {})
      .filter(([, v]) => v > 0 && v <= 1).reduce((a, [, v]) => a + v, 0);
    const comDist = armas.filter((id) => Math.abs(soma(id) - 1) < 0.01);
    conferir(comDist.length === 456, `${comDist.length} armas carregam o próprio dano (soma 1)`);
    const semDist = armas.filter((id) => soma(id) === 0);
    conferir(semDist.length === 129, `${semDist.length} não carregam dano próprio (arco, cajado, varinha)`);
    // Acima de 1 o item carrega mais de um tipo de dano a valor cheio: quase toda munição, e dez
    // armas especiais — tocha, punhos, granada e as três iniciais, que batem de corte e de impacto.
    const municao = armas.filter((id) => soma(id) > 1.01);
    const ammo = municao.filter((id) => /Ammo_/.test(items[id]?.debug || ''));
    conferir(municao.length === 83 && ammo.length === 73,
      `${municao.length} somam mais de 1 (${ammo.length} de munição, ${municao.length - ammo.length} armas especiais)`);
    conferir(comDist.every((id) => {
      const a = m.danoBase(id, 10);
      const b = m.danoBase(id, 40);
      return a > 0 && b > a;
    }), 'em todas elas o dano é positivo e cresce com o nível');

    // os arcos são o caso conhecido sem dano próprio: a distribuição deles é zero
    const arcos = armas.filter((id) => /Bow/i.test(items[id]?.debug || ''));
    const arcosSemDist = arcos.filter((id) => {
      const s = Object.values(stats.itens[id].d.dist || {}).filter((v) => v > 0 && v <= 1).reduce((a, v) => a + v, 0);
      return s === 0;
    });
    conferir(arcos.length > 0 && arcosSemDist.length === arcos.length,
      `os ${arcos.length} arcos não têm dano próprio (vem da flecha), e isso continua verdade`);
  } finally {
    definirIdioma('pt-BR');
  }
}
