//! Hành động UI: mọi nút đều dẫn tới một `Action` có hiệu lực thật (không nút rỗng).

use super::vdom::{HoldButton, HoldFillBar, UiAction};
use super::{GearSort, Tab, Toasts, View};
use crate::audio::Sfx;
use crate::i18n::{t, tf};
use crate::save_io;
use crate::settings::{Edge, Settings};
use crate::sim::{Offline, Sim};
use bevy::app::AppExit;
use bevy::clipboard::{Clipboard, ClipboardRead};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use cm_core::{
    BuyMode, CatId, CmdError, Command, Currency, Game, ItemId, Rarity, Slot, TutorialId,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VolKind {
    Master,
    Music,
    Sfx,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SetOp {
    Lang(crate::i18n::Lang),
    Scale(u32),
    Width(u32),
    Edge(Edge),
    Monitor(usize),
    Opacity(u32),
    Ghost(bool),
    OnTop(bool),
    Text(u32),
    Vol(VolKind, u32),
    Fps(u32),
    Autostart(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Cmd(Command),
    TogglePanel(Tab),
    OpenTab(Tab),
    ClosePanel,
    SelectCat(Option<CatId>),
    SelectItem(Option<ItemId>),
    BuyMode(BuyMode),
    FilterSlot(Option<Slot>),
    FilterRarity(Option<Rarity>),
    Sort(GearSort),
    Set(SetOp),
    Export,
    Import,
    Reset,
    Quit,
    ClaimOffline,
    DismissTutorial(TutorialId),
}

/// Hàng đợi hành động (observer `Activate` đẩy vào, `run_actions` xử lý).
#[derive(Resource, Default)]
pub struct PendingActions(pub Vec<Action>);

/// Đang giữ nút xác nhận: (hành động, giây đã giữ).
#[derive(Resource, Default)]
pub struct HoldState(pub Option<(Action, f32, f32)>);

#[derive(Resource, Default)]
pub struct ClipboardJob(pub Option<ClipboardRead>);

pub fn on_activate(ev: On<Activate>, q: Query<&UiAction>, mut pending: ResMut<PendingActions>) {
    if let Ok(a) = q.get(ev.entity) {
        pending.0.push(a.0.clone());
    }
}

/// Nút giữ-để-xác-nhận: giữ chuột trái trên nút đủ `secs` giây.
pub fn hold_buttons(
    time: Res<Time<Real>>,
    mouse: Res<ButtonInput<MouseButton>>,
    q: Query<(&HoldButton, &Hovered, &Children)>,
    mut fills: Query<&mut Node, With<HoldFillBar>>,
    mut hold: ResMut<HoldState>,
    mut pending: ResMut<PendingActions>,
) {
    let mut active: Option<(Action, f32)> = None;
    if mouse.pressed(MouseButton::Left) {
        for (hb, hov, _) in &q {
            if hov.get() {
                active = Some((hb.action.clone(), hb.secs));
            }
        }
    }
    match (&mut hold.0, active) {
        (Some((a, t, secs)), Some((b, _))) if *a == b => {
            if *t < *secs {
                *t += time.delta_secs();
                if *t >= *secs {
                    pending.0.push(a.clone());
                }
            }
        }
        (h, Some((b, secs))) => {
            // chỉ bắt đầu khi vừa nhấn (không “trượt” từ nút khác sang)
            *h = if mouse.just_pressed(MouseButton::Left) {
                Some((b, 0.0, secs))
            } else {
                None
            };
        }
        (h, None) => *h = None,
    }
    for (hb, _, children) in &q {
        let frac = match &hold.0 {
            Some((a, t, secs)) if *a == hb.action => (t / secs).min(1.0),
            _ => 0.0,
        };
        for c in children.iter() {
            if let Ok(mut n) = fills.get_mut(c) {
                let w = Val::Percent(frac * 100.0);
                if n.width != w {
                    n.width = w;
                }
            }
        }
    }
}

fn err_key(e: CmdError) -> &'static str {
    match e {
        CmdError::NotEnough(Currency::Fish) => "err_fish",
        CmdError::NotEnough(Currency::Yarn) => "err_yarn",
        CmdError::NotEnough(Currency::Soul) => "err_soul",
        CmdError::OnCooldown | CmdError::NotEnough(Currency::Catnip) => "err_catnip",
        _ => "err_generic",
    }
}

pub fn run_actions(
    mut pending: ResMut<PendingActions>,
    mut sim: ResMut<Sim>,
    mut view: ResMut<View>,
    mut settings: ResMut<Settings>,
    mut toasts: ResMut<Toasts>,
    mut sfx: ResMut<Sfx>,
    mut offline: ResMut<Offline>,
    mut clipboard: ResMut<Clipboard>,
    mut job: ResMut<ClipboardJob>,
    mut exit: MessageWriter<AppExit>,
) {
    // kết quả đọc clipboard (nhập save) có thể về trễ
    if let Some(read) = &mut job.0
        && let Some(res) = read.poll_result()
    {
        job.0 = None;
        let lang = settings.lang;
        match res
            .ok()
            .and_then(|s| save_io::decode_import(&s))
            .map(|j| Game::load(&j))
        {
            Some(Ok(g)) => {
                sim.replace(g);
                sim.save_now();
                toasts.push(t(lang, "toast_import_ok"));
                sfx.play("unlock");
            }
            _ => {
                toasts.push(t(lang, "toast_import_fail"));
                sfx.play("ui_error");
            }
        }
    }
    let actions = std::mem::take(&mut pending.0);
    for a in actions {
        let lang = settings.lang;
        match a {
            Action::Cmd(cmd) => match sim.game.command(cmd) {
                Ok(()) => {
                    // âm báo cho lệnh không sinh Event riêng
                    if matches!(
                        cmd,
                        Command::Equip { .. }
                            | Command::Unequip { .. }
                            | Command::EquipBest { .. }
                            | Command::SetAutoSalvage { .. }
                    ) {
                        sfx.play("ui_click");
                    }
                    if let Command::Salvage { item } = cmd
                        && view.sel_item == Some(item)
                    {
                        view.sel_item = None;
                    }
                }
                Err(e) => {
                    toasts.push(t(lang, err_key(e)));
                    sfx.play("ui_error");
                }
            },
            Action::TogglePanel(tab) => {
                view.panel = if view.panel.is_some() {
                    None
                } else {
                    Some(tab)
                };
                sfx.play("ui_tab");
            }
            Action::OpenTab(tab) => {
                view.panel = Some(tab);
                sfx.play("ui_tab");
            }
            Action::ClosePanel => {
                view.panel = None;
                sfx.play("ui_click");
            }
            Action::SelectCat(c) => {
                view.sel_cat = c;
                sfx.play("ui_click");
            }
            Action::SelectItem(i) => {
                view.sel_item = i;
                sfx.play("ui_click");
            }
            Action::BuyMode(m) => {
                view.buy = m;
                sfx.play("ui_click");
            }
            Action::FilterSlot(s) => {
                view.filter_slot = s;
                sfx.play("ui_click");
            }
            Action::FilterRarity(r) => {
                view.filter_rarity = r;
                sfx.play("ui_click");
            }
            Action::Sort(s) => {
                view.sort = s;
                sfx.play("ui_click");
            }
            Action::Set(op) => {
                apply_setting(&mut settings, op);
                sfx.play("ui_click");
            }
            Action::Export => {
                let enc = save_io::encode_export(&sim.game.save(save_io::now_unix()));
                let clip_ok = cfg!(any(windows, target_arch = "wasm32"))
                    && clipboard.set_text(enc.clone()).is_ok();
                if clip_ok {
                    toasts.push(t(lang, "toast_export_clip"));
                } else {
                    match save_io::write(save_io::EXPORT_KEY, &enc) {
                        Ok(p) => toasts.push(&tf(lang, "toast_export_file", &[&p])),
                        Err(e) => toasts.push(&e),
                    }
                }
                sfx.play("ui_click");
            }
            Action::Import => {
                // Linux không có clipboard hệ thống trong bản build này: đọc file export cạnh save.
                if cfg!(any(windows, target_arch = "wasm32")) {
                    job.0 = Some(clipboard.fetch_text());
                } else {
                    match save_io::read(save_io::EXPORT_KEY)
                        .and_then(|s| save_io::decode_import(&s))
                        .map(|j| Game::load(&j))
                    {
                        Some(Ok(g)) => {
                            sim.replace(g);
                            sim.save_now();
                            toasts.push(t(lang, "toast_import_ok"));
                        }
                        _ => {
                            toasts.push(t(lang, "toast_import_fail"));
                            sfx.play("ui_error");
                        }
                    }
                }
            }
            Action::Reset => {
                save_io::remove(save_io::SAVE_KEY);
                let seed = save_io::now_unix();
                sim.replace(Game::new(seed));
                sim.save_now();
                *view = View {
                    panel: view.panel,
                    ..View::default()
                };
                toasts.push(t(lang, "toast_reset"));
                sfx.play("prestige");
            }
            Action::Quit => {
                sim.save_now();
                exit.write(AppExit::Success);
            }
            Action::ClaimOffline => {
                offline.report = None;
                sfx.play("coin");
            }
            Action::DismissTutorial(id) => {
                let _ = sim.game.command(Command::DismissTutorial { id });
                sfx.play("ui_click");
            }
        }
    }
}

pub fn apply_setting(s: &mut Settings, op: SetOp) {
    match op {
        SetOp::Lang(l) => s.lang = l,
        SetOp::Scale(v) => s.scale = v,
        SetOp::Width(v) => s.width_pct = v,
        SetOp::Edge(e) => s.edge = e,
        SetOp::Monitor(m) => s.monitor = m,
        SetOp::Opacity(v) => s.opacity = v,
        SetOp::Ghost(v) => s.ghost = v,
        SetOp::OnTop(v) => s.always_on_top = v,
        SetOp::Text(v) => s.text_size = v,
        SetOp::Vol(VolKind::Master, v) => s.vol_master = v,
        SetOp::Vol(VolKind::Music, v) => s.vol_music = v,
        SetOp::Vol(VolKind::Sfx, v) => s.vol_sfx = v,
        SetOp::Fps(v) => s.fps_limit = v,
        SetOp::Autostart(v) => s.autostart = v,
    }
    s.sanitize();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_ops_are_sanitized() {
        let mut s = Settings::default();
        apply_setting(&mut s, SetOp::Opacity(30));
        assert_eq!(s.opacity, 40);
        apply_setting(&mut s, SetOp::Vol(VolKind::Music, 120));
        assert_eq!(s.vol_music, 100);
        apply_setting(&mut s, SetOp::Edge(Edge::Top));
        assert_eq!(s.edge, Edge::Top);
    }
}
