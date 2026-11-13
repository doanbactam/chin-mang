# Hợp đồng giữa các phần

| Hợp đồng | Nguồn sự thật | Chủ sở hữu | Người dùng |
|---|---|---|---|
| API lõi ↔ view | `crates/core/src/api.rs` + chữ ký công khai trong `crates/core/src/game.rs` | core | game, bot |
| Tên sprite | SPEC §8.3 + `tools/art/manifest.mjs` | art | game |
| Định dạng atlas | `assets/art/atlas.json`: `{ "size":[w,h], "frames": { name: {x,y,w,h,ax,ay, slice?} } }` (anchor tính từ góc trên-trái của sprite, đơn vị pixel) | art | game |
| Âm thanh | SPEC §9, `assets/audio/<tên>.ogg` (hoặc `.wav`), `assets/audio/audio.json` liệt kê tên + loại (sfx/music) | audio | game |
| Font | `assets/fonts/ui.ttf` + `assets/fonts/LICENSE-ui.txt` | audio/font | game |
| Ảnh chụp web | `?shot=<tên>` (SPEC §10.2), trang báo sẵn sàng bằng `window.__CM_READY = true` (sau ≥ 20 frame), lỗi bằng console.error | game | verify |
| Bản web | `dist/web/index.html` + `chin_mang.js` + `chin_mang_bg.wasm` + `assets/` | verify (build script) | verify |

Đổi hợp đồng nào ⇒ sửa bảng này + báo người dùng hợp đồng.
