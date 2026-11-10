# Changelog

Format mengikuti [Keep a Changelog](https://keepachangelog.com/) dan
[Semantic Versioning](https://semver.org/). Nomor versi disebarkan ke seluruh
file oleh `scripts/version.mjs`.

## [Unreleased]

## [0.7.1]

### Fixed

- GUI: **semua dialog konfirmasi menganggap "Ya" sebagai "Batal"**. `openModal`
  memanggil `onClose` di dalam `close()`, dan `confirmDialog` memakai
  `onClose: () => resolve(false)`. Handler tombol konfirmasi memanggil `close()`
  lebih dulu, jadi promise sudah terlanjur resolve `false` — tombol seperti
  "Forget key", "Hapus provider", dan konfirmasi lain seolah tidak berfungsi:
  dialog muncul, dikonfirmasi, lalu tidak terjadi apa-apa. Sekarang `resolve`
  dipanggil sebelum `close()`, dan `web/confirm.test.js` menjaga urutannya.

## [0.7.0]

### Added

- Reasoning massal: endpoint `/models` tidak pernah memberi tahu dukungan
  thinking, jadi hasil sync dulu masuk sebagai `reasoning: false` semua. Sekarang
  model baru ditebak dari tiga sumber berurutan: katalog pi
  (`<agent dir>/models-store.json`, termasuk `thinkingLevelMap` aslinya), preset
  bawaan, lalu pola id model (`gpt-5*`, `o3*`, `claude-*-5*`, `deepseek-v4*`,
  `glm-*`, `kimi-k*`, `grok-*`, `qwen3*`, `gemini-*`, `minimax-m*`, ...).
  Sumbernya ikut dilaporkan supaya "terdeteksi" bisa dibedakan dari "perkiraan".
- Form model: tombol **Tandai terdeteksi** dan **Tandai semua** untuk menandai
  reasoning sekaligus, tanpa mencentang satu per satu.
- Command Tauri `reasoning_hints` untuk deteksi di atas.
- pi: notice `pi.reasoning_missing` + perbaikan otomatis untuk config lama:
  model yang menurut katalog pi mendukung thinking tetapi tersimpan tanpa
  `reasoning`. Perbaikan sekaligus menulis `thinkingLevelMap` (map katalog kalau
  ada, kalau tidak map standar).

### Changed

- Notice `catalog.credential-without-provider` sekarang punya aksi: **Forget the
  stored credential**. Sebelumnya kartu "Perlu diketahui" hanya memberi tahu
  tanpa jalan keluar, padahal kredensial di `auth.json` bisa dihapus dari sini
  (dengan backup). Kalau kredensialnya hanya di environment, aksinya tidak
  ditawarkan karena aplikasi tidak bisa menyentuh shell pengguna.

## [0.6.9]

### Fixed

- pi: `defaultModel` sekarang ditulis sebagai **id model saja**, bukan
  `provider/model`. pi membaca key itu apa adanya lalu mencari
  `getModel(defaultProvider, defaultModel)`; dengan prefiks provider modelnya
  tidak pernah ketemu, default diabaikan, dan pi memilih provider lain — biasanya
  provider bawaan yang punya kredensial. Gejalanya persis seperti yang terlihat:
  Hakonfig menampilkan default `opencode-go/deepseek-v4-pro`, tetapi pi mulai di
  `deepseek/deepseek-v4-pro`.
- pi: notice `pi.default_model_prefixed` + perbaikan otomatis untuk config lama
  yang sudah terlanjur ditulis dengan format itu. Perbaikan juga terjadi otomatis
  pada toggle berikutnya.
- pi: menonaktifkan provider yang **bukan** default tidak lagi menghapus default
  yang sehat (regresi dari perbaikan self-healing 0.6.4).

## [0.6.8]

### Added

- GUI: kartu "Perlu diketahui" di halaman harness, berisi semua temuan untuk
  harness itu — termasuk yang tidak punya perbaikan otomatis. Sebelumnya GUI
  hanya menampilkan notice yang punya tombol fix, sehingga penjelasan seperti
  "provider ini masih punya kredensial, jadi pi tetap memakainya" tidak pernah
  terlihat di aplikasi dan hanya muncul di `doctor`.
- Command Tauri `list_notices` supaya GUI bisa membaca temuan tanpa menunggu
  ada perubahan config.

## [0.6.7]

### Fixed

- Oh My Pi: `doctor` tidak lagi melaporkan provider yang sengaja dinonaktifkan
  sebagai masalah. Notice `omp.disabled` beserta tombol "Remove from
  disabledProviders" dihapus: itu keadaan normal hasil tombol toggle, dan
  `doctor --fix` malah menyalakan kembali provider yang dipilih untuk dimatikan.

## [0.6.6]

### Fixed

- pi: `doctor` sekarang melaporkan provider yang **dinonaktifkan di models.json
  tetapi masih punya kredensial**. Sebelumnya provider yang diparkir dianggap
  "sudah dikonfigurasi", jadi kasus paling membingungkan — semua provider mati,
  config bersih, tetapi pi tetap memakainya karena provider itu bawaan pi dan
  kredensialnya (auth.json / `*_API_KEY`) masih ada — tidak pernah disebut.
  Pesannya kini menyebut sumber kredensialnya dan apa yang harus dihapus.

## [0.6.5]

### Fixed

- Semua harness: menonaktifkan provider yang menjadi default sekarang juga
  membereskan default di harness lain, bukan hanya pi.
  - oh-my-pi: `modelRoles.default` dipindah ke provider aktif (atau dikosongkan),
    dan referensi scope `enabledModels` milik provider itu ikut dibuang.
  - OpenCode: `model` dipindah ke provider aktif lain (atau dihapus).
  - Hermes: `model.provider`/`default`/`base_url` dikosongkan karena daftar
    modelnya diambil dari endpoint, jadi tidak ada model pengganti yang pasti.

## [0.6.4]

### Fixed

- pi: default yang sudah menunjuk provider non-aktif sekarang dibereskan pada
  setiap toggle, bukan hanya ketika provider itu yang sedang dimatikan. Ini
  kasus nyata: semua provider dimatikan (models.json bersih), tetapi
  `settings.json` masih menyimpan `defaultProvider`/`defaultModel` dari provider
  yang diparkir versi lama, sehingga pi tetap memakai model itu.
- pi: label perbaikan pada notice `pi.default_disabled` sekarang "Clear the
  default" kalau tidak ada provider aktif lagi, bukan menjanjikan pemindahan
  yang tidak mungkin dilakukan.

## [0.6.3]

### Changed

- pi: pesan saat default dikosongkan (semua provider nonaktif) sekarang menyebut
  bahwa pi akan memilih modelnya sendiri dan provider yang masih punya
  kredensial — termasuk provider bawaan — bisa terpilih. "Tidak ada default"
  bukan berarti "tidak ada model", dan itu lebih baik dikatakan daripada ditebak
  pengguna saat harness tiba-tiba memakai model lain.

## [0.6.2]

### Added

- pi: setiap model ber-reasoning yang didaftarkan lewat Hakonfig sekarang membawa
  `thinkingLevelMap` lengkap — none (`off`), low, medium, high, max — sehingga
  level `max` benar-benar ada di pemilih model. Tanpa map, pi hanya menawarkan
  level sampai `high` karena `xhigh`/`max` dianggap tidak didukung.
  `minimal` dan `xhigh` ditandai `null` (tidak didukung) supaya variant yang
  dijanjikan tepat lima.
- pi: notice `pi.thinking_map` + perbaikan otomatis untuk model lama yang belum
  punya map, jadi config yang sudah ada tidak perlu diedit satu per satu.

### Fixed

- pi: `SetModels` sekarang menggabungkan draft dengan entri model yang sudah ada
  (per id), bukan membangun ulang. Field yang tidak diurus form — mis.
  `thinkingLevelMap` buatan pengguna atau `cost` — tidak lagi hilang saat daftar
  model disimpan ulang. Map yang sudah ada juga tidak ditimpa map bawaan.

## [0.6.1]

### Fixed

- GUI: menonaktifkan provider yang sedang menjadi default sekarang ikut
  memindahkan `defaultProvider`/`defaultModel` ke provider aktif lain; kalau
  semua provider nonaktif, default dikosongkan. Sebelumnya default tetap
  menunjuk provider mati dan harness diam-diam memakai model lain.
- GUI: hasil `set_enabled` menampilkan peringatan dari engine (perpindahan
  default, provider bawaan) dan notice yang bisa diperbaiki sekali klik.
  Sebelumnya tombol toggle hanya bilang "dinonaktifkan" walau harness masih
  menampilkan provider itu.
- pi: notice `pi.default_disabled` (dengan perbaikan otomatis) untuk config lama
  yang default-nya menunjuk provider yang sudah dinonaktifkan.

### Added

- GUI: aksi "Lupakan kredensial tersimpan" di menu provider. Untuk provider
  bawaan pi (deepseek, openai, anthropic, ...) menghapus entri di `models.json`
  tidak cukup: pi tetap mengenalinya selama kredensialnya ada. Aksi ini
  menghapus entri kredensial (dengan backup) dan memberi tahu kalau API key
  masih hidup di environment shell.
- pi: saat provider dinonaktifkan, engine melaporkan kalau provider itu juga
  dikenali pi secara bawaan (probe `pi auth check --provider ... --json`),
  supaya tidak terlihat seperti config yang tidak tersimpan.

## [0.6.0]

### Added

- GUI: `Settings → Skala tampilan` — zoom webview 90/100/110/125/150% atau auto,
  dengan `Ctrl` + `+` / `-` / `0`. Zoom yang sama diterapkan sebelum halaman
  pertama dilukis, jadi tidak ada satu frame UI kecil saat aplikasi dibuka.
- GUI: mode auto menambal WebKitGTK yang melaporkan `devicePixelRatio` 1x di
  layar 2x (`zoom = skala monitor / devicePixelRatio`, diabaikan bila < 1.1).
- GUI: panel Settings menampilkan angka diagnosa (`monitor`, `webview`,
  `viewport`, `zoom`) supaya laporan "kecil di laptop saya" bisa dipastikan
  penyebabnya.
- GUI: jendela dibatasi ke 90% work area monitor saat pertama dibuka, dan
  minimum jendela ikut mengecil di layar sempit (dulu `minWidth 940` lebih besar
  dari ruang CSS laptop 1366x768 @150% sehingga jendela tidak bisa muat).

### Changed

- Tipografi pindah ke rem: 147 kelas `text-[Npx]` menjadi token `text-2xs` …
  `text-4xl`, dan 48 deklarasi `font-size`/`height`/`padding` di CSS juga rem.
  Ukuran teks sekarang ikut preferensi font sistem. Test `web/scale.test.js`
  menolak `text-[13px]` atau `h-[148px]` baru.
- Default jendela minimum `940x620` → `720x480`.

## [0.5.2]

### Added

- CLI: `--as-user <USER>` — menjalankan perintah yang sama sebagai pemilik home
  harness, dengan `sudo -n -u <user> -H` (tanpa prompt, jadi skrip tidak
  menggantung). Kalau sudo butuh password, perintah siap-tempel dicetak lengkap
  dengan argumen yang dipakai. Ini jalan pintas dari resep `sudo -u` yang
  sebelumnya harus ditulis sendiri, dan menjaga file tetap milik user agent.

### Changed

- Setelah menulis, Hakonfig membandingkan pemilik setiap file yang ditulis
  dengan pemilik home target. Ketidakcocokan menjadi notice
  `ownership_mismatch` berisi `chown` yang harus dijalankan — gejala khas
  "sudah di-add tapi agent tidak membaca apa pun" sekarang muncul tepat saat
  penulisan, bukan hanya di `doctor`.

## [0.5.1]

### Added

- CLI: `--home <DIR>` / `HAKONFIG_HOME` untuk mengelola harness milik user
  lain — mis. agent ADE dengan home sendiri di `/var/lib/openbot`. Semua
  direktori harness di-resolve dengan layout asli di bawah home target
  (`<home>/.omp`, `<home>/.pi`, ...), probe versi memakai `<home>/.local/bin`,
  dan `--home` menang atas `--root`. Berbeda dari `--root` yang layout-nya
  layout test (`<root>/<harness>`) dan gagal senyap kalau dipakai untuk home
  nyata.
- Error tulis `Permission denied` kini menyebut jalan keluarnya:
  `sudo -u <user> -H hakonfig-cli ...` (gejala khas "sudah di-add tapi tidak
  terbaca di sisi agent").
- Dokumentasi baru di `docs/CLI.md`: "Harnesses that run as another user
  (ADE / VPS)" — pola `sudo -u <user> -H`, peringatan root-ownership, dan
  perbedaan `--home` vs `--root`.
- CLI: perintah `targets` — pindai `/etc/passwd` untuk home user lain
  (pola ADE/VPS) yang punya harness terpasang (config asli atau binary di
  `<home>/.local/bin`), keluaran `USER/HOME/HARNESS` plus `--json`.
- CLI: flag global `--target <USER|DIR>` — singkatan `--home` dengan lookup
  username lewat `/etc/passwd` (mis. `hakonfig-cli --target openbot show omp`).
- CLI: harness kini dianggap terpasang bila binary-nya ada di
  `<home>/.local/bin`, walau config belum pernah dibuat (home ADE yang baru).
- Doctor: dua notice baru — `ade_home_mismatch` (harness terpasang di home
  user lain, dengan perintah `--home` yang tepat) dan `ownership_mismatch`
  (pemilik file ≠ pemilik home, gejala "sudah di-add tapi agent tidak baca";
  menyertakan `chown` yang harus dijalankan — Hakonfig tidak pernah chown
  otomatis).

### Fixed

- Kredensial dari environment (`*_API_KEY`) tidak lagi dihitung saat `--home`
  menunjuk home user lain. Environment itu milik yang menjalankan perintah,
  bukan milik agent: sebelumnya `doctor --home /var/lib/openbot` melaporkan
  kredensial yang tidak pernah dimiliki agent tersebut.
- `--home` relatif kini dibuat absolut (dan file ditolak dengan pesan jelas),
  supaya path di output tidak berubah arti kalau cwd berpindah.

### Changed

- Output menyebut environment yang dibaca: `list`, `providers`, dan `default`
  menambah kolom `ENV` saat target eksplisit, dan JSON-nya menambah `home`
  (plus `user` bila dikenali dari `/etc/passwd`). Tanpa target, output tidak
  berubah.
- Aplikasi desktop menampilkan banner home lain: `list_targets` tersedia
  sebagai Tauri command, dan beranda menyebut user/home/harness beserta
  perintah `hakonfig-cli --target <user> providers`.

## [0.5.0]

Perintah CLI dipisah menurut pekerjaannya, mengikuti aplikasi desktop. Ini
breaking untuk `providers` (lihat di bawah).

### Added

- `providers` sekarang menampilkan **provider yang terpasang**, dan di terminal
  menjadi pintu kelola: pilih harness → provider → menu detail (ganti key,
  pilih/sync model, jadikan default, ubah endpoint+nama, hapus).
- `catalog` (alias `presets`, `provider-list`) untuk katalog preset yang
  didukung build ini — isi `providers` sebelum 0.5.0.
- `default` menampilkan default provider + model tiap harness.
- `set key|models|default`, masing-masing menyentuh satu hal saja.
  `set default` selalu memilih harness lebih dulu karena default milik harness.
- `remove provider` untuk menghapus provider beserta entry kredensialnya
  (konfirmasi + backup).
- `add` membuka detail provider yang sudah terpasang alih-alih menawarkan menu
  kedua, dan setelah provider baru tersimpan detailnya langsung dibuka.

### Changed

- `providers --search` / `--free` diteruskan ke `catalog` supaya skrip lama
  tidak patah.
- `add` pada provider terpasang tanpa flag sempit tetap kompatibel: hanya key →
  `set_key`, `--models`/`--sync` → `set_models`, `--default-model` →
  `set_default`, `--everything` → tulis ulang penuh.
- `set key` menyimpan key di config secara default; pakai `--env` untuk
  menyimpan referensi environment.

### Fixed

- Aksi "Change endpoint and name" tidak lagi meminta API key baru: kredensial
  yang tersimpan dipakai untuk uji koneksi dan tidak ditulis ulang.
- Provider custom yang sudah terpasang tidak lagi ditolak dengan "not in the
  catalog" saat diubah dari pintu kelola.

## [0.4.8]

*Choose models (key stays)* sekarang bisa sync daftar model dari endpoint
memakai API key yang sudah tersimpan — tanpa mengetik key lagi.

### Added

- CLI: *Choose models (key stays)* sekarang bisa **sync daftar model dari
  endpoint** memakai API key yang sudah tersimpan, jadi daftar model bisa
  di-refresh tanpa mengetik ulang key. Berlaku juga untuk skrip:
  `add --models-only --sync`.
- core: `api::stored_key(harness, provider)` — membaca key tersimpan tanpa
  membuka config manual; dipakai CLI untuk sync.

## [0.4.7]

Satu menu saja untuk provider yang sudah terpasang, dan *Cancel* benar-benar
berhenti tanpa meminta API key lagi.

### Fixed

- CLI: provider yang sudah terpasang (✓) tidak lagi menampilkan dua menu
  berurutan — menu lama "Change key / models" yang muncul lebih dulu sudah
  dihapus, jadi yang tampil langsung empat aksi: *Change the API key only*,
  *Choose models (key stays)*, *Set the default model*, *Change everything*.
- CLI: memilih *Cancel* sekarang benar-benar berhenti. Sebelumnya pembatalan
  jatuh ke alur `add` biasa sehingga pengguna tetap dimintai API key.

## [0.4.6]

`hakonfig-cli add` untuk provider yang sudah terpasang sekarang memisahkan
ganti key, pilih model, dan jadikan default — sama seperti aplikasi desktop.

### Added

- CLI: `add` untuk provider yang **sudah terpasang** sekarang memisahkan tiga
  aksi seperti di aplikasi desktop — *Change the API key only*, *Choose models
  (key stays)*, *Set the default model* — ditambah *Change everything* dan
  *Cancel*. Tanpa terminal, `add` dengan hanya `--key-stdin` berarti
  mengganti key saja (dulu daftar model ikut ditimpa daftar preset), dan
  tersedia flag eksplisit `--key-only`, `--models-only`, serta
  `--default-model <ID>`.

### Fixed

- CLI: mengganti API key tidak lagi menulis ulang `models.json` (dan membuat
  backup tambahan) ketika tidak ada `apiKey` usang yang perlu dibersihkan.

## [0.4.5]

Aksi per provider dipisah jadi tiga (ganti key, pilih model, jadikan default)
supaya mengganti satu hal tidak perlu mengirim ulang key.

### Added

- Aplikasi desktop: aksi per provider dipisah menjadi tiga, jadi tidak perlu
  mengirim ulang draft (dan key) hanya untuk mengganti satu hal:
  - **Change key** → `EditOp::SetKey` (hanya file kredensial, atau referensi
    env; model/endpoint/nama tidak disentuh),
  - **Choose models** → `EditOp::SetModels` (checkbox per model; key dan
    endpoint tidak disentuh, daftar baru menggantikan yang lama dengan backup),
  - **Set as default** → `EditOp::SetDefault` untuk model mana pun milik
    provider itu.
  "Edit" dan "Delete" tetap ada, dipisah garis.
- pi: `SetModels` hanya menjamin model yang dipilih tercakup `enabledModels`
  (menambah `<provider>/<model>` bila belum tercakup), bukan melebar ke
  seluruh provider seperti saat menambah provider baru.

## [0.4.4]

`doctor` kini juga melihat hal di luar file config (kredensial yang memicu
refresh katalog, cache katalog kosong, dan dengan `--online` kegagalan
permintaan katalog), dan pi menjaga `enabledModels` tetap masuk akal saat
provider dinonaktifkan, diaktifkan lagi, atau modelnya berubah.

### Added

- pi: `enabledModels` sekarang diselaraskan setiap kali config berubah, bukan
  hanya saat menulis provider:
  - provider dinonaktifkan atau dihapus → polanya dibuang dari scope (pi tidak
    punya saklar disable, jadi pola itu hanya menyisakan peringatan "no models
    match" di pemilih model);
  - provider diaktifkan lagi → dijamin tercakup `<id>/**`, jadi modelnya
    langsung terlihat tanpa memilih ulang dan Ctrl+S;
  - model yang dihapus dari provider → id eksplisitnya (termasuk yang memakai
    sufiks thinking seperti `gateway/a:high`) dibuang;
  - scope yang jadi kosong dihapus, supaya pi kembali menampilkan semua model.

### Added

- `doctor` juga melaporkan hal di luar file config yang membuat model tidak
  tampil: `catalog.credential-without-provider` (pi punya kredensial untuk
  provider yang tidak ada di `models.json`, jadi pi me-refresh katalognya
  tanpa ada yang bisa dipakai) dan `catalog.empty-cache` (cache katalog
  pi.dev tersimpan kosong — permintaan terakhirnya gagal/ditolak).
- `doctor --online` memeriksa katalog jarak jauh yang di-refresh harness
  (satu permintaan per provider, 4 detik per percobaan, dua retry seperti pi)
  dan melaporkan status/error aslinya sebagai `catalog.refresh`. Ini
  satu-satunya bagian `doctor` yang memakai jaringan; tanpa flag itu doctor
  tetap sepenuhnya offline. Base URL bisa diarahkan lewat
  `HAKONFIG_PI_CATALOG_BASE`.
- Gateway custom tidak pernah dimasukkan ke pemeriksaan katalog (pi.dev tidak
  mengenal id-nya), jadi tidak ada alarm palsu.

## [0.4.3]

Sync daftar model dari endpoint, perapian config yang tidak terlihat harness,
dan perintah `doctor` untuk memeriksa sisanya.

### Added

- CLI: `hakonfig-cli add --sync` mengambil daftar model langsung dari endpoint
  (`GET {base}/models`) alih-alih memakai daftar preset, untuk skrip dan
  server tanpa TTY.
- CLI: di terminal, setelah key dimasukkan, `add` sekarang menawarkan sync
  daftar model (default ya) untuk **semua** provider — termasuk 34 preset yang
  sudah punya daftar model dan provider custom. Id hasil sync digabung dengan
  metadata preset (context window dari endpoint diutamakan, nama/max
  token/reasoning dari preset, id baru tetap ditambahkan) dan semua model
  terpilih secara default, sama seperti aplikasi desktop setelah Test
  Connection. Kalau sync gagal, perintah lanjut dengan daftar preset dan
  menampilkan alasannya.
- CLI: perintah `hakonfig-cli doctor [--harness <id>] [--fix] [--json]`.
  Memeriksa config yang sudah ditulis tetapi tidak akan terlihat harness-nya
  dan (dengan `--fix`) memperbaikinya dengan backup. Read-only tanpa `--fix`;
  exit code non-zero selama masih ada masalah yang bisa diperbaiki otomatis.
- Core: `Notice` — temuan pasca-tulis per provider (`pi.scope`,
  `omp.disabled`, `opencode.disabled`, `harness.builtin-id`) beserta aksi
  perbaikannya, dikembalikan `apply_edit` dan bisa diterapkan lewat
  `fix_notice`. Aplikasi desktop menampilkan tombol "Perbaiki" untuk notice
  yang punya `fix`.
- Menulis provider sekarang sekalian merapikan hal yang membuatnya tidak
  terlihat: scope `enabledModels` pi dilebarkan (`<id>/*` diganti `<id>/**`,
  pola lain dipertahankan, tidak pernah mengarang scope baru) dan provider
  yang ditulis dikeluarkan dari `disabledProviders` (omp) atau
  `disabled_providers` (OpenCode). Semua perubahan itu masuk pratinjau dan
  backup seperti edit biasa, dan dilaporkan sebagai warning — jadi pengguna
  tidak perlu menjalankan perintah tambahan.
- Core: `engine::glob` — pencocokan pola `enabledModels` gaya minimatch
  (`*` tidak melewati `/`, `**` melewati), dipakai untuk mendeteksi scope pi
  yang terlalu sempit.

### Fixed

- CLI: provider custom tidak pernah mencoba sync daftar model sama sekali
  (permintaan sebelumnya selalu memakai base URL preset, yang tidak ada untuk
  provider custom).
- Visibilitas: model yang sudah ditulis sekarang dilaporkan bila pi
  menyaringnya lewat `enabledModels` di `settings.json`. Kasus yang paling
  sering: pola `provider/*` tidak cocok untuk id model bersarang
  (`commandcode/deepseek/deepseek-v4-pro`), jadi 71 dari 94 model commandcode
  tidak pernah muncul tanpa pesan apa pun. Cache katalog (`models-store.json`
  di pi, `models.db` di omp, kv `models-dev` di OpenCode,
  `provider_models_cache.json` di Hermes) tidak ikut diutak-atik: semuanya
  di-merge **di bawah** config pengguna, jadi tidak bisa menyembunyikan model
  yang ditulis.

## [0.4.2]

### Fixed

- npm: binari semua platform kini ikut di dalam satu paket `hakonfig`
  (≈5 MB terkompresi), menggantikan rencana paket per-platform. Dua percobaan
  sebelumnya gagal karena alasan di luar kode kita: nama unscoped
  `hakonfig-win32-x64` ditolak permanen dengan `403 Package name triggered spam
  detection`, lalu paket scoped `@perogeremmer/hakonfig-*` tersimpan
  *restricted* (packument 404 untuk pengunjung anonim) dan token rilis tidak
  boleh mengubah visibilitasnya. Dengan binari gabungan, `npm i -g hakonfig`
  bekerja di macOS, Linux, dan Windows tanpa `optionalDependencies` sama sekali.
- Rilis: draft release sekarang benar-benar di-publish (`gh release edit
  --draft=false --latest`) di akhir workflow. Sebelumnya tidak ada langkah ini,
  jadi setiap rilis tertinggal sebagai draft dan tautan unduhan 404.
- Rilis: kegagalan publish crates.io (mis. email akun crates.io belum
  diverifikasi — kasus yang terjadi pada 0.4.1) tidak lagi menggagalkan
  workflow yang sama dengan aset desktop; sebagai gantinya muncul warning.
- npm: `scripts/publish-npm.mjs` memverifikasi hasil publish lewat registry
  tanpa autentikasi, jadi publish yang tersimpan *private* langsung ketahuan
  alih-alih baru ketahuan saat pengguna meng-install.

## [0.4.1]

Dirilis untuk memperbaiki distribusi npm: paket platform dipindah ke scope
`@perogeremmer/` setelah nama unscoped kena spam detection. Ternyata paket
scoped tersimpan *restricted*, jadi 0.4.1 tetap tidak terpakai lewat npm —
digantikan 0.4.2 (binari gabungan). Aset desktop 0.4.1 sendiri normal.

### Fixed

- npm: paket binary platform dipindah ke scope `@perogeremmer/`
  (`@perogeremmer/hakonfig-darwin`, `-linux-x64`, `-win32-x64`). npm menolak
  publish nama unscoped `hakonfig-win32-x64` dengan `403 Package name
  triggered spam detection` — proteksi terhadap nama berpola auto-generated
  dari akun baru — sehingga `npm i -g hakonfig` di Windows gagal karena
  optional dependency-nya tidak pernah ada. Pola scoped ini sama dengan
  distribusi esbuild/swc. Paket utama tetap `hakonfig` (tanpa scope), jadi
  perintah install tidak berubah.
- npm: `hakonfig@0.4.0` serta dua paket platform unscoped ditandai
  *deprecated* supaya pengguna versi lama dapat peringatan saat install.

## [0.4.0]

### Added

- Crate CLI `hakonfig-cli`: kelola config harness tanpa GUI, memakai jalur
  kode yang sama dengan aplikasi desktop (`hakonfig_core::api`). Perintah:
  `list`, `show`, `providers`, `paths`, `add` (wizard interaktif atau
  non-interaktif penuh untuk skrip/server), dengan `--json` untuk output
  yang stabil. Dokumentasi ada di `docs/CLI.md`.
- Instalasi CLI satu perintah: `install.sh` (macOS/Linux) dan `install.ps1`
  (Windows) yang mengunduh binary release, memverifikasi `SHA256SUMS`, dan
  memasang ke `~/.local/bin`. Jalur lain: Homebrew tap
  (`brew install perogeremmer/tap/hakonfig-cli`), npm (`npm i -g hakonfig`,
  paket `hakonfig` dengan binary per-platform), dan crates.io
  (`cargo install hakonfig-cli`).
- Release workflow kini mengunggah binary `hakonfig-cli` per platform
  (macOS universal, Linux, Windows) plus `SHA256SUMS` ke draft release,
  lalu mem-publish ke npm dan crates.io (dilewati dengan pesan jelas bila
  secret-nya belum di-set). Workflow `homebrew-tap.yml` (manual, per tag)
  menulis ulang formula di `perogeremmer/homebrew-tap`.
- CI kini menjalankan `cargo clippy -D warnings` dan `cargo fmt --check`.

### Changed

- CLI: `--test` eksplisit yang gagal kini menggagalkan perintah
  (exit non-zero, tidak ada yang ditulis), karena automation
  memintanya sebagai gerbang sebelum menulis. Tanpa `--test`/`--no-test`,
  mode non-interaktif tetap tidak mencoba endpoint sama sekali.
- CLI: `--json` dan stdout yang dipipe kini memaksa mode non-interaktif,
  sehingga wizard tidak pernah tercampur dengan output JSON.
- CLI: kuota potongan diff dibagi merata antara baris yang dihapus dan
  ditambah (sisa satu sisi diberikan ke sisi lain), bukan dihabiskan pada
  baris yang dihapus lebih dulu.
- String warning/error di `hakonfig-core` kini seluruhnya English (sebelumnya
  Indonesia). Nama semua test Rust dan JS kini juga English.

### Notes

- String warning engine yang English berlaku juga di aplikasi desktop:
  pengguna desktop yang memakai Bahasa Indonesia akan melihat warning dalam
  bahasa Inggris sampai string engine masuk katalog i18n yang sama.

## [0.3.2] - 2025

### Fixed

- Katalog: nama yang benar untuk DeepSeek Harness (bukan "DeepSeek chat").

## [0.3.1] - 2025

### Fixed

- Default model benar-benar diterapkan di Pi dan Oh My Pi.

## [0.3.0] - 2025

### Added

- Katalog tool: web tools dan OpenHands.

## [0.2.1] - 2025

### Changed

- README: "Get started" dipindah ke atas dan logo brand ditambahkan.

## [0.2.0] - 2025

### Fixed

- CI: `Cargo.lock` tetap sinkron dengan versi aplikasi.

## [0.1.4] - 2025

### Added

- 37 preset provider baru dengan logo lokal; semua aset sepenuhnya offline.

## [0.1.3] - 2025

### Changed

- UI: checkbox digambar sendiri dengan sudut membulat yang lembut.

## [0.1.2] - 2025

### Added

- Brand Hakonfig: mark, app icon, dan design token.

## [0.1.1] - 2025

### Fixed

- UI: nama variabel env diambil dari provider, bukan dari baris model.

## [0.1.0]

Rilis pertama.

- Empat adapter harness: Pi, Oh My Pi, OpenCode, Hermes (eksperimental).
- 24 preset provider OpenAI-compatible dengan logo lokal.
- Pratinjau diff sebelum menulis, backup otomatis (maksimal 5 per harness),
  dan riwayat pemulihan.
- Panduan awal saat aplikasi pertama kali dibuka: deteksi harness + model.
- Bilingual (English / Bahasa Indonesia), tema terang dan gelap.
- Bundle: macOS (dmg), Windows (NSIS), Linux (deb).
