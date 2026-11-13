//! Thế giới pixel-perfect (SPEC §5): render vào texture thấp `W_logic × 48` rồi phóng hệ số nguyên
//! bằng sampler nearest. Mỗi frame dựng danh sách lệnh vẽ (`draw.rs`, thuần) rồi gán vào một bể sprite.

pub mod anim;
pub mod draw;
pub mod fx;

use crate::assets::{AssetsReady, Atlas};
use crate::settings::Settings;
use crate::sim::Sim;
use crate::ui::CurrentLayout;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use cm_core::{CatId, STRIP_H};
use draw::{DrawCmd, GROUND_BOTTOM};
use fx::Fx;

pub const WORLD_LAYER: usize = 1;

/// Con trỏ trên thế giới (để tooltip mèo, click hộp, mèo nhìn theo con trỏ).
#[derive(Resource, Default, Debug, Clone)]
pub struct WorldHover {
    /// tọa độ logic của con trỏ (nếu đang trên dải)
    pub logic: Option<Vec2>,
    pub cat: Option<CatId>,
    pub over_box: bool,
}

#[derive(Resource)]
struct WorldTarget {
    image: Handle<Image>,
    w: u32,
    camera: Entity,
    canvas: Entity,
    pool: Vec<Entity>,
}

/// Vị trí camera thế giới (px logic, mép trái) — tròn số nguyên.
#[derive(Resource, Default, Clone, Copy)]
pub struct WorldCam {
    pub left: i32,
    /// rung màn hình còn lại (giây)
    pub shake: f32,
}

#[derive(Component)]
struct PoolSprite;
#[derive(Component)]
struct OuterCamera;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldHover>()
            .init_resource::<WorldCam>()
            .init_resource::<Fx>()
            .add_systems(Startup, setup_cameras)
            .add_systems(
                Update,
                (ensure_target, fx::collect, hover, render)
                    .chain()
                    .run_if(|r: Res<AssetsReady>, s: Option<Res<Sim>>| r.0 && s.is_some()),
            );
    }
}

fn setup_cameras(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        Msaa::Off,
        OuterCamera,
        IsDefaultUiCamera,
    ));
}

/// Tạo / tạo lại texture thế giới khi bề rộng logic đổi.
fn ensure_target(
    mut commands: Commands,
    layout: Res<CurrentLayout>,
    target: Option<ResMut<WorldTarget>>,
    mut images: ResMut<Assets<Image>>,
) {
    let w = layout.0.world_w.max(16);
    if let Some(t) = &target
        && t.w == w
    {
        return;
    }
    let image = images.add(Image::new_target_texture(
        w,
        STRIP_H,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    match target {
        Some(mut t) => {
            commands
                .entity(t.camera)
                .insert(RenderTarget::Image(image.clone().into()));
            commands
                .entity(t.canvas)
                .insert(Sprite::from_image(image.clone()));
            t.image = image;
            t.w = w;
        }
        None => {
            let camera = commands
                .spawn((
                    Camera2d,
                    Camera {
                        order: -1,
                        clear_color: ClearColorConfig::Custom(Color::NONE),
                        ..default()
                    },
                    RenderTarget::Image(image.clone().into()),
                    Msaa::Off,
                    RenderLayers::layer(WORLD_LAYER),
                    Transform::default(),
                ))
                .id();
            let canvas = commands
                .spawn((
                    Sprite::from_image(image.clone()),
                    Anchor::TOP_LEFT,
                    Transform::default(),
                ))
                .id();
            commands.insert_resource(WorldTarget {
                image,
                w,
                camera,
                canvas,
                pool: Vec::new(),
            });
        }
    }
}

/// Cửa sổ → tọa độ logic thế giới.
pub fn window_to_logic(
    cursor: Vec2,
    strip: crate::layout::Rect,
    scale: u32,
    cam_left: i32,
) -> Option<Vec2> {
    if !strip.contains(cursor.x, cursor.y) {
        return None;
    }
    let s = scale as f32;
    Some(Vec2::new(
        cam_left as f32 + (cursor.x - strip.x) / s,
        GROUND_BOTTOM as f32 + (strip.bottom() - cursor.y) / s,
    ))
}

fn hover(
    window: Single<&Window, With<PrimaryWindow>>,
    layout: Res<CurrentLayout>,
    cam: Res<WorldCam>,
    sim: Res<Sim>,
    mut hover: ResMut<WorldHover>,
) {
    let lay = layout.0;
    let logic = window
        .cursor_position()
        .and_then(|c| window_to_logic(c, lay.strip, lay.scale, cam.left));
    let mut h = WorldHover {
        logic,
        cat: None,
        over_box: false,
    };
    if let Some(p) = logic {
        for c in sim.game.cats() {
            if (p.x - c.x).abs() <= 7.0 && p.y >= c.y && p.y <= c.y + 14.0 {
                h.cat = Some(c.cat);
            }
        }
        if let Some(b) = sim.game.carton()
            && b.opening.is_none()
            && (p.x - b.x).abs() <= 9.0
            && p.y >= b.y - 1.0
            && p.y <= b.y + 14.0
        {
            h.over_box = true;
        }
    }
    if h.cat != hover.cat || h.over_box != hover.over_box || h.logic != hover.logic {
        *hover = h;
    }
}

fn render(
    mut commands: Commands,
    atlas: Res<Atlas>,
    sim: Res<Sim>,
    fixed: Res<Time<Fixed>>,
    time: Res<Time<Real>>,
    layout: Res<CurrentLayout>,
    settings: Res<Settings>,
    hover: Res<WorldHover>,
    mut fxs: ResMut<Fx>,
    mut cam: ResMut<WorldCam>,
    mut target: ResMut<WorldTarget>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut q: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<PoolSprite>>,
    mut cams: Query<&mut Transform, (Without<PoolSprite>, Without<OuterCamera>)>,
) {
    let lay = layout.0;
    let alpha = if sim.paused {
        1.0
    } else {
        fixed.overstep_fraction()
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    cam.shake = (cam.shake - dt).max(0.0);
    fxs.advance(dt);
    let input = draw::Input {
        game: &sim.game,
        sim: &sim,
        alpha,
        world_w: target.w as i32,
        atlas: &atlas.def,
        cursor: hover.logic,
        fx: &fxs,
        now,
    };
    let (left, cmds) = draw::build(&input);
    let shake = if cam.shake > 0.0 {
        if (now * 30.0) as i32 % 2 == 0 { 1 } else { -1 }
    } else {
        0
    };
    cam.left = left;

    // camera thế giới: mép trái nguyên, tâm = mép trái + W/2
    if let Ok(mut t) = cams.get_mut(target.camera) {
        let c = Vec3::new(
            left as f32 + target.w as f32 / 2.0 + shake as f32,
            GROUND_BOTTOM as f32 + STRIP_H as f32 / 2.0,
            0.0,
        );
        if t.translation != c {
            t.translation = c;
        }
    }
    // canvas: đặt ở dải, phóng hệ số nguyên
    if let Ok(mut t) = cams.get_mut(target.canvas) {
        let (ww, wh) = (window.width(), window.height());
        let pos = Vec3::new(-ww / 2.0 + lay.strip.x, wh / 2.0 - lay.strip.y, 0.0);
        let s = Vec3::new(lay.scale as f32, lay.scale as f32, 1.0);
        if t.translation != pos || t.scale != s {
            t.translation = pos;
            t.scale = s;
        }
    }
    let opacity = if settings.ghost && cfg!(windows) {
        0.6
    } else {
        settings.opacity as f32 / 100.0
    };
    commands.entity(target.canvas).insert(Sprite {
        color: Color::srgba(1.0, 1.0, 1.0, opacity),
        ..Sprite::from_image(target.image.clone())
    });

    // bể sprite
    while target.pool.len() < cmds.len() {
        let e = commands
            .spawn((
                Sprite::from_image(atlas.image.clone()),
                Anchor::TOP_LEFT,
                Transform::default(),
                Visibility::Hidden,
                RenderLayers::layer(WORLD_LAYER),
                PoolSprite,
            ))
            .id();
        target.pool.push(e);
    }
    for (i, &e) in target.pool.iter().enumerate() {
        let Ok((mut sprite, mut tf, mut vis)) = q.get_mut(e) else {
            continue;
        };
        match cmds.get(i) {
            Some(c) => apply_cmd(&atlas, c, &mut sprite, &mut tf),
            None => {
                if *vis != Visibility::Hidden {
                    *vis = Visibility::Hidden;
                }
                continue;
            }
        }
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
}

fn apply_cmd(atlas: &Atlas, c: &DrawCmd, sprite: &mut Sprite, tf: &mut Transform) {
    let Some(f) = atlas.frame(&c.name) else {
        return;
    };
    let mut rect = f.rect();
    if let Some((ox, oy, w, h)) = c.sub {
        rect = Rect::new(
            rect.min.x + ox as f32,
            rect.min.y + oy as f32,
            rect.min.x + (ox + w) as f32,
            rect.min.y + (oy + h) as f32,
        );
    }
    let new = Sprite {
        image: atlas.image.clone(),
        rect: Some(rect),
        flip_x: c.flip,
        color: c.tint,
        custom_size: c.size.map(|(w, h)| Vec2::new(w as f32, h as f32)),
        ..default()
    };
    if *sprite != new {
        *sprite = new;
    }
    let t = Vec3::new(c.left as f32, c.top as f32, c.z);
    if tf.translation != t {
        tf.translation = t;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_maps_to_logic_pixels() {
        let strip = crate::layout::Rect {
            x: 0.0,
            y: 404.0,
            w: 1920.0,
            h: 96.0,
        };
        assert_eq!(window_to_logic(Vec2::new(10.0, 300.0), strip, 2, 100), None);
        let p = window_to_logic(Vec2::new(200.0, 404.0 + 96.0 - 20.0), strip, 2, 100).unwrap();
        assert_eq!(p, Vec2::new(200.0, GROUND_BOTTOM as f32 + 10.0));
    }
}
