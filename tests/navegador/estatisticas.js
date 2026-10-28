// As linhas de atributo e as melhorias têm de sair iguais às do jogo.
//
// A referência é a captura de tela que acompanha a issue #2: o Gemini Ring, lendário, com as
// cinco melhorias aplicadas. O que o jogo mostra ali está transcrito abaixo, linha por linha, nos
// dois idiomas. Se o formato do arquivo de dados ou a montagem mudarem, isto reprova.
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

const ANEL = 1234854024; // Gemini Ring / Anel de Gêmeos — o item da captura da issue #2

const ESPERADO = {
  en: {
    atributos: [
      ['+20', 'Health'],
      ['+4', 'Stamina Regeneration'],
      ['+5%', 'Damage against Fell'],
    ],
    melhorias: [
      [0, '+1', 'Backstab Mastery', 1311628529],
      [1, '+1', 'Thrust', 2506510512],
      [2, '+1', 'Dagger Mastery', 770186136],
      [3, '+3', 'Health Leech', null],
      [4, '+1', 'Fatal Precision', 3005573120],
    ],
  },
  pt: {
    atributos: [
      ['+20', 'Saúde'],
      ['+4', 'Regeneração de Vigor'],
      ['+5%', 'Dano contra Caído'],
    ],
    melhorias: [
      [0, '+1', 'Golpe nas Costas Magistral', 1311628529],
      [1, '+1', 'Ímpeto', 2506510512],
      [2, '+1', 'Domínio de Adagas', 770186136],
      [3, '+3', 'Roubo de vida', null],
      [4, '+1', 'Precisão Fatal', 3005573120],
    ],
  },
};

export default async function (conferir) {
  try {
    for (const campo of ['pt', 'en']) {
      const m = carregar(campo);
      const esperado = ESPERADO[campo];

      const atributos = m.atributosDe(ANEL, 4);
      conferir(atributos.length === esperado.atributos.length,
        `${campo}: ${atributos.length} linhas de atributo`);
      esperado.atributos.forEach(([valor, nome], i) => {
        const veio = atributos[i] || {};
        conferir(veio.valor === valor && veio.nome === nome,
          `${campo}: atributo ${i} = ${veio.valor} ${veio.nome}`);
      });

      // as cinco melhorias, com o item em 5/5 upgrades como na captura
      const melhorias = m.melhoriasDe(ANEL, 4, 5);
      conferir(melhorias.length === esperado.melhorias.length,
        `${campo}: ${melhorias.length} melhorias`);
      esperado.melhorias.forEach(([nivel, valor, nome, no], i) => {
        const veio = melhorias[i] || {};
        const l = (veio.linhas || [])[0] || {};
        conferir(veio.nivel === nivel && l.valor === valor && l.nome === nome && veio.no === no && veio.aplicada,
          `${campo}: nível ${veio.nivel} = ${l.valor} ${l.nome}${veio.no ? ` (nó ${veio.no})` : ''}`);
      });
    }

    // com menos upgrades, só as melhorias até ali contam como aplicadas
    const m = carregar('en');
    const parciais = m.melhoriasDe(ANEL, 2, 2);
    conferir(parciais.filter((x) => x.aplicada).length === 2,
      `com 2 melhorias aplicadas, ${parciais.filter((x) => x.aplicada).length} marcadas`);

    // ------------------------------------------- armadura e melhorias, contra a captura do jogo
    //
    // Referência: captura das Botas de Andarilho do Vento das Cinzas equipadas, nível 255/255, com as
    // cinco melhorias aplicadas. O que o jogo mostra ali está transcrito aqui. Esta é a medição que
    // transformou a fórmula da armadura de hipótese em fato: 23 + 15×255 = 3848, × 0,06 das botas =
    // 230,88, e o jogo escreve 231 — e não 51, que é o que daria se houvesse corte no nível 55.
    const BOTAS = 749923180; // Armor_T7_Ranger_Archer_DPS_Boots

    {
      const mod = carregar('pt');
      const arm = mod.armaduraDe(BOTAS, 255);
      conferir(arm?.fisica === 231 && arm?.magica === 231,
        `botas em 255: armadura ${arm?.fisica}/${arm?.magica} (o jogo mostra 231/231)`);
      conferir(mod.armaduraDe(BOTAS, 45)?.fisica === 42,
        `botas no nível nominal 45: ${mod.armaduraDe(BOTAS, 45)?.fisica}`);

      // as quatro linhas de atributo, na ordem e no formato da captura
      const esperado = [
        ['+21', 'Regeneração de Vigor'],
        ['-0,5s', 'Atraso na regeneração da resistência'],
        ['1:30 m', 'Redução de Duração de Molhado'],
        ['+2', 'Resistência a gelo'],
      ];
      const linhas = mod.atributosDe(BOTAS, 255);
      conferir(linhas.length === esperado.length,
        `botas: ${linhas.length} linhas de atributo (o jogo mostra ${esperado.length}; "Nível do Item" fica no cabeçalho)`);
      esperado.forEach(([valor, nome], i) => {
        conferir(linhas[i]?.valor === valor && linhas[i]?.nome === nome,
          `botas: ${linhas[i]?.valor} ${linhas[i]?.nome}`);
      });

      // as cinco melhorias: rótulo com o NÍVEL do item naquele passo, a coluna de armadura, e as
      // melhorias de armadura calculadas no nível da própria linha
      const esperadasM = [
        [251, '+114', 'Armadura Contra Corte'],
        [252, '+114', 'Armadura Contra Perfuração'],
        [253, '+1', 'Corrida'],
        [254, '+21', 'Regeneração de Vigor'],
        [255, '+1', 'Corrida'],
      ];
      const ms = mod.melhoriasDe(BOTAS, 255, 5);
      conferir(ms.length === 5, `botas: ${ms.length} melhorias`);
      esperadasM.forEach(([nivel, valor, nome], i) => {
        const m = ms[i] || {};
        const efeito = m.armadura ? { valor: `+${m.armadura.valor}`, nome: m.armadura.nome } : (m.linhas || [])[0] || {};
        conferir(m.nivel === nivel && efeito.valor === valor && efeito.nome === nome && m.ganho?.fisica === 1,
          `botas: nível ${m.nivel} | +${m.ganho?.fisica}/+${m.ganho?.magica} | ${efeito.valor} ${efeito.nome}`);
      });
    }

    // --------------------------------------------------- duração e nível nominal
    //
    // O formato de duração vem em MILISSEGUNDOS: num print do Machado Folheado a Ouro a velocidade de
    // ataque apareceu como "774s". E a linha "Nível do Item" vinha com o nível nominal do arquivo
    // (+40 num machado de nível 11), por isso ela não vai mais para a lista.
    const MACHADO = 246967425;  // Tool_T6_Axe_Steel_Gold_Wetlands
    const ANEL_VIGOR = 4079983357; // Ring_T6_Ring_of_Stamina

    for (const [campo, velocidade, decimal] of [
      ['pt', '0,8s', 'vírgula'],
      ['en', '0.8s', 'ponto'],
    ]) {
      const mod = carregar(campo);
      const axe = mod.atributosDe(MACHADO, 11);
      const vel = axe.find((l) => /Velocidade|Attack Speed/.test(l.nome));
      conferir(vel?.valor === velocidade, `${campo}: velocidade de ataque = ${vel?.valor} (decimal com ${decimal})`);
      conferir(!axe.some((l) => /Nível do Item|Item Level/.test(l.nome)),
        `${campo}: "Nível do Item" não aparece na lista`);

      const anel = mod.atributosDe(ANEL_VIGOR, 255);
      const seco = anel.find((l) => /Molhado|Wet/.test(l.nome));
      conferir(seco?.valor === '1:00 m', `${campo}: 60000 ms viram ${seco?.valor}`);
      // e o que NÃO escala continua sem escalar, mesmo com o item em 255: os valores do jogo são
      // autorais por tier (a família vai de +4 no nível 5 a +10 no 45), não calculados do nível
      const vigor = anel.find((l) => /^Vigor$|^Stamina$/.test(l.nome));
      conferir(vigor?.valor === '+9', `${campo}: anel em 255 continua ${vigor?.valor} Vigor, como no jogo`);
    }

    // ------------------------------------------- quantas melhorias o item mostra é a raridade
    //
    // O vetor do jogo tem sempre cinco posições, mas a raridade diz quantas o item aceita (nível
    // máximo = nível base + raridade). Mostrar as cinco em todo item fazia um Comum anunciar
    // melhorias de Lendário, que nunca iam acontecer. Quem corta usa `pos`, a posição no vetor
    // original — a lista devolvida descarta as posições vazias, então o índice dela não serve.
    {
      const mod = carregar('pt');
      const CALCAS = 9501187; // Armor_T7_Ranger_Archer_DPS_Legs
      const porRaridade = (r, upgrades = 0) => mod.melhoriasDe(CALCAS, 56 + upgrades, upgrades).filter((x) => x.pos < r);

      for (const [raridade, quantas] of [[1, 1], [3, 3], [5, 5]]) {
        conferir(porRaridade(raridade).length === quantas,
          `raridade ${raridade}: ${porRaridade(raridade).length} melhorias`);
      }
      // as posições têm de ser as do vetor original, em ordem e sem buraco neste item
      conferir(porRaridade(5).every((x, i) => x.pos === i), 'as posições seguem o vetor do jogo');

      // com 3 aplicadas, as três primeiras vêm marcadas e as demais não
      const r3 = porRaridade(5, 3);
      conferir(r3.filter((x) => x.aplicada).length === 3 && r3.slice(0, 3).every((x) => x.aplicada),
        `com 3 melhorias, ${r3.filter((x) => x.aplicada).length} marcadas e as três primeiras`);
    }

    // item sem nada a mostrar não inventa seção
    conferir(m.temEstatisticas(999999999) === false, 'id desconhecido não tem estatísticas');
    conferir(m.atributosDe(999999999, 1).length === 0, 'id desconhecido não produz linhas');

    // nenhuma linha pode sair com número inventado: valor null é o que marca "depende do nível"
    const escaladas = Object.entries(stats.itens)
      .filter(([, v]) => (v.a || []).some((l) => l.e));
    conferir(escaladas.every(([id]) => m.atributosDe(Number(id), 40).some((l) => l.valor === null || !l.variaComNivel)),
      `${escaladas.length} itens com linha que escala, nenhum com número inventado`);
  } finally {
    definirIdioma('pt-BR');
  }
}
