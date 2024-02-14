CREATE TABLE IF NOT EXISTS "project"
(
    "id" UUID PRIMARY KEY NOT NULL,
    "name" VARCHAR(256) NOT NULL
);

CREATE TABLE IF NOT EXISTS "unit"
(
    "id" UUID PRIMARY KEY NOT NULL,
    "project_id" UUID NOT NULL,
    "title" VARCHAR(256) NOT NULL,
    FOREIGN KEY("project_id") REFERENCES "project"("id")
);

CREATE TABLE IF NOT EXISTS "source"
(
    "unit_id" UUID NOT NULL,
    "sq" INTEGER NOT NULL,
    "content" VARCHAR NOT NULL,
    "meta" VARCHAR NOT NULL,
    FOREIGN KEY("unit_id") REFERENCES "unit"("id"),
    PRIMARY KEY("unit_id", "sq")
);

CREATE TABLE IF NOT EXISTS "commit"
(
    "id" UUID PRIMARY KEY NOT NULL,
    "unit_id" UUID NOT NULL,
    "created_at" TIMESTAMP NOT NULL,
    FOREIGN KEY("unit_id") REFERENCES "unit"("id")
);

CREATE TABLE IF NOT EXISTS "record"
(
    "commit_id" UUID NOT NULL,
    "sq" INTEGER NOT NULL,
    "content" VARCHAR NOT NULL,
    FOREIGN KEY("commit_id") REFERENCES "commit"("id"),
    PRIMARY KEY("commit_id", "sq")
);
