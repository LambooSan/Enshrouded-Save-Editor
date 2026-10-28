import { createRoot, html } from './react.js';
import App from './App.js';
import { registerFonts } from './platform.js';

registerFonts();

createRoot(document.getElementById('root')).render(html`<${App} />`);
