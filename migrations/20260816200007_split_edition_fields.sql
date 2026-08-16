-- Rename existing text column to edition
ALTER TABLE books RENAME COLUMN edition_number TO edition;

-- Add the new integer column for edition numbers
ALTER TABLE books ADD COLUMN edition_number INTEGER;

-- Migrate purely numeric text from the old edition field into the new integer field
UPDATE books 
SET edition_number = CAST(edition AS INTEGER) 
WHERE edition ~ '^[0-9]+$';

-- Clear the string field for those records since it is now properly numeric
UPDATE books 
SET edition = NULL 
WHERE edition ~ '^[0-9]+$';
