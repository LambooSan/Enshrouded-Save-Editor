// Soquete de joia das armas: leitura e escrita dos 5 bytes, sem dependência de interface.

// A joia encaixada na arma mora nos 5 bytes do soquete: itemId da joia (u32 little-endian) e o
// nível dela (u8) — a mesma forma de uma entrada de item. Soquete zerado é arma sem joia.
const SEM_JOIA = '0000000000';

function leSoquete(hex) {
  if (!hex || hex.length !== 10 || hex === SEM_JOIA) return { itemId: 0, level: 0 };
  const b = hex.match(/../g).map((x) => parseInt(x, 16));
  return { itemId: ((b[3] << 24) | (b[2] << 16) | (b[1] << 8) | b[0]) >>> 0, level: b[4] };
}

function escreveSoquete({ itemId, level }) {
  if (!itemId) return SEM_JOIA;
  const b = [itemId & 0xff, (itemId >>> 8) & 0xff, (itemId >>> 16) & 0xff, (itemId >>> 24) & 0xff, level & 0xff];
  return b.map((x) => ('0' + x.toString(16)).slice(-2)).join('');
}

export { SEM_JOIA, leSoquete, escreveSoquete };
