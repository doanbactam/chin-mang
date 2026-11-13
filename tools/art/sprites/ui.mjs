// UI 9-slice frames (SPEC §7, §8.3). Warm "cat café" theme: wooden frames, cream paper, carton tabs.
// Every frame is built from concentric rings (outside → in) so the 4 px border (2 px for bars) holds all
// the detail and the centre is flat or a tileable 2 px pattern. Anchor top-left; `slice` from manifest.
import { Px } from '../pixel.mjs';
import { dots } from './_lib.mjs';

// rings: array of [topLeftColour, bottomRightColour] from the outer edge inwards; fill(x,y) for the centre.
// round: corner cut radius (1 or 2). openBottom: tabs merge into the panel below.
function frame(w, h, rings, fill, { round = 1, openBottom = false } = {}) {
  const p = new Px(w, h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const dx = Math.min(x, w - 1 - x), dy = openBottom ? y : Math.min(y, h - 1 - y);
    if (round >= 1 && dx === 0 && dy === 0) continue;
    if (round >= 2 && ((dx === 1 && dy === 0) || (dx === 0 && dy === 1))) continue;
    let d = Math.min(dx, dy);
    if (round >= 2 && dx === 1 && dy === 1) d = 0;
    let c;
    if (d < rings.length) {
      const [tl, br] = rings[d];
      const bottomRight = (dy === d && (openBottom ? false : y > h / 2)) || (dx === d && x > w / 2);
      c = bottomRight ? br : tl;
    } else c = fill(x, y);
    p.set(x, y, c);
  }
  return p;
}
const flat = (c) => () => c;

const panel = () => frame(16, 16, [['k', 'k'], ['h', 'g'], ['g', 'f'], ['i', 'i']], flat('W'), { round: 2 });
// dark translucent plate with a 2 px dot grid (centre tiles cleanly)
const plate = () => frame(12, 12, [['Q', 'Q'], ['P', 'P'], ['P', 'P'], ['P', 'P']], (x, y) => (x % 2 === 0 && y % 2 === 0 ? 'Q' : 'P'), { round: 1 });
function button(state) {
  const R = {
    normal: [['k', 'k'], ['i', 'f'], ['h', 'g'], ['h', 'h']],
    hover: [['k', 'k'], ['w', 'g'], ['i', 'h'], ['i', 'i']],
    press: [['k', 'k'], ['f', 'h'], ['g', 'g'], ['g', 'g']],
    disabled: [['s', 's'], ['u', 's'], ['t', 't'], ['t', 't']],
  }[state];
  const fill = { normal: 'h', hover: 'i', press: 'g', disabled: 't' }[state];
  const p = frame(12, 12, R, flat(fill), { round: 2 });
  // 3D lip: thicker shadow along the bottom (pressed = lip gone, content sits 1 px lower)
  if (state === 'normal') for (let x = 2; x < 10; x++) p.set(x, 10, 'f');
  if (state === 'hover') { for (let x = 2; x < 10; x++) p.set(x, 10, 'g'); dots(p, [[2, 2, 'w'], [3, 2, 'w']]); }
  if (state === 'press') for (let x = 2; x < 10; x++) p.set(x, 1, 'f');
  if (state === 'disabled') for (let x = 2; x < 10; x++) p.set(x, 10, 's');
  return p;
}
function tab(active) {
  const R = active ? [['k', 'k'], ['w', 'i'], ['W', 'W'], ['W', 'W']] : [['k', 'k'], ['i', 'g'], ['h', 'h'], ['h', 'h']];
  const p = frame(12, 12, R, flat(active ? 'W' : 'h'), { round: 2, openBottom: true });
  if (!active) for (let x = 1; x < 11; x++) p.set(x, 11, 'k'); // inactive tab is closed at the bottom
  // carton tape stripe on the top edge
  dots(p, active ? [[5, 1, 'z'], [6, 1, 'z']] : [[5, 1, 'v'], [6, 1, 'v']]);
  return p;
}
function slot(r) {
  // inset paper slot; rarity: single border (rare), double border (epic), border + sparkles (legendary)
  const base = [['k', 'k'], ['g', 'i'], ['h', 'W'], ['i', 'i']];
  const R = {
    common: base,
    rare: [['k', 'k'], ['C', 'B'], ['h', 'W'], ['i', 'i']],
    epic: [['k', 'k'], ['L', 'M'], ['i', 'i'], ['M', 'm']],
    legendary: [['k', 'k'], ['z', 'y'], ['y', 'x'], ['i', 'i']],
  }[r];
  const p = frame(16, 16, R, flat('i'), { round: 2 });
  if (r === 'legendary') dots(p, [[1, 2, 'w'], [2, 1, 'w'], [13, 1, 'w'], [14, 2, 'w'], [1, 13, 'w'], [2, 14, 'w'], [14, 13, 'w'], [13, 14, 'w']]);
  return p;
}
const tooltip = () => frame(12, 12, [['k', 'k'], ['u', 't'], ['n', 'n'], ['n', 'n']], flat('n'), { round: 2 });
function bar(kind) {
  const C = { bg: ['n', 's'], hp: ['o', 'R'], xp: ['D', 'C'], boss: ['L', 'M'], cd: ['z', 'y'] }[kind];
  const p = new Px(8, 4);
  for (let y = 0; y < 4; y++) for (let x = 0; x < 8; x++) {
    const edge = x === 0 || x === 7 || y === 0 || y === 3;
    if (edge && (x === 0 || x === 7) && (y === 0 || y === 3)) continue;
    p.set(x, y, edge ? 'k' : y === 1 ? C[0] : C[1]);
  }
  return p;
}

export default function build() {
  return [
    { name: 'ui_panel', px: panel() },
    { name: 'ui_plate', px: plate() },
    { name: 'ui_btn', px: button('normal') },
    { name: 'ui_btn_hover', px: button('hover') },
    { name: 'ui_btn_press', px: button('press') },
    { name: 'ui_btn_disabled', px: button('disabled') },
    { name: 'ui_tab', px: tab(false) },
    { name: 'ui_tab_active', px: tab(true) },
    { name: 'ui_slot', px: slot('common') },
    { name: 'ui_slot_rare', px: slot('rare') },
    { name: 'ui_slot_epic', px: slot('epic') },
    { name: 'ui_slot_legendary', px: slot('legendary') },
    { name: 'ui_tooltip', px: tooltip() },
    { name: 'ui_bar_bg', px: bar('bg') },
    ...['hp', 'xp', 'boss', 'cd'].map((b) => ({ name: `ui_bar_${b}`, px: bar(b) })),
  ];
}
