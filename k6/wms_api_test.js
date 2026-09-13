import http from 'k6/http';
import { check, sleep } from 'k6';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

export const options = {
    vus: 5,
    duration: '15s',
    thresholds: {
        http_req_failed: ['rate<0.01'],
        http_req_duration: ['p(95)<500'],
    },
};

function authHeaders(token) {
    return { headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' } };
}

export default function () {
    const health = http.get(`${BASE_URL}/health`);
    check(health, { 'health status 200': (r) => r.status === 200 });

    const uniqueEmail = `k6_${__VU}_${__ITER}_${Date.now()}@test.local`;
    const registerRes = http.post(
        `${BASE_URL}/api/auth/register`,
        JSON.stringify({ name: 'K6 Tester', email: uniqueEmail, password: 'password123' }),
        { headers: { 'Content-Type': 'application/json' } },
    );
    check(registerRes, { 'register status 200': (r) => r.status === 200 });

    const loginRes = http.post(
        `${BASE_URL}/api/auth/login`,
        JSON.stringify({ email: 'admin@wms.test', password: 'password123' }),
        { headers: { 'Content-Type': 'application/json' } },
    );
    check(loginRes, {
        'login status 200': (r) => r.status === 200,
        'login has token': (r) => !!r.json('token'),
    });

    const token = loginRes.json('token');

    const unauthorizedRes = http.get(`${BASE_URL}/api/users`, {
        responseCallback: http.expectedStatuses(401),
    });
    check(unauthorizedRes, { 'unauthenticated users list returns 401': (r) => r.status === 401 });

    const usersRes = http.get(`${BASE_URL}/api/users`, authHeaders(token));
    check(usersRes, { 'list users status 200': (r) => r.status === 200 });

    const warehouseName = `K6 Warehouse ${__VU}-${__ITER}-${Date.now()}`;
    const createWarehouseRes = http.post(
        `${BASE_URL}/api/warehouses`,
        JSON.stringify({ name: warehouseName, address: 'Jl. K6 Testing No. 1' }),
        authHeaders(token),
    );
    check(createWarehouseRes, { 'create warehouse status 200': (r) => r.status === 200 });

    const warehouseId = createWarehouseRes.json('id');

    const getWarehouseRes = http.get(`${BASE_URL}/api/warehouses/${warehouseId}`, authHeaders(token));
    check(getWarehouseRes, { 'get warehouse status 200': (r) => r.status === 200 });

    const deleteWarehouseRes = http.del(`${BASE_URL}/api/warehouses/${warehouseId}`, null, authHeaders(token));
    check(deleteWarehouseRes, { 'delete warehouse status 200': (r) => r.status === 200 });

    const categoriesRes = http.get(`${BASE_URL}/api/categories`, authHeaders(token));
    check(categoriesRes, { 'list categories status 200': (r) => r.status === 200 });

    const listProductsRes = http.get(`${BASE_URL}/api/products`, authHeaders(token));
    check(listProductsRes, {
        'list products status 200': (r) => r.status === 200,
        'seeded products present': (r) => r.json().length >= 4,
    });

    const sku = `SKU-K6-${__VU}-${__ITER}-${Date.now()}`;
    const createProductRes = http.post(
        `${BASE_URL}/api/products`,
        JSON.stringify({ sku: sku, name: 'K6 Product', unit: 'pcs' }),
        authHeaders(token),
    );
    check(createProductRes, { 'create product status 200': (r) => r.status === 200 });

    const productId = createProductRes.json('id');

    const getProductRes = http.get(`${BASE_URL}/api/products/${productId}`, authHeaders(token));
    check(getProductRes, {
        'get product status 200': (r) => r.status === 200,
        'product starts active': (r) => r.json('is_active') === true,
    });

    // Master data is deactivated, never deleted: other services hold this id and
    // there is no foreign key across the boundary to protect them.
    const deactivateRes = http.del(`${BASE_URL}/api/products/${productId}`, null, authHeaders(token));
    check(deactivateRes, { 'deactivate product status 200': (r) => r.status === 200 });

    const afterDeactivateRes = http.get(`${BASE_URL}/api/products/${productId}`, authHeaders(token));
    check(afterDeactivateRes, {
        'deactivated product still readable': (r) => r.status === 200,
        'deactivated product is inactive': (r) => r.json('is_active') === false,
    });

    sleep(1);
}
