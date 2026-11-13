//! Probe: sprite PNG nearest-sampled + chữ tiếng Việt TTF trên nền trong suốt.
//! Báo sẵn sàng bằng `window.__CM_READY = true` sau 20 frame (giống hợp đồng của game).
use bevy::prelude::*;

const W: u32 = 1280;
const H: u32 = 600;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "cm-probe".into(),
                        transparent: true,
                        decorations: false,
                        resolution: bevy::window::WindowResolution::new(W, H),
                        canvas: Some("#bevy".into()),
                        fit_canvas_to_parent: false,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, setup)
        .add_systems(Update, ready_after_frames)
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn(Camera2d);
    let font = assets.load("fonts/probe.ttf");
    // Sprite 16x16 phóng x8 (nguyên) -> phải sắc cạnh nếu sampler nearest.
    commands.spawn((
        Sprite::from_image(assets.load("cat.png")),
        Transform::from_xyz(-400.0, -180.0, 0.0).with_scale(Vec3::splat(8.0)),
    ));
    commands.spawn((
        Sprite::from_image(assets.load("cat.png")),
        Transform::from_xyz(-250.0, -220.0, 0.0).with_scale(Vec3::splat(2.0)),
    ));
    // Text2d (thế giới)
    commands.spawn((
        Text2d::new("Chín Mạng — Ặ Ẫ Ỡ ữ ợ ỷ đ"),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(40.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.85, 0.2)),
        Transform::from_xyz(120.0, -180.0, 1.0),
    ));
    // Chữ UI (bevy_ui), cỡ small 11 px
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(16),
            top: px(16),
            padding: UiRect::all(px(8)),
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.85)))
        .with_child((
            Text::new("UI 11px: Bầy mèo đã ngủ 3 giờ 12 phút — Ặ Ẫ Ỡ"),
            TextFont {
                font: font.into(),
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn ready_after_frames(mut n: Local<u32>) {
    *n += 1;
    if *n == 20 {
        info!("probe ready after 20 frames");
        #[cfg(target_arch = "wasm32")]
        {
            let _ = js_sys::Reflect::set(
                &js_sys::global(),
                &"__CM_READY".into(),
                &true.into(),
            );
        }
    }
}
