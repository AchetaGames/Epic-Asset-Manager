create table unreal_project_latest_engine
(
    project TEXT,
    engine TEXT,
    constraint unreal_project_latest_engine_pk
        unique (project)
);

create index unreal_project_latest_engine_project_index
    on unreal_project_latest_engine (project);