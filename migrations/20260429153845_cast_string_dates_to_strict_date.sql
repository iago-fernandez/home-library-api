-- Up Migration: Convert VARCHAR dates to strict DATE types safely
ALTER TABLE books
ALTER COLUMN publish_date TYPE DATE USING (NULLIF(publish_date, '')::DATE),
    ALTER COLUMN original_publish_date TYPE DATE USING (NULLIF(original_publish_date, '')::DATE),
    ALTER COLUMN purchase_date TYPE DATE USING (NULLIF(purchase_date, '')::DATE),
    ALTER COLUMN date_started TYPE DATE USING (NULLIF(date_started, '')::DATE),
    ALTER COLUMN date_finished TYPE DATE USING (NULLIF(date_finished, '')::DATE);