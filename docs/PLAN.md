# Rencana Project: `labu`, test bench reliability untuk agent (Rust)

*Draft v2.5 · 17 November 2026 · project open source RedPumpkin.AI · repo: `redpumpkin-ai/labu`*

## 1. Keputusan

**Bikin satu tool Rust yang kepake sendiri dulu.** Tidak perlu jadi nomor 1, tidak perlu kategori baru. Ukuran sukses pertama: *kita pakai tiap minggu dan dia nangkep bug beneran.*

Tujuan jangka panjang tetap sama: diakui sebagai expert agentic AI. Jalurnya: **pakai sendiri → catat apa yang ketangkep → publish temuannya.**

**Kenapa arah ini (ringkasan research):**
- Semua kategori tool general untuk agent sudah punya 5–40 repo kecil (record/replay, chaos testing, verifikasi klaim, document firewall, framework Rust, MCP gateway). Kategori kosong sudah tidak ada.
- Tapi tool-tool itu **terpisah-pisah**, kebanyakan Python, padahal arsitekturnya sama: proxy lokal di antara agent dan LLM/MCP.
- Peluang: **satu binary Rust yang menyatukan semuanya** (pola `uv`/`ruff`).
- Tidak butuh subject matter expert. Domainnya engineering agent, dan itu keahlian kita sendiri.

---

## 2. Apa itu `labu`

Proxy lokal + CLI. Agent cukup ganti `base_url` (dan/atau config MCP) supaya lewat `labu`. Jalan untuk agent bahasa/framework apa pun.

```
labu record  -- <command>     rekam semua call LLM + tool ke cassette
labu replay  -- <command>     ulang dari cassette: deterministik, offline, gratis
labu run -n 5 -- <command>    jalankan N kali, kumpulkan hasil
labu stats                    pass@k, pass^k, cost, latency, token
labu chaos --faults f.yaml    suntik timeout / error / JSON rusak / respons kosong (ber-seed)
labu verify                   cek klaim "selesai" di jawaban akhir vs bukti di trace
labu compare A B              dua run: beda beneran atau cuma kebetulan
labu swap --model X           tool dibekukan dari cassette, model baru jalan live
labu import <otel|langfuse>   bawa trace produksi untuk verify + stats
labu report                   laporan HTML statis
```

### Arsitektur

```
agent ──HTTP──► labu proxy ──► API LLM (Anthropic / OpenAI-compatible)
agent ──MCP───► labu mcp-proxy ──► MCP server asli
                    │
                    ├─ cassette JSONL (bisa di-commit & di-diff)
                    ├─ index SQLite/DuckDB (untuk stats)
                    └─ engine: replay · fault · verify
```

### Keputusan teknis (usulan, bisa dikoreksi)

| Hal | Pilihan | Alasan |
|---|---|---|
| Proxy HTTP | `axum` + `hyper` + `reqwest`, `tokio` | Standar, async, streaming SSE |
| Provider | Inti agnostik + `trait Provider` (dua fungsi: normalisasi kunci matching, ambil usage). Provider pertama = **yang dipakai agent kita**; OpenAI-compatible mencakup banyak backend sekaligus | Inti tidak tahu nama provider; menambah provider = menambah satu adapter |
| Integrasi bahasa | Tanpa SDK: agent cukup set `*_BASE_URL` / `endpoint_url`. Distribusi opsional sebagai binary wheel (pip). Plugin pytest → `LATER.md` | Jalan untuk agent bahasa apa pun |
| Format cassette | JSONL, satu event per baris | Gampang di-diff di PR, di-grep |
| Matching saat replay | Hash request yang dinormalisasi + fallback urutan | Deterministik; tidak ada "mirip-mirip" |
| Rahasia | API key & header sensitif di-redact saat rekam | Cassette aman di-commit |
| Fault | YAML + seed | Hasil chaos bisa diulang persis |
| Verify v1 | Aturan deterministik, tanpa LLM judge | Cepat, murah, bisa dipercaya |
| MCP | stdio dulu, lalu Streamable HTTP (SDK Rust `rmcp`) | stdio paling umum dipakai lokal |
| CLI | `clap`; report pakai `minijinja` | Satu binary, tanpa runtime lain |

---

## 3. Milestone (dogfood dulu)

**Aturan:** tiap versi harus dipakai di agent sungguhan milik sendiri sebelum versi berikutnya dimulai. Tiap versi ditutup dengan catatan pendek: *apa yang ketangkep.* Catatan itu jadi bahan tulisan.

| Versi | Minggu | Isi | Tes "kepake" |
|---|---|---|---|
| **v0.1** | 1–2 | `record` / `replay` dengan inti agnostik provider dan **satu** adapter: **OpenAI-compatible**, karena agent kita memanggil gateway Bifrost lewat API itu (lihat `docs/adr/0001-v0.1-decisions.md`). Bedrock langsung (SigV4) jadi adapter kedua, lihat bagian 5.7 dan 9. Streaming, cassette JSONL, redaction header + pola PII Indonesia | Test agent kita jalan offline di CI, biaya nol |
| **v0.2** | 3–4 | `run -n`, `stats` dengan pass@k, pass^k, **interval kepercayaan**, cost yang menghitung **token cache & reasoning**. `compare` dua run (signifikan atau tidak). Provider kedua | Tahu apakah perubahan prompt beneran lebih baik atau cuma kebetulan |
| **v0.3** | 5–7 | MCP proxy (stdio) rekam/replay tool call. `chaos` dengan 5 fault dasar, ber-seed. Matching aman untuk tool call paralel | Tahu apa yang terjadi kalau tool timeout / balikin sampah |
| **v0.4** | 8–10 | `verify` (klaim vs bukti, rule pack **Indonesia + Inggris**, state probe opsional). **`chaos` × `verify`**: setelah fault, otomatis cek false success. `report` HTML, contoh GitHub Actions | CI gagal kalau agent bilang "beres" padahal tool error |
| **v0.5** | 11–13 | `swap`: tool dibekukan dari cassette, model jalan live (tes migrasi model). `import` trace dari OTel/Langfuse. Cassette terenkripsi | Bisa jawab "aman nggak pindah ke model X?" dengan data |
| **v0.6** | 14+ | **Studi:** 3–4 framework agent × fault yang sama → tingkat false success, recovery. Tulisan + laporan | Ada temuan yang layak dipublish |

### Cakupan v0.1 (sengaja kecil)
- Masuk: inti agnostik + `trait Provider` dengan satu adapter, streaming, record, replay, redaction (header + PII Indonesia di body), mode `replay` yang error kalau cassette tidak ada (untuk CI), tanpa perlu pasang CA cert.
- Tidak masuk dulu: MCP, chaos, verify, UI, multi-provider.

---

## 4. Ukuran sukses

**Tahap 1 (kepake):**
- Dipakai di minimal satu agent kita tiap minggu.
- Menangkap minimal satu bug/regresi nyata.
- `cargo install` → jalan dalam < 5 menit dari README.

**Tahap 2 (bisa dibanggakan):**
- 3–5 catatan "apa yang ketangkep" jadi tulisan publik.
- Studi perbandingan framework terbit (blog; arXiv kalau datanya kuat).
- Ada orang luar yang pakai / buka issue.

---

## 5. Risiko dan mitigasi

Prinsip: mitigasi harus berupa **mekanisme di desain + gate di CI**, bukan niat baik. Tiap risiko punya "bukti" yang bisa dicek.

### 5.1 Scope melebar (sekarang 10 subcommand, bukan 5)

| Mekanisme | Bukti / gate |
|---|---|
| **Gate antar-versi:** versi N+1 baru boleh mulai setelah versi N dipakai di agent sungguhan ≥ 2 minggu dan catatan "apa yang ketangkep" ditulis | Catatan ada di `docs/caught/vN.md` |
| **Batas WIP:** maksimal satu subcommand dikerjakan pada satu waktu | Satu branch fitur aktif |
| **`LATER.md`:** semua ide baru masuk sini, hanya dibuka di batas versi | Ide tidak langsung jadi issue |
| **Fitur eksperimen disembunyikan** lewat cargo feature, tidak muncul di `--help` | `labu --help` hanya menampilkan yang stabil |
| **Aturan potong, bukan molor:** kalau v0.1 belum kepake di minggu ke-3, yang dipotong fiturnya, bukan tanggalnya | Urutan potong: timing replay → provider kedua → redaction PII body |
| `compare`, `swap`, `import`, enkripsi = **opsional** (v0.5+), boleh dibatalkan tanpa merusak inti | Inti = record, replay, run, stats, chaos, verify |

### 5.2 Streaming & variasi API tiap provider

| Mekanisme | Bukti / gate |
|---|---|
| **Simpan byte mentah respons** (frame SSE / event-stream persis). Replay mengembalikan byte yang sama, tanpa serialisasi ulang | Tes round-trip: record → replay → byte-equal |
| **Adapter provider setipis mungkin:** hanya (a) normalisasi kunci matching, (b) ambil angka usage. Field tak dikenal diteruskan apa adanya | Field baru dari provider tidak mematahkan replay |
| **Tes pakai SDK resmi** (Python & TS) yang diarahkan ke proxy | Kalau SDK bisa parse hasil replay tanpa error, lulus |
| **Golden fixture** per provider: stream asli yang direkam lalu di-redact | Dijalankan di setiap PR |
| **Canary mingguan** (opsional, butuh API key): prompt mini ke API asli untuk deteksi perubahan format | Job terjadwal; biaya recehan |
| Cassette menyimpan versi API + versi format `labu` | `labu migrate` untuk format lama |

### 5.3 Replay "bocor" ke jaringan

| Mekanisme | Bukti / gate |
|---|---|
| **Mustahil secara konstruksi:** engine replay tidak pernah memegang HTTP client keluar | Dijamin oleh struktur tipe, bukan oleh `if` |
| **Strict otomatis** saat env `CI=true` terdeteksi | Miss = error, tidak pernah forward |
| Miss menghasilkan status khusus + error terstruktur + **diff ke cassette terdekat** (field mana yang beda) | Debug miss tidak perlu nebak |
| **Tes tanpa jaringan:** suite replay dijalankan di namespace tanpa network | Gate CI |
| Ringkasan akhir: `live calls: 0` dan exit code ≠ 0 kalau ada miss | Terlihat di log CI |

### 5.4 Rahasia masuk cassette

| Mekanisme | Bukti / gate |
|---|---|
| **Allowlist header, bukan denylist:** hanya header yang dikenal aman yang disimpan; sisanya dibuang | Header baru yang tak dikenal otomatis tidak tersimpan |
| Kredensial tidak pernah ditulis: `Authorization`, `x-api-key`, header & token SigV4 | Dikecualikan sebelum sampai ke penulis file |
| **Scanner saat menulis:** pola kunci (`sk-…`, `AKIA…`, JWT, PEM) + PII Indonesia (NIK, NPWP, no. HP, no. rekening). Mode fail-closed tersedia | Rekaman gagal kalau ada temuan |
| `labu lint` + pre-commit hook + gate CI untuk semua cassette di repo | PR ditolak kalau ada temuan |
| **Tes canary:** rahasia palsu disisipkan di request, lalu dipastikan tidak pernah muncul di disk | Property test di setiap PR |
| Enkripsi cassette untuk data klien (v0.5) | — |

*Catatan jujur: redaction tidak pernah sempurna. Cassette yang berisi data klien asli tetap harus direview sebelum di-commit, atau jangan di-commit sama sekali.*

### 5.5 Tool serupa sudah banyak

| Mekanisme | Bukti / gate |
|---|---|
| Ukuran sukses tahap 1 = **dipakai internal**, bukan jumlah bintang | Risiko kompetisi rendah untuk tujuan ini |
| **Interop, bukan duel:** impor/ekspor format tool lain bila dibutuhkan | Lihat bagian 9 "jangan ditiru dulu" |
| Peta kompetitor dicek ulang di tiap batas versi (script perbandingan README disimpan) | `docs/landscape.md` diperbarui |
| README menulis jelas apa yang **bukan** tujuan `labu` | Ekspektasi pengguna luar terkelola |

### 5.6 Nama & lisensi

**Sudah dicek 16 Nov 2026:**

| Tempat | `labu` | Catatan |
|---|---|---|
| crates.io | Kosong | `labu-cli` dan `redpumpkin` juga kosong |
| PyPI | Kosong | Berguna kalau nanti binary dibagikan lewat `pip` |
| npm | Kosong | `@redpumpkin/labu` juga kosong |
| GitHub | Akun pribadi `labu` sudah ada | Tidak masalah kalau repo ada di bawah org |
| GitHub org | `redpumpkin-ai` dan `redpumpkin` **sudah ada** | **Pastikan `redpumpkin-ai` memang milik kita** |
| Pencarian web | Tidak ditemukan dev tool bernama "labu" | Ada CLI bernama `lab` (GitLab); risiko salah ketik kecil |

**Masih harus dikerjakan manusia:**
- Amankan nama dengan merilis versi awal yang beneran (bukan placeholder kosong) di crates.io; PyPI/npm kalau memang akan dipakai.
- Cek merek dagang (mis. database PDKI untuk Indonesia) bersama legal. Ini belum dan tidak bisa dicek dari sini.
- Putuskan lisensi. Konvensi ekosistem Rust: **dual `MIT OR Apache-2.0`**; Apache-2.0 memuat klausul paten eksplisit. Ini informasi umum, bukan nasihat hukum.
- Tambah `CONTRIBUTING.md` + DCO sebelum menerima kontribusi luar.

### 5.7 Risiko baru: Bedrock lebih rumit dari API biasa

*Tidak berlaku untuk v0.1: provider pertama adalah OpenAI-compatible lewat Bifrost. Bagian ini berlaku saat Bedrock langsung dikerjakan sebagai adapter kedua (paling cepat v0.2). Spike awal sudah ada di `spikes/bedrock/`, hasilnya dicatat di ADR 0001.*

| Mekanisme | Bukti / gate |
|---|---|
| **Spike 2–3 hari di minggu 1** khusus SigV4 (proxy harus menandatangani ulang karena host berubah) dan framing event-stream, sebelum komit ke v0.1 | Demo: satu call ConverseStream terekam dan ter-replay |
| **Fallback:** kalau spike gagal, v0.1 pakai API Anthropic / OpenAI-compatible dulu; Bedrock geser ke v0.2 | Keputusan di akhir minggu 1 |
| Saat replay, tanda tangan tidak diverifikasi, jadi CI tidak butuh kredensial AWS | Tes replay jalan tanpa env AWS |

---

## 6. Yang diparkir (bukan dibuang)

- **Benchmark agent dokumen Indonesia** (three-way match dengan rulebook perusahaan fiktif; track pajak menunggu co-author ahli pajak). Bisa jadi *showcase* `labu` nanti.
- **Track injection di dokumen.** Cocok sebagai paket fault tambahan di `labu chaos`.

---

## 7. Pertanyaan terbuka

1. Agent yang sekarang kita bangun pakai apa: bahasa, framework, provider LLM? (Menentukan v0.1.)
2. Pakai MCP sekarang, atau tool biasa di dalam kode?
3. Level Rust: nyaman atau belajar sambil jalan?
4. Waktu per minggu?

---

## 8. Peta yang sudah ada (hasil research)

| Kategori | Contoh yang sudah ada |
|---|---|
| Record/replay | cassetteai, agent-replay, llm-rewind, reel-vcr, agentape, promptecho |
| Chaos / fault injection | AgentBreak, Ruptor, Khaos, pytest-agent-chaos, mcp-chaos, fault-mcp, AgentChaos |
| Verifikasi klaim selesai | agent-claimcheck, agent-completion-verifier, veritas, horkos, axiom (topik GitHub `agent-verification`: 43 repo) |
| Analisis trace skala besar | Docent (Transluce) |
| Framework agent Rust | Rig, rs-agent, ADK-Rust, agentkit, yoagent, Rustant |
| MCP gateway Rust | Vellaveto, arbit, Kvlar, unified-mcp-gateway |
| Document firewall | Prompt Warden, pdf-injection-scanner, hidden-text-detector, DocFirewall |

**Riset yang jadi dasar fitur:**
- False success: 22,8% baseline → 0,8% dengan kontrak bukti terstruktur (FTA, arXiv 2609.35732) → dasar `labu verify`.
- Agentic abstention: agent terbaik 59,5% paired accuracy (AgentAbstain, arXiv 2607.10059).
- Root-cause kegagalan skala besar & cost masih jadi gap (survey arXiv 2503.16416).
- Harness sama pentingnya dengan model (arXiv 2606.20683) → dasar studi v0.5.

**Link:**
- https://arxiv.org/abs/2609.35732v1 · https://github.com/AntiQuality/agentabstain · https://arxiv.org/html/2503.16416v2 · https://arxiv.org/abs/2606.20683
- https://pypi.org/project/llm-rewind/ · https://pypi.org/project/agentbreak/ · https://github.com/shyon123/map-chaos · https://github.com/B0yko/agent-claimcheck · https://docs.transluce.org/analysis/docent-agent

---

## 9. Gap analysis: 28 project terkait (16 Nov 2026)

**Metode:** README dan metadata dari 28 project (20 repo GitHub + 8 paket PyPI) diunduh dan dibandingkan per kemampuan. Ini analisis berbasis README, **bukan audit kode**, jadi "tidak ada" berarti "tidak disebut di README".

**Yang diperiksa:** llm-rewind, cassetteai, reel-vcr, promptecho, agent-replay, agentape, agent-vcr, vcr-langchain, codesweep-ai/vcr, mcp-recorder, mcp-tape, Drifter-MCP, AgentBreak, pytest-agent-chaos, Khaos, AgentChaos, agentic-chaos (2 project berbeda), mcp-chaos, fault-mcp, chaosgraph, ace-monorepo, agent-claimcheck, agent-completion-verifier, veritas, horkos, axiom.

### Temuan besar

1. **Belum ada pemenang.** Bintang tertinggi di seluruh kategori: agentic-chaos (83) dan vcr-langchain (81). Sisanya di bawah 15, kebanyakan 0–4.
2. **Tidak ada yang Rust** di 28 sampel (ada 1 repo Rust kecil di luar sampel, 1 bintang). Hanya 1 yang single binary (Go).
3. **Semuanya spesialis.** Tidak ada satu pun yang menggabungkan record/replay LLM + MCP + chaos + verifikasi klaim + statistik.
4. Menurut README llm-rewind, Docker cagent juga sudah punya record/replay bawaan. Artinya replay saja sudah jadi *table stakes*.

### Yang mereka tidak punya → fitur `labu`

| # | Celah | Bukti dari 28 project | Fitur `labu` |
|---|---|---|---|
| 1 | **AWS Bedrock** | Hanya 1 yang menyebut Bedrock, dan statusnya *tidak didukung* (promptecho) | Dukungan kelas satu: Converse, ConverseStream, InvokeModel, termasuk SigV4 dan event-stream. Relevan langsung untuk produk RedPumpkin di Bedrock |
| 2 | **Tanpa CA cert / MITM** | llm-rewind butuh mitmproxy + trust CA cert | Mode `base_url` sebagai default; satu binary, tanpa runtime |
| 3 | **Oracle hasil yang lemah** | llm-rewind mengaku oracle-nya cuma kesamaan stdout; agent-claimcheck tidak menjalankan agent sama sekali | `chaos` × `verify`: suntik fault, lalu cek klaim vs trace + state probe |
| 4 | **Statistik yang benar** | 3/28 menyebut interval kepercayaan atau uji statistik | Interval kepercayaan di pass rate, pass^k, `compare` dua run |
| 5 | **Biaya akurat** | 0/28 menghitung token prompt cache; 1/28 menangani blok reasoning | Cost dengan cache read/write + reasoning token |
| 6 | **Privasi untuk industri teregulasi** | 0/28 punya enkripsi cassette; redaction body lemah (agentape sendiri bilang tidak bisa mendeteksi PII di payload) | Enkripsi cassette + redaction body dengan pola Indonesia (NIK, NPWP, no. HP, no. rekening) |
| 7 | **Bahasa selain Inggris** | agent-claimcheck: benchmark Inggris; non-Inggris masih rencana | Rule pack `verify` Indonesia + Inggris ("sudah saya kirim", "berhasil dibuat", dst.) |
| 8 | **Impor trace produksi** | Masih roadmap di agent-claimcheck; adapter OTel agent-vcr belum rilis | `labu import` dari OTel / Langfuse, lalu `verify` + `stats` offline |
| 9 | **Tes migrasi model** | Tidak ditemukan di tool proxy LLM (Drifter-MCP punya versi sisi MCP) | `labu swap`: dunia tool dibekukan, model diganti dan jalan live |
| 10 | **Fidelitas waktu** | Hanya reel-vcr | Opsional: replay dengan jeda antar-chunk asli |
| 11 | **Tool call paralel** | Tidak terdokumentasi jelas di mana pun (belum terverifikasi) | Matching sadar-urutan untuk call paralel/sub-agent, dites khusus |

### Yang sudah mereka kerjakan dengan baik (jangan ditiru dulu)

- **Bisect + penjelasan sebab-akibat + mutation testing + leaderboard fragility**: llm-rewind.
- **Mutasi skema MCP & regresi server**: Drifter-MCP.
- **Chaos keamanan (rug pull, tool poisoning, A2A)**: RicoKomenda/agentic-chaos, AgentBreak.
- **Classifier terkalibrasi untuk false success**: agent-claimcheck.
- **Share link / hosted replay**: mcp-tape.

Kalau nanti dibutuhkan, lebih baik `labu` bisa **ekspor/impor format mereka** daripada menulis ulang.

### Tiga pembeda utama (urutan prioritas)

1. **Bedrock kelas satu** (kebutuhan sendiri + hampir kosong).
2. **`chaos` × `verify` dalam satu alur** (sinergi yang tidak dimiliki tool terpisah).
3. **Siap untuk klien teregulasi di Indonesia**: enkripsi, redaction PII lokal, rule pack bahasa Indonesia.
