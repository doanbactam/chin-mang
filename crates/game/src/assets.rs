//! Nạp asset: atlas (`art/atlas.png` + `art/atlas.json`), font UI, âm thanh (`audio/audio.json`).
//! Atlas dùng rect thủ công (tên → rect/anchor/slice), sampler nearest toàn cục (`ImagePlugin::default_nearest`).

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use std::collections::HashMap;

/// Tài liệu JSON thô (atlas.json, audio.json).
#[derive(Asset, TypePath, Debug)]
pub struct JsonDoc(pub serde_json::Value);

#[derive(Default, TypePath)]
pub struct JsonLoader;

impl AssetLoader for JsonLoader {
    type Asset = JsonDoc;
    type Settings = ();
    type Error = BevyError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _s: &(),
        _cx: &mut LoadContext<'_>,
    ) -> Result<JsonDoc, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(JsonDoc(serde_json::from_slice(&bytes)?))
    }
    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// Một frame trong atlas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// anchor tính từ góc trên-trái (px)
    pub ax: i32,
    pub ay: i32,
    /// inset 9-slice (px), 0 nếu không có
    pub slice: u32,
}

impl Frame {
    pub fn rect(&self) -> Rect {
        Rect::new(
            self.x as f32,
            self.y as f32,
            (self.x + self.w) as f32,
            (self.y + self.h) as f32,
        )
    }
}

/// Định nghĩa atlas đã phân tích (thuần, test được).
#[derive(Clone, Debug, Default)]
pub struct AtlasDef {
    pub frames: HashMap<String, Frame>,
    /// "cat_muop_walk" → 4 (đếm theo hậu tố `_i` liên tiếp từ 0)
    pub anim_len: HashMap<String, usize>,
}

impl AtlasDef {
    pub fn parse(v: &serde_json::Value) -> Result<AtlasDef, String> {
        let size = v
            .get("size")
            .and_then(|s| s.as_array())
            .ok_or("thiếu `size`")?;
        let w = size.first().and_then(|x| x.as_u64()).ok_or("size[0]")? as u32;
        let h = size.get(1).and_then(|x| x.as_u64()).ok_or("size[1]")? as u32;
        let frames_v = v
            .get("frames")
            .and_then(|f| f.as_object())
            .ok_or("thiếu `frames`")?;
        let mut frames = HashMap::with_capacity(frames_v.len());
        for (name, f) in frames_v {
            let num = |k: &str| f.get(k).and_then(|x| x.as_f64());
            let (Some(x), Some(y), Some(fw), Some(fh)) = (num("x"), num("y"), num("w"), num("h"))
            else {
                return Err(format!("frame `{name}` thiếu x/y/w/h"));
            };
            let frame = Frame {
                x: x as u32,
                y: y as u32,
                w: fw as u32,
                h: fh as u32,
                ax: num("ax").unwrap_or(fw / 2.0).round() as i32,
                ay: num("ay").unwrap_or(fh).round() as i32,
                slice: num("slice").unwrap_or(0.0) as u32,
            };
            if frame.x + frame.w > w || frame.y + frame.h > h {
                return Err(format!("frame `{name}` ra ngoài atlas"));
            }
            frames.insert(name.clone(), frame);
        }
        let mut anim_len: HashMap<String, usize> = HashMap::new();
        for name in frames.keys() {
            if let Some((base, idx)) = name.rsplit_once('_')
                && idx == "0"
            {
                let mut n = 1;
                while frames.contains_key(&format!("{base}_{n}")) {
                    n += 1;
                }
                anim_len.insert(base.to_string(), n);
            }
        }
        Ok(AtlasDef { frames, anim_len })
    }

    pub fn get(&self, name: &str) -> Option<&Frame> {
        self.frames.get(name)
    }

    pub fn anim_len(&self, base: &str) -> usize {
        self.anim_len.get(base).copied().unwrap_or(0)
    }
}

/// Atlas sẵn dùng (sau khi nạp xong).
#[derive(Resource, Clone)]
pub struct Atlas {
    pub image: Handle<Image>,
    pub def: AtlasDef,
}

impl Atlas {
    pub fn frame(&self, name: &str) -> Option<Frame> {
        self.def.get(name).copied()
    }
}

/// Font UI (thường + đậm).
#[derive(Resource, Clone)]
pub struct Fonts {
    pub regular: Handle<Font>,
    pub bold: Handle<Font>,
}

/// Âm thanh đã khai báo trong `audio/audio.json`.
#[derive(Clone, Debug)]
pub struct SoundDef {
    pub handle: Handle<AudioSource>,
    /// hệ số tuyến tính từ `gain_db` (≤ 1)
    pub gain: f32,
}

#[derive(Resource, Clone, Default)]
pub struct Sounds {
    pub map: HashMap<String, SoundDef>,
}

/// Handle đang chờ nạp.
#[derive(Resource)]
struct Pending {
    atlas_json: Handle<JsonDoc>,
    atlas_png: Handle<Image>,
    audio_json: Handle<JsonDoc>,
    reported: bool,
}

/// Trạng thái nạp: true khi atlas + font đã sẵn sàng.
#[derive(Resource, Default)]
pub struct AssetsReady(pub bool);

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<JsonDoc>()
            .register_asset_loader(JsonLoader)
            .init_resource::<AssetsReady>()
            .add_systems(Startup, start_loading)
            .add_systems(Update, finish_loading.run_if(|r: Res<AssetsReady>| !r.0));
    }
}

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Pending {
        atlas_json: server.load("art/atlas.json"),
        atlas_png: server.load("art/atlas.png"),
        audio_json: server.load("audio/audio.json"),
        reported: false,
    });
    commands.insert_resource(Fonts {
        regular: server.load("fonts/ui.ttf"),
        bold: server.load("fonts/ui-bold.ttf"),
    });
}

fn finish_loading(
    mut commands: Commands,
    mut pending: ResMut<Pending>,
    server: Res<AssetServer>,
    docs: Res<Assets<JsonDoc>>,
    fonts: Res<Fonts>,
    mut ready: ResMut<AssetsReady>,
    sounds: Option<Res<Sounds>>,
) {
    // âm thanh: không chặn khởi động
    if sounds.is_none()
        && let Some(doc) = docs.get(&pending.audio_json)
    {
        commands.insert_resource(parse_sounds(&doc.0, &server));
    }
    let loaded = |id: bevy::asset::UntypedAssetId| server.is_loaded_with_dependencies(id);
    let atlas_ok =
        loaded(pending.atlas_json.id().untyped()) && loaded(pending.atlas_png.id().untyped());
    let fonts_ok = loaded(fonts.regular.id().untyped()) && loaded(fonts.bold.id().untyped());
    if atlas_ok && fonts_ok {
        let Some(doc) = docs.get(&pending.atlas_json) else {
            return;
        };
        match AtlasDef::parse(&doc.0) {
            Ok(def) => {
                commands.insert_resource(Atlas {
                    image: pending.atlas_png.clone(),
                    def,
                });
                ready.0 = true;
            }
            Err(e) => {
                if !pending.reported {
                    error!("atlas.json lỗi: {e}");
                    pending.reported = true;
                }
            }
        }
    } else if !pending.reported {
        for h in [
            pending.atlas_json.id().untyped(),
            pending.atlas_png.id().untyped(),
            fonts.regular.id().untyped(),
        ] {
            if let Some(bevy::asset::LoadState::Failed(err)) = server.get_load_state(h) {
                error!("không nạp được asset: {err}");
                pending.reported = true;
            }
        }
    }
}

fn parse_sounds(v: &serde_json::Value, server: &AssetServer) -> Sounds {
    let mut map = HashMap::new();
    if let Some(list) = v.get("sounds").and_then(|s| s.as_array()) {
        for s in list {
            let (Some(name), Some(file)) = (
                s.get("name").and_then(|x| x.as_str()),
                s.get("file").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            let db = s
                .get("gain_db")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0)
                .min(0.0);
            map.insert(
                name.to_string(),
                SoundDef {
                    handle: server.load(format!("audio/{file}")),
                    gain: 10f32.powf(db as f32 / 20.0),
                },
            );
        }
    }
    Sounds { map }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_atlas_and_counts_anims() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{ "size": [64, 32], "frames": {
                "cat_muop_walk_0": {"x":0,"y":0,"w":16,"h":16,"ax":8,"ay":16},
                "cat_muop_walk_1": {"x":16,"y":0,"w":16,"h":16,"ax":8,"ay":16},
                "cat_muop_walk_2": {"x":32,"y":0,"w":16,"h":16,"ax":8,"ay":16},
                "ui_panel": {"x":48,"y":0,"w":16,"h":16,"ax":0,"ay":0,"slice":4},
                "proj_bone_rain": {"x":0,"y":16,"w":6,"h":8}
            } }"#,
        )
        .unwrap();
        let a = AtlasDef::parse(&v).unwrap();
        assert_eq!(a.anim_len("cat_muop_walk"), 3);
        assert_eq!(a.anim_len("cat_muop_idle"), 0);
        assert_eq!(a.get("ui_panel").unwrap().slice, 4);
        let p = a.get("proj_bone_rain").unwrap();
        assert_eq!((p.ax, p.ay), (3, 8));
    }
    #[test]
    fn rejects_out_of_bounds() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{ "size": [8, 8], "frames": { "a": {"x":4,"y":0,"w":8,"h":8} } }"#,
        )
        .unwrap();
        assert!(AtlasDef::parse(&v).is_err());
    }
}
