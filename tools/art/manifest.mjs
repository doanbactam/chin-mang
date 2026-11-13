// AUTHORITATIVE list of sprite names (SPEC §8.3, docs/CONTRACT.md). The view references these names
// verbatim; build-atlas.mjs fails if any is missing, has the wrong size, or is not listed here.
// Each entry: { name, group, kind, w, h, anim?, slice? }
//   group = which tools/art/sprites/<group>.mjs draws it (also: one contact sheet per group)
//   kind  = unit | boss | proj | fx | drop | tile | prop | icon | ui | font (drives lint + default anchor)
//   anim  = animation key (name without the trailing _i) for frames of one animation (dup-frame lint)
//   slice = 9-slice inset in px (number, same on all four sides) for UI frames
// Facing: cats face RIGHT, enemies and bosses face LEFT (they walk toward the party). The view mirrors if needed.
export const MANIFEST = [];
const add = (group, kind, name, w, h, extra = {}) => MANIFEST.push({ name, group, kind, w, h, ...extra });
const anim = (group, kind, base, n, w, h) => { for (let i = 0; i < n; i++) add(group, kind, `${base}_${i}`, w, h, { anim: base }); };

// ── cats 16×16 ─────────────────────────────────────────────────────────────────
export const CATS = ['muop', 'xiem', 'map', 'mun'];
export const CAT_ANIMS = { idle: 2, walk: 4, attack: 3, hurt: 1, death: 3, sleep: 2, skill: 3 };
for (const c of CATS) {
  for (const [a, n] of Object.entries(CAT_ANIMS)) anim('cats', 'unit', `cat_${c}_${a}`, n, 16, 16);
  add('cats', 'icon', `portrait_${c}`, 16, 16);
}

// ── enemies 16×16 ──────────────────────────────────────────────────────────────
export const ENEMIES = ['rat', 'roach', 'crow', 'snake', 'owl', 'dog', 'tick', 'vacuum'];
export const ENEMY_ANIMS = { walk: 4, attack: 2, hurt: 1, death: 3 };
for (const e of ENEMIES) for (const [a, n] of Object.entries(ENEMY_ANIMS)) anim('enemies', 'unit', `en_${e}_${a}`, n, 16, 16);

// ── bosses 32×32 ───────────────────────────────────────────────────────────────
export const BOSSES = ['rat_king', 'crow_lord', 'eel', 'mastiff', 'vacuum_titan'];
export const BOSS_ANIMS = { walk: 4, attack: 3, special: 3, hurt: 1, death: 4 };
for (const b of BOSSES) for (const [a, n] of Object.entries(BOSS_ANIMS)) anim('bosses', 'boss', `boss_${b}_${a}`, n, 32, 32);

// ── projectiles + fx ───────────────────────────────────────────────────────────
for (const p of ['fishbone', 'curse', 'feather', 'spark']) anim('fx', 'proj', `proj_${p}`, 2, 8, 8);
add('fx', 'proj', 'proj_bone_rain', 6, 8);
export const FX = { hit: [3, 10, 10], slash: [4, 16, 16], aoe: [5, 24, 24], poof: [4, 16, 16], levelup: [5, 16, 24],
  catnip: [4, 16, 16], stun: [4, 16, 8], poison: [3, 12, 12], crit: [4, 16, 16] };
for (const [f, [n, w, h]] of Object.entries(FX)) anim('fx', 'fx', `fx_${f}`, n, w, h);
add('fx', 'fx', 'shadow_s', 10, 3);
add('fx', 'fx', 'shadow_l', 20, 4);

// ── drops + carton box ─────────────────────────────────────────────────────────
export const RARITIES = ['common', 'rare', 'epic', 'legendary'];
add('drops', 'drop', 'drop_fish', 8, 8);
add('drops', 'drop', 'drop_yarn', 8, 8);
for (const r of RARITIES) add('drops', 'drop', `drop_gear_${r}`, 8, 8);
add('drops', 'drop', 'box_closed', 14, 12);
anim('drops', 'drop', 'box_open', 3, 14, 14);

// ── world: ground strips (tile horizontally) + background deco ─────────────────
export const DECO_SIZES = {
  1: [[14, 18], [20, 14], [10, 24], [16, 10]],   // trash can, carton stack, street lamp, puddle+bottle
  2: [[14, 22], [12, 24], [22, 20], [12, 12]],   // chimney, TV antenna, water tank, flower pot
  3: [[32, 22], [18, 12], [12, 12], [10, 22]],   // fish stall, fish crate, bucket, lantern post
  4: [[26, 14], [24, 24], [8, 24], [20, 12]],    // bench, tree, lamp post, flower bed
  5: [[24, 16], [16, 16], [12, 14], [30, 20]],   // tire stack, broken TV, oil barrel, scrap heap
};
for (let z = 1; z <= 5; z++) {
  for (let i = 0; i < 4; i++) add('world', 'tile', `ground_z${z}_${i}`, 16, 6);
  for (let i = 0; i < 4; i++) add('world', 'prop', `deco_z${z}_${i}`, ...DECO_SIZES[z][i]);
}

// ── icons ──────────────────────────────────────────────────────────────────────
for (const i of ['fish', 'yarn', 'soul', 'catnip', 'bag', 'settings', 'close', 'lock', 'up', 'cats', 'gear', 'lives', 'trophy', 'sort', 'filter', 'check', 'back'])
  add('icons', 'icon', `ic_${i}`, 12, 12);
for (const s of ['collar', 'bell', 'claws']) for (const r of RARITIES) add('icons', 'icon', `item_${s}_${r}`, 16, 16);
for (const c of CATS) add('icons', 'icon', `skill_${c}`, 16, 16);
for (const t of ['power', 'vigor', 'frenzy', 'pierce', 'thorns', 'hex']) add('icons', 'icon', `talent_${t}`, 12, 12);
export const SOUL_SHOP = ['claw_sharpen', 'thick_fur', 'fat_wallet', 'long_nap', 'auto_catnip', 'head_start', 'lucky_paw'];
for (const s of SOUL_SHOP) add('icons', 'icon', `soul_${s}`, 12, 12);
for (let i = 1; i <= 9; i++) add('icons', 'icon', `life_${i}`, 10, 10);
for (const a of ['zone', 'boss', 'cat', 'level', 'loot', 'lives', 'box', 'wealth']) add('icons', 'icon', `ach_${a}`, 16, 16);

// ── UI 9-slice ─────────────────────────────────────────────────────────────────
add('ui', 'ui', 'ui_panel', 16, 16, { slice: 4 });
add('ui', 'ui', 'ui_plate', 12, 12, { slice: 4 });
for (const s of ['', '_hover', '_press', '_disabled']) add('ui', 'ui', `ui_btn${s}`, 12, 12, { slice: 4 });
add('ui', 'ui', 'ui_tab', 12, 12, { slice: 4 });
add('ui', 'ui', 'ui_tab_active', 12, 12, { slice: 4 });
add('ui', 'ui', 'ui_slot', 16, 16, { slice: 4 });
for (const r of ['rare', 'epic', 'legendary']) add('ui', 'ui', `ui_slot_${r}`, 16, 16, { slice: 4 });
add('ui', 'ui', 'ui_tooltip', 12, 12, { slice: 4 });
// bars are 4 px tall: a 4 px inset would leave no centre row, so bars slice 1 px (DECISIONS)
add('ui', 'ui', 'ui_bar_bg', 8, 4, { slice: 1 });
for (const b of ['hp', 'xp', 'boss', 'cd']) add('ui', 'ui', `ui_bar_${b}`, 8, 4, { slice: 1 });

// ── pixel digits for floating damage numbers ───────────────────────────────────
export const FONT_GLYPHS = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'dot', 'K', 'M', 'B', 'T', 'plus', 'minus'];
for (const g of FONT_GLYPHS) add('font', 'font', `fd_${g}`, 4, 6);

export const NAMES = new Set(MANIFEST.map((m) => m.name));
export const GROUPS = [...new Set(MANIFEST.map((m) => m.group))];
