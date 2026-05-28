-- Drop existing constraints
-- Find the unique constraint for catalog_number
ALTER TABLE books DROP CONSTRAINT IF EXISTS books_catalog_number_key;

-- Drop the default sequence assigned by SERIAL
ALTER TABLE books ALTER COLUMN catalog_number DROP DEFAULT;

-- Add library-scoped unique constraint
ALTER TABLE books ADD CONSTRAINT books_library_id_catalog_number_key UNIQUE (library_id, catalog_number);

-- Re-number existing books partitioned by library
WITH numbered AS (
    SELECT id, row_number() over (partition by library_id order by created_at, catalog_number) as new_cat_num
    FROM books
)
UPDATE books SET catalog_number = numbered.new_cat_num
FROM numbered WHERE books.id = numbered.id;

-- Create trigger to auto-increment per library safely
CREATE OR REPLACE FUNCTION set_catalog_number() RETURNS TRIGGER AS $$
BEGIN
  -- Lock the specific library row to prevent race conditions during concurrent inserts
  PERFORM 1 FROM libraries WHERE id = NEW.library_id FOR UPDATE;
  
  -- Calculate next catalog number for this library
  IF NEW.catalog_number IS NULL THEN
      SELECT COALESCE(MAX(catalog_number), 0) + 1 INTO NEW.catalog_number 
      FROM books 
      WHERE library_id = NEW.library_id;
  END IF;
  
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_catalog_number
BEFORE INSERT ON books
FOR EACH ROW
EXECUTE FUNCTION set_catalog_number();
