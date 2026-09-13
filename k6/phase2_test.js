import http from 'k6/http';
import { check, sleep } from 'k6';

// Gateway fronts users/roles over gRPC; warehouses still live in the monolith
// until Phase 3. Two base URLs is what a real strangler-fig migration looks like.
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const LEGACY_URL = __ENV.LEGACY_URL || 'http://localhost:8081';

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
    check(health, { 'gateway health 200': (r) => r.status === 200 });

    const uniqueEmail = `k6_${__VU}_${__ITER}_${Date.now()}@test.local`;
    const registerRes = http.post(
        `${BASE_URL}/api/auth/register`,
        JSON.stringify({ name: 'K6 Tester', email: uniqueEmail, password: 'password123' }),
        { headers: { 'Content-Type': 'application/json' } },
    );
    check(registerRes, { 'register via gRPC 200': (r) => r.status === 200 });

    const loginRes = http.post(
        `${BASE_URL}/api/auth/login`,
        JSON.stringify({ email: 'admin@wms.test', password: 'password123' }),
        { headers: { 'Content-Type': 'application/json' } },
    );
    check(loginRes, {
        'login via gRPC 200': (r) => r.status === 200,
        'login has token': (r) => !!r.json('token'),
        'login never leaks password': (r) => r.body.indexOf('password') === -1,
    });

    const token = loginRes.json('token');

    const unauthorizedRes = http.get(`${BASE_URL}/api/users`, {
        responseCallback: http.expectedStatuses(401),
    });
    check(unauthorizedRes, { 'unauthenticated users list 401': (r) => r.status === 401 });

    const usersRes = http.get(`${BASE_URL}/api/users`, authHeaders(token));
    check(usersRes, { 'list users via gRPC 200': (r) => r.status === 200 });

    const rolesRes = http.get(`${BASE_URL}/api/roles`, authHeaders(token));
    check(rolesRes, { 'list roles via gRPC 200': (r) => r.status === 200 });

    // A token minted by user-service, accepted by a different process entirely.
    const warehouseName = `K6 Warehouse ${__VU}-${__ITER}-${Date.now()}`;
    const createWarehouseRes = http.post(
        `${LEGACY_URL}/api/warehouses`,
        JSON.stringify({ name: warehouseName, address: 'Jl. K6 Testing No. 1' }),
        authHeaders(token),
    );
    check(createWarehouseRes, {
        'cross-service token accepted': (r) => r.status === 200,
    });

    const warehouseId = createWarehouseRes.json('id');

    const getWarehouseRes = http.get(`${LEGACY_URL}/api/warehouses/${warehouseId}`, authHeaders(token));
    check(getWarehouseRes, { 'get warehouse 200': (r) => r.status === 200 });

    const deleteWarehouseRes = http.del(`${LEGACY_URL}/api/warehouses/${warehouseId}`, null, authHeaders(token));
    check(deleteWarehouseRes, { 'delete warehouse 200': (r) => r.status === 200 });

    sleep(1);
}
