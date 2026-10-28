// window.api da interface: a implementação Tauri (src/tauri-api.js).
import tauriApi from './tauri-api.js';

export const api = tauriApi;
// recursos do jogo: protocolo game:// do Rust; no Tauri (Windows) o esquema vira http://game.localhost/
export const ASSET_BASE = 'http://game.localhost/';

// fontes do jogo (UiFontResource), registradas aqui porque o endereço depende da plataforma
export function registerFonts() {
  const faces = [
    ['Harmonique', 'font00.otf', 400, 'normal'], ['Harmonique', 'font04.otf', 400, 'italic'],
    ['Harmonique', 'font24.otf', 700, 'normal'], ['Harmonique', 'font08.otf', 700, 'italic'],
    ['NotoSerifGame', 'font10.ttf', 400, 'normal'], ['NotoSerifGame', 'font26.ttf', 700, 'normal'],
  ];
  const css = faces.map(([f, file, w, s]) => `@font-face { font-family: '${f}'; src: url('${ASSET_BASE}fonts/${file}'); font-weight: ${w}; font-style: ${s}; }`).join('\n');
  const el = document.createElement('style');
  el.textContent = css;
  document.head.appendChild(el);
}
