-- Seed default roles
INSERT INTO public.roles (name) VALUES
    ('admin'),
    ('warehouse_manager'),
    ('staff')
ON CONFLICT (name) DO NOTHING;
