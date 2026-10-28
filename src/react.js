// React sem etapa de build: React e ReactDOM vêm dos UMD em vendor/ (carregados no index.html antes
// dos módulos) e o htm faz o papel do JSX, com a mesma árvore de elementos.
import htm from './vendor/htm.module.js';

const React = window.React;
export default React;
export const { Fragment, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } = React;
export const { createRoot } = window.ReactDOM;
export const html = htm.bind(React.createElement);
