# Mini Warehouse WMS

Mini project REST API sederhana untuk manajemen gudang (Warehouse Management System), dibuat sebagai bahan latihan mendalami bahasa Rust menggunakan [Axum](https://github.com/tokio-rs/axum) dan [SQLx](https://github.com/launchbadge/sqlx) dengan PostgreSQL.

## Fitur

- Autentikasi berbasis JWT (register & login)
- Otorisasi berbasis role (`admin`, `warehouse_manager`, `staff`)
- CRUD Users, Roles, dan Warehouses
- Assign / remove role ke user
- Soft delete untuk warehouse
- Migrasi database otomatis (embedded di binary, jalan otomatis saat aplikasi start)
- Seeder data awal (roles, users, user_roles)
- Docker & Docker Compose untuk deployment
- Load/API test menggunakan [k6](https://k6.io/)

## Tech Stack

| Layer | Teknologi |
|---|---|
| Web framework | Axum |
| Database | PostgreSQL + SQLx (async, tanpa ORM) |
| Auth | JWT (`jsonwebtoken`) + bcrypt |
| Async runtime | Tokio |
| Error handling | `thiserror` + `anyhow` |
| Container | Docker / Docker Compose |
| Load testing | k6 |

## Struktur Project

```
src/
  configs/        koneksi database & konfigurasi dari environment variable
  models/         struct request/response & representasi row tabel
  repositories/   query SQL ke database (CRUD murni)
  middlewares/    ekstraktor autentikasi (validasi JWT)
  routes/         handler HTTP tiap endpoint
  utils/          error handling, JWT, hashing password
  bin/seed.rs     binary terpisah untuk seeding data awal
  main.rs         entry point aplikasi
migrations/       file migrasi SQL (naik/turun)
seeders/          file SQL data awal (roles, users, user_roles)
k6/               script load test
```

Lihat `PENJELASAN_KODE.md` (tidak ikut di-commit) untuk penjelasan detail tiap file — dibuat khusus untuk catatan belajar pribadi.

## Prasyarat

- [Rust](https://www.rust-lang.org/tools/install) (edisi 2024, gunakan versi stable terbaru)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (untuk menjalankan via container / load test k6)
- PostgreSQL (jika ingin jalan tanpa Docker) — bisa pakai [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) untuk migrasi manual: `cargo install sqlx-cli --no-default-features --features postgres`

## Instalasi & Menjalankan

### 1. Clone & siapkan environment variable

```bash
cp .env.example .env
```

Sesuaikan isi `.env` sesuai kebutuhan (URL database, JWT secret, dsb). **Jangan commit file `.env`** — sudah otomatis di-ignore lewat `.gitignore`.

### 2A. Menjalankan lewat Docker (disarankan)

```bash
# Jalankan database + aplikasi
docker compose up -d postgres app

# Isi data awal (roles, users, user_roles)
docker compose --profile tools run --rm seed

# Cek aplikasi hidup
curl http://localhost:8080/health
```

Migrasi database berjalan otomatis saat aplikasi start (embedded di binary lewat `sqlx::migrate!()`), jadi tidak perlu langkah migrasi terpisah.

Untuk mematikan:

```bash
docker compose down
```

Data PostgreSQL tetap tersimpan di Docker volume `postgres_data` walau container dimatikan. Untuk menghapus data sekalian: `docker compose down -v`.

### 2B. Menjalankan secara lokal (tanpa Docker)

Pastikan PostgreSQL sudah jalan dan `DATABASE_URL` di `.env` sudah sesuai.

```bash
# Jalankan aplikasi (migrasi otomatis jalan saat startup)
cargo run

# Di terminal lain, isi data awal
./run_seeder.sh
```

### 3. Coba API

User hasil seeder (password default: `password123`):

| Email | Role |
|---|---|
| admin@wms.test | admin |
| manager@wms.test | warehouse_manager |
| staff1@wms.test | staff |

```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@wms.test","password":"password123"}'
```

Pakai token yang didapat sebagai header `Authorization: Bearer <token>` untuk mengakses endpoint terproteksi.

## Endpoint API

| Method | Endpoint | Auth | Keterangan |
|---|---|---|---|
| GET | `/health` | - | Health check |
| POST | `/api/auth/register` | - | Registrasi user baru |
| POST | `/api/auth/login` | - | Login, mengembalikan JWT |
| GET | `/api/users` | ✓ | List semua user |
| GET | `/api/users/{id}` | ✓ | Detail user |
| PUT | `/api/users/{id}` | ✓ (diri sendiri / admin) | Update user |
| DELETE | `/api/users/{id}` | ✓ (admin) | Hapus user |
| GET | `/api/users/{id}/roles` | ✓ | List role milik user |
| POST | `/api/users/{id}/roles` | ✓ (admin) | Assign role ke user |
| DELETE | `/api/users/{id}/roles/{role_id}` | ✓ (admin) | Cabut role dari user |
| GET | `/api/roles` | ✓ | List role |
| POST | `/api/roles` | ✓ (admin) | Buat role |
| PUT | `/api/roles/{id}` | ✓ (admin) | Update role |
| DELETE | `/api/roles/{id}` | ✓ (admin) | Hapus role |
| GET | `/api/warehouses` | ✓ | List warehouse |
| POST | `/api/warehouses` | ✓ (admin / warehouse_manager) | Buat warehouse |
| PUT | `/api/warehouses/{id}` | ✓ (admin / warehouse_manager) | Update warehouse |
| DELETE | `/api/warehouses/{id}` | ✓ (admin) | Soft delete warehouse |

## Load Testing (k6)

k6 dijalankan lewat Docker image resmi (`grafana/k6`), jadi tidak perlu install k6 secara lokal.

```bash
# pastikan stack sudah jalan lewat docker compose (postgres + app)
docker compose up -d postgres app

./run_k6.sh
```

Script test ada di `k6/wms_api_test.js` — mencakup health check, register, login, akses tanpa token (harus ditolak), serta CRUD warehouse.

## Catatan

Dependency `reqwest`, `url`, `urlencoding`, dan `uuid` sudah disiapkan di `Cargo.toml` untuk pengembangan fitur selanjutnya (mis. upload foto ke Supabase Storage) — belum dipakai di kode saat ini.
