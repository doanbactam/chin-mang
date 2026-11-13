//! UI (bevy_ui, độ phân giải màn hình). Mỗi lớp (HUD, bảng, lớp phủ, tooltip) là một cây `VNode`
//! dựng lại mỗi frame từ trạng thái rồi so khớp (`vdom`).

pub mod actions;
pub mod hud;
pub mod kit;
pub mod offline;
pub mod panel;
pub mod tokens;
pub mod tutorial;
pub mod vdom;

use crate::assets::{AssetsReady, Atlas, Fonts};
use crate::layout::LayoutOut;
use crate::platform::Caps;
use crate::settings::Settings;
use crate::sim::{Offline, Sim};
use crate::world::WorldHover;
use actions::{ClipboardJob, HoldState, PendingActions};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowFocused};
use cm_core::{BuyMode, CatId, ItemId, Rarity, Slot};
use kit::Cx;
use tokens::*;
use vdom::{BtnSkin, Mounted, Tip, UiRes, VNode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Cats,
    Gear,
    Souls,
    Achievements,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GearSort {
    #[default]
    Power,
    Rarity,
    Newest,
}

/// Trạng thái hiển thị (không thuộc lõi).
#[derive(Resource, Clone, Debug)]
pub struct View {
    pub panel: Option<Tab>,
    pub sel_cat: Option<CatId>,
    pub sel_item: Option<ItemId>,
    pub buy: BuyMode,
    pub filter_slot: Option<Slot>,
    pub filter_rarity: Option<Rarity>,
    pub sort: GearSort,
    /// chế độ chụp ảnh: không đóng bảng khi mất focus, không phát âm
    pub shot_mode: bool,
    /// tooltip cố định (ảnh chụp): (chuỗi, vị trí trong cửa sổ)
    pub forced_tip: Option<(Vec<String>, Vec2)>,
}

impl Default for View {
    fn default() -> Self {
        View {
            panel: None,
            sel_cat: None,
            sel_item: None,
            buy: BuyMode::One,
            filter_slot: None,
            filter_rarity: None,
            sort: GearSort::Power,
            shot_mode: false,
            forced_tip: None,
        }
    }
}

/// Thông báo ngắn trên dải.
#[derive(Resource, Default)]
pub struct Toasts(pub Vec<(String, f32)>);

impl Toasts {
    pub const TTL: f32 = 2.8;
    /// Thông báo quan trọng (save hỏng / khôi phục / không lưu được).
    pub const TTL_LONG: f32 = 8.0;
    pub fn push(&mut self, s: &str) {
        self.push_for(s, Self::TTL);
    }
    pub fn push_for(&mut self, s: &str, ttl: f32) {
        if self.0.last().is_some_and(|(t, _)| t == s) {
            if let Some(last) = self.0.last_mut() {
                last.1 = last.1.max(ttl);
            }
            return;
        }
        self.0.push((s.to_string(), ttl));
        if self.0.len() > 2 {
            self.0.remove(0);
        }
    }
}

/// Bố cục hiện hành (platform cập nhật).
#[derive(Resource, Clone, Copy, Debug)]
pub struct CurrentLayout(pub LayoutOut);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Layer {
    Hud,
    Panel,
    Overlay,
    Tooltip,
}
const LAYERS: [Layer; 4] = [Layer::Hud, Layer::Panel, Layer::Overlay, Layer::Tooltip];

#[derive(Resource, Default)]
struct Trees(Vec<Option<Mounted>>);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<View>()
            .init_resource::<Toasts>()
            .init_resource::<PendingActions>()
            .init_resource::<HoldState>()
            .init_resource::<ClipboardJob>()
            .init_resource::<Trees>()
            .add_observer(actions::on_activate)
            .add_systems(
                Update,
                (
                    actions::hold_buttons,
                    keyboard_and_focus,
                    world_clicks,
                    actions::run_actions,
                    tick_toasts,
                    rebuild,
                    button_skins,
                )
                    .chain()
                    .run_if(|r: Res<AssetsReady>, s: Option<Res<Sim>>| r.0 && s.is_some()),
            );
    }
}

fn tick_toasts(
    time: Res<Time<Real>>,
    mut toasts: ResMut<Toasts>,
    mut offline: ResMut<Offline>,
    view: Res<View>,
    mut notices: ResMut<crate::sim::Notices>,
    mut sim: ResMut<Sim>,
    settings: Res<Settings>,
) {
    for key in notices.0.drain(..) {
        toasts.push_for(crate::i18n::t(settings.lang, key), Toasts::TTL_LONG);
    }
    if sim.save_error.take().is_some() {
        toasts.push_for(
            crate::i18n::t(settings.lang, "toast_save_fail"),
            Toasts::TTL_LONG,
        );
    }
    let dt = time.delta_secs();
    for t in &mut toasts.0 {
        t.1 -= dt;
    }
    toasts.0.retain(|t| t.1 > 0.0);
    // thẻ vắng mặt tự nhận sau 10 s (SPEC §7.4); ảnh chụp giữ nguyên
    if offline.report.is_some() && !view.shot_mode {
        offline.shown += dt;
        if offline.shown >= 10.0 {
            offline.report = None;
        }
    }
}

/// Esc đóng bảng; mất focus cửa sổ (click ra ngoài) đóng bảng.
fn keyboard_and_focus(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: MessageReader<WindowFocused>,
    mut view: ResMut<View>,
) {
    let lost = focus.read().any(|f| !f.focused);
    if view.panel.is_some() && (keys.just_pressed(KeyCode::Escape) || (lost && !view.shot_mode)) {
        view.panel = None;
    }
}

/// Click vào thế giới: hộp carton (mở), mèo (nhấp đúp mở bảng ở tab Mèo).
fn world_clicks(
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    hover: Res<WorldHover>,
    ui_hover: Query<&Hovered, Or<(With<vdom::UiAction>, With<vdom::HoldButton>, With<Tip>)>>,
    mut last_click: Local<(f32, Option<CatId>)>,
    mut pending: ResMut<PendingActions>,
) {
    if !mouse.just_pressed(MouseButton::Left) || ui_hover.iter().any(|h| h.get()) {
        return;
    }
    let now = time.elapsed_secs();
    if hover.over_box {
        pending
            .0
            .push(actions::Action::Cmd(cm_core::Command::OpenBox));
        return;
    }
    if let Some(cat) = hover.cat {
        if last_click.1 == Some(cat) && now - last_click.0 < 0.4 {
            pending.0.push(actions::Action::OpenTab(Tab::Cats));
            pending.0.push(actions::Action::SelectCat(Some(cat)));
            *last_click = (0.0, None);
        } else {
            *last_click = (now, Some(cat));
        }
    }
}

/// Đổi skin nút theo hover / press.
fn button_skins(
    atlas: Res<Atlas>,
    mut q: Query<
        (&BtnSkin, &Hovered, Has<bevy::ui::Pressed>, &mut ImageNode),
        Without<bevy::ui::InteractionDisabled>,
    >,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    for (skin, hov, pressed, mut img) in &mut q {
        let name = if pressed || (hov.get() && mouse.pressed(MouseButton::Left)) {
            format!("{}_press", skin.base)
        } else if hov.get() {
            format!("{}_hover", skin.base)
        } else {
            skin.base.clone()
        };
        if let Some(f) = atlas.frame(&name) {
            let r = Some(f.rect());
            if img.rect != r {
                img.rect = r;
            }
        }
    }
}

fn rebuild(
    mut commands: Commands,
    mut trees: ResMut<Trees>,
    atlas: Res<Atlas>,
    fonts: Res<Fonts>,
    sim: Res<Sim>,
    view: Res<View>,
    settings: Res<Settings>,
    layout: Res<CurrentLayout>,
    toasts: Res<Toasts>,
    offline: Res<Offline>,
    hover: Res<WorldHover>,
    caps: Res<Caps>,
    tips: Query<(&Tip, &Hovered)>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
    time: Res<Time<Real>>,
) {
    let lay = layout.0;
    if ui_scale.0 != lay.ui_scale {
        ui_scale.0 = lay.ui_scale;
    }
    let win = Vec2::new(window.width(), window.height()) / lay.ui_scale;
    let strip = crate::layout::Rect {
        x: lay.strip.x / lay.ui_scale,
        y: lay.strip.y / lay.ui_scale,
        w: lay.strip.w / lay.ui_scale,
        h: lay.strip.h / lay.ui_scale,
    };
    let base = Cx {
        lang: settings.lang,
        ts: settings.text_scale(),
        k: PANEL_ART as f32,
        light: false,
    };
    // HUD: art theo hệ số thế giới, chữ ≥ token
    let hud_k = hud::hud_k(&lay);
    let hud_cx = Cx {
        k: hud_k,
        ts: base.ts * (hud_k / 2.0).max(1.0),
        ..base
    };
    let game = &sim.game;
    let res = UiRes {
        atlas: &atlas,
        fonts: &fonts,
    };
    let now = time.elapsed_secs();

    let cursor = window.cursor_position().map(|c| c / lay.ui_scale);
    for (i, layer) in LAYERS.iter().enumerate() {
        let root_node = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(win.x),
            height: Val::Px(win.y),
            ..default()
        };
        let mut root = VNode::boxed(root_node).z(10 + i as i32 * 10);
        match layer {
            Layer::Hud => root = root.kid(hud::build(&hud_cx, game, strip, &view, &settings, now)),
            Layer::Panel => {
                if let (Some(tab), Some(p)) = (view.panel, lay.panel) {
                    let pr = crate::layout::Rect {
                        x: p.x / lay.ui_scale,
                        y: p.y / lay.ui_scale,
                        w: p.w / lay.ui_scale,
                        h: p.h / lay.ui_scale,
                    };
                    root = root.kid(panel::build(
                        &base.lit(),
                        game,
                        &view,
                        &settings,
                        &caps,
                        tab,
                        pr,
                    ));
                }
            }
            Layer::Overlay => {
                root = root.kids(offline::build(&hud_cx, &offline, strip));
                root = root.kids(tutorial::build(&hud_cx, game, strip, view.panel.is_some()));
                root = root.kids(toast_nodes(&hud_cx, &toasts, strip));
            }
            Layer::Tooltip => {
                let mut lines: Option<Vec<VNode>> = None;
                let mut at = cursor.unwrap_or_default();
                if let Some((ls, pos)) = &view.forced_tip {
                    lines = Some(tip_lines(&base, ls));
                    at = *pos;
                } else if let Some((tip, _)) = tips.iter().find(|(_, h)| h.get()) {
                    lines = Some(tip_lines(
                        &base,
                        &tip.0.split('\n').map(str::to_string).collect::<Vec<_>>(),
                    ));
                } else if let Some(cat) = hover.cat {
                    lines = Some(hud::cat_tooltip(&base, game, cat));
                } else if hover.over_box {
                    lines = Some(tip_lines(&base, &[base.t("box_tip").to_string()]));
                }
                if let Some(lines) = lines {
                    // đặt cạnh con trỏ, giữ trong cửa sổ
                    let w = TOOLTIP_MAX_W;
                    let x = if at.x + 12.0 + w > win.x {
                        (at.x - 12.0 - w).max(0.0)
                    } else {
                        at.x + 12.0
                    };
                    let below = at.y + 16.0;
                    let y = if below + 60.0 > win.y {
                        (at.y - 64.0).max(0.0)
                    } else {
                        below
                    };
                    root = root.kid(
                        VNode::boxed(Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x.floor()),
                            top: Val::Px(y.floor()),
                            ..default()
                        })
                        .kid(kit::tooltip_box(&base, lines)),
                    );
                }
            }
        }
        if trees.0.len() <= i {
            trees.0.push(None);
        }
        match &mut trees.0[i] {
            Some(m) => vdom::patch(&mut commands, m, &root, &res),
            slot @ None => *slot = Some(vdom::spawn(&mut commands, &root, &res)),
        }
    }
}

fn tip_lines(cx: &Cx, lines: &[String]) -> Vec<VNode> {
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                kit::text_full(cx, l.clone(), Fs::Body, Ink::Text, true)
            } else {
                kit::para(cx, l.clone(), Fs::Small, Ink::Muted, TOOLTIP_MAX_W - 16.0)
            }
        })
        .collect()
}

fn toast_nodes(cx: &Cx, toasts: &Toasts, strip: crate::layout::Rect) -> Vec<VNode> {
    if toasts.0.is_empty() {
        return vec![];
    }
    let col = VNode::boxed(Node {
        position_type: PositionType::Absolute,
        left: Val::Px(strip.x),
        width: Val::Px(strip.w),
        top: Val::Px(strip.y + strip.h * 0.42),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(S1),
        ..default()
    });
    vec![col.kids(toasts.0.iter().map(|(s, _)| {
        kit::plate(cx, Node::default()).kid(kit::text(cx, s.clone(), Fs::Small, Ink::Text))
    }))]
}
