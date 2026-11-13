# Hợp đồng giữa các phần

| Hợp đồng | Nguồn sự thật | Chủ sở hữu | Người dùng |
|---|---|---|---|
| API lõi ↔ view | `crates/core/src/api.rs` + chữ ký công khai trong `crates/core/src/game.rs` | core | game, bot |
| Tên sprite | SPEC §8.3 + `tools/art/manifest.mjs` | art | game |
| Định dạng atlas | `assets/art/atlas.json`: `{ "size":[w,h], "frames": { name: {x,y,w,h,ax,ay, slice?} } }` (anchor tính từ góc trên-trái của sprite, đơn vị pixel) | art | game |
| Âm thanh | SPEC §9, `assets/audio/<tên>.ogg` (hoặc `.wav`), `assets/audio/audio.json` liệt kê tên + loại (sfx/music) | audio | game |
| Font | `assets/fonts/ui.ttf` + `assets/fonts/LICENSE-ui.txt` | audio/font | game |
| Ảnh chụp web | `index.html?shot=<tên>&bg=white\|black\|busy&lang=vi\|en&scale=1..4` (mặc định bg=busy, lang=vi, scale=2; tên theo SPEC §10.2, danh sách trong `tools/verify/shots.mjs`). Game tự đọc `location.search`; báo sẵn sàng bằng `window.__CM_READY = true` (sau ≥ 20 frame, cảnh đã dựng xong); lỗi bằng console.error. shots.mjs fail khi: console.error / pageerror / HTTP ≥ 400 / quá 120 s / canvas trống | game | verify |
| Bản web | `node tools/verify/build-web.mjs` → `dist/web/index.html` + `chin_mang.js` + `chin_mang_bg.wasm` + `assets/`. Harness `tools/verify/web/index.html` có sẵn `<canvas id="bevy" width=1280 height=600>`, nền `?bg` vẽ bằng CSS dưới canvas. Game trên wasm PHẢI: `Window { canvas: Some("#bevy"), fit_canvas_to_parent: false, resolution: 1280×600, transparent: true, .. }`, `ClearColor(Color::NONE)`, `AssetPlugin { meta_check: AssetMetaCheck::Never }`, không vẽ nền desktop. Feature Bevy: khai báo trong `crates/game/Cargo.toml` (không có feature riêng cho wasm ngoài `bevy/web` + `wasm-bindgen =0.2.130`, `js-sys`, `web-sys`); bin `chin-mang` → wasm-bindgen `--out-name chin_mang`. Xem DECISIONS (verify, game) | verify (build script, harness) | game, verify |

Đổi hợp đồng nào ⇒ sửa bảng này + báo người dùng hợp đồng.

## Thay đổi

- 2026-11-13 · core · Không đổi chữ ký nào trong `api.rs` / `game.rs`. Ghi chú ngữ nghĩa: `Hud.max_global_stage` và `PrestigePreview` dùng max **trong mạng hiện tại** (reset về màn xuất phát khi Chín Mạng); thành tựu khu/vòng dùng max mọi thời đại. `enemies()` gồm cả quái đang chết (`anim = Death`, `hp = 0`) cho tới khi hết animation. `carton().x` bám theo `party_x + 60`. `Game` là `Send + Sync`.
- 2026-11-13 · core · **Thêm** (yêu cầu người điều phối): `AchId::Fish1M` (key `fish_1m`) và `AchId::Fish1B` (key `fish_1b`), group `wealth`, điều kiện `Stats.fish_earned ≥ 1e6 / 1e9`; thêm vào **cuối** enum, `AchId::ALL` thành `[AchId; 26]`, `achievements()` trả 26 phần tử. Bản lưu cũ (24 thành tựu) vẫn tải được. View cần chuỗi i18n `fish_1m` / `fish_1b` và icon `ach_wealth`.
- 2026-11-13 · core · Ngữ nghĩa (không đổi chữ ký): `PrestigePreview.required_global_stage` giờ là `max(25, màn xuất phát của mạng + 10)` (không còn hằng 25); `souls_gain = 0` khi `available = false`. `Command::Equip` với món đang mặc trên mèo khác sẽ hoán đổi món giữa hai mèo. `Game::load` kẹp/sửa dữ liệu không hợp lệ thay vì báo lỗi (chỉ lỗi khi JSON hỏng / thiếu trường bắt buộc / sai kiểu).
