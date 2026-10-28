// Painel de detalhes do item (moldura fancy_box do jogo), usado na Mochila e no Personagem.
import { html, Fragment } from './react.js';
import { items, itemName, CATEGORY_PT } from './gamedata.js';
import { ui, tex, ttex, itemIcon, rgba, rarityOf, rarityColor } from './ui.js';
import { GameButton, Stepper, RarityGems, UpgradePips, MeterBar, Separator, tierOf } from './components.js';
import { t, localeAtual } from './i18n.js';
import ItemStats from './ItemStatsView.js';
import { leSoquete, escreveSoquete } from './soquete.js';

export function FancyPanel({ children, style }) {
  return html`
    <div className="detail" style=${style}>
      <img className="detail-bg" src=${ttex('panel_bg')} alt="" />
      <div className="detail-pattern" style=${{ backgroundImage: `url(${tex(ui.menu.fancyBox.backgroundPattern)})` }} />
      <img className="detail-frame" src=${tex(ui.menu.fancyBox.frameFull)} alt="" />
      <div className="detail-body">${children}</div>
    </div>
  `;
}

function Row({ label, children }) {
  return html`<div className="ed-row"><div className="ed-label">${label}</div><div className="ed-ctrl">${children}</div></div>`;
}

// A joia encaixada na arma (leSoquete/escreveSoquete) fica em soquete.js.

// as joias do catálogo, para o seletor; só os ids, porque nome e ordem dependem do idioma
const JOIAS = Object.keys(items)
  .filter((id) => items[id].category === 'WeaponGem')
  .map((id) => Number(id));

export default function ItemEditor({ entry, onChange, onRemove, onDuplicate, caption }) {
  const it = items[entry.itemId];
  const rar = rarityOf(entry);
  const set = (k, v) => onChange({ ...entry, [k]: v });
  // no jogo o nível do item já inclui as melhorias (Arco Ajustado: base 11 + 5 = 16);
  // mudar as melhorias muda o nível junto, senão o jogo descarta a mudança
  const baseLevel = entry.level !== undefined ? entry.level - (entry.upgrades ?? 0) : undefined;
  const setUpgrades = (u, rarity) => onChange({
    ...entry, upgrades: u, ...(rarity !== undefined ? { maxUpgrades: rarity } : {}),
    ...(entry.level !== undefined ? { level: Math.max(0, baseLevel + u) } : {}),
  });
  const tier = tierOf(entry.itemId);
  const stack = it?.maxStack > 1;
  const joia = entry.socket !== undefined ? leSoquete(entry.socket) : null;
  const setJoia = (mudanca) => set('socket', escreveSoquete({ ...joia, ...mudanca }));
  // Nível: o nível cabe num byte, então o campo aceita até 255. O servidor corta o que passa do
  // maxLevel do item ao carregar (um 255 num arco de faixa 1-55 virou 55), a não ser que a
  // faixa daquele item tenha sido aberta por mod no servidor. Como o editor não tem como
  // saber disso, aqui a gente avisa em vez de impedir.
  // Joia encaixada: trocar, tirar, e mexer no nível dela (é o nível que define a força
  // do efeito, pela curva WeaponPowerCreep).
  return html`
    <${Fragment}>
      ${caption && html`<div className="caps small">${caption}</div>`}
      <div className="ed-head">
        <div className="ed-icon" style=${{ background: `radial-gradient(circle, ${rarityColor(rar).replace(/[\d.]+\)$/, '0.45)')} 0%, transparent 70%)` }}>
          <img src=${itemIcon(entry.itemId)} alt="" />
        </div>
        <div className="ed-title">
          <div className="detail-title" style=${{ color: rarityColor(rar) }}>${itemName(entry.itemId)}</div>
          <div className="detail-sub">
            ${t(CATEGORY_PT[it?.category] || it?.category)}${tier ? ` · ${t('Tier {n}', { n: tier })}` : ''}${stack ? ` · ${t('pilha de até {n}', { n: it.maxStack })}` : ''}
          </div>
        </div>
      </div>
      <img className="stripe" src=${ttex(`stripe_${rar}`)} alt="" />

      <div className="ed-rows">
        ${entry.maxUpgrades !== undefined && html`
          <${Row} label=${t('Raridade')}><${RarityGems} value=${entry.maxUpgrades} onChange=${(v) => setUpgrades(Math.min(entry.upgrades ?? 0, v), v)} /><//>
        `}
        ${(stack || entry.count !== 1) && html`
          <${Row} label=${t('Quantidade')}>
            <${Stepper} value=${entry.count} min=${1} max=${Math.max(it?.maxStack || 1, entry.count)} steps=${[1, 10]} onChange=${(v) => set('count', v)}
              extra=${stack && html`<${GameButton} small onClick=${() => set('count', it.maxStack)}>${t('Máx')}<//>`} />
          <//>
        `}
        ${entry.level !== undefined && html`
          <${Row} label=${t('Nível')}>
            <${Stepper} value=${entry.level} min=${entry.upgrades ?? 0} max=${255} onChange=${(v) => set('level', v)} />
            ${entry.upgrades !== undefined && html`<span className="hint">${t('= base {base} + {n} melhorias', { base: baseLevel, n: entry.upgrades })}</span>`}
            ${it?.maxLevel > 0 && entry.level > it.maxLevel && html`
              <span className="hint warn">
                ${t('acima do máximo deste item ({max}): o servidor corta na carga, a menos que a faixa dele tenha sido aberta por mod', { max: it.maxLevel })}
              </span>
            `}
          <//>
        `}
        ${entry.upgrades !== undefined && entry.maxUpgrades !== undefined && html`
          <${Row} label=${t('Melhorias')}><${UpgradePips} value=${entry.upgrades} max=${Math.max(entry.maxUpgrades, entry.upgrades)} onChange=${(v) => setUpgrades(v)} /><span className="hint">${t('nível base {base}', { base: baseLevel })}</span><//>
        `}
        ${joia && html`
          <${Fragment}>
            <${Row} label=${t('Joia encaixada')}>
              <select className="ed-select" value=${joia.itemId}
                onChange=${(e) => {
                  const id = Number(e.target.value);
                  setJoia({ itemId: id, level: id && !joia.itemId ? Math.max(1, items[id]?.minLevel || 1) : joia.level });
                }}>
                <option value=${0}>${t('— sem joia —')}</option>
                ${[...JOIAS].sort((a, b) => itemName(a).localeCompare(itemName(b), localeAtual()))
                  .map((id) => html`<option key=${id} value=${id}>${itemName(id)}</option>`)}
              </select>
              ${joia.itemId > 0 && !items[joia.itemId] && html`<span className="hint warn">${t('id {id} não está no catálogo', { id: joia.itemId })}</span>`}
            <//>
            ${joia.itemId > 0 && html`
              <${Row} label=${t('Nível da joia')}>
                <${Stepper} value=${joia.level} min=${1} max=${255} onChange=${(v) => setJoia({ level: v })} />
                ${items[joia.itemId]?.maxLevel > 0 && joia.level > items[joia.itemId].maxLevel && html`
                  <span className="hint warn">
                    ${t('acima do máximo desta joia ({max}): o servidor corta na carga, a menos que a faixa dela tenha sido aberta por mod', { max: items[joia.itemId].maxLevel })}
                  </span>
                `}
              <//>
            `}
          <//>
        `}
        ${entry.durability !== undefined && html`
          <${Row} label=${t('Durabilidade')}>
            <${MeterBar} value=${entry.durability} max=${entry.maxDurability} color=${rgba(entry.durability / entry.maxDurability < 0.25 ? ui.itemSlot.durabilityLow : ui.itemSlot.durability)} onChange=${(v) => set('durability', v)} />
            <${GameButton} small onClick=${() => set('durability', entry.maxDurability)}>${t('Reparar')}<//>
          <//>
        `}
        ${entry.liquid !== undefined && html`
          <${Row} label=${t('Água')}>
            <${MeterBar} value=${entry.liquid} max=${entry.maxLiquid} color=${rgba(ui.itemSlot.water)} onChange=${(v) => set('liquid', v)} />
            <${GameButton} small onClick=${() => set('liquid', entry.maxLiquid)}>${t('Encher')}<//>
          <//>
        `}
      </div>
      ${entry.maxUpgrades !== undefined && html`<div className="hint">${t('A raridade define quantas melhorias o item aceita: nível máximo = nível base + raridade.')}</div>`}

      <${ItemStats} entry=${entry} />

      <div className="ed-actions">
        <${Separator} />
        <div className="row-gap">
          ${onDuplicate && html`<${GameButton} onClick=${onDuplicate} title=${t('Copia o item para o primeiro espaço livre')}>${t('Duplicar')}<//>`}
          ${onRemove && html`<${GameButton} danger onClick=${onRemove} title="Delete">${t('Remover')}<//>`}
        </div>
        <div className="debug">${it?.debug} · ${t('espaço {n}', { n: entry.slot })}</div>
      </div>
    <//>
  `;
}
