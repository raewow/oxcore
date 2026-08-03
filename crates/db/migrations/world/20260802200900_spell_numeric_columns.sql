-- PostgreSQL migration: world / spell_numeric_columns
-- The source unsigned BIGINT columns fit PostgreSQL BIGINT in the base data and are
-- decoded by the runtime as checked u64 values. NUMERIC prevents SQLx from decoding them.

ALTER TABLE world.spell_template
    ALTER COLUMN "effectItemType1" TYPE BIGINT USING "effectItemType1"::BIGINT,
    ALTER COLUMN "effectItemType2" TYPE BIGINT USING "effectItemType2"::BIGINT,
    ALTER COLUMN "effectItemType3" TYPE BIGINT USING "effectItemType3"::BIGINT,
    ALTER COLUMN "spellFamilyFlags" TYPE BIGINT USING "spellFamilyFlags"::BIGINT;
