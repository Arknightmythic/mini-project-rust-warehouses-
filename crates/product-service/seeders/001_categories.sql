INSERT INTO public.categories (name) VALUES
    ('Sayuran'),
    ('Buah'),
    ('Bahan Pokok')
ON CONFLICT (name) DO NOTHING;
