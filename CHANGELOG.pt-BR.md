# Mudanças

*Read this in [English](CHANGELOG.md).*

## 1.0.2 — 25/10/2026

- **Os ícones da árvore de habilidades voltaram.** A 1.0.1 saiu com um `data/skills.json` regerado
  por só uma das ferramentas que o constroem, e o passo que acrescenta os ícones não rodou — os 43
  nós que têm um perderam, e a árvore ficou sem nenhum. Nada falhou; o campo simplesmente sumiu.
- Um teste passou a exigir que cada arquivo de `data/` tenha todos os campos que a interface lê, e
  que toda imagem citada por eles exista em `build-assets`.

## 1.0.1 — 24/10/2026

Primeira rodada de correções depois do lançamento, puxada por duas issues e muita captura de
tela do jogo.

### O que o item mostra ([#2](../../issues/2))

Os itens agora mostram **o que de fato fazem**: as linhas de atributo e as melhorias por nível,
lidas dos recursos do próprio jogo e dispostas como o jogo dispõe.

- **Armadura** é calculada e mostrada por peça. A fórmula vem de um comentário que o próprio
  estúdio compilou no jogo (`Base Armor = offset + ( gradient * item level )`), com as duas
  constantes saindo da mesma tabela, e o resultado foi conferido contra o jogo numa peça vestida:
  o valor calculado e o da tela batem.
- **Dano de arma** sai de `seed × dmgMod × speedMod × (1 + 0,15 × nível)`, e com ele as melhorias
  que são uma fração dele — fração que o jogo escreve na tela ("10% de dano base"). Conferido num
  par de adagas de nível 50: o jogo mostra +5.5, aqui mostra +5.5.
- Melhorias que dão **armadura ou dano de um tipo** aparecem com o seu valor. As de armadura são
  calculadas no nível da própria linha, que é como o jogo faz.
- Cada linha de melhoria é rotulada com o **nível do item** naquele passo (251, 252, …), e não
  com a posição dela.
- Quantas linhas de melhoria aparecem é a **raridade** do item. Um Comum não anuncia mais as
  cinco melhorias de um Lendário.

### Limites conhecidos

- **Arco, cajado e varinha mostram menos que o jogo.** A distribuição de dano deles é zero: não
  carregam dano, ele vem do projétil. O número mostrado é só a parte que vem da arma, então um
  arco marca 21,6 onde o jogo marca 21,5. Arma de corpo a corpo está exata.
- **Os atributos na tela Personagem ficam abaixo dos do jogo.** Eles contam o que a árvore dá. O
  jogo soma ainda os nós que dependem do nível da Chama e os bônus do equipamento, e nenhum dos
  dois está no save do personagem. A tela avisa.

### Idioma ([#1](../../issues/1))

Nome de item e de habilidade segue o idioma escolhido, em todo lugar. Antes seguia em alguns e
em outros não: a dica da árvore de habilidades, o catálogo de itens e a ordem dele, o seletor de
joias, o aviso da mochila, o relatório de recusa e as mensagens de validação da árvore ficavam em
português com a interface em inglês. A busca sempre funcionou, porque compara contra os dois
idiomas — que foi exatamente o que o relato dizia.

Os **conhecimentos** também seguem o idioma: 6784 dos 8335 são um item ou a receita de um item, e
o nome deles agora sai da tabela de itens em vez de vir gravado em português. O resto são ações do
jogo sem item por trás, e para essas o jogo traz um nome só.

A **descrição** dos nós de habilidade também passou a ser traduzida — o jogo traz o texto dele em
15 idiomas e o dado só carregava o português.

Agora um teste varre o código-fonte e reprova qualquer linha que leia o campo português direto.

### Correções

- **Durações** vêm em milissegundos e o jogo troca de unidade pelo tamanho: a velocidade de
  ataque de uma arma marcava `774s` e agora marca `0,8s`; um efeito de 90 segundos marca
  `1:30 m`.
- A linha **"Nível do Item"** saiu da lista de atributos. O jogo a mantém no cabeçalho e não a
  repete, e o valor gravado no arquivo é o nível *nominal* — então um machado de nível 11
  anunciava 40.
- O boneco de equipamento estava com as **colunas de armadura e aparência trocadas**. A armadura
  vestida ocupa os bits 5 a 9 da máscara, que estavam rotulados "(2)" — por isso a coluna da
  esquerda parecia vazia e a da direita cheia.
- O painel de detalhes empurrava **Duplicar e Remover para fora**; agora ele rola, como o tooltip
  do próprio jogo.
- A janela abre em **1920×1080** (era 1600×900).

### Por dentro

- `data/item-stats.json`: linhas de atributo, melhorias e distribuições de dano e armadura de
  1986 itens, mais as constantes de balanceamento (~700 KB).
- `tests/todas-as-armas.mjs` varre os 1986 itens em três níveis e nos dois idiomas, reprovando
  qualquer linha sem rótulo, com número inválido ou sem nada a mostrar. É o teste que teria pego
  as linhas de melhoria em branco nos arcos.
- 114 conferências em 6 conjuntos.

## 1.0.0 — 24/10/2026

Primeiro lançamento público.
