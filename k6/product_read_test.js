import http from 'k6/http';
import { check } from 'k6';

// Read-heavy load against product master data, which is what the cache exists to
// serve. Run it twice - once with REDIS_URL set, once without - and compare p95.
// A cache rarely wins on a single idle request; it earns its keep under load, by
// keeping traffic off the database.
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export const options = {
    scenarios: {
        reads: {
            executor: 'constant-vus',
            vus: 50,
            duration: '20s',
        },
    },
    thresholds: {
        http_req_failed: ['rate<0.01'],
    },
};

export function setup() {
    const res = http.post(
        `${BASE_URL}/api/auth/login`,
        JSON.stringify({ email: 'admin@wms.test', password: 'password123' }),
        { headers: { 'Content-Type': 'application/json' } },
    );
    return { token: res.json('token') };
}

export default function (data) {
    const headers = { headers: { Authorization: `Bearer ${data.token}` } };

    const one = http.get(`${BASE_URL}/api/products/1`, headers);
    check(one, { 'get product 200': (r) => r.status === 200 });

    const list = http.get(`${BASE_URL}/api/products`, headers);
    check(list, { 'list products 200': (r) => r.status === 200 });
}
