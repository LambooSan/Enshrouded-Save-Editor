<p align="center">
  <img src=".github/capa.png" alt="Enshrouded Save Editor" width="100%">
</p>

<h1 align="center">Enshrouded Save Editor</h1>

<p align="center">
  <b>Edite o seu personagem do Enshrouded com uma interface que parece o próprio jogo.</b><br>
  Mochila, equipamento, árvore de habilidades, joias e conhecimento — nos saves locais ou no servidor dedicado.
</p>

<p align="center">
  <a href="../../releases/latest"><img src="https://img.shields.io/github/v/release/nicolasferreiraxn/Enshrouded-Save-Editor?style=for-the-badge&label=baixar&color=d4a537" alt="Baixar"></a>
  <a href="../../actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/nicolasferreiraxn/Enshrouded-Save-Editor/build.yml?style=for-the-badge&label=build" alt="Build"></a>
  <img src="https://img.shields.io/badge/Enshrouded-1.0-8b3fd1?style=for-the-badge" alt="Enshrouded 1.0">
  <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-2b6cb0?style=for-the-badge&logo=windows" alt="Windows">
  <a href="LICENSE"><img src="https://img.shields.io/badge/licença-MIT-3c9d5d?style=for-the-badge" alt="MIT"></a>
</p>

<p align="center">
  🇧🇷 Português · 🇺🇸 <a href="README.md">English</a>
</p>

---


**Mudanças:** veja o [changelog](CHANGELOG.pt-BR.md).

## ✨ O que ele faz

| | |
|---|---|
| 🧙 **Personagem** | nível e xp; a árvore de habilidades **de verdade**, com o traçado e as posições do jogo — e ele se recusa a gravar uma árvore que o jogo rejeitaria (o que faria o jogo reembolsar todos os pontos) |
| 🎒 **Mochila** | os 40 espaços, a barra ALT e a barra de ação no lugar certo; o contêiner da mochila vestida, com o tamanho real; catálogo com todos os itens do jogo, arrastar para mover, duplicar, remover; filtros por categoria, raridade e tier |
| 🛡️ **Equipamento** | o boneco com os 16 espaços (armadura, aparência, anéis, protetor, gancho, planador, mochila) — e ele impede tirar uma mochila que ainda tem coisa dentro |
| 💎 **Qualquer item em detalhe** | nível, quantidade, durabilidade, melhorias, raridade e **a joia encaixada na arma** (trocar, tirar, mudar o nível) |
| ⚔️ **O que o item faz** | as linhas de atributo e as melhorias por nível, lidas dos recursos do próprio jogo e mostradas como o jogo mostra |
| 📜 **Conhecimento** | descobertas e receitas, com busca por nome ou id e filtro por nível |
| 🏰 **Servidor dedicado** | com o [Enshrouded Server Guardian](https://github.com/nicolasferreiraxn/Enshrouded-Server-Guardian), as cópias de personagem do servidor aparecem na mesma lista — edite, expulse o jogador, e ele volta com a sua versão |
| 💾 **Backup sempre** | backup antes de cada gravação, restaurável de dentro do aplicativo |
| 🌍 **Português e inglês** | troca na barra de baixo; acrescentar um idioma é um arquivo JSON |

Todo item novo nasce com os campos extras que aquele tipo de item realmente guarda, medidos em saves reais — por
isso o que você cria no editor funciona no jogo.

## 🚀 Começando

1. Baixe o **`enshrouded-save-editor.exe`** na página de **[Releases](../../releases/latest)**.
2. Rode. Não precisa instalar nada — tudo vai dentro do executável.
3. Escolha o seu save e edite. **Ctrl+S** grava (com backup antes).

> ⚠️ Edite com o **jogo fechado**. O jogo regrava o save enquanto está aberto.

## 🏰 Com ou sem servidor

**Sem o guardião**, ele abre os `.ksd` que o seu jogo grava localmente.

**Com o guardião**, as cópias guardadas no servidor aparecem junto dos saves locais:

1. Edite a cópia de um jogador e salve.
2. Expulse o jogador (ou peça para ele relogar).
3. Na entrada seguinte o guardião impõe a cópia guardada — ele volta com a sua versão.

O editor também lê a pasta `recusados/` do guardião e explica, em palavras, o que um save recusado trazia:
diferença de nível e xp, itens a mais ou a menos, itens que só trocaram de lugar e descobertas que divergem.

<details>
<summary><b>Configurando com o guardião, passo a passo</b></summary>
<br>

**1. No servidor.** Instale o guardião: pare o servidor, ponha o `dbghelp.dll` ao lado do `enshrouded_server.exe`
e suba o servidor. **Peça para cada jogador entrar uma vez** — o guardião só tem a cópia de alguém depois de ver o
save dele. Para confirmar, procure a pasta `guardiao-dados/` com uma subpasta por Steam ID.

**2. Dados de SFTP.** No painel do seu host (costuma ser *File Manager → SFTP Details*): endereço, porta, usuário e
senha. A porta **não** é 22 na maioria dos hosts de jogo — na Nodecraft, por exemplo, é 2022. Se você hospeda em
casa, é o SSH/SFTP da própria máquina.

**3. No editor.** Aba **Servidor → Configurar servidor**:

| campo | o que vai nele |
|---|---|
| **Endereço** | o endereço do servidor, sem `sftp://` |
| **Porta (SFTP)** | a do painel; 2022 em muitos hosts de jogo |
| **Usuário** | o usuário do SFTP (muitas vezes não é o login do painel) |
| **Senha** | a senha do SFTP — fica guardada só na sua máquina |
| **Chave do servidor** | deixe vazio na primeira vez |
| **Pasta do guardião** | `guardiao-dados`, a não ser que você tenha mudado |

**Salvar → Testar conexão.** Ele informa o modo do guardião: `aplicar` (impondo a cópia) ou `observar` (só
registrando). Depois **Fixar chave vista → Salvar**, para que uma chave SSH diferente no futuro derrube a conexão em
vez de ser aceita em silêncio.

**4. Editando um jogador.** Escolha o jogador, edite, **Ctrl+S**, e expulse o jogador ou peça para ele relogar.

**Antes de depender disso:**
- Jogador conectado sobrescreve a cópia a cada ~30 s. O editor avisa quando a cópia muda e não descarta a sua
  edição — mas o momento confiável para editar é com o jogador fora.
- Backups de cada gravação vão para `Documentos\Enshrouded-Backups\servidor\`.
- A senha fica em texto puro em `%APPDATA%\enshrouded-save-editor\servidor.json`, como na maioria dos programas de
  desktop. Se isso te incomoda, use uma conta de SFTP restrita àquele servidor.
</details>

## 🌍 Traduzindo

A interface vem em português e inglês. Acrescentar um idioma é um arquivo, e a chave da tradução **é o próprio
texto em português** — então o que faltar traduzir aparece em português, e nada quebra.

1. Copie `src/i18n/en.json` para `src/i18n/<código>.json` (`es.json`, `de.json`…).
2. Traduza os **valores**, sem mexer nas chaves, e mantenha os `{marcadores}` (podem mudar de lugar na frase).
3. Registre em `IDIOMAS`, no começo de [`src/i18n.js`](src/i18n.js):

```js
import es from './i18n/es.json';

export const IDIOMAS = [
  { codigo: 'pt-BR', nome: 'Português', locale: 'pt-BR', dicionario: null },
  { codigo: 'en', nome: 'English', locale: 'en-US', dicionario: en },
  { codigo: 'es', nome: 'Español', locale: 'es-ES', dicionario: es },
];
```

Nomes de item e de habilidade vêm da localização do próprio jogo (`data/items.json`, em português e inglês).

## 🛠️ Compilando

Os releases são compilados pelo **GitHub Actions** a partir deste repositório, com
[log público](../../actions/workflows/build.yml).

Localmente, com Node 20+ e Rust (`x86_64-pc-windows-msvc`):

```
npm ci
npm run dist        # src-tauri/target/release/enshrouded-save-editor.exe
```

Sem pipeline de recursos, sem Python: os recursos da interface já vêm prontos em `build-assets/`. Para desenvolver,
`npm run dev`. Para testar, `npm test` — inclui uma ida e volta que exige o save remontado **byte a byte idêntico**
ao original. Sozinho ele roda num bloco sintético que cobre toda combinação de campos extras de um item, o que
confere o leitor contra o escritor. Provar o formato em si exige saves reais:
`node tests/ida-e-volta.mjs <pasta com A.bin>`.

<details>
<summary><b>Notas do formato do save</b></summary>
<br>

O formato não é documentado em lugar nenhum; tudo aqui saiu de medição.

- **A lista de equipamento é densa e indexada por máscara de bits.** A peça `k` ocupa o k-ésimo bit aceso numa
  máscara de 32 bits, seguida de um u16 de espaços livres.
- **A flag `0x08` é aparência (4 bytes).** Sem conhecê-la, o parser "achava" o equipamento no meio da mochila e
  gravava um save quebrado.
- **O nível de um item já inclui as melhorias.** Mudar melhoria sem mudar nível faz o jogo descartar a edição.
- **A joia encaixada são 5 bytes:** itemId da joia (u32) + nível (u8).
- **A mochila vestida define o tamanho do contêiner extra**, achada pelo tipo do item, não pelo índice.
- **Conhecimento** é um cabeçalho de 8 bytes (`containerState`, `n`), depois `n` ids e `n` valores. Receita não fica
  gravada como "liberada": o jogo a recalcula a partir das descobertas.
</details>

<details>
<summary><b>Dados do jogo</b></summary>
<br>

| pasta | o que é |
|---|---|
| `data/*.json` | tabela de itens, árvore de habilidades, mapa de conhecimento, campos extras por tipo de item e as linhas de atributo e melhorias de cada item (~2,5 MB) |
| `build-assets/game/` | ícones, texturas de interface e fontes, reduzidos ao que é usado (~26 MB, embutidos no exe) |

Preparados a partir de uma cópia instalada do jogo. Regerar não faz parte deste repositório: depois de uma
atualização que mude ids ou nomes, os dados ficam velhos até alguém regerar.
</details>

## 🧩 Da mesma família

| | |
|---|---|
| 🗺️ **[Enshrouded Interactive Map](https://github.com/nicolasferreiraxn/Enshrouded-Interactive-Map)** | mapa em tempo real no segundo monitor: sua posição, amigos e tudo o que já foi feito |
| 🛡️ **[Enshrouded Server Guardian](https://github.com/nicolasferreiraxn/Enshrouded-Server-Guardian)** | faz o servidor dedicado ser a autoridade sobre o personagem de cada jogador |

## ❤️ Créditos

- **[kfc-parser / Enshrouded Mod Loader](https://github.com/Brabb3l/kfc-parser)**, do brabb3l — usado para ler os
  recursos do jogo.
- **Keen Games** — pelo Enshrouded.

---

<p align="center">
  <sub>
    Projeto de fã, sem vínculo com a Keen Games. Sem suporte oficial — use, leia, modifique e publique à vontade.<br>
    Código sob <a href="LICENSE">MIT</a>.
  </sub>
</p>
