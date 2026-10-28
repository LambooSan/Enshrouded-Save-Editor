// Roda um teste de tests/navegador/ (?t=<nome>) e imprime o resultado num <pre id="resultado">,
// que o src-tauri/tests/interface.rs lê do DOM. A última linha é "FIM <certos> <errados>".
const nome = new URLSearchParams(location.search).get('t');
const linhas = [];
let ok = 0;
let ruim = 0;
const conferir = (certo, descricao) => {
  if (certo) ok += 1; else ruim += 1;
  linhas.push(`${certo ? 'ok  ' : 'ERRO'} ${descricao}`);
};

try {
  const teste = await import(`./${nome}.js`);
  await teste.default(conferir);
} catch (e) {
  ruim += 1;
  linhas.push(`ERRO o teste lançou: ${e?.stack ?? e}`);
}

const pre = document.createElement('pre');
pre.id = 'resultado';
pre.textContent = `${linhas.join('\n')}\nFIM ${ok} ${ruim}`;
document.body.append(pre);
