// @generated automatically by Diesel CLI.

diesel::table! {
    commit (id) {
        id -> Uuid,
        unit_id -> Uuid,
        created_at -> Timestamp,
    }
}

diesel::table! {
    project (id) {
        id -> Uuid,
        #[max_length = 256]
        name -> Varchar,
    }
}

diesel::table! {
    record (commit_id, sq) {
        commit_id -> Uuid,
        sq -> Int4,
        content -> Varchar,
    }
}

diesel::table! {
    source (unit_id, sq) {
        unit_id -> Uuid,
        sq -> Int4,
        content -> Varchar,
        meta -> Varchar,
    }
}

diesel::table! {
    unit (id) {
        id -> Uuid,
        project_id -> Uuid,
        #[max_length = 256]
        title -> Varchar,
    }
}

diesel::joinable!(commit -> unit (unit_id));
diesel::joinable!(record -> commit (commit_id));
diesel::joinable!(source -> unit (unit_id));
diesel::joinable!(unit -> project (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    commit,
    project,
    record,
    source,
    unit,
);
