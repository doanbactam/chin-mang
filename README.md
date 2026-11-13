# Chín Mạng

Game idle auto-battler: một bầy mèo pixel-art tự đi, tự đánh quái, nhặt đồ trên một **dải trong suốt bám đáy màn hình desktop** trong lúc bạn làm việc khác. Thỉnh thoảng ghé vào nâng cấp mèo, trang bị, và "dùng một mạng" (prestige). Rust + Bevy 0.20.1. Đích phát hành: Windows 10/11; có bản Linux và bản web (WASM/WebGL2) để demo và chụp ảnh kiểm tra. Đặc tả đầy đủ: [SPEC.md](SPEC.md).

| Thư mục | Nội dung |
|---|---|
| `crates/core` | lõi tất định (không Bevy) |
| `crates/bot` | bot + cổng cân bằng (`cm-bot gates`) |
| `crates/game` | game Bevy (overlay, UI, âm thanh, chế độ `?shot=` cho web) |
| `tools/art`, `tools/audio` | sinh sprite/atlas và âm thanh bằng code |
| `tools/verify` | build web, chụp ảnh, kiểm chứng toàn bộ |

## Yêu cầu

- Rust stable (edition 2024), Node ≥ 20.
- Linux: thư viện ALSA + udev (`libasound2-dev libudev-dev pkg-config`, xem `docs/DECISIONS.md` nếu không có sudo).

## Chạy (native)

```sh
cargo run -p chin-mang --release
```

## Build Windows

Trên Windows (MSVC):

```sh
cargo build -p chin-mang --release
# → target/release/chin-mang.exe, chạy cạnh thư mục assets/
```

Từ Linux chỉ kiểm biên dịch (không link):

```sh
rustup target add x86_64-pc-windows-gnu
CARGO_TARGET_DIR=target/win cargo check -p chin-mang --target x86_64-pc-windows-gnu
```

CI (`.github/workflows/ci.yml`) build bản release trên `windows-latest` và đính kèm artifact `chin-mang-windows` (exe + assets).

## Build web

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-bindgen-cli --version 0.2.130   # = phiên bản wasm-bindgen trong Cargo.lock
node tools/verify/build-web.mjs            # → dist/web/ (thêm --fast để build nhanh, --opt để chạy wasm-opt)
node tools/verify/lib/serve.mjs dist/web 8080
# mở http://127.0.0.1:8080/?bg=busy  (bg=white|black|busy, lang=vi|en, scale=1..4, shot=<tên ảnh>)
```

## Kiểm chứng

```sh
npm install && npx playwright install chromium-headless-shell   # một lần
node tools/verify/verify.mjs                  # toàn bộ SPEC §10.1, thoát ≠ 0 nếu lỗi
node tools/verify/verify.mjs --skip-web       # nhanh: bỏ build wasm + chụp ảnh
node tools/verify/verify.mjs --allow-blocked  # bước bị môi trường chặn → ⚠ BLOCKED (vẫn in rõ)
node tools/verify/shots.mjs --only strip-z1   # chụp lại một ảnh → docs/shots/
```

Chromium chạy headless, WebGL2 qua SwiftShader nên không cần GPU. Log từng bước: `target/verify-logs/`. Kết quả gần nhất: [docs/VERIFY.md](docs/VERIFY.md); kiểm tay trên máy thật: [docs/MANUAL-CHECK.md](docs/MANUAL-CHECK.md).

## Hyprland (Wayland)

Trên Wayland, ứng dụng không tự đặt được vị trí cửa sổ, nên cần một window rule. Cửa sổ game có class (app_id) là `chin-mang`. Dưới đây dải mặc định ở hệ số 2 (cao 96 px), bám mép dưới, rộng hết màn hình.

Hyprland cấu hình bằng Lua (bản mới, cú pháp `hl.window_rule`, theo wiki.hypr.land, mục *Configuring → Rules → Window Rules*):

```lua
hl.window_rule({
  name = "chin-mang",
  match = { class = "^(chin-mang)$" },
  float = true,
  pin = true,              -- hiện ở mọi workspace
  border_size = 0,
  no_shadow = true,
  no_blur = true,
  no_anim = true,
  no_focus = true,         -- không cướp focus (P1)
  size = { "monitor_w", 96 },
  move = { 0, "monitor_h - 96" },
})
```

Hyprland bản cũ (`hyprland.conf`, cú pháp `windowrulev2`):

```ini
windowrulev2 = float, class:^(chin-mang)$
windowrulev2 = pin, class:^(chin-mang)$
windowrulev2 = noborder, class:^(chin-mang)$
windowrulev2 = noshadow, class:^(chin-mang)$
windowrulev2 = nofocus, class:^(chin-mang)$
windowrulev2 = size 100% 96, class:^(chin-mang)$
windowrulev2 = move 0 100%-96, class:^(chin-mang)$
```

Lưu ý:
- Đổi `96` thành `48 × hệ số phóng` nếu bạn đổi hệ số.
- `no_focus`/`nofocus` làm bàn phím không vào được game (Esc không đóng bảng). Tùy phiên bản Hyprland, có thể cả việc bấm chuột vào nút cũng bị ảnh hưởng. Nếu gặp, thay bằng `no_initial_focus = true` (Lua) hoặc `noinitialfocus` (conf).
- Cú pháp window rule của Hyprland đổi khá thường xuyên. Nếu `hyprctl configerrors` báo lỗi, đối chiếu wiki theo đúng phiên bản bạn đang dùng.

## Giấy phép

Mã: MIT. Font `assets/fonts/ui*.ttf` (Inter): SIL OFL 1.1, xem `assets/fonts/LICENSE-ui.txt`. Toàn bộ sprite và âm thanh được sinh bằng code trong repo.
