//! Cây UI khai báo nhẹ: các builder (HUD, bảng…) dựng `VNode` thuần từ trạng thái mỗi frame,
//! rồi `reconcile` so với cây đã gắn và chỉ chèn lại component của node thay đổi.
//! Giữ nguyên entity khi cấu trúc không đổi ⇒ không mất trạng thái hover/press của picking.

use super::actions::Action;
use crate::assets::{Atlas, Fonts};
use bevy::picking::Pickable;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::text::{FontSmoothing, LineHeight};
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Button;

#[derive(Clone, Debug, PartialEq)]
pub struct Skin {
    pub name: String,
    /// hệ số phóng nguyên của art
    pub k: f32,
    pub tint: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSpec {
    pub text: String,
    /// cỡ chữ cuối cùng (px logic)
    pub size: f32,
    pub color: Color,
    pub bold: bool,
    pub justify: Justify,
    pub wrap: bool,
    /// bóng chữ tối 1 px (chữ sáng trên nền tối / nền desktop)
    pub shadow: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IconSpec {
    pub name: String,
    pub k: f32,
    pub tint: Color,
    pub flip: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Box,
    Text(TextSpec),
    Icon(IconSpec),
    /// Thanh tiến độ của nút giữ-để-xác-nhận (bề rộng do hệ thống hold cập nhật).
    HoldFill,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Props {
    pub node: Node,
    pub kind: Kind,
    pub skin: Option<Skin>,
    pub bg: Option<Color>,
    pub action: Option<Action>,
    /// giây cần giữ (nút giữ-để-xác-nhận)
    pub hold: Option<f32>,
    pub tip: Option<String>,
    pub disabled: bool,
    pub z: Option<i32>,
    /// true: nhận hover/click (chặn lớp dưới)
    pub pick: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VNode {
    pub p: Props,
    pub kids: Vec<VNode>,
}

impl VNode {
    pub fn new(kind: Kind, node: Node) -> VNode {
        VNode {
            p: Props {
                node,
                kind,
                skin: None,
                bg: None,
                action: None,
                hold: None,
                tip: None,
                disabled: false,
                z: None,
                pick: false,
            },
            kids: Vec::new(),
        }
    }
    pub fn boxed(node: Node) -> VNode {
        VNode::new(Kind::Box, node)
    }
    pub fn kids(mut self, kids: impl IntoIterator<Item = VNode>) -> VNode {
        self.kids.extend(kids);
        self
    }
    pub fn kid(mut self, k: VNode) -> VNode {
        self.kids.push(k);
        self
    }
    pub fn skin(mut self, name: impl Into<String>, k: f32) -> VNode {
        self.p.skin = Some(Skin {
            name: name.into(),
            k,
            tint: Color::WHITE,
        });
        self
    }
    pub fn skin_tint(mut self, tint: Color) -> VNode {
        if let Some(s) = &mut self.p.skin {
            s.tint = tint;
        }
        self
    }
    pub fn bg(mut self, c: Color) -> VNode {
        self.p.bg = Some(c);
        self
    }
    pub fn action(mut self, a: Action) -> VNode {
        self.p.action = Some(a);
        self.p.pick = true;
        self
    }
    pub fn hold(mut self, secs: f32) -> VNode {
        self.p.hold = Some(secs);
        self
    }
    pub fn tip(mut self, t: impl Into<String>) -> VNode {
        self.p.tip = Some(t.into());
        self.p.pick = true;
        self
    }
    pub fn disabled(mut self, d: bool) -> VNode {
        self.p.disabled = d;
        self
    }
    pub fn z(mut self, z: i32) -> VNode {
        self.p.z = Some(z);
        self
    }
    /// Bề mặt chặn click (vd nền bảng) để click không lọt xuống thế giới.
    pub fn solid(mut self) -> VNode {
        self.p.pick = true;
        self
    }
}

// ───────────────────────────── gắn vào ECS ─────────────────────────────

/// Hành động khi bấm (đọc bởi `actions::on_activate`).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct UiAction(pub Action);

/// Nút giữ-để-xác-nhận.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct HoldButton {
    pub action: Action,
    pub secs: f32,
}

#[derive(Component, Clone, Debug)]
pub struct HoldFillBar;

/// Nút có skin đổi theo hover/press (tên gốc `ui_btn`/`ui_tab`…).
#[derive(Component, Clone, Debug)]
pub struct BtnSkin {
    pub base: String,
}

#[derive(Component, Clone, Debug)]
pub struct Tip(pub String);

pub struct Mounted {
    pub entity: Entity,
    pub props: Props,
    pub kids: Vec<Mounted>,
}

pub struct UiRes<'a> {
    pub atlas: &'a Atlas,
    pub fonts: &'a Fonts,
}

fn same_kind(a: &Kind, b: &Kind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub fn slice_image(atlas: &Atlas, name: &str, k: f32, tint: Color) -> ImageNode {
    let Some(f) = atlas.frame(name) else {
        return ImageNode {
            color: Color::NONE,
            ..default()
        };
    };
    let mode = if f.slice > 0 {
        let tile = SliceScaleMode::Tile { stretch_value: k };
        NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(f.slice as f32),
            center_scale_mode: tile,
            sides_scale_mode: tile,
            max_corner_scale: k,
        })
    } else {
        NodeImageMode::Stretch
    };
    ImageNode {
        color: tint,
        image: atlas.image.clone(),
        rect: Some(f.rect()),
        image_mode: mode,
        visual_box: VisualBox::BorderBox,
        ..default()
    }
}

fn apply(ec: &mut EntityCommands, old: Option<&Props>, p: &Props, r: &UiRes) {
    let mut node = p.node.clone();
    match &p.kind {
        Kind::Box | Kind::HoldFill => {}
        Kind::Text(t) => {
            let font = if t.bold {
                r.fonts.bold.clone()
            } else {
                r.fonts.regular.clone()
            };
            ec.insert((
                Text::new(t.text.clone()),
                TextFont {
                    font: font.into(),
                    font_size: bevy::text::FontSize::Px(t.size),
                    font_smoothing: FontSmoothing::AntiAliased,
                    ..default()
                },
                // dòng cao 1.65× để dấu chồng tiếng Việt (Ặ Ẫ Ỡ) không bị cắt (DECISIONS)
                LineHeight::RelativeToFont(1.65),
                TextColor(t.color),
                TextLayout::new(
                    t.justify,
                    if t.wrap {
                        bevy::text::LineBreak::WordBoundary
                    } else {
                        bevy::text::LineBreak::NoWrap
                    },
                ),
            ));
            if t.shadow {
                ec.insert(TextShadow {
                    offset: Vec2::new(1.0, 1.0),
                    color: crate::ui::tokens::Ink::Outline.color(),
                });
            } else {
                ec.remove::<TextShadow>();
            }
        }
        Kind::Icon(i) => {
            if let Some(f) = r.atlas.frame(&i.name) {
                node.width = Val::Px(f.w as f32 * i.k);
                node.height = Val::Px(f.h as f32 * i.k);
                ec.insert(ImageNode {
                    color: i.tint,
                    image: r.atlas.image.clone(),
                    rect: Some(f.rect()),
                    flip_x: i.flip,
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                });
            }
        }
    }
    ec.insert(node);
    if matches!(p.kind, Kind::HoldFill) {
        ec.insert(HoldFillBar);
    }
    // nền 9-slice
    match &p.skin {
        Some(s) => {
            ec.insert(slice_image(r.atlas, &s.name, s.k, s.tint));
        }
        None => {
            if old.is_some_and(|o| o.skin.is_some()) && !matches!(p.kind, Kind::Icon(_)) {
                ec.remove::<ImageNode>();
            }
        }
    }
    match p.bg {
        Some(c) => {
            ec.insert(BackgroundColor(c));
        }
        None => {
            if old.is_some_and(|o| o.bg.is_some()) {
                ec.remove::<BackgroundColor>();
            }
        }
    }
    if let Some(z) = p.z {
        ec.insert(GlobalZIndex(z));
    }
    ec.insert(if p.pick {
        Pickable::default()
    } else {
        Pickable::IGNORE
    });
    // hành động
    match (&p.action, p.hold) {
        (Some(a), None) => {
            ec.insert((Button, Hovered::default(), UiAction(a.clone())));
            ec.remove::<HoldButton>();
        }
        (Some(a), Some(secs)) => {
            ec.insert((
                Hovered::default(),
                HoldButton {
                    action: a.clone(),
                    secs,
                },
            ));
            ec.remove::<(Button, UiAction)>();
        }
        (None, _) => {
            if old.is_some_and(|o| o.action.is_some()) {
                ec.remove::<(Button, UiAction, HoldButton)>();
            }
        }
    }
    match (&p.skin, &p.action) {
        (Some(s), Some(_)) if s.name.starts_with("ui_btn") && !p.disabled => {
            ec.insert(BtnSkin {
                base: "ui_btn".into(),
            });
        }
        _ => {
            if old.is_some_and(|o| o.action.is_some()) {
                ec.remove::<BtnSkin>();
            }
        }
    }
    if p.disabled {
        ec.insert(InteractionDisabled);
    } else if old.is_some_and(|o| o.disabled) {
        ec.remove::<InteractionDisabled>();
    }
    match &p.tip {
        Some(t) => {
            ec.insert((Tip(t.clone()), Hovered::default()));
        }
        None => {
            if old.is_some_and(|o| o.tip.is_some()) {
                ec.remove::<Tip>();
            }
        }
    }
}

pub fn spawn(c: &mut Commands, v: &VNode, r: &UiRes) -> Mounted {
    let mut ec = c.spawn_empty();
    apply(&mut ec, None, &v.p, r);
    let entity = ec.id();
    let kids: Vec<Mounted> = v.kids.iter().map(|k| spawn(c, k, r)).collect();
    if !kids.is_empty() {
        let ids: Vec<Entity> = kids.iter().map(|m| m.entity).collect();
        c.entity(entity).add_children(&ids);
    }
    Mounted {
        entity,
        props: v.p.clone(),
        kids,
    }
}

/// So khớp `m` với `v`; trả về false nếu phải thay entity (khác loại).
pub fn patch(c: &mut Commands, m: &mut Mounted, v: &VNode, r: &UiRes) {
    if m.props != v.p {
        let mut ec = c.entity(m.entity);
        apply(&mut ec, Some(&m.props), &v.p, r);
        m.props = v.p.clone();
    }
    patch_kids(c, m.entity, &mut m.kids, &v.kids, r);
}

pub fn patch_kids(
    c: &mut Commands,
    parent: Entity,
    kids: &mut Vec<Mounted>,
    new: &[VNode],
    r: &UiRes,
) {
    for (i, v) in new.iter().enumerate() {
        if i < kids.len() {
            if same_kind(&kids[i].props.kind, &v.p.kind) {
                patch(c, &mut kids[i], v, r);
            } else {
                c.entity(kids[i].entity).despawn();
                let m = spawn(c, v, r);
                c.entity(parent).insert_child(i, m.entity);
                kids[i] = m;
            }
        } else {
            let m = spawn(c, v, r);
            c.entity(parent).add_child(m.entity);
            kids.push(m);
        }
    }
    while kids.len() > new.len() {
        if let Some(m) = kids.pop() {
            c.entity(m.entity).despawn();
        }
    }
}
