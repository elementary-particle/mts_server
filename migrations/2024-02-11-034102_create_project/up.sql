CREATE TABLE "project"
(
    "id" UUID PRIMARY KEY NOT NULL,
    "name" VARCHAR(256) NOT NULL
);

CREATE TABLE "unit"
(
    "id" UUID PRIMARY KEY NOT NULL,
    "project_id" UUID NOT NULL,
    "title" VARCHAR(256) NOT NULL,
    "commit_id" UUID,
    FOREIGN KEY("project_id") REFERENCES "project"("id")
);

CREATE TABLE "source"
(
    "unit_id" UUID NOT NULL,
    "sq" INTEGER NOT NULL,
    "content" VARCHAR NOT NULL,
    "meta" VARCHAR NOT NULL,
    FOREIGN KEY("unit_id") REFERENCES "unit"("id"),
    PRIMARY KEY("unit_id", "sq")
);

CREATE TABLE "commit"
(
    "id" UUID PRIMARY KEY NOT NULL,
    "unit_id" UUID NOT NULL,
    "created_at" TIMESTAMP NOT NULL,
    FOREIGN KEY("unit_id") REFERENCES "unit"("id")
);

CREATE TABLE "record"
(
    "commit_id" UUID NOT NULL,
    "sq" INTEGER NOT NULL,
    "content" VARCHAR NOT NULL,
    FOREIGN KEY("commit_id") REFERENCES "commit"("id"),
    PRIMARY KEY("commit_id", "sq")
);

CREATE FUNCTION update_latest_commit() RETURNS TRIGGER AS $on_insert_commit$
    BEGIN
        UPDATE "unit" SET "commit_id" = NEW."id" WHERE "id" = NEW."unit_id";
        RETURN NULL;
    END;
$on_insert_commit$ LANGUAGE plpgsql;

CREATE TRIGGER on_insert_commit AFTER INSERT
    ON "commit"
    FOR EACH ROW EXECUTE FUNCTION update_latest_commit();
