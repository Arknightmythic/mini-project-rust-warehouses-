-- Seed default users
-- Default password for all seeded users is: password123
INSERT INTO public.users (name, email, password) VALUES
    ('Admin', 'admin@wms.test', '$2b$10$Bp1xh73angGWpyoGHJ7xbO.CpYJY/Fu5dGTih8dZO109dTBNzn3kG'),
    ('Warehouse Manager', 'manager@wms.test', '$2b$10$Bp1xh73angGWpyoGHJ7xbO.CpYJY/Fu5dGTih8dZO109dTBNzn3kG'),
    ('Staff One', 'staff1@wms.test', '$2b$10$Bp1xh73angGWpyoGHJ7xbO.CpYJY/Fu5dGTih8dZO109dTBNzn3kG')
ON CONFLICT (email) DO NOTHING;
