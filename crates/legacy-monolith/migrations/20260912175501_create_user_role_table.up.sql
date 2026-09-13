-- Add up migration script here
CREATE TABLE IF NOT EXISTS public.user_roles (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

alter table public.user_roles
add constraint fk_user_role_user
FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE CASCADE;

alter table public.user_roles
add constraint fk_user_role_role
FOREIGN KEY (role_id) REFERENCES public.roles (id) ON DELETE CASCADE;

alter table public.user_roles
add constraint uni_user_role_user_id_role_id UNIQUE (user_id, role_id);

Create index if not exists idx_user_roles_user_id on public.user_roles (user_id);
Create index if not exists idx_user_roles_role_id on public.user_roles (role_id); 
create index if not exists idx_user_roles_created_at on public.user_roles (created_at);