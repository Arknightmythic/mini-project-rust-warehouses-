import http from 'k6/http';
import { check } from 'k6';

// 200 concurrent receipts of quantity 1 against the SAME (warehouse, product).
// If the balance update were a SELECT followed by an UPDATE from Rust, increments
// would be silently lost here and the final number would come out below 200.
// The single ON CONFLICT DO UPDATE statement is what makes this safe.
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const VUS = 20;
const ITERATIONS = 10;
const TOTAL = VUS * ITERATIONS;
const PRODUCT_ID = 1;

export const options = {
    scenarios: {
        race: {
            executor: 'per-vu-iterations',
            vus: VUS,
            iterations: ITERATIONS,
            maxDuration: '90s',
        },
    },
    thresholds: {
        http_req_failed: ['rate<0.01'],
        checks: ['rate==1.0'],
    },
};

function authHeaders(token) {
    return { headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' } };
}

export function setup() {
    const loginRes = http.post(
        `${BASE_URL}/api/auth/login`,
        JSON.stringify({ email: 'admin@wms.test', password: 'password123' }),
        { headers: { 'Content-Type': 'application/json' } },
    );
    const token = loginRes.json('token');

    const warehouseRes = http.post(
        `${BASE_URL}/api/warehouses`,
        JSON.stringify({ name: `Race WH ${Date.now()}`, address: 'Jl. Race No. 1' }),
        authHeaders(token),
    );
    const warehouseId = warehouseRes.json('id');

    return { token, warehouseId };
}

export default function (data) {
    const res = http.post(
        `${BASE_URL}/api/inventory/receipts`,
        JSON.stringify({
            warehouse_id: data.warehouseId,
            idempotency_key: `race-${data.warehouseId}-${__VU}-${__ITER}`,
            items: [{ product_id: PRODUCT_ID, quantity: 1 }],
        }),
        authHeaders(data.token),
    );
    check(res, { 'receipt accepted': (r) => r.status === 200 });
}

export function teardown(data) {
    const res = http.get(
        `${BASE_URL}/api/inventory/balances/${data.warehouseId}/${PRODUCT_ID}`,
        authHeaders(data.token),
    );

    const onHand = res.json('qty_on_hand');
    console.log(`final qty_on_hand = ${onHand}, expected = ${TOTAL}`);

    check(res, {
        'no increment was lost under concurrency': () => onHand === TOTAL,
    });
}
