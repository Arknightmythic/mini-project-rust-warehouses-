-- Link seeded users to their roles
INSERT INTO public.user_roles (user_id, role_id)
SELECT u.id, r.id
FROM public.users u
JOIN public.roles r ON (
    (u.email = 'admin@wms.test' AND r.name = 'admin') OR
    (u.email = 'manager@wms.test' AND r.name = 'warehouse_manager') OR
    (u.email = 'staff1@wms.test' AND r.name = 'staff')
)
ON CONFLICT (user_id, role_id) DO NOTHING;
