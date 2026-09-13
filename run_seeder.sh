#!/usr/bin/env bash
set -e

echo "Running database seeders..."
cargo run -p legacy-monolith --bin legacy-monolith-seed
echo "Seeding complete."

set -a
source .env
set +a

echo ""
echo "Verifying seeded data..."
psql "$DATABASE_URL" \
    -c "SELECT id, name, email FROM public.users ORDER BY id;" \
    -c "SELECT id, name FROM public.roles ORDER BY id;" \
    -c "SELECT id, user_id, role_id FROM public.user_roles ORDER BY id;"
