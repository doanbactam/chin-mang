# CHÍN MẠNG — SPEC v1.0

> **Thể loại:** idle auto-battler trên một **dải trong suốt bám đáy màn hình desktop**. Một bầy mèo pixel-art tự đi, tự đánh quái, nhặt đồ trong lúc người chơi làm việc khác; thỉnh thoảng ghé vào nâng cấp.
> **Engine:** Bevy 0.20.1 + Rust (edition 2024). **Đích phát hành:** Windows 10/11 (desktop). **Máy dev:** Linux (Hyprland/X11). **Bản web (WASM/WebGL2):** dùng để AI tự chụp ảnh kiểm tra và làm demo.
> **Người đọc:** AI coding agent làm việc tự chủ. Mọi thứ cần để làm xong game nằm ở đây.

## 0. Luật thi hành
1. **Ưu tiên khi mâu thuẫn:** §12 Definition of Done → các bảng số → phần mô tả. Thiếu chi tiết: tự chọn phương án phục vụ §1, ghi 1 dòng vào `docs/DECISIONS.md`, làm tiếp. Không dừng để hỏi.
2. **Số cân bằng là điểm khởi đầu**, được chỉnh để qua cổng ở §10.3; ghi thay đổi vào `docs/DECISIONS.md`. Danh sách nội dung và luật chơi không được cắt.
3. **Bị coi là chưa làm:** thực thể gameplay vẽ bằng hình khối/emoji/chữ thay vì sprite pixel-art qua pipeline §8; nút không làm gì; `todo!()`, `unimplemented!()`, `TODO`, `placeholder`, hàm rỗng; báo xong mà chưa chạy `node tools/verify/verify.mjs` và chưa tự mở ảnh chụp ra xem.
4. **IP gốc.** Lấy cảm hứng *thể loại* từ các game idle chạy trên taskbar. Không dùng tên, nhân vật, hình ảnh, bố cục UI, âm thanh của game có sẵn. Mọi asset tự sinh bằng code trong repo.
5. **Không đọc input toàn hệ thống** (không keylogger, không global hook bàn phím/chuột). Chỉ nhận input trong cửa sổ của game. Ngoại lệ duy nhất: §7.6 Ghost mode được đọc *vị trí* con trỏ (không đọc nút/phím) trên Windows.

## 1. Trụ cột
| # | Trụ cột | Nghĩa là |
|---|---|---|
| P1 | **Không làm phiền** | Dải mỏng (mặc định 96 px), không cướp focus, không modal, âm thanh mặc định nhỏ, < 1% CPU khi chạy nền. |
| P2 | **Liếc là hiểu** | Nhìn dải 1 giây biết: đang ở đâu, đội khỏe không, có gì đáng bấm không. |
| P3 | **Luôn có việc tiếp theo** | Mỗi lần mở bảng luôn có ≥ 1 nâng cấp mua được hoặc mục tiêu gần; thanh "mục tiêu kế" hiện rõ. |
| P4 | **Tiến lên, không mất trắng** | Thua chỉ lùi 1 màn. Vắng mặt chỉ có thưởng, không có phạt. |
| P5 | **Dễ thương và có hồn** | Mèo pixel 16px có tính cách: ngáp khi đứng yên, nhìn theo con trỏ, ngủ khi rảnh. |

## 2. Vòng chơi

### 2.1 Thế giới
- Thế giới là một **làn 1 chiều**. Tọa độ `x` tính bằng *pixel logic* (1 px logic = 1 pixel sprite trước khi phóng). Mặt đất ở `y = 0`; dải cao **48 px logic**.
- Đội mèo đi từ trái sang phải với tốc độ `PARTY_SPEED = 24 px/s`. Camera của view bám đội (đội đứng ở ~35% chiều rộng màn hình).
- **Khu (zone)** 1..5, mỗi khu 10 **màn (stage)**. Màn 10 của mỗi khu là boss. Sau khu 5 thế giới lặp lại khu 1..5 với **Vòng (cycle)** +1 (độ khó tiếp tục tăng theo công thức, không có trần).
- `global_stage = (cycle * 5 + (zone - 1)) * 10 + (stage - 1)` (bắt đầu từ 0).

| Khu | Tên (vi / en) | Quái | Boss |
|---|---|---|---|
| 1 | Hẻm Nhỏ / Back Alley | chuột, gián | Vua Chuột (`rat_king`) |
| 2 | Mái Nhà / Rooftops | quạ, gián, chuột | Quạ Chúa (`crow_lord`) |
| 3 | Chợ Cá / Fish Market | rắn, chuột, cú | Lươn Điện (`eel`) |
| 4 | Công Viên / City Park | chó hoang, cú, bọ ve | Chó Ngao (`mastiff`) |
| 5 | Bãi Phế Liệu / Junkyard | robot hút bụi, bọ ve, chó hoang | Robot Hút Bụi Khổng Lồ (`vacuum_titan`) |

### 2.2 Màn chơi
- Mỗi màn thường là **1 đợt** gồm `3 + floor(stage/3)` quái (tối đa 7) chọn ngẫu nhiên tất định từ danh sách khu. Quái xuất hiện ở bên phải ngoài màn hình (`party_x + 220`) và đi về phía đội.
- Diệt hết đợt → nhặt đồ rơi (tự bay về đội trong 0.6 s) → **qua màn**.
- Màn boss: boss + 2 quái thường. Có **thời hạn 30 s**; hết giờ hoặc đội chết hết = thất bại.
- **Thất bại** (đội chết hết, hoặc hết giờ boss): lùi về màn trước (`stage - 1`, tối thiểu màn 1 của khu hiện tại), toàn đội hồi sinh đầy máu sau 3 s. Nếu thất bại ở boss: bật cờ `farming = true` (ở lại màn 9 cày) cho tới khi người chơi bấm "Thử lại boss" hoặc tự động thử lại mỗi 60 s.
- Mèo chết giữa trận thì nằm "ngủ" (sprite death frame cuối) cho tới hết màn, rồi hồi sinh với 50% máu. Giữa các màn mèo hồi 25% máu tối đa.

### 2.3 Mèo (tối đa 4 trong đội)
| id | Tên | Vai | Tầm (px) | HP | DMG | Đòn/s | Mở khóa |
|---|---|---|---|---|---|---|---|
| `muop` | Mướp | cận chiến, đánh nhanh | 14 | 100 | 8 | 1.25 | có sẵn |
| `xiem` | Xiêm | tầm xa, ném xương cá (đạn bay) | 90 | 65 | 7 | 1.0 | đạt khu 1 màn 5, giá 50 cá |
| `map` | Mập | đỡ đòn: quái ưu tiên đánh Mập | 12 | 260 | 4 | 0.7 | đạt khu 2 màn 1, giá 600 cá |
| `mun` | Mun | phép: đạn nổ vùng bán kính 18 px | 75 | 60 | 11 | 0.5 | đạt khu 3 màn 1, giá 8 000 cá |

- Xếp hàng theo thứ tự cố định: Mập trước, Mướp, Xiêm, Mun sau (cách nhau 14 px).
- **Cấp mèo:** `level` 1..∞. Giá lên cấp `cost = base_cost * 1.13^(level-1)` cá với `base_cost`: muop 10, xiem 20, map 40, mun 60. Chỉ số: `stat = base * 1.10^(level-1) * mult_gear * mult_talent * mult_soul`.
- **Kỹ năng tự động** (tự dùng khi hồi xong, cần có quái trong tầm):
  - Mướp — *Cào Liên Hoàn*: 3 đòn liên tiếp, mỗi đòn 120% DMG. Hồi 6 s.
  - Xiêm — *Mưa Xương Cá*: 5 xương rơi trong vùng 40 px quanh quái gần nhất, mỗi xương 80% DMG. Hồi 9 s.
  - Mập — *Ngồi Đè*: choáng mọi quái trong 20 px 1.5 s, khiêu khích. Hồi 10 s.
  - Mun — *Lời Nguyền Mèo Đen*: quái mục tiêu nhận thêm 30% sát thương trong 5 s. Hồi 8 s.
- **Tài năng (talent):** mỗi mèo 3 tài năng bị động, mỗi cái 0..5 bậc. Mèo nhận **1 điểm tài năng mỗi 10 cấp**. Bậc tài năng mở cho mọi mèo:
  - `power`: +8% DMG/bậc · `vigor`: +10% HP/bậc · tài năng thứ 3 riêng: muop `frenzy` +6% tốc đánh/bậc; xiem `pierce` xương xuyên thêm 1 quái mỗi 2 bậc (làm tròn lên); map `thorns` phản 6% sát thương nhận/bậc; mun `hex` +6% sát thương vùng & +0.5 s nguyền/bậc.

### 2.4 Quái
Chỉ số cơ sở nhân `1.115^global_stage` (HP, DMG). Phần thưởng cá nhân `1.10^global_stage`.

| id | Tên | Ghi chú | HP | DMG | Đòn/s | Tốc độ | Tầm | Cá |
|---|---|---|---|---|---|---|---|---|
| `rat` | Chuột | cơ bản | 22 | 3 | 1.0 | 20 | 10 | 2 |
| `roach` | Gián | nhanh, yếu | 12 | 2 | 1.4 | 34 | 8 | 1 |
| `crow` | Quạ | bay (vẽ cao hơn 10 px), cận chiến lao xuống | 18 | 4 | 0.9 | 28 | 10 | 3 |
| `snake` | Rắn | đòn gây độc 2 s (40% DMG/s) | 30 | 3 | 0.8 | 16 | 12 | 3 |
| `owl` | Cú | tầm xa 70 px, bắn lông | 20 | 5 | 0.6 | 14 | 70 | 4 |
| `dog` | Chó hoang | trâu | 70 | 6 | 0.8 | 18 | 12 | 6 |
| `tick` | Bọ Ve | hút máu: hồi 50% sát thương gây ra | 26 | 4 | 1.0 | 22 | 8 | 4 |
| `vacuum` | Robot Hút Bụi | rất trâu, chậm | 140 | 7 | 0.5 | 10 | 14 | 10 |

Boss (HP ×1, sprite 32×32):
| id | HP | DMG | Đòn/s | Tầm | Đòn đặc biệt (mỗi 6 s) | Cá |
|---|---|---|---|---|---|---|
| `rat_king` | 400 | 9 | 0.8 | 16 | gọi 2 chuột | 60 |
| `crow_lord` | 520 | 11 | 0.9 | 16 | lao tới đánh mèo đứng sau cùng 200% DMG | 90 |
| `eel` | 650 | 10 | 0.7 | 30 | phóng điện vùng 40 px, choáng 1 s | 120 |
| `mastiff` | 900 | 14 | 0.7 | 16 | gầm: +30% tốc đánh bản thân 4 s | 160 |
| `vacuum_titan` | 1200 | 16 | 0.6 | 20 | hút: kéo cả đội lại gần 20 px, choáng 0.8 s | 220 |

### 2.5 Tài nguyên
| id | Tên | Nguồn | Dùng |
|---|---|---|---|
| `fish` | Cá khô | quái, boss, hộp carton | lên cấp mèo, mở khóa mèo |
| `yarn` | Cuộn len | 15% quái rơi `1 + floor(global_stage/10)`; boss rơi 5×; rã đồ | nâng cấp trang bị |
| `soul` | Hồn mèo | chỉ từ Chín Mạng (§2.8) | cửa hàng Hồn mèo |
| `catnip` | Catnip (lượt) | hộp carton | không bắt buộc: dùng lượt để bỏ hồi chiêu catnip |

### 2.6 Trang bị
- 3 ô mỗi mèo: `collar` (vòng cổ: +HP%), `bell` (chuông: +tốc đánh% và +chí mạng%), `claws` (móng: +DMG%).
- Độ hiếm: `common` (xám), `rare` (xanh), `epic` (tím), `legendary` (vàng). Hệ số: 1 / 1.6 / 2.5 / 4. Tỉ lệ rơi theo thứ tự 70/22/7/1 %.
- Quái thường rơi trang bị 3%, boss 100% (tối thiểu `rare`). Mỗi món có `ilvl = global_stage` lúc rơi.
- Chỉ số món: `collar: hp% = (5 + ilvl*0.6) * rarity_mult`; `claws: dmg% = (4 + ilvl*0.5) * rarity_mult`; `bell: aspd% = (2 + ilvl*0.12) * rarity_mult` và `crit% = 2 * rarity_mult` (chí mạng ×2 sát thương, trần 60%).
- Nâng cấp món: `+1..+10`, mỗi bậc +10% chỉ số món, giá `yarn = 5 * (bậc+1)^2 * rarity_mult` (làm tròn).
- Rã món: được `yarn = 3 * rarity_mult * (1 + upgrade)` (làm tròn).
- Kho chứa 40 món. Kho đầy: món mới rơi tự rã. **Tự rã** (cài đặt): rã ngay các món có độ hiếm ≤ ngưỡng chọn (tắt / thường / hiếm).
- Nút **"Mặc đồ tốt nhất"** cho từng mèo (so theo điểm `power_score`, xem §2.6.1).
- 2.6.1 `power_score(item) = tổng phần trăm chỉ số sau nâng cấp` (crit tính ×1.5).

### 2.7 Hộp carton và catnip
- **Hộp carton** rơi xuống dải mỗi `180..360 s` (ngẫu nhiên tất định), nằm 90 s rồi biến mất (không phạt). Bấm vào để mở: 50% cá = 60 s thu nhập hiện tại, 25% len = `10 + global_stage`, 15% trang bị (`rare`+), 10% +1 lượt catnip.
- **Catnip (kỹ năng người chơi):** +50% tốc đánh, +40% tốc chạy toàn đội trong 12 s. Hồi 120 s. Dùng 1 lượt catnip để dùng ngay khi đang hồi.
- **Tự dùng catnip**: mở bằng cửa hàng Hồn mèo (§2.8).

### 2.8 Chín Mạng (prestige)
- Mở khi `max_global_stage ≥ 25` (khu 3 màn 6). Nút "Dùng một mạng".
- Nhận `soul = floor( (max_global_stage - 20)^1.5 / 4 )` (tối thiểu 1) + thưởng mạng đầu: không có.
- Reset: cấp mèo về 1, mèo đã mở khóa giữ nguyên nhưng về cấp 1, cá về 0, len giữ 25%, trang bị **xóa hết trừ món đang mặc `legendary`**, tài năng reset (điểm trả lại theo cấp), vị trí về khu 1 màn 1 (hoặc màn bắt đầu từ cửa hàng). Giữ: hồn mèo, cửa hàng hồn, thành tựu, thống kê, cài đặt.
- `lives_used` tăng 1. Mạng thứ 1..9 có tên (ví dụ "Mạng thứ Hai") và mỗi mạng mở một **dấu ấn** cố định +5% cá toàn cục (cộng dồn). Sau mạng 9 vẫn prestige được (không thêm dấu ấn).
- **Cửa hàng Hồn mèo** (mỗi dòng: tên, hiệu ứng mỗi bậc, giá bậc n = `base * growth^n`):

| id | Hiệu ứng / bậc | Bậc tối đa | base | growth |
|---|---|---|---|---|
| `claw_sharpen` | +15% DMG toàn đội | ∞ | 1 | 1.35 |
| `thick_fur` | +15% HP toàn đội | ∞ | 1 | 1.35 |
| `fat_wallet` | +20% cá | ∞ | 2 | 1.4 |
| `long_nap` | +2 h giới hạn offline | 4 | 3 | 2.0 |
| `auto_catnip` | tự dùng catnip khi hồi xong | 1 | 5 | — |
| `head_start` | bắt đầu mạng mới ở màn `+5*bậc` | 5 | 4 | 1.8 |
| `lucky_paw` | +20% tỉ lệ rơi trang bị | 10 | 2 | 1.5 |

### 2.9 Vắng mặt (offline)
- Lưu `saved_at` (unix giây). Khi mở lại: `elapsed = min(now - saved_at, offline_cap)`, `offline_cap = 8 h + long_nap`. Bỏ qua nếu `elapsed < 60 s`.
- Thưởng: `fish = fish_rate * elapsed * 0.6`, `yarn = yarn_rate * elapsed * 0.6`, trong đó `fish_rate`/`yarn_rate` là trung bình thu nhập/giây thực tế trong 300 s gần nhất (lưu trong save; nếu chưa có dữ liệu thì ước tính từ màn hiện tại). **Không** tiến màn khi offline.
- Bonus "Vừa ngủ dậy": `x2 cá` trong `min(elapsed/60, 480) s`.
- Hiện **màn tóm tắt vắng mặt** (§7.4) — chỉ có thưởng, không có phạt.

### 2.10 Thành tựu (26, lưu cục bộ)
Đạt khu 2/3/4/5, hoàn thành vòng 1; hạ mỗi boss lần đầu (5); mở khóa mỗi mèo (3); mèo bất kỳ cấp 25/50/100; nhặt trang bị `legendary` đầu tiên; nâng 1 món lên +10; dùng mạng thứ 2/5/9; mở 10/100 hộp carton; dùng catnip 50 lần; tổng cá kiếm được 1e6 / 1e9. Mỗi thành tựu có id, tên vi/en, mô tả, icon chung theo nhóm. Có trang thành tựu trong bảng Cài đặt.

## 3. Hợp đồng lõi (crate `cm-core`)
- **Không phụ thuộc Bevy** hay I/O. Thuần, **tất định**: cùng `seed` + cùng chuỗi lệnh ở cùng tick → cùng trạng thái (có test hash).
- Bước cố định `DT = 1/30 s`. Không dùng `HashMap` để lặp (dùng `Vec`/`BTreeMap`). PRNG tự viết (SplitMix64/PCG) lưu trong state.
- API công khai: xem `crates/core/src/api.rs` — **đây là hợp đồng giữa lõi và view**. Đổi hợp đồng thì phải cập nhật `docs/CONTRACT.md` cùng lúc.
- Save: `serde_json`, có `version: u32`, migrate được. Lưu dữ liệu tối thiểu để tái tạo.
- Định dạng số: `fmt_num(f64) -> String`: `< 1000` → số nguyên; rồi `1.23K`, `45.6M`, `789B`, `1.00T`, sau đó `aa, ab, …` (3 chữ số có nghĩa).

## 4. Bot và cân bằng (crate `cm-bot`)
- `GreedyBot`: mỗi 1 s game: mua lên cấp rẻ nhất nhân trọng số vai trò, mở khóa mèo ngay khi đủ, mặc đồ tốt nhất, nâng đồ đang mặc khi dư len, tiêu điểm tài năng, dùng catnip khi có, mở hộp ngay, prestige khi soul nhận được ≥ `max(3, 1.0 * soul hiện có)`, mua cửa hàng hồn rẻ nhất.
- `IdleBot`: chỉ lên cấp mỗi 30 phút game (mô phỏng người chơi treo máy).
- Chạy headless nhanh: **1 giờ game < 2 s CPU** ở release.

## 5. Kiến trúc Bevy (crate `chin-mang`)
- Core là một `Resource`; `FixedUpdate` 30 Hz gọi `step()`. View đồng bộ sprite theo **id thực thể** của core, nội suy vị trí giữa 2 tick.
- **Pixel-perfect:** thế giới render vào texture thấp `W_logic × 48` rồi phóng **hệ số nguyên** (1–4, mặc định 2) với sampler nearest. DPI lẻ (125/150%) làm tròn về hệ số nguyên gần nhất. Không sprite nào ở vị trí lẻ pixel logic.
- **UI** (`bevy_ui`) vẽ ở độ phân giải màn hình, dùng 9-slice từ atlas phóng hệ số nguyên, chữ TTF (§8.4). Bộ widget riêng: `Panel, Button, IconButton, Tab, Tooltip, Bar, NumberLabel, Toggle, Slider, Slot` trong `crates/game/src/ui/kit.rs`; mọi khoảng cách/màu/cỡ chữ lấy từ `crates/game/src/ui/tokens.rs`.
- `bevy_egui` **chỉ** cho bảng debug (feature `debug`), không có trong bản phát hành.
- Tiết kiệm tài nguyên: `WinitSettings` chế độ tiết kiệm: 30 fps khi có hoạt động, 10 fps khi đội đang đi không có gì xảy ra, 2 fps khi cửa sổ bị che/thu nhỏ; core vẫn chạy đủ 30 tick/s (bù tick khi frame thưa). Tắt mọi plugin 3D/PBR không dùng.
- Mục tiêu: **< 1% CPU** trung bình và **< 150 MB RAM** trên máy 4 nhân khi treo 10 phút (đo trên máy thật ở Windows; trên Linux đo bằng `ps`).

## 6. Cửa sổ overlay
- Một cửa sổ: `transparent: true`, `decorations: false`, `resizable: false`, `window_level: AlwaysOnTop`, `skip_taskbar: true` (nếu Bevy hỗ trợ), `ClearColor(NONE)`; composite alpha phù hợp nền tảng.
- Chiều cao: dải `48 * scale` px. Khi mở bảng (§7.3), cửa sổ **cao thêm** đúng chiều cao bảng và dịch lên để mép dưới đứng yên; đóng bảng thì co lại. Rộng = rộng màn hình đã chọn (hoặc 50%/75%/100% theo cài đặt).
- Vị trí: bám mép dưới **vùng làm việc** (trên taskbar) của màn hình đã chọn. Windows: lấy work area bằng `windows-sys` (`SystemParametersInfoW(SPI_GETWORKAREA)` / `MonitorFromWindow`+`GetMonitorInfoW`). Linux X11: mép dưới màn hình trừ 0. Wayland: client không đặt được vị trí → README có snippet window rule Hyprland (`float`, `pin`, `noborder`, `noshadow`, `nofocus`, `move`, `size`).
- Tùy chọn "Mép trên" đặt dải ở mép trên.
- **Bản web:** canvas cố định 1280×600, nền giả desktop (ảnh nền chọn bằng `?bg=white|black|busy`), dải ở đáy canvas, bảng mở ngay trên dải. Cùng mã UI.

## 7. UI/UX (gọn, chuẩn phát hành)

### 7.1 Token
- Lưới **4 px** (khoảng cách 4/8/12/16). Bo góc theo 9-slice pixel. Chữ: 3 cỡ — `small 11 px`, `body 13 px`, `title 16 px` (nhân `ui_scale` 1.0/1.25/1.5 khi bật "Chữ lớn"). Số dùng cùng font, căn phải, tabular nếu font hỗ trợ.
- Màu UI lấy từ bảng màu §8.2. Độ hiếm luôn kèm **ký hiệu** (◆ thường không viền, viền đơn hiếm, viền kép sử thi, viền + lấp lánh huyền thoại) — không chỉ dựa vào màu.
- Mọi chữ/thanh nằm trên **nền lót** (`ui_plate`) hoặc có viền chữ 1 px — đọc rõ trên nền trắng, đen và nền nhiều chi tiết.

### 7.2 Dải chính (HUD, cao 48 px logic)
- Trái: cụm tiền tệ dọc 2 dòng trên nền lót: `ic_fish 1.23K` và `ic_yarn 45` (soul hiện khi > 0).
- Giữa-trên: nhãn `Khu 2 · 7/10` + thanh tiến độ đợt mảnh (2 px logic). Màn boss: thanh máu boss + đồng hồ 30 s.
- Mỗi mèo: thanh máu 1 px logic ngay trên đầu (chỉ hiện khi < 100% hoặc đang đánh).
- Phải: nút `catnip` (icon + vòng hồi chiêu, số lượt), nút `túi` (mở bảng, chấm đỏ khi có nâng cấp mua được), nút `cài đặt`.
- Số sát thương nổi (chữ pixel từ atlas `font_digits`), chí mạng màu vàng to hơn. Đồ rơi bay về đội.
- Hover/click vào mèo: tooltip nhỏ (tên, cấp, HP, DMG) — không mở bảng. Double-click mèo: mở bảng ở tab Mèo, chọn mèo đó.
- Mèo đứng gần con trỏ (trong 24 px) quay đầu nhìn con trỏ (chỉ cosmetic).

### 7.3 Bảng (mở ngay trên dải, rộng 360 px, cao tối đa 420 px, neo theo nút túi)
Thanh tiêu đề: 4 tab icon + chữ ngắn: **Mèo · Đồ · Hồn · Thành tựu**, nút đóng. Esc / click ra ngoài cửa sổ (mất focus) đóng bảng. Không có cửa sổ con modal; xác nhận dùng nút giữ-2-giây (hold-to-confirm) hoặc nút xác nhận inline.
- **Mèo:** danh sách 4 hàng (portrait, tên, cấp, DPS rút gọn, nút `Lên cấp — giá` có chế độ `×1 / ×10 / Max` chuyển ở đầu tab). Mèo khóa hiện điều kiện + giá. Chọn một mèo mở chi tiết inline: chỉ số, 3 ô trang bị, 3 tài năng (nút +, bậc x/5), kỹ năng tự động (icon + mô tả).
- **Đồ:** lưới ô 8×5 (ô 32 px), lọc theo ô/độ hiếm, sắp xếp; chọn món → so sánh với món đang mặc của mèo đang chọn (mũi tên xanh/đỏ + số), nút Mặc / Nâng (giá len) / Rã. Nút "Mặc đồ tốt nhất (cả đội)". Cài đặt tự rã.
- **Hồn:** số hồn, nút "Dùng một mạng" (giữ 2 s) kèm số hồn sẽ nhận và điều kiện; danh sách cửa hàng hồn; dải 9 dấu ấn mạng.
- **Thành tựu:** lưới 26 icon (khóa = mờ + ổ khóa), tooltip tên + mô tả.
- **Thanh "Mục tiêu kế"** ở đáy bảng: một dòng (vd: "Hạ Vua Chuột — khu 1 màn 10", "Mở Mập: còn 230 cá").

### 7.4 Màn tóm tắt vắng mặt
Thẻ nhỏ trên dải khi mở game (sau ≥ 60 s vắng): "Bầy mèo đã ngủ 3 giờ 12 phút" + cá/len nhận được + buff vừa ngủ dậy; nút "Nhận" (tự nhận sau 10 s nếu không bấm).

### 7.5 Cài đặt (tab trong bảng, mở bằng nút bánh răng)
Ngôn ngữ (Tiếng Việt / English, mặc định theo locale hệ thống), hệ số phóng (Tự động / 1× / 2× / 3× / 4×), chiều rộng dải (50/75/100%), mép (dưới/trên), màn hình (danh sách), độ mờ dải (40–100%), Ghost mode (Windows), luôn trên cùng, chữ lớn, âm lượng tổng/nhạc/hiệu ứng (mặc định 30/0/40 — nhạc tắt mặc định), giới hạn FPS (10/20/30), tự khởi động cùng Windows (registry `HKCU\...\Run`, chỉ Windows), tự rã đồ, xuất/nhập save (copy chuỗi base64 vào clipboard nếu khả dụng; nếu không thì ghi file cạnh save), "Xóa tiến trình" (giữ 3 s), Thoát game.

### 7.6 Ghost mode (Windows)
Bật: toàn dải click-xuyên-qua (hit test tắt) và mờ 60%. Mỗi 100 ms đọc vị trí con trỏ (`GetCursorPos`); khi con trỏ nằm trên vùng nút HUD bên phải thì bật lại hit test cho tới khi rời đi. Trên Linux mục này ẩn.

### 7.7 Hướng dẫn lần đầu
3 gợi ý ngắn dạng bong bóng trên dải (không chặn): (1) "Bầy mèo tự đánh. Bấm túi để nâng cấp." khi có 10 cá lần đầu; (2) "Mở khóa Xiêm!" khi đủ điều kiện; (3) "Hộp carton! Bấm để mở." khi hộp đầu tiên rơi. Mỗi gợi ý hiện 1 lần, lưu trong save.

### 7.8 i18n
Toàn bộ chuỗi trong `crates/game/src/i18n/{vi,en}.rs` (hoặc `.ron`), key giống nhau, có test so khớp key và test font chứa đủ glyph (§8.4).

## 8. Art bible và pipeline
### 8.1 Phong cách
Pixel art **16 px** (mèo, quái), **32 px** (boss), viền tối 1 px, ánh sáng từ trên-trái, bóng đổ elip dưới chân. Mèo **dáng tròn, đầu to (đầu ≈ 55% chiều cao)**, mắt 2 px, mỗi vai phân biệt bằng **dáng + phụ kiện** chứ không chỉ màu lông: Mướp (vằn cam, khăn đỏ quàng cổ), Xiêm (kem + mặt/tai/đuôi nâu đậm, túi đeo chéo đựng xương cá), Mập (xám tròn to 16 px ngang, mũ bảo hiểm nồi), Mun (đen, mắt vàng, mũ phù thủy nhỏ tím).

### 8.2 Bảng màu
Dùng `tools/art/pixel.mjs` (bảng màu 34 màu có sẵn, mỗi màu một ký tự). Được thêm tối đa 8 màu cho lông mèo (ghi trong `pixel.mjs` + DECISIONS). Không màu nào ngoài bảng (lint).

### 8.3 Pipeline
- `tools/art/manifest.mjs` (danh sách tên **bắt buộc**), `tools/art/sprites/*.mjs` (mỗi nhóm một file), `tools/art/build-atlas.mjs` → `assets/art/atlas.png` + `assets/art/atlas.json` (`{ size:[w,h], frames: { name: {x,y,w,h,ax,ay} } }`, anchor tính từ góc trên-trái của sprite, mặc định đáy-giữa cho unit/boss/drop, giữa cho fx/proj, trên-trái cho ui) + contact sheet `docs/art/contact-<group>.png` phóng ×4 trên nền ca rô.
- Lint (thoát ≠ 0 nếu lỗi): đúng kích thước khai báo, chỉ màu trong bảng, có viền 1 px với unit/boss, không frame trùng hệt nhau trong cùng 1 animation, mọi tên trong manifest có sprite.

**Tên sprite (hợp đồng với view):**
- Mèo `cat_{muop|xiem|map|mun}_{anim}_{i}` 16×16: `idle` 2, `walk` 4, `attack` 3, `hurt` 1, `death` 3, `sleep` 2, `skill` 3; `portrait_{id}` 16×16.
- Quái `en_{id}_{anim}_{i}` 16×16: `walk` 4, `attack` 2, `hurt` 1, `death` 3.
- Boss `boss_{id}_{anim}_{i}` 32×32: `walk` 4, `attack` 3, `special` 3, `hurt` 1, `death` 4.
- Đạn `proj_{fishbone|curse|feather|spark}_{i}` (2 frame mỗi loại, ≤ 8×8); `proj_bone_rain` 1.
- FX `fx_{hit|slash|aoe|poof|levelup|catnip|stun|poison|crit}_{i}` (3–5 frame, ≤ 24×24); `shadow_s` 10×3, `shadow_l` 20×4.
- Rơi đồ `drop_fish`, `drop_yarn`, `drop_gear_{common|rare|epic|legendary}` 8×8; hộp `box_closed` 14×12, `box_open_{0..2}` 14×14.
- Mặt đất mỗi khu `ground_z{1..5}_{0..3}` 16×6 (lát ngang, mép trên có cỏ/mái/ván/sỏi…); đồ trang trí nền `deco_z{1..5}_{0..3}` ≤ 32×24 (thùng rác, ống khói, sạp cá, ghế đá, lốp xe…).
- Icon 12×12: `ic_{fish|yarn|soul|catnip|bag|settings|close|lock|up|cats|gear|lives|trophy|sort|filter|check|back}`; icon trang bị 16×16 `item_{collar|bell|claws}_{common|rare|epic|legendary}`; icon kỹ năng `skill_{muop|xiem|map|mun}` 16×16; tài năng `talent_{power|vigor|frenzy|pierce|thorns|hex}` 12×12; icon cửa hàng hồn `soul_{id}` 12×12; dấu ấn mạng `life_{1..9}` 10×10; thành tựu `ach_{zone|boss|cat|level|loot|lives|box|wealth}` 16×16.
- UI 9-slice (đường cắt 4 px, khai báo `slice: 4` trong atlas.json): `ui_panel` 16×16, `ui_plate` 12×12 (nền lót bán trong suốt — dùng màu tối + lưới chấm), `ui_btn`, `ui_btn_hover`, `ui_btn_press`, `ui_btn_disabled` 12×12, `ui_tab`, `ui_tab_active` 12×12, `ui_slot`, `ui_slot_{rare|epic|legendary}` 16×16, `ui_tooltip` 12×12, `ui_bar_bg` 8×4, `ui_bar_{hp|xp|boss|cd}` 8×4.
- Chữ số pixel `font_digits` cho số sát thương: glyph `0-9 . K M B T + -` 4×6 (atlas tên `fd_{c}` với c ∈ `0..9, dot, K, M, B, T, plus, minus`).

### 8.4 Font chữ
TTF giấy phép OFL/CC0 nằm trong `assets/fonts/` kèm file license. Phải chứa **mọi ký tự** xuất hiện trong chuỗi vi/en (kiểm bằng test Rust dùng `ttf-parser`). Ảnh chụp chứng minh dấu chồng (Ặ, Ẫ, Ỡ) không bị cắt ở cỡ `small`.

## 9. Âm thanh
Sinh bằng code (`tools/audio/*.mjs`, không tải file ngoài) → `assets/audio/*.ogg` hoặc `.wav`. Danh sách: `hit_{1..3}`, `crit`, `cat_attack_{muop|xiem|map|mun}`, `skill_{muop|xiem|map|mun}`, `enemy_die`, `boss_appear`, `boss_die`, `cat_down`, `level_up`, `unlock`, `coin`, `yarn`, `gear_drop`, `gear_legendary`, `box_land`, `box_open`, `catnip`, `ui_click`, `ui_tab`, `ui_error`, `prestige`, `achievement`, `stage_clear`, `meow_{1..4}` (tiếng mèo synth dễ thương), nhạc `music_calm` (loop ≥ 60 s, êm, lofi chiptune). Mọi âm ngắn, âm lượng chuẩn hóa (−16 LUFS ± 2 xấp xỉ bằng RMS), không âm chói > 6 kHz kéo dài. Mặc định nhỏ (P1).

## 10. Kiểm chứng
### 10.1 `node tools/verify/verify.mjs`
Chạy theo thứ tự, thoát ≠ 0 nếu bước nào lỗi: `cargo fmt --check` → `cargo clippy --workspace -- -D warnings` (trừ game nếu cần feature) → `cargo test --workspace` → bot balance gates (`cargo run -p cm-bot --release -- gates`) → `node tools/art/build-atlas.mjs` (lint) → `node tools/audio/verify.mjs` → kiểm font → `cargo check -p chin-mang --target x86_64-pc-windows-gnu` (nếu đã cài target) → build wasm (`wasm-bindgen`) → chụp ảnh bằng Chromium headless (`tools/verify/shots.mjs`) không có lỗi console.
### 10.2 Ảnh chụp bắt buộc (`docs/shots/`)
`strip-z1.png, strip-z3-boss.png, strip-z5.png` (nền busy), `strip-white.png`, `strip-black.png`, `panel-cats.png`, `panel-cat-detail.png`, `panel-gear.png`, `panel-souls.png`, `panel-achievements.png`, `panel-settings.png`, `offline-summary.png`, `tutorial-bubble.png`, `box.png`, `vi-diacritics.png`, `en.png`, `scale1.png`, `scale3.png`. Bản web hỗ trợ `?shot=<tên>` dựng trạng thái tất định cho từng ảnh.
### 10.3 Cổng cân bằng (GreedyBot, seed 1..5, trung bình)
- Mở Xiêm trước phút 3; mở Mập trước phút 15; mở Mun trước phút 50.
- Hạ boss khu 1 trong 4–12 phút; đạt khu 3 trong 30–75 phút.
- Prestige đầu tiên trong 1.5–4 giờ game. Sau 24 giờ game (gồm prestige): đã dùng 3–7 mạng, `max_global_stage` 60–160.
- Không có "tường": GreedyBot không bao giờ đứng ở cùng một `global_stage` cao nhất quá 25 phút trước prestige đầu tiên.
- Luôn có việc: trong 2 giờ đầu, khoảng cách giữa hai lần mua được (bất kỳ nâng cấp nào) ≤ 4 phút ở trung vị (P3).
- IdleBot sau 8 giờ vẫn tiến tới ít nhất khu 2.
### 10.4 Kiểm tra trên máy người dùng (không tự động được)
Checklist `docs/MANUAL-CHECK.md`: trong suốt, click (không) xuyên, mép dưới đúng trên taskbar, đa màn hình, DPI 125/150%, CPU/RAM khi treo 10 phút, Ghost mode, tự khởi động.

## 11. Cấu trúc repo
```
crates/core   cm-core   lõi tất định (api.rs = hợp đồng; data.rs; sim/*; save.rs; fmt.rs)
crates/bot    cm-bot    bot + cổng cân bằng (`cm-bot gates`, `cm-bot sim --hours 24 --seed 1`)
crates/game   chin-mang Bevy: overlay, render, ui/, i18n/, audio, save_io, shots (web)
tools/art     pixel.mjs, manifest.mjs, sprites/*.mjs, build-atlas.mjs
tools/audio   sfxr.mjs tracker.mjs analyze.mjs synth.mjs verify.mjs
tools/verify  verify.mjs, shots.mjs (Playwright), web/ (index.html harness)
assets/       art/ audio/ fonts/
docs/         CONTRACT.md DECISIONS.md VERIFY.md MANUAL-CHECK.md shots/ art/
```

## 12. Definition of Done
1. `node tools/verify/verify.mjs` thoát 0 (mọi bước §10.1; bước nào bị chặn bởi môi trường thì ghi rõ trong `docs/VERIFY.md` kèm log, không được bỏ bước khác).
2. Đủ ảnh §10.2, đã được tự mở và tự chấm theo 6 câu: (a) đọc được trên cả nền trắng/đen/busy? (b) mèo 4 vai phân biệt được không cần màu? (c) không có chữ bị cắt/tràn, dấu tiếng Việt đủ? (d) khoảng cách đều theo lưới 4 px? (e) pixel sắc, không nhòe? (f) trông như game phát hành, không như bản mẫu?
3. Mọi nội dung §2 có hành vi thật, có test lõi tương ứng.
4. Cổng cân bằng §10.3 xanh.
5. README: chạy, build Windows, build web, snippet Hyprland. `docs/MANUAL-CHECK.md` có checklist §10.4.
