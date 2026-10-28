// Conhecimento do personagem: pares (knowledgeId -> estado), onde o estado é um conjunto de bits.
// A leitura e a escrita do formato ficam no Rust (src-tauri/src/formato/conhecimento.rs).
//
// Bits do estado: item -> 1 pegou, 2 usou, 4 inspecionou; receita -> 1 vista, 2 fabricada.
const BITS_ITEM = [[1, 'pegou'], [2, 'usou'], [4, 'inspecionou']];
const BITS_RECEITA = [[1, 'vista'], [2, 'fabricada']];

export function estadoTexto(valor, tipo) {
  const bits = tipo === 'r' ? BITS_RECEITA : BITS_ITEM;
  const nomes = bits.filter(([b]) => valor & b).map(([, n]) => n);
  return nomes.length ? nomes.join('+') : String(valor);
}
