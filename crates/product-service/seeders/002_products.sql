INSERT INTO public.products (sku, name, description, unit, category_id)
SELECT v.sku, v.name, v.description, v.unit, c.id
FROM (VALUES
    ('SKU-BYM-001', 'Bayam Hijau',  'Bayam segar per ikat',    'ikat', 'Sayuran'),
    ('SKU-WRT-001', 'Wortel',       'Wortel grade A',          'kg',   'Sayuran'),
    ('SKU-APL-001', 'Apel Fuji',    'Apel impor',              'kg',   'Buah'),
    ('SKU-BRS-001', 'Beras Premium','Beras pulen kemasan 5kg', 'sak',  'Bahan Pokok')
) AS v(sku, name, description, unit, category_name)
JOIN public.categories c ON c.name = v.category_name
ON CONFLICT (sku) DO NOTHING;
