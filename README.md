# Mini Warehouse WMS — v2 (Microservices)

Warehouse Management System sederhana yang dipecah menjadi **enam service Rust**,
dibuat sebagai latihan mendalami Rust sekaligus arsitektur microservice yang
benar-benar berjalan: gRPC untuk komunikasi sinkron, RabbitMQ untuk event,
database terpisah per service, dan distributed tracing lintas semuanya.

> **Branch `v1-monolith`** berisi versi monolith dari sistem yang sama (satu proses
> Axum, satu Postgres) dan sengaja dijaga tetap bisa dijalankan sebagai pembanding.

## Arsitektur

```
                    ┌──────────────┐
   HTTP :8080  ───▶ │ api-gateway  │  verifikasi JWT, komposisi response
                    └──────┬───────┘
                           │ gRPC
       ┌───────────────┬───┴────────┬────────────────┐
       ▼               ▼            ▼                ▼
 user-service   warehouse-svc  product-svc   inventory-service
   :50051          :50052        :50053          :50054
       │               │            │                │
   postgres        postgres     postgres         postgres
    :5432           :5437        :5433            :5434
                       │            │                │
                     redis        redis              │ publish
                                    ▲                ▼
                                    │          ┌──────────┐
                                    └──────────┤ RabbitMQ │
                                      consume  └────┬─────┘
                                                    │ consume
                                            notification-service
                                                    │
                                                postgres :5436
```

| Service | Port | Tabel yang dimiliki | Peran |
|---|---|---|---|
| **api-gateway** | HTTP 8080 | — (stateless) | Satu-satunya pintu HTTP. Verifikasi JWT, terjemahkan REST ↔ gRPC, komposisi data lintas service |
| **user-service** | gRPC 50051 | `users`, `roles`, `user_roles` | Satu-satunya penerbit JWT |
| **warehouse-service** | gRPC 50052 | `warehouses` | Master data gudang + cache Redis |
| **product-service** | gRPC 50053 | `products`, `categories` | Master data produk + cache + konsumen event (rollup stok) |
| **inventory-service** | gRPC 50054 | `stock_balances`, `stock_movements`, `inbound_receipts`, `outbound_shipments`, `outbox` | Inti transaksi: receiving, saga pengiriman, outbox |
| **notification-service** | — | `notifications`, `processed_events` | Murni konsumen event, tanpa HTTP maupun gRPC server |

Infrastruktur pendukung: **RabbitMQ** (`:5672`, UI `:15672`), **Redis** (`:6379`),
**Jaeger** (UI `:16686`, OTLP `:4317`).

## Fitur

- Autentikasi JWT + otorisasi berbasis role (`admin`, `warehouse_manager`, `staff`)
- CRUD Users, Roles, Warehouses, Products, Categories
- **Receiving stok** dengan idempotency key dan validasi lintas dua service
- **Saga** pengiriman: `reserve → confirm → compensate`, plus sweeper otomatis
  untuk reservasi menggantung
- **Transactional outbox** — event dan perubahan stok ditulis dalam satu transaksi
- **Read model** — `total_stock_cached` di product-service diperbarui lewat event
- **Cache Redis** dengan batas waktu keras, jadi Redis mati hanya memperlambat
- **Distributed tracing** W3C lintas HTTP, gRPC, dan AMQP
- DLQ + alternate exchange sejak awal, agar pesan tidak pernah hilang diam-diam
- Migrasi otomatis per service (embedded lewat `sqlx::migrate!`)

## Tech Stack

| Layer | Teknologi |
|---|---|
| HTTP (gateway) | Axum 0.8 |
| RPC antar service | tonic 0.14 + prost (protobuf) |
| Codegen protobuf | `protox` — compiler murni Rust, **tidak butuh `protoc`** |
| Database | PostgreSQL 17 + SQLx 0.9 (async, tanpa ORM) |
| Messaging | RabbitMQ 4 + lapin 4.11 |
| Cache | Redis 7 |
| Auth | JWT (`jsonwebtoken`, HS256) + bcrypt |
| Observability | OpenTelemetry 0.32 + Jaeger 2 |
| Async runtime | Tokio |
| Container | Docker / Docker Compose |
| Load testing | k6 |

## Struktur Project

Satu Cargo workspace, sembilan crate:

```
Cargo.toml              [workspace] — semua versi dipin di sini
Dockerfile              satu file untuk semua service (ARG SERVICE)
docker-compose.yml      14 container
crates/
  wms-core/             infrastruktur bersama — NOL domain
                        error, config, db, jwt (verify saja), identity,
                        telemetry, grpc (metadata + trace), cache
  wms-proto/            kontrak .proto + hasil codegen (di-commit)
  wms-events/           envelope event, topologi AMQP, publisher, consumer
  api-gateway/          routes/, dto/, middlewares/
  user-service/         models/ repositories/ service.rs migrations/ seeders/
  warehouse-service/    bentuk sama
  product-service/      bentuk sama + consumer.rs
  inventory-service/    bentuk sama + saga.rs, sweeper.rs, outbox.rs
  notification-service/ handler.rs (tanpa server sama sekali)
k6/                     script load test
```

Tiap service domain **mempertahankan layout internal v1 apa adanya**
(`models/`, `repositories/`, `migrations/`). Yang berubah hanya tepinya:
`routes/` menjadi `service.rs` berisi implementasi gRPC. Ini disengaja, supaya
pertanyaan *"apa sih yang sebenarnya berubah saat jadi microservice?"* bisa
dijawab lewat satu diff.

**Aturan keras untuk `wms-core`:** tidak boleh punya modul `models/` atau
`repositories/`, dan tidak boleh menyebut kata benda domain (product, warehouse,
stock, receipt). Kalau dua service butuh struct yang sama, struct itu milik
`.proto` atau payload event — bukan shared Rust. Crate `common-models` adalah
cara nomor satu monorepo microservice diam-diam berubah jadi distributed monolith.

Lihat `PENJELASAN_KODE_V2.md` (tidak ikut di-commit) untuk pembahasan detail tiap
file dan alasan di balik tiap keputusan — dibuat khusus sebagai catatan belajar.

## Prasyarat

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) — cara termudah menjalankan semuanya
- [Rust](https://www.rust-lang.org/tools/install) edisi 2024 (hanya jika ingin `cargo run` di host)
- **Tidak perlu** install `protoc` maupun `k6` — keduanya sudah ditangani (protox & Docker image)

## Menjalankan

### 1. Siapkan environment variable

```bash
cp .env.example .env
```

Untuk jalan lewat Docker, `.env` bahkan tidak wajib — semua variabel sudah
diisi di `docker-compose.yml`. **Jangan commit `.env`.**

### 2. Jalankan seluruh stack

```bash
docker compose up -d --build

# Isi data awal (roles, users, categories, products)
docker compose --profile tools run --rm user-seed
docker compose --profile tools run --rm product-seed

curl http://localhost:8080/health
```

Migrasi database jalan otomatis saat tiap service start, **masing-masing hanya
untuk schema miliknya sendiri**.

Mematikan: `docker compose down` (data tetap di volume) atau
`docker compose down -v` (hapus sekalian).

### 3. Mode pengembangan (disarankan saat ngoding)

Jangan develop di dalam Docker — rebuild image tiap kali ganti satu baris itu
menyiksa. Jalankan infrastrukturnya saja lewat Docker, lalu service-nya di host:

```bash
# infrastruktur saja
docker compose up -d postgres-user postgres-warehouse postgres-product \
                    postgres-inventory postgres-notification rabbitmq redis jaeger

# service yang sedang dikerjakan
cargo run -p user-service
cargo run -p api-gateway
```

Semua port database sudah di-publish ke localhost justru untuk ini. Selalu
`cargo build -p <satu-crate>`, jangan `cargo build` polos — sembilan crate sekaligus
itu lama.

### 4. Coba API

User hasil seeder (password: `password123`):

| Email | Role |
|---|---|
| admin@wms.test | admin |
| manager@wms.test | warehouse_manager |
| staff1@wms.test | staff |

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@wms.test","password":"password123"}' | jq -r .token)

# Terima stok — satu request HTTP yang memvalidasi ke dua service lain
curl -X POST http://localhost:8080/api/inventory/receipts \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"warehouse_id":1,"idempotency_key":"po-001",
       "items":[{"product_id":1,"quantity":50,"unit_cost":15000}]}'
```

## Endpoint API

| Method | Endpoint | Auth | Keterangan |
|---|---|---|---|
| GET | `/health` | - | Health check |
| POST | `/api/auth/register` | - | Registrasi user baru |
| POST | `/api/auth/login` | - | Login, mengembalikan JWT |
| GET | `/api/users` | ✓ | List user |
| GET/PUT/DELETE | `/api/users/{id}` | ✓ | Detail / update / hapus user |
| GET/POST | `/api/users/{id}/roles` | ✓ | List / assign role |
| DELETE | `/api/users/{id}/roles/{role_id}` | ✓ (admin) | Cabut role |
| GET/POST | `/api/roles` | ✓ | List / buat role |
| GET/PUT/DELETE | `/api/roles/{id}` | ✓ | Detail / update / hapus role |
| GET/POST | `/api/warehouses` | ✓ | List / buat warehouse |
| GET/PUT/DELETE | `/api/warehouses/{id}` | ✓ | Detail / update / soft delete |
| GET/POST | `/api/categories` | ✓ | List / buat kategori |
| GET/POST | `/api/products` | ✓ | List / buat produk |
| GET/PUT/DELETE | `/api/products/{id}` | ✓ | Detail / update / **nonaktifkan** |
| POST | `/api/inventory/receipts` | ✓ | Terima stok (butuh `idempotency_key`) |
| GET | `/api/inventory/balances` | ✓ | List saldo stok |
| GET | `/api/inventory/balances/{wh}/{product}` | ✓ | Saldo satu produk |
| GET | `/api/inventory/movements` | ✓ | Kartu stok (ledger) |
| GET | `/api/inventory/report` | ✓ | Laporan gabungan stok + nama produk |
| POST | `/api/inventory/shipments` | ✓ | **Saga langkah 1** — reservasi stok |
| GET | `/api/inventory/shipments/{id}` | ✓ | Detail shipment |
| POST | `/api/inventory/shipments/{id}/confirm` | ✓ | **Langkah 2** — stok benar-benar keluar |
| POST | `/api/inventory/shipments/{id}/cancel` | ✓ | **Langkah 3** — kompensasi |

Produk **dinonaktifkan, bukan dihapus**: service lain menyimpan id-nya dan tidak
ada foreign key lintas boundary yang melindungi mereka.

## Observability

| Alat | URL | Untuk apa |
|---|---|---|
| Jaeger | <http://localhost:16686> | Satu request HTTP tampil sebagai satu trace utuh lintas service, termasuk yang melewati RabbitMQ |
| RabbitMQ | <http://localhost:15672> (guest/guest) | Cek tab **Bindings** setiap kali pesan "hilang" — itu tersangka nomor satu |

Trace context mengalir lewat header `traceparent` (standar W3C): HTTP → metadata
gRPC → header AMQP → kembali ke span consumer.

## Load Testing (k6)

k6 jalan lewat Docker image resmi, tidak perlu install:

```bash
./run_k6.sh                             # suite lengkap
./run_k6.sh k6/inventory_race_test.js   # 20 VU rebutan produk yang sama
./run_k6.sh k6/product_read_test.js     # 50 VU baca-berat, menguji cache
```

- `wms_api_test.js` — 1485 check: auth, CRUD, idempotency, validasi lintas service
- `inventory_race_test.js` — membuktikan larangan *read-then-write* benar-benar
  ditegakkan: 20 VU × 10 request ke produk yang sama, saldo akhir harus tepat
- `product_read_test.js` — beban baca untuk menguji cache

## Mendemokan konsep-konsepnya

Bagian paling berguna untuk belajar. Semuanya bisa dijalankan sekarang:

```bash
# BLAST RADIUS — matikan satu service, lihat apa yang ikut mati
docker compose stop product-service
# receipts 503, tapi CRUD warehouse & user tetap sehat

# EVENTUAL CONSISTENCY — jendela waktunya benar-benar ada
docker compose stop notification-service
# buat 3 receipt, lalu hidupkan lagi: ketiganya tetap terproses (durable queue)

# TRUST BOUNDARY — panggil inventory langsung, mengaku admin tanpa token
cargo run -p inventory-service --example forge -- 1 1
# ditolak Unauthenticated sejak Phase 10; sebelum itu, ini berhasil

# DUAL-WRITE — bug yang sengaja dibiarkan sampai Phase 11
EVENT_DELIVERY=direct CRASH_AFTER_COMMIT=1 docker compose up -d inventory-service
# stok bertambah, event hilang selamanya
EVENT_DELIVERY=outbox CRASH_AFTER_COMMIT=1 docker compose up -d inventory-service
# stok bertambah, event selamat di tabel outbox, terkirim setelah restart
```

## Catatan

**Fidelity gap yang perlu disadari.** Satu workspace = satu `Cargo.lock` = semua
service memakai versi dependency yang sama. Microservice sungguhan tidak begitu.
Independent deployability tetap didapat (binary, image, dan database terpisah),
tapi independent dependency evolution hilang. Untuk belajar ini trade yang tepat —
cukup tahu bahwa ini celah, bukan kemenangan gratis.

**Pemisahan `warehouse-service` agak artifisial.** WMS sungguhan sering menggabung
warehouse + lokasi + stok justru karena ketiganya *transactionally coupled*.
Dipisah di sini supaya masalah konsistensi lintas service benar-benar muncul dan
bisa dipelajari — dan memang muncul: `ReceiveStock` harus memvalidasi ke dua
service sebelum boleh menyentuh database.

**Di luar scope.** Service discovery (Consul/etcd) tidak dipakai: DNS Docker
Compose sudah berfungsi sebagai service discovery di skala ini, dan menambah
Consul mengajarkan ops tooling, bukan Rust atau distributed systems.
