ALTER TABLE books
ALTER COLUMN loan_date TYPE DATE USING loan_date::DATE,
ALTER COLUMN expected_return_date TYPE DATE USING expected_return_date::DATE;
