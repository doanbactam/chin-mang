# Kết quả kiểm chứng

Cập nhật mỗi lần chạy `node tools/verify/verify.mjs`. Log đầy đủ: `target/verify-logs/<bước>.log`, tóm tắt máy đọc: `target/verify-logs/summary.json`.

**Môi trường:** máy cloud Ubuntu 24.04, 4 nhân, 7 GB RAM, không GPU, không sudo. Rust 1.100.0, Node 22.23. Chromium headless shell 158 (Playwright 1.65), WebGL 2.0 qua ANGLE → SwiftShader Vulkan. wasm-bindgen 0.2.130, binaryen `wasm-opt` 133.

## Pipeline web (đã chứng minh)

Probe `tools/verify/wasm-probe/` (Bevy 0.20.1, crate tách khỏi workspace) build wasm32, chạy qua wasm-bindgen, rồi chụp bằng Chromium headless:

- `tools/verify/wasm-probe/probe-busy.png`: nền desktop giả (CSS) hiện xuyên canvas, tức là canvas trong suốt đúng. Sprite 16×16 phóng ×8 và ×2 sắc cạnh (sampler nearest). Text2d 40 px "Chín Mạng — Ặ Ẫ Ỡ ữ ợ ỷ đ" và chữ UI 11 px không bị cắt dấu.
- `tools/verify/wasm-probe/probe-black.png`: như trên, trên nền đen.
- Thời gian tới `__CM_READY` khoảng 6 s mỗi trang. Không có console error.

Chạy lại: `node tools/verify/wasm-probe/run.mjs [--no-build]`.

## SPEC §10.1: các bước của verify.mjs

Lần chạy đầy đủ gần nhất: 2026-11-13 khoảng 11:45, lệnh `node tools/verify/verify.mjs` (không dùng cờ), kết quả **✔ PASS**, exit 0 (ok 11, fail 0, blocked 0, skipped 0), tổng 270.6 s. Lượt trước đó (11:20, `--keep-going`) FAIL ở clippy, và `strip-z5` gặp race `dist/web`. Cả hai đã được sửa.

| # | Bước | id | Trạng thái | Bằng chứng / ghi chú |
|---|---|---|---|---|
| 1 | `cargo fmt --check` | fmt | ✔ | `target/verify-logs/fmt.log` |
| 2 | `cargo clippy --workspace -D warnings` | clippy | ✔ | Lượt 11:20 còn lỗi dead_code ở `ui/kit.rs:41`, game đã sửa |
| 3 | `cargo test --workspace` | test | ✔ | `test.log` |
| 4 | Cổng cân bằng `cm-bot gates` | gates | ✔ | `gates.log`: 17/17 cổng ĐẠT (Xiêm 1.8 phút, Mập 13.7 phút, Mun 44.2 phút, boss khu 1 9.8 phút, khu 3 39.2 phút, prestige đầu 1.8 h, 24 h: 6.0 mạng và max_gs 137.8, tường dài nhất 22.4 phút, trung vị khoảng mua 19.4 s, IdleBot 8 h gs 59, 1 h game mất 0.03 s CPU) |
| 5 | `node tools/art/build-atlas.mjs` (lint) | art | ✔ | `art.log` |
| 6 | `node tools/audio/verify.mjs` | audio | ✔ | `audio.log` |
| 7 | Kiểm font (`tools/verify/fontcheck.mjs`) | font | ✔ | `font.log` |
| – | Quét `todo!`/`TODO` (§0.3, chỉ cảnh báo) | stubs | ✔ | không còn chỗ nào |
| 8 | `cargo check -p chin-mang --target x86_64-pc-windows-gnu` | win | ✔ | `CARGO_TARGET_DIR=target/win`. Không cần mingw (check không link, không có build script C cho target này). Lần đầu mất 8 phút 38 giây |
| 9 | Build wasm (`build-web.mjs`, profile `wasm-release`) | wasm | ✔ | 104 s (lần đầu 360 s); `dist/web/chin_mang_bg.wasm` 34.3 MB (chưa qua wasm-opt) |
| 10 | Ảnh chụp Chromium headless (`shots.mjs`) | shots | ✔ 18/18 | 73.9 s, không có console error. build-web giờ dựng vào thư mục tạm rồi đổi tên, để lượt chụp không thấy `dist/web` dở dang khi agent khác build song song |

## SPEC §12: Definition of Done

| # | Mục | Trạng thái | Bằng chứng |
|---|---|---|---|
| 12.1 | `verify.mjs` thoát 0 | ✔ | lượt 11:45, `target/verify-logs/summary.json` |
| 12.2 | Đủ 18 ảnh §10.2, đã tự mở và chấm theo 6 câu | ✔ | Đã mở cả 18 ảnh. So pixel với lượt 11:30 thì chỉ `offline-summary` và `panel-cat-detail` đổi, hai ảnh này đã được mở lại |
| 12.3 | Mọi nội dung §2 có hành vi thật và test lõi | (core/game tự báo) | `cargo test` ✔ |
| 12.4 | Cổng cân bằng §10.3 xanh | ✔ | bước 4 |
| 12.5 | README (chạy, build Windows, build web, Hyprland) và MANUAL-CHECK | ✔ | `README.md`, `docs/MANUAL-CHECK.md` |

## Chấm ảnh (§12.2), lượt 11:45

Các câu: (a) đọc được trên nền trắng, đen và busy; (b) phân biệt 4 mèo không cần màu; (c) không chữ cắt hoặc tràn, đủ dấu; (d) lưới 4 px; (e) pixel sắc; (f) trông như game phát hành.

| Ảnh | a | b | c | d | e | f | Ghi chú |
|---|---|---|---|---|---|---|---|
| strip-z1 / z3-boss / z5 | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | Nhãn khu nằm trên nền lót, đọc rõ trên nền busy. Boss z3 có thanh máu và đồng hồ 0:28 |
| strip-white / strip-black | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | Nền trong suốt, HUD có nền lót |
| panel-cats | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | |
| panel-cat-detail | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | Nút "Lên cấp ≺ 565" đã nằm trọn trong panel |
| panel-gear | ✔ | — | ✔ | ✔ | ✔ | ✔ | Lưới 8×5, 9/40. Độ hiếm có ký hiệu ◆◆/★ kèm màu. Có so sánh ▼ đỏ |
| panel-souls / achievements / settings | ✔ | — | ✔ | ✔ | ✔ | ✔ | Thành tựu "Đã mở 8/26": danh sách liệt kê ở §2.10 đếm ra đúng 26 mục, còn tiêu đề ghi 24, nên 26 khớp với danh sách |
| offline-summary | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | Thẻ nằm dưới nhãn khu, không che |
| tutorial-bubble / box | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | Có hộp carton trên đất |
| vi-diacritics | ✔ | — | ✔ | ✔ | ✔ | ✔ | Hộp mẫu "SO VỚI ĐỒ ĐANG MẶC" (Ặ) không bị cắt dấu |
| en | ✔ | — | ✔ | ✔ | ✔ | ✔ | |
| scale1 / scale3 | ✔ | ~ | ✔ | ✔ | ✔ | ✔ | Ở hệ số 1 mèo rất nhỏ; dải cao 48 px và 144 px đúng hệ số |

Ảnh probe: [`tools/verify/wasm-probe/probe-busy.png`](../tools/verify/wasm-probe/probe-busy.png).
